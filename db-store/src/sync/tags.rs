//! Tag phase: full-sync tag attachment and single-write tag replacement.

use std::collections::HashMap;

use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};

use crate::entity::{record_tags, tags};
use crate::Result;

use super::InsertedRecord;

/// Upserts each inserted record's tags by `(project_id, name)` and links
/// them via `record_tags`. Safe against the `UNIQUE(project_id, name)`
/// constraint because [`super::clear_project`] already emptied this
/// project's tags before either pass runs, so a name is inserted at most
/// once per run.
pub(super) async fn insert_tags<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    inserted: &[InsertedRecord],
) -> Result<()> {
    let mut tag_ids: HashMap<String, i32> = HashMap::new();

    for record in inserted {
        for name in &record.tags {
            let tag_id = ensure_tag(conn, project_id, &mut tag_ids, name).await?;
            record_tags::ActiveModel {
                record_id: ActiveValue::Set(record.id),
                tag_id: ActiveValue::Set(tag_id),
            }
            .insert(conn)
            .await?;
        }
    }

    Ok(())
}

async fn ensure_tag<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    cache: &mut HashMap<String, i32>,
    name: &str,
) -> Result<i32> {
    if let Some(&id) = cache.get(name) {
        return Ok(id);
    }

    let inserted = tags::ActiveModel {
        project_id: ActiveValue::Set(project_id),
        name: ActiveValue::Set(name.to_owned()),
        ..Default::default()
    }
    .insert(conn)
    .await?;

    cache.insert(name.to_owned(), inserted.id);
    Ok(inserted.id)
}

/// Replaces `record_id`'s tag links with `tag_names`, creating any tag
/// `project_id` does not already have. Looks tags up by name against the
/// database rather than assuming absence the way [`insert_tags`]'s cache
/// does during a from-empty sync run, so re-writing a record that reuses an
/// existing project tag does not violate `UNIQUE(project_id, name)`.
pub(super) async fn replace_write_tags<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    record_id: i32,
    tag_names: &[String],
) -> Result<()> {
    record_tags::Entity::delete_many()
        .filter(record_tags::Column::RecordId.eq(record_id))
        .exec(conn)
        .await?;

    for name in tag_names {
        let tag_id = find_or_create_tag(conn, project_id, name).await?;
        record_tags::ActiveModel {
            record_id: ActiveValue::Set(record_id),
            tag_id: ActiveValue::Set(tag_id),
        }
        .insert(conn)
        .await?;
    }
    Ok(())
}

async fn find_or_create_tag<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    name: &str,
) -> Result<i32> {
    if let Some(existing) = tags::Entity::find()
        .filter(tags::Column::ProjectId.eq(project_id))
        .filter(tags::Column::Name.eq(name))
        .one(conn)
        .await?
    {
        return Ok(existing.id);
    }

    let inserted = tags::ActiveModel {
        project_id: ActiveValue::Set(project_id),
        name: ActiveValue::Set(name.to_owned()),
        ..Default::default()
    }
    .insert(conn)
    .await?;

    Ok(inserted.id)
}
