//! Export round-trip coverage for a record carrying `owner:` in db-mode:
//! the read path must surface the stored value, and the exported markdown
//! must be byte-identical to the canonical source, `owner` in canonical
//! position included.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use db_store::{connect, migrate, sync_project, DbDocStore};
use living_docs_core::store::DocStore;

struct MemoryStore {
    files: BTreeMap<PathBuf, String>,
}

impl DocStore for MemoryStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(self
            .files
            .keys()
            .filter(|path| path.starts_with(root))
            .cloned()
            .collect())
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))
    }

    fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
        Ok(())
    }
}

const OWNED_DOC: &str = "---\ntype: ADR\ntitle: Owned Decision\ndescription: A decision with an accountable owner.\nowner: Ana Costa\nstatus: Accepted\ntimestamp: 2026-08-27T00:00:00Z\n---\n# Owned Decision\n\nBody.\n";

/// [`OWNED_DOC`] reconstructed through [`db_store::record::to_canonical_markdown`]'s
/// fixed field order: the serializer always opens the body with a blank
/// line, which [`OWNED_DOC`] omits (matching the source-authoring
/// convention every other fixture in this suite already uses).
const OWNED_DOC_CANONICAL: &str = "---\ntype: ADR\ntitle: Owned Decision\ndescription: A decision with an accountable owner.\nowner: Ana Costa\nstatus: Accepted\ntimestamp: 2026-08-27T00:00:00Z\n---\n\n# Owned Decision\n\nBody.\n";

fn owned_corpus() -> (MemoryStore, PathBuf) {
    let bundle = PathBuf::from("/bundle-parity-owner");
    let mut files = BTreeMap::new();
    files.insert(
        bundle.join("adr").join("0001-owned-decision.md"),
        OWNED_DOC.to_owned(),
    );
    (MemoryStore { files }, bundle)
}

fn temp_sqlite_url(label: &str) -> (PathBuf, String) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let path = std::env::temp_dir()
        .join(format!("living-docs-db-store-parity-owner-{label}-{nanos}"))
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

fn setup_synced_db(url: &str, store: &dyn DocStore, bundle: &Path, project_slug: &str) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build setup runtime");
    runtime.block_on(async {
        let conn = connect(url).await.expect("connect");
        migrate(&conn).await.expect("migrate");
        sync_project(&conn, store, bundle, project_slug)
            .await
            .expect("sync project");
    });
}

#[test]
fn db_read_of_a_synced_record_with_owner_carries_the_stored_owner_value() {
    let (db_path, db_url) = temp_sqlite_url("owner-value");
    let (store, bundle) = owned_corpus();
    setup_synced_db(&db_url, &store, &bundle, "team-a");

    let db_store =
        DbDocStore::for_project(&db_url, bundle.clone(), "team-a").expect("open db doc store");
    let path = bundle.join("adr").join("0001-owned-decision.md");

    let markdown = db_store.read(&path).expect("read canonical markdown");
    let reparsed = db_store::record::extract_record(&path, &markdown);

    assert_eq!(reparsed.owner, Some("Ana Costa".to_owned()));

    cleanup_sqlite_file(&db_path);
}

#[test]
fn db_export_of_a_synced_record_with_owner_is_byte_identical_to_the_canonical_source() {
    let (db_path, db_url) = temp_sqlite_url("owner-byte-identical");
    let (store, bundle) = owned_corpus();
    setup_synced_db(&db_url, &store, &bundle, "team-a");

    let db_store =
        DbDocStore::for_project(&db_url, bundle.clone(), "team-a").expect("open db doc store");
    let path = bundle.join("adr").join("0001-owned-decision.md");

    let markdown = db_store.read(&path).expect("read canonical markdown");

    assert_eq!(markdown, OWNED_DOC_CANONICAL);

    cleanup_sqlite_file(&db_path);
}

#[test]
fn db_read_of_a_synced_record_without_owner_keeps_owner_none() {
    let (db_path, db_url) = temp_sqlite_url("owner-absent");
    let bundle = PathBuf::from("/bundle-parity-owner-absent");
    let no_owner_doc = "---\ntype: ADR\ntitle: Unowned Decision\ndescription: d.\nstatus: Accepted\ntimestamp: 2026-08-27T00:00:00Z\n---\n# Unowned Decision\n\nBody.\n";
    let no_owner_doc_canonical = "---\ntype: ADR\ntitle: Unowned Decision\ndescription: d.\nstatus: Accepted\ntimestamp: 2026-08-27T00:00:00Z\n---\n\n# Unowned Decision\n\nBody.\n";
    let mut files = BTreeMap::new();
    files.insert(
        bundle.join("adr").join("0001-unowned-decision.md"),
        no_owner_doc.to_owned(),
    );
    let store = MemoryStore { files };
    setup_synced_db(&db_url, &store, &bundle, "team-a");

    let db_store =
        DbDocStore::for_project(&db_url, bundle.clone(), "team-a").expect("open db doc store");
    let path = bundle.join("adr").join("0001-unowned-decision.md");

    let markdown = db_store.read(&path).expect("read canonical markdown");
    let reparsed = db_store::record::extract_record(&path, &markdown);

    assert_eq!(reparsed.owner, None);
    assert_eq!(markdown, no_owner_doc_canonical);

    cleanup_sqlite_file(&db_path);
}
