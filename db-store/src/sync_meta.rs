//! Read/write helpers over the `sync_meta` table (ADR 0042, issue 0034):
//! [`upsert`] is `sync`'s own last step; [`sync_meta`] is the check
//! `search` and the web front read against a freshly recomputed
//! [`living_docs_core::fingerprint::tree_fingerprint`].

use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter,
};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::entity::projects::{Column as ProjectColumn, Entity as Projects};
use crate::entity::sync_meta::{ActiveModel, Entity as SyncMetaEntity};
use crate::Result;

/// A project's last-successful-sync marker: when it completed (Unix
/// seconds, UTC) and the records-tree fingerprint at that moment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncMeta {
    pub last_sync_completed_at: i64,
    pub tree_fingerprint: String,
}

/// Looks up `project_slug`'s [`SyncMeta`] row. `None` when the project does
/// not exist, or exists but has never completed a sync — both read as
/// stale to a caller (ADR 0042 decision 3).
pub async fn sync_meta(conn: &DatabaseConnection, project_slug: &str) -> Result<Option<SyncMeta>> {
    let Some(project) = Projects::find()
        .filter(ProjectColumn::Slug.eq(project_slug))
        .one(conn)
        .await?
    else {
        return Ok(None);
    };

    let row = SyncMetaEntity::find_by_id(project.id).one(conn).await?;
    Ok(row.map(|row| SyncMeta {
        last_sync_completed_at: row.last_sync_completed_at,
        tree_fingerprint: row.tree_fingerprint,
    }))
}

/// Upserts `project_id`'s [`SyncMeta`] row to `fingerprint` at the current
/// time — `sync_project`'s last step on a successful run, inside the same
/// transaction so a failed sync never writes it.
pub(crate) async fn upsert<C: ConnectionTrait>(
    conn: &C,
    project_id: i32,
    fingerprint: &str,
) -> Result<()> {
    let now = current_unix_seconds();
    match SyncMetaEntity::find_by_id(project_id).one(conn).await? {
        Some(_) => {
            ActiveModel {
                project_id: ActiveValue::Unchanged(project_id),
                last_sync_completed_at: ActiveValue::Set(now),
                tree_fingerprint: ActiveValue::Set(fingerprint.to_owned()),
            }
            .update(conn)
            .await?;
        }
        None => {
            ActiveModel {
                project_id: ActiveValue::Set(project_id),
                last_sync_completed_at: ActiveValue::Set(now),
                tree_fingerprint: ActiveValue::Set(fingerprint.to_owned()),
            }
            .insert(conn)
            .await?;
        }
    }
    Ok(())
}

fn current_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::sync_project;
    use crate::{connect_in_memory, migrate};
    use living_docs_core::store::DocStore;
    use std::io;
    use std::path::{Path, PathBuf};

    struct EmptyStore;

    impl DocStore for EmptyStore {
        fn list(&self, _root: &Path) -> io::Result<Vec<PathBuf>> {
            Ok(Vec::new())
        }

        fn read(&self, _path: &Path) -> io::Result<String> {
            Err(io::Error::new(io::ErrorKind::NotFound, "no records"))
        }

        fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn sync_meta_is_none_for_a_project_that_was_never_synced() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");

        let meta = sync_meta(&conn, "no-such-project").await.expect("query");

        assert!(meta.is_none());
    }

    #[tokio::test]
    async fn sync_meta_is_populated_after_a_successful_sync() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");
        sync_project(&conn, &EmptyStore, Path::new("/bundle"), "default")
            .await
            .expect("sync project");

        let meta = sync_meta(&conn, "default")
            .await
            .expect("query")
            .expect("row exists after a successful sync");

        assert_eq!(
            meta.tree_fingerprint,
            living_docs_core::fingerprint::tree_fingerprint(Path::new("/bundle")).unwrap()
        );
        assert!(meta.last_sync_completed_at > 0);
    }

    #[tokio::test]
    async fn sync_meta_upsert_replaces_the_prior_row_rather_than_duplicating_it() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");
        sync_project(&conn, &EmptyStore, Path::new("/bundle"), "default")
            .await
            .expect("first sync");
        sync_project(&conn, &EmptyStore, Path::new("/bundle"), "default")
            .await
            .expect("second sync");

        let meta = sync_meta(&conn, "default")
            .await
            .expect("query")
            .expect("row exists");

        assert_eq!(
            meta.tree_fingerprint,
            living_docs_core::fingerprint::tree_fingerprint(Path::new("/bundle")).unwrap()
        );
    }
}
