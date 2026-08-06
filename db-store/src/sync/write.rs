//! Public write path: single-record upsert/insert/update (ADR 0016).

use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};

use crate::record::ExtractedRecord;
use crate::Result;

use super::relations::resolve_write_relations;
use super::tags::replace_write_tags;
use super::{frontmatter, records};

/// Upserts one record by `(project_id, path)` from an already-extracted
/// write, replacing its frontmatter tail and best-effort re-resolving its
/// `supersedes`/`superseded_by` relations and tags against the project's
/// already-persisted records (ADR 0007, issue 0006 slice 0006-C2). Runs in
/// its own transaction so a mid-write failure leaves no partial row.
pub(crate) async fn upsert_record(
    conn: &DatabaseConnection,
    project_id: i32,
    path: &str,
    extracted: ExtractedRecord,
) -> Result<()> {
    let txn = conn.begin().await?;

    let record_id = records::upsert_record_row(&txn, project_id, path, &extracted).await?;
    frontmatter::replace_frontmatter_tail(&txn, record_id, &extracted.frontmatter_tail).await?;
    resolve_write_relations(&txn, project_id, path, record_id, &extracted).await?;
    replace_write_tags(&txn, project_id, record_id, &extracted.tags).await?;

    txn.commit().await?;
    Ok(())
}

/// Create-only counterpart to [`upsert_record`]: inserts a brand-new record
/// row and resolves its frontmatter tail, relations, and tags, without
/// checking whether one already exists at `path` and without opening its
/// own transaction — so `db_store::DbDocStore::write_checked` (issue 0010
/// slice 2) can run the insert inside a transaction it already owns, then
/// gate the commit on `check` before ever committing it.
pub(crate) async fn insert_new_record<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    path: &str,
    extracted: &ExtractedRecord,
) -> Result<i32> {
    let record_id = records::insert_record_row(conn, project_id, path, extracted).await?;
    frontmatter::replace_frontmatter_tail(conn, record_id, &extracted.frontmatter_tail).await?;
    resolve_write_relations(conn, project_id, path, record_id, extracted).await?;
    replace_write_tags(conn, project_id, record_id, &extracted.tags).await?;
    Ok(record_id)
}

/// Update counterpart to [`insert_new_record`]: replaces an already-existing
/// `record_id`'s row, frontmatter tail, resolved relations, and tags from
/// `extracted`, and bumps its `revision` to `new_revision` — the
/// revision-aware edit path `db_store::DbDocStore::update_checked` (ADR
/// 0016, issue 0011) runs inside its own caller-owned transaction. Kept
/// separate from [`update_record_row`] (behind [`upsert_record`]/
/// [`upsert_record_row`]), which CLI supersede/status keep using in db-mode
/// and which never bumps `revision`.
pub(crate) async fn update_existing_record<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    path: &str,
    record_id: i32,
    extracted: &ExtractedRecord,
    new_revision: i64,
) -> Result<()> {
    records::update_record_row_with_revision(conn, record_id, extracted, new_revision).await?;
    frontmatter::replace_frontmatter_tail(conn, record_id, &extracted.frontmatter_tail).await?;
    resolve_write_relations(conn, project_id, path, record_id, extracted).await?;
    replace_write_tags(conn, project_id, record_id, &extracted.tags).await?;
    Ok(())
}
