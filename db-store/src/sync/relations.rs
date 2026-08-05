//! Supersede-relation phase: full-sync resolution and single-write resolution.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};

use crate::entity::{relations, Column, Entity as Records};
use crate::record::ExtractedRecord;
use crate::Result;

use super::InsertedRecord;

/// Resolves every inserted record's `supersedes`/`superseded_by` target
/// against this same sync run's other records and inserts one
/// `kind = "supersede"` relation per resolved link. A record that declares
/// both sides of the same link (the common case left by
/// `living-docs supersede`) yields exactly one row, not two.
pub(super) async fn insert_supersede_relations<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    inserted: &[InsertedRecord],
) -> Result<()> {
    let lookup = build_relation_lookup(inserted);
    let mut seen = HashSet::new();

    for record in inserted {
        let dir = record_dir(&record.relative_path);

        if let Some(target_id) = record
            .supersedes
            .as_deref()
            .and_then(|raw| resolve_target(&lookup, &dir, raw))
        {
            insert_supersede_relation(conn, project_id, &mut seen, record.id, target_id).await?;
        }

        if let Some(source_id) = record
            .superseded_by
            .as_deref()
            .and_then(|raw| resolve_target(&lookup, &dir, raw))
        {
            insert_supersede_relation(conn, project_id, &mut seen, source_id, record.id).await?;
        }
    }

    Ok(())
}

/// Maps `(sibling directory, zero-padded NNNN)` to a record id, mirroring
/// how `living_docs_core::check::records` resolves a `supersedes`/
/// `superseded_by` target to a sibling `<NNNN>-*.md` file.
fn build_relation_lookup(inserted: &[InsertedRecord]) -> HashMap<(String, String), i32> {
    inserted
        .iter()
        .filter_map(|record| relation_key(&record.relative_path).map(|key| (key, record.id)))
        .collect()
}

fn relation_key(relative_path: &str) -> Option<(String, String)> {
    let path = Path::new(relative_path);
    let dir = path.parent()?.to_string_lossy().into_owned();
    let number = numeric_prefix(path.file_name()?.to_str()?)?;
    Some((dir, number))
}

fn numeric_prefix(filename: &str) -> Option<String> {
    let stem = filename.strip_suffix(".md")?;
    let digits: String = stem.chars().take_while(char::is_ascii_digit).collect();
    normalize_number(&digits)
}

fn normalize_number(raw: &str) -> Option<String> {
    let parsed: u32 = raw.trim().parse().ok()?;
    Some(format!("{parsed:04}"))
}

fn record_dir(relative_path: &str) -> String {
    Path::new(relative_path)
        .parent()
        .map(|parent| parent.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn resolve_target(
    lookup: &HashMap<(String, String), i32>,
    dir: &str,
    raw_target: &str,
) -> Option<i32> {
    let number = normalize_number(raw_target)?;
    lookup.get(&(dir.to_owned(), number)).copied()
}

/// The `relations.kind` value every supersede edge carries, whether
/// resolved by a full sync ([`insert_supersede_relations`]) or a single
/// write ([`resolve_write_relations`]).
const SUPERSEDE_RELATION_KIND: &str = "supersede";

async fn insert_supersede_relation<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    seen: &mut HashSet<(i32, i32)>,
    from_record_id: i32,
    to_record_id: i32,
) -> Result<()> {
    if !seen.insert((from_record_id, to_record_id)) {
        return Ok(());
    }

    relations::ActiveModel {
        project_id: ActiveValue::Set(project_id),
        from_record_id: ActiveValue::Set(from_record_id),
        to_record_id: ActiveValue::Set(to_record_id),
        kind: ActiveValue::Set(SUPERSEDE_RELATION_KIND.to_owned()),
        ..Default::default()
    }
    .insert(conn)
    .await?;

    Ok(())
}

/// Resolves `record_id`'s `supersedes`/`superseded_by` frontmatter against
/// the project's already-persisted records — not just this write's own
/// batch, the way [`insert_supersede_relations`] resolves during a full
/// sync — and links any match. A target that does not (yet) exist is
/// skipped, not inserted as a dangling relation; the FK is the backstop.
pub(super) async fn resolve_write_relations<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    path: &str,
    record_id: i32,
    extracted: &ExtractedRecord,
) -> Result<()> {
    let dir = record_dir(path);

    if let Some(target_id) =
        resolve_write_target(conn, project_id, &dir, extracted.supersedes.as_deref()).await?
    {
        insert_supersede_relation_if_absent(conn, project_id, record_id, target_id).await?;
    }
    if let Some(source_id) =
        resolve_write_target(conn, project_id, &dir, extracted.superseded_by.as_deref()).await?
    {
        insert_supersede_relation_if_absent(conn, project_id, source_id, record_id).await?;
    }
    Ok(())
}

async fn resolve_write_target<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    dir: &str,
    raw_target: Option<&str>,
) -> Result<Option<i32>> {
    let Some(raw_target) = raw_target else {
        return Ok(None);
    };
    let Some(number) = normalize_number(raw_target) else {
        return Ok(None);
    };
    find_record_id_in_dir(conn, project_id, dir, &number).await
}

/// The id of the record in `project_id` whose path sits directly under
/// `dir` and whose `number` matches `zero_padded_number`, mirroring
/// [`relation_key`]'s sibling-directory resolution but querying persisted
/// rows instead of one sync run's in-memory batch.
async fn find_record_id_in_dir<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    dir: &str,
    zero_padded_number: &str,
) -> Result<Option<i32>> {
    let Ok(number) = zero_padded_number.parse::<i32>() else {
        return Ok(None);
    };
    let prefix = format!("{dir}/");
    let candidates = Records::find()
        .filter(Column::ProjectId.eq(project_id))
        .filter(Column::Number.eq(number))
        .all(conn)
        .await?;
    Ok(candidates
        .into_iter()
        .find(|record| record.path.starts_with(&prefix))
        .map(|record| record.id))
}

async fn insert_supersede_relation_if_absent<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    from_record_id: i32,
    to_record_id: i32,
) -> Result<()> {
    let exists = relations::Entity::find()
        .filter(relations::Column::Kind.eq(SUPERSEDE_RELATION_KIND))
        .filter(relations::Column::FromRecordId.eq(from_record_id))
        .filter(relations::Column::ToRecordId.eq(to_record_id))
        .one(conn)
        .await?
        .is_some();
    if exists {
        return Ok(());
    }
    insert_supersede_relation(
        conn,
        project_id,
        &mut HashSet::new(),
        from_record_id,
        to_record_id,
    )
    .await
}
