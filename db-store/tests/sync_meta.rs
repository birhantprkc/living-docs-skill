//! `sync_meta` staleness fitness test (ADR 0042, issue 0034): a fresh sync
//! writes a `sync_meta` row whose fingerprint matches the records tree;
//! editing a record afterward makes that fingerprint stale until the next
//! sync recomputes it; a project that was never synced carries no row at all.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use db_store::{connect, migrate, sync_meta, sync_project};
use living_docs_core::fingerprint::tree_fingerprint;
use living_docs_core::store::DocStore;

/// Mirrors `fs-store`'s recursive `.md` walk (the established convention in
/// `check_parity.rs`/`ingestion.rs`/`dual_engine.rs`: each test file in this
/// crate carries its own small local `DocStore` rather than depending on the
/// `fs-store` crate).
struct LocalFsStore;

impl DocStore for LocalFsStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        let mut found = Vec::new();
        collect_md_files(root, &mut found);
        found.sort();
        Ok(found)
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)
    }
}

fn collect_md_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_md_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

fn temp_sqlite_url(label: &str) -> (PathBuf, String) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let path = std::env::temp_dir()
        .join(format!("living-docs-sync-meta-{label}-{nanos}"))
        .join("index.db");
    let url = format!("sqlite://{}?mode=rwc", path.display());
    (path, url)
}

fn cleanup_sqlite_file(db_path: &Path) {
    let _ = fs::remove_file(db_path);
    if let Some(parent) = db_path.parent() {
        let _ = fs::remove_dir(parent);
    }
}

fn scratch_bundle_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("living-docs-sync-meta-bundle-{label}-{nanos}"))
}

fn write_record(root: &Path, filename: &str, title: &str, body: &str) {
    let type_dir = root.join("adr");
    fs::create_dir_all(&type_dir).expect("create adr dir");
    let contents = format!(
        "---\ntype: ADR\ntitle: {title}\ndescription: d.\nstatus: Accepted\n---\n\n# {title}\n\n{body}\n"
    );
    fs::write(type_dir.join(filename), contents).expect("write record");
}

#[tokio::test]
async fn sync_writes_a_sync_meta_row_matching_the_current_tree_fingerprint() {
    let root = scratch_bundle_root("fresh");
    write_record(
        &root,
        "0001-quokka.md",
        "Quokka",
        "Body about quokka caching.",
    );
    let (db_path, url) = temp_sqlite_url("fresh");
    let store = LocalFsStore;

    let conn = connect(&url).await.expect("connect");
    migrate(&conn).await.expect("migrate");
    sync_project(&conn, &store, &root, "default")
        .await
        .expect("sync project");

    let meta = sync_meta::sync_meta(&conn, "default")
        .await
        .expect("query sync_meta")
        .expect("sync writes a sync_meta row");
    let fingerprint = tree_fingerprint(&root).expect("fingerprint the tree");
    assert_eq!(meta.tree_fingerprint, fingerprint);

    cleanup_sqlite_file(&db_path);
    let _ = fs::remove_dir_all(&root);
}

#[tokio::test]
async fn editing_a_record_after_sync_makes_the_stored_fingerprint_stale_until_resynced() {
    let root = scratch_bundle_root("edit-then-resync");
    write_record(
        &root,
        "0001-quokka.md",
        "Quokka",
        "Body about quokka caching.",
    );
    let (db_path, url) = temp_sqlite_url("edit-then-resync");
    let store = LocalFsStore;

    let conn = connect(&url).await.expect("connect");
    migrate(&conn).await.expect("migrate");
    sync_project(&conn, &store, &root, "default")
        .await
        .expect("sync project");

    write_record(&root, "0002-second.md", "Second", "More quokka content.");
    let stale_meta = sync_meta::sync_meta(&conn, "default")
        .await
        .expect("query sync_meta")
        .expect("sync_meta row still exists");
    let fingerprint_after_edit = tree_fingerprint(&root).expect("fingerprint the edited tree");
    assert_ne!(stale_meta.tree_fingerprint, fingerprint_after_edit);

    sync_project(&conn, &store, &root, "default")
        .await
        .expect("resync project");
    let fresh_meta = sync_meta::sync_meta(&conn, "default")
        .await
        .expect("query sync_meta")
        .expect("sync_meta row still exists after resync");
    assert_eq!(fresh_meta.tree_fingerprint, fingerprint_after_edit);

    cleanup_sqlite_file(&db_path);
    let _ = fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_project_that_was_never_synced_carries_no_sync_meta_row() {
    let (db_path, url) = temp_sqlite_url("never-synced");

    let conn = connect(&url).await.expect("connect");
    migrate(&conn).await.expect("migrate");

    let meta = sync_meta::sync_meta(&conn, "never-synced")
        .await
        .expect("query sync_meta");
    assert!(meta.is_none());

    cleanup_sqlite_file(&db_path);
}
