//! Full-rebuild sync from a [`living_docs_core::store::DocStore`] into the
//! `records` read-model, plus its backend-native search index (ADR 0004,
//! issue 0002 slice S2b; ParadeDB branch issue 0004 slice 0004-B;
//! default-project assignment issue 0005 slice 0005-A; per-project ingestion
//! + relations/tags issue 0005 slice 0005-B).

use std::path::Path;

use living_docs_core::store::DocStore;
use sea_orm::sea_query::Query;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend,
    DbErr, EntityTrait, QueryFilter, Statement, TransactionTrait,
};

use crate::entity::{projects, record_tags};
use crate::entity::{Column, Entity as Records, Model};
use crate::record::is_reserved;
use crate::Result;

mod frontmatter;
mod records;
mod relations;
mod tags;
mod write;

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
mod tests;

pub(crate) use frontmatter::load_frontmatter_tail;
pub(crate) use write::{insert_new_record, update_existing_record, upsert_record};

/// The slug [`sync`] assigns every record to: the single project every
/// caller that does not (yet) think in terms of named projects keeps
/// syncing into, unchanged since issue 0005 slice 0005-A.
const DEFAULT_PROJECT_SLUG: &str = "default";

/// Rebuilds the single default project's records exactly as slice 0005-A
/// did, for every caller that has not been upgraded to name a project.
/// Equivalent to `sync_project(conn, store, bundle, "default")`.
pub async fn sync(conn: &DatabaseConnection, store: &dyn DocStore, bundle: &Path) -> Result<usize> {
    sync_project(conn, store, bundle, DEFAULT_PROJECT_SLUG).await
}

/// Rebuilds `project_slug`'s slice of the `records`/`relations`/`tags`/
/// `record_tags` tables and the backend-native search index, from every
/// non-reserved `.md` doc `store` lists under `bundle`, in one transaction.
/// Idempotent: running twice over an unchanged corpus yields identical rows
/// for this project. Only this project's rows are cleared first — a
/// re-sync never touches another project's records, relations, or tags.
/// The project is upserted by `project_slug` (rooted at `bundle` on first
/// use). Insertion is two-pass: every record lands first, then each
/// record's `supersedes`/`superseded_by` frontmatter is resolved against
/// its *own* project's just-inserted records and tags are attached — a
/// target that does not resolve within the project is skipped, not
/// inserted as a dangling relation. Returns the number of records inserted.
pub async fn sync_project(
    conn: &DatabaseConnection,
    store: &dyn DocStore,
    bundle: &Path,
    project_slug: &str,
) -> Result<usize> {
    let paths = store.list(bundle).map_err(io_err_to_db_err)?;
    let txn = conn.begin().await?;

    let project_id = ensure_project(&txn, project_slug, bundle).await?;
    clear_project(&txn, project_id).await?;

    let mut inserted = Vec::new();
    for path in paths {
        if is_reserved(&path) {
            continue;
        }
        inserted.push(records::insert_record(&txn, store, bundle, &path, project_id).await?);
    }
    let count = inserted.len();

    relations::insert_supersede_relations(&txn, project_id, &inserted).await?;
    tags::insert_tags(&txn, project_id, &inserted).await?;

    rebuild_search_index(&txn).await?;
    txn.commit().await?;
    Ok(count)
}

/// A single record just inserted this sync run, carrying the frontmatter
/// slice_id 0005-B needs to resolve relations and tags in the following
/// passes.
struct InsertedRecord {
    id: i32,
    relative_path: String,
    supersedes: Option<String>,
    superseded_by: Option<String>,
    tags: Vec<String>,
}

/// Finds `slug`'s project, inserting it (rooted at `bundle`) the first time
/// a sync targets it. Returns the project's id either way.
async fn ensure_project<C: ConnectionTrait>(conn: &C, slug: &str, bundle: &Path) -> Result<i32> {
    if let Some(existing) = projects::Entity::find()
        .filter(projects::Column::Slug.eq(slug))
        .one(conn)
        .await?
    {
        return Ok(existing.id);
    }

    let inserted = projects::ActiveModel {
        slug: ActiveValue::Set(slug.to_owned()),
        name: ActiveValue::Set(slug.to_owned()),
        root_path: ActiveValue::Set(Some(bundle.to_string_lossy().into_owned())),
        ..Default::default()
    }
    .insert(conn)
    .await?;

    Ok(inserted.id)
}

/// Deletes `project_id`'s rows from `record_tags`, `relations`, `tags`, and
/// `records`, in FK-safe order, leaving every other project's rows intact.
async fn clear_project<C: ConnectionTrait>(conn: &C, project_id: i32) -> Result<()> {
    delete_record_tags_for_project(conn, project_id).await?;
    crate::entity::relations::Entity::delete_many()
        .filter(crate::entity::relations::Column::ProjectId.eq(project_id))
        .exec(conn)
        .await?;
    crate::entity::tags::Entity::delete_many()
        .filter(crate::entity::tags::Column::ProjectId.eq(project_id))
        .exec(conn)
        .await?;
    Records::delete_many()
        .filter(Column::ProjectId.eq(project_id))
        .exec(conn)
        .await?;
    Ok(())
}

/// `record_tags` carries no `project_id` of its own, so scoping its delete
/// to `project_id` goes through the owning record. Built with SeaORM's
/// query builder (`in_subquery`), not a raw placeholder, so it renders
/// `?`/`$1` correctly on both SQLite and Postgres/ParadeDB.
async fn delete_record_tags_for_project<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
) -> Result<()> {
    let project_record_ids = Query::select()
        .column(Column::Id)
        .from(Records)
        .and_where(Column::ProjectId.eq(project_id))
        .to_owned();

    record_tags::Entity::delete_many()
        .filter(record_tags::Column::RecordId.in_subquery(project_record_ids))
        .exec(conn)
        .await
        .map(|_| ())
}

fn relative_path(bundle: &Path, path: &Path) -> String {
    path.strip_prefix(bundle)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

pub(crate) async fn find_record<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    path: &str,
) -> Result<Option<Model>> {
    Records::find()
        .filter(Column::ProjectId.eq(project_id))
        .filter(Column::Path.eq(path))
        .one(conn)
        .await
}

/// Rebuilds the backend-native search index over `records`. SQLite's FTS5
/// external-content index is stale after a bulk write and must be told to
/// rebuild; Postgres's `pg_search` BM25 index updates automatically on
/// insert, so this is a no-op there.
async fn rebuild_search_index<C: ConnectionTrait>(conn: &C) -> Result<()> {
    match conn.get_database_backend() {
        DbBackend::Sqlite => conn
            .execute(Statement::from_string(
                DbBackend::Sqlite,
                "INSERT INTO records_fts(records_fts) VALUES('rebuild')".to_owned(),
            ))
            .await
            .map(|_| ()),
        DbBackend::Postgres | DbBackend::MySql => Ok(()),
    }
}

fn io_err_to_db_err(err: std::io::Error) -> DbErr {
    DbErr::Custom(err.to_string())
}
