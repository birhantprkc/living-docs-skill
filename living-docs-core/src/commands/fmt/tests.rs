use super::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;

struct MapStore {
    files: RefCell<BTreeMap<PathBuf, String>>,
}

impl MapStore {
    fn seeded(seed: &[(&str, &str)]) -> Self {
        let files = seed
            .iter()
            .map(|(path, contents)| (PathBuf::from(path), (*contents).to_string()))
            .collect();
        Self {
            files: RefCell::new(files),
        }
    }

    fn contents(&self, path: &str) -> String {
        self.files
            .borrow()
            .get(&PathBuf::from(path))
            .cloned()
            .unwrap_or_default()
    }
}

impl DocStore for MapStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(self
            .files
            .borrow()
            .keys()
            .filter(|path| path.starts_with(root))
            .cloned()
            .collect())
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        self.files
            .borrow()
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        self.files
            .borrow_mut()
            .insert(path.to_path_buf(), contents.to_string());
        Ok(())
    }
}

#[test]
fn is_reserved_file_matches_index_and_log_only() {
    assert!(is_reserved_file(Path::new("docs/index.md")));
    assert!(is_reserved_file(Path::new("docs/adr/log.md")));
    assert!(!is_reserved_file(Path::new("docs/adr/0001-doc.md")));
}

#[test]
fn canonicalize_record_leaves_a_frontmatter_free_file_untouched() {
    let store = MapStore::seeded(&[("/bundle/notes.md", "# Just a heading\n\nBody.\n")]);

    let rewritten = canonicalize_record(&store, Path::new("/bundle/notes.md"));

    assert!(!rewritten);
    assert_eq!(
        store.contents("/bundle/notes.md"),
        "# Just a heading\n\nBody.\n"
    );
}

#[test]
fn canonicalize_record_leaves_an_already_canonical_record_byte_identical() {
    let canonical = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.\n---\n\n# Quokka Caching\n\nBody.\n";
    let store = MapStore::seeded(&[("/bundle/adr/0001-doc.md", canonical)]);

    let rewritten = canonicalize_record(&store, Path::new("/bundle/adr/0001-doc.md"));

    assert!(!rewritten);
    assert_eq!(store.contents("/bundle/adr/0001-doc.md"), canonical);
}

#[test]
fn run_a_second_time_over_a_freshly_canonicalized_record_rewrites_nothing() {
    let store = MapStore::seeded(&[(
        "/bundle/adr/0001-doc.md",
        "---\ntitle: Quokka Caching\ntype: ADR\ndescription: Adopt quokka caching.\n---\n# Quokka Caching\n\nBody.\n",
    )]);
    let bundle = PathBuf::from("/bundle");

    canonicalize_bundle(&store, &store.list(&bundle).unwrap());
    let after_first_pass = store.contents("/bundle/adr/0001-doc.md");
    let rewritten_second_pass = canonicalize_record(&store, Path::new("/bundle/adr/0001-doc.md"));

    assert!(!rewritten_second_pass);
    assert_eq!(store.contents("/bundle/adr/0001-doc.md"), after_first_pass);
}

#[test]
fn run_reports_zero_rewrites_over_an_empty_bundle() {
    let store = MapStore::seeded(&[]);
    let bundle = std::env::temp_dir();

    let code = run(&store, &bundle);

    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
}

#[test]
fn run_exits_with_code_two_when_the_bundle_root_is_missing() {
    let store = MapStore::seeded(&[]);
    let missing = std::env::temp_dir().join("living-docs-fmt-missing-bundle");

    let code = run(&store, &missing);

    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::from(2)));
}

#[test]
fn canonicalize_record_reflows_a_wrapped_paragraph_and_reports_a_rewrite() {
    let contents = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.\n---\n\n# Quokka Caching\n\nThis paragraph\nwraps over\nthree lines.\n";
    let store = MapStore::seeded(&[("/bundle/adr/0001-doc.md", contents)]);

    let rewritten = canonicalize_record(&store, Path::new("/bundle/adr/0001-doc.md"));

    assert!(rewritten);
    assert_eq!(
        store.contents("/bundle/adr/0001-doc.md"),
        "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.\n---\n\n# Quokka Caching\n\nThis paragraph wraps over three lines.\n"
    );
}

#[test]
fn canonicalize_record_leaves_a_fenced_code_block_in_the_body_untouched() {
    let canonical = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.\n---\n\n# Quokka Caching\n\n```\ncode line\n```\n";
    let store = MapStore::seeded(&[("/bundle/adr/0001-doc.md", canonical)]);

    let rewritten = canonicalize_record(&store, Path::new("/bundle/adr/0001-doc.md"));

    assert!(!rewritten);
    assert_eq!(store.contents("/bundle/adr/0001-doc.md"), canonical);
}
