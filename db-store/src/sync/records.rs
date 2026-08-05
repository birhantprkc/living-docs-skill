//! Record-row phase: insert, upsert-dispatch, and update against `records`.

use std::path::Path;

use living_docs_core::store::DocStore;
use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait};

use crate::entity::ActiveModel;
use crate::record::{extract_record, ExtractedRecord};
use crate::Result;

use super::frontmatter;
use super::{find_record, io_err_to_db_err, relative_path, InsertedRecord};

pub(super) async fn insert_record<C: ConnectionTrait>(
    conn: &C,
    store: &dyn DocStore,
    bundle: &Path,
    path: &Path,
    project_id: i32,
) -> Result<InsertedRecord> {
    let relative = relative_path(bundle, path);
    let contents = store.read(path).map_err(io_err_to_db_err)?;
    let extracted = extract_record(Path::new(&relative), &contents);
    let frontmatter_tail = extracted.frontmatter_tail;

    let inserted = ActiveModel {
        project_id: ActiveValue::Set(project_id),
        path: ActiveValue::Set(relative.clone()),
        doc_type: ActiveValue::Set(extracted.doc_type),
        number: ActiveValue::Set(extracted.number),
        concept_id: ActiveValue::Set(extracted.concept_id),
        identity_kind: ActiveValue::Set(extracted.identity_kind),
        title: ActiveValue::Set(extracted.title),
        description: ActiveValue::Set(extracted.description),
        body: ActiveValue::Set(extracted.body),
        status: ActiveValue::Set(extracted.status),
        ..Default::default()
    }
    .insert(conn)
    .await?;

    frontmatter::insert_frontmatter_tail(conn, inserted.id, &frontmatter_tail).await?;

    Ok(InsertedRecord {
        id: inserted.id,
        relative_path: relative,
        supersedes: extracted.supersedes,
        superseded_by: extracted.superseded_by,
        tags: extracted.tags,
    })
}

pub(super) async fn upsert_record_row<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    path: &str,
    extracted: &ExtractedRecord,
) -> Result<i32> {
    match find_record(conn, project_id, path).await? {
        Some(existing) => update_record_row(conn, existing.id, extracted).await,
        None => insert_record_row(conn, project_id, path, extracted).await,
    }
}

pub(super) async fn insert_record_row<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    path: &str,
    extracted: &ExtractedRecord,
) -> Result<i32> {
    let inserted = ActiveModel {
        project_id: ActiveValue::Set(project_id),
        path: ActiveValue::Set(path.to_owned()),
        doc_type: ActiveValue::Set(extracted.doc_type.clone()),
        number: ActiveValue::Set(extracted.number),
        concept_id: ActiveValue::Set(extracted.concept_id.clone()),
        identity_kind: ActiveValue::Set(extracted.identity_kind.clone()),
        title: ActiveValue::Set(extracted.title.clone()),
        description: ActiveValue::Set(extracted.description.clone()),
        body: ActiveValue::Set(extracted.body.clone()),
        status: ActiveValue::Set(extracted.status.clone()),
        ..Default::default()
    }
    .insert(conn)
    .await?;
    Ok(inserted.id)
}

async fn update_record_row<C: ConnectionTrait>(
    conn: &C,
    record_id: i32,
    extracted: &ExtractedRecord,
) -> Result<i32> {
    let model = ActiveModel {
        id: ActiveValue::Set(record_id),
        doc_type: ActiveValue::Set(extracted.doc_type.clone()),
        number: ActiveValue::Set(extracted.number),
        concept_id: ActiveValue::Set(extracted.concept_id.clone()),
        identity_kind: ActiveValue::Set(extracted.identity_kind.clone()),
        title: ActiveValue::Set(extracted.title.clone()),
        description: ActiveValue::Set(extracted.description.clone()),
        body: ActiveValue::Set(extracted.body.clone()),
        status: ActiveValue::Set(extracted.status.clone()),
        ..Default::default()
    };
    let updated = model.update(conn).await?;
    Ok(updated.id)
}

pub(super) async fn update_record_row_with_revision<C: ConnectionTrait>(
    conn: &C,
    record_id: i32,
    extracted: &ExtractedRecord,
    new_revision: i64,
) -> Result<i32> {
    let model = ActiveModel {
        id: ActiveValue::Set(record_id),
        doc_type: ActiveValue::Set(extracted.doc_type.clone()),
        number: ActiveValue::Set(extracted.number),
        concept_id: ActiveValue::Set(extracted.concept_id.clone()),
        identity_kind: ActiveValue::Set(extracted.identity_kind.clone()),
        title: ActiveValue::Set(extracted.title.clone()),
        description: ActiveValue::Set(extracted.description.clone()),
        body: ActiveValue::Set(extracted.body.clone()),
        status: ActiveValue::Set(extracted.status.clone()),
        revision: ActiveValue::Set(new_revision),
        ..Default::default()
    };
    let updated = model.update(conn).await?;
    Ok(updated.id)
}
