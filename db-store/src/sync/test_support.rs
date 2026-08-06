//! Shared in-memory `DocStore` and corpora reused by sync tests crate-wide.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use living_docs_core::store::DocStore;

pub(crate) struct MemoryStore {
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

const QUOKKA_DOC: &str = "---\ntype: ADR\ntitle: Quokka Caching Strategy\ndescription: Adopt quokka-based caching for the read model.\nstatus: Accepted\n---\n# 0001. Quokka Caching Strategy\n\nWe adopt an aggressive quokka caching strategy for search results.\n";
const UNRELATED_DOC: &str = "---\ntype: ADR\ntitle: Unrelated Decision\ndescription: Something else entirely.\nstatus: Accepted\n---\n# 0002. Unrelated Decision\n\nThis document discusses logging conventions.\n";

pub(crate) fn seeded_corpus() -> (MemoryStore, PathBuf) {
    let bundle = PathBuf::from("/bundle");
    let mut files = BTreeMap::new();
    files.insert(
        bundle.join("adr").join("0001-quokka-caching.md"),
        QUOKKA_DOC.to_owned(),
    );
    files.insert(
        bundle.join("adr").join("0002-unrelated.md"),
        UNRELATED_DOC.to_owned(),
    );
    files.insert(bundle.join("index.md"), "# Index\n".to_owned());
    (MemoryStore { files }, bundle)
}

/// A single-record corpus at `bundle_root`, always relative-pathed
/// `adr/0001-quokka-caching.md` regardless of `bundle_root` — lets a
/// test sync two different projects that each carry a record at the
/// same relative path, to exercise project-scoped path lookups.
pub(crate) fn single_record_corpus_at(bundle_root: &str, title: &str) -> (MemoryStore, PathBuf) {
    let bundle = PathBuf::from(bundle_root);
    let doc = format!(
        "---\ntype: ADR\ntitle: {title}\ndescription: d.\nstatus: Accepted\n---\n# {title}\n\nBody.\n"
    );
    let mut files = BTreeMap::new();
    files.insert(bundle.join("adr").join("0001-quokka-caching.md"), doc);
    (MemoryStore { files }, bundle)
}

/// Two ADR records: one with a `status:` frontmatter key, one without,
/// so a sync test can assert the read-model's `status` column is
/// populated for the first and `NULL`/`None` for the second (issue
/// 0008, ADR 0015, S1).
pub(crate) fn corpus_with_and_without_status() -> (MemoryStore, PathBuf) {
    let bundle = PathBuf::from("/bundle-status");
    let mut files = BTreeMap::new();
    files.insert(
        bundle.join("adr").join("0001-with-status.md"),
        "---\ntype: ADR\ntitle: With Status\ndescription: d.\nstatus: Accepted\n---\nBody.\n"
            .to_owned(),
    );
    files.insert(
        bundle.join("adr").join("0002-without-status.md"),
        "---\ntype: ADR\ntitle: Without Status\ndescription: d.\n---\nBody.\n".to_owned(),
    );
    (MemoryStore { files }, bundle)
}

/// Three records spanning two doc types and non-sequential filesystem
/// insertion order, so a nav-listing test can assert the query itself
/// orders by doc type, then number, then path, rather than relying on
/// insertion order (issue 0008, ADR 0015, S1).
pub(crate) fn mixed_type_corpus() -> (MemoryStore, PathBuf) {
    let bundle = PathBuf::from("/bundle-mixed");
    let mut files = BTreeMap::new();
    files.insert(
        bundle.join("bdr").join("0001-first-bdr.md"),
        "---\ntype: BDR\ntitle: First BDR\ndescription: d.\n---\nBody.\n".to_owned(),
    );
    files.insert(
        bundle.join("adr").join("0002-second-adr.md"),
        "---\ntype: ADR\ntitle: Second ADR\ndescription: d.\n---\nBody.\n".to_owned(),
    );
    files.insert(
        bundle.join("adr").join("0001-first-adr.md"),
        "---\ntype: ADR\ntitle: First ADR\ndescription: d.\n---\nBody.\n".to_owned(),
    );
    (MemoryStore { files }, bundle)
}

/// A superseded/superseding ADR pair, each carrying tags, so a
/// `record_meta` test can assert both supersede directions resolve to
/// the related record's path+title and that tags are attached (issue
/// 0008, ADR 0015, S1).
pub(crate) fn superseding_corpus() -> (MemoryStore, PathBuf) {
    let bundle = PathBuf::from("/bundle-supersede");
    let mut files = BTreeMap::new();
    files.insert(
        bundle.join("adr").join("0001-quokka-caching.md"),
        "---\ntype: ADR\ntitle: Quokka Caching\ndescription: d.\nsuperseded_by: 0002\ntags: [caching]\n---\nBody.\n"
            .to_owned(),
    );
    files.insert(
        bundle.join("adr").join("0002-quokka-caching-v2.md"),
        "---\ntype: ADR\ntitle: Quokka Caching V2\ndescription: d.\nstatus: Accepted\nsupersedes: 0001\ntags: [caching, performance]\n---\nBody.\n"
            .to_owned(),
    );
    (MemoryStore { files }, bundle)
}

/// A single issue-style record whose tail carries a non-empty
/// list-valued `labels:` key and an empty list-valued `blocked_by:` key
/// (ADR 0019 slice S3b), so a sync test can assert both survive the
/// insert/load round trip through `frontmatter_fields`.
pub(crate) fn list_valued_tail_corpus() -> (MemoryStore, PathBuf) {
    let bundle = PathBuf::from("/bundle-list-tail");
    let mut files = BTreeMap::new();
    files.insert(
        bundle.join("issues").join("0001-list-tail.md"),
        "---\ntype: Issue\ntitle: List Tail\ndescription: d.\nstatus: open\nlabels: [slice, skeleton]\nblocked_by: []\n---\nBody.\n"
            .to_owned(),
    );
    (MemoryStore { files }, bundle)
}
