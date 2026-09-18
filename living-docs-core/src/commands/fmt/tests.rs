use super::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

/// Creates a real, empty temp file/dir tree mirroring `paths` so
/// [`resolve_targets`]'s `is_file`/`is_dir` checks (deliberately real-fs,
/// since `fmt` is fs-backend only) see the same shape a `MapStore`-seeded
/// test exercises through `read`/`write`. Returns the temp root.
fn real_temp_tree(label: &str, paths: &[&str]) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("living-docs-fmt-core-test-{label}-{nanos}"));
    for rel in paths {
        let path = root.join(rel.trim_start_matches('/'));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "").unwrap();
    }
    root
}

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

    let code = run(&store, &bundle, false);

    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
}

#[test]
fn run_exits_with_code_two_when_the_bundle_root_is_missing() {
    let store = MapStore::seeded(&[]);
    let missing = std::env::temp_dir().join("living-docs-fmt-missing-bundle");

    let code = run(&store, &missing, false);

    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::from(2)));
}

#[test]
fn canonicalize_record_leaves_a_hard_wrapped_paragraph_body_byte_identical() {
    let contents = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.\n---\n\n# Quokka Caching\n\nThis paragraph\nwraps over\nthree lines.\n";
    let store = MapStore::seeded(&[("/bundle/adr/0001-doc.md", contents)]);

    let rewritten = canonicalize_record(&store, Path::new("/bundle/adr/0001-doc.md"));

    assert!(!rewritten);
    assert_eq!(store.contents("/bundle/adr/0001-doc.md"), contents);
}

#[test]
fn canonicalize_record_leaves_a_multi_line_reference_list_line_count_unchanged() {
    let contents = "---\ntitle: Quokka Caching\ntype: ADR\ndescription: Adopt quokka caching.\n---\n\n# Quokka Caching\n\nBody.\n\n# References\n\n- First reference\n- Second reference\n- Third reference\n";
    let store = MapStore::seeded(&[("/bundle/adr/0001-doc.md", contents)]);

    canonicalize_record(&store, Path::new("/bundle/adr/0001-doc.md"));

    let rewritten = store.contents("/bundle/adr/0001-doc.md");
    let original_body_lines = contents.lines().count();
    let rewritten_body_lines = rewritten.lines().count();
    assert_eq!(original_body_lines, rewritten_body_lines);
    assert!(rewritten.contains("- First reference\n- Second reference\n- Third reference\n"));
}

#[test]
fn canonicalize_record_leaves_a_fenced_code_block_in_the_body_untouched() {
    let canonical = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.\n---\n\n# Quokka Caching\n\n```\ncode line\n```\n";
    let store = MapStore::seeded(&[("/bundle/adr/0001-doc.md", canonical)]);

    let rewritten = canonicalize_record(&store, Path::new("/bundle/adr/0001-doc.md"));

    assert!(!rewritten);
    assert_eq!(store.contents("/bundle/adr/0001-doc.md"), canonical);
}

#[test]
fn run_against_a_single_record_file_canonicalizes_only_that_file() {
    let non_canonical =
        "---\ntitle: Quokka Caching\ntype: ADR\ndescription: Adopt quokka caching.\n---\n\n# Quokka Caching\n\nBody.\n";
    let other = "---\ntitle: Other\ntype: ADR\ndescription: Other doc.\n---\n\n# Other\n\nBody.\n";
    let root = real_temp_tree("single-file", &["adr/0001-doc.md", "adr/0002-other.md"]);
    let target = root.join("adr/0001-doc.md");
    let other_path = root.join("adr/0002-other.md");
    let store = MapStore::seeded(&[
        (target.to_str().unwrap(), non_canonical),
        (other_path.to_str().unwrap(), other),
    ]);

    let code = run(&store, &target, false);

    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
    assert_eq!(
        store.contents(target.to_str().unwrap()),
        "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.\n---\n\n# Quokka Caching\n\nBody.\n"
    );
    assert_eq!(store.contents(other_path.to_str().unwrap()), other);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn run_check_on_a_non_canonical_record_writes_nothing_and_reports_it() {
    let non_canonical =
        "---\ntitle: Quokka Caching\ntype: ADR\ndescription: Adopt quokka caching.\n---\n\n# Quokka Caching\n\nBody.\n";
    let root = real_temp_tree("check-pending", &["adr/0001-doc.md"]);
    let target = root.join("adr/0001-doc.md");
    let store = MapStore::seeded(&[(target.to_str().unwrap(), non_canonical)]);

    let code = run(&store, &root, true);

    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::from(1)));
    assert_eq!(store.contents(target.to_str().unwrap()), non_canonical);
    let _ = std::fs::remove_dir_all(root);
}

fn superseded_record(body_heading: &str) -> String {
    format!(
        "---\ntype: ADR\ntitle: Old\ndescription: d\nstatus: Superseded\nsuperseded_by: 0002\n---\n\n# {body_heading}\n\nBody text.\n"
    )
}

#[test]
fn canonicalize_record_adds_a_missing_callout_to_a_retired_record() {
    let store = MapStore::seeded(&[
        ("/bundle/adr/0001-old.md", &superseded_record("Old")),
        (
            "/bundle/adr/0002-new-record.md",
            "---\ntype: ADR\ntitle: New\ndescription: d\n---\n\n# New\n",
        ),
    ]);

    let rewritten = canonicalize_record(&store, Path::new("/bundle/adr/0001-old.md"));

    assert!(rewritten);
    let contents = store.contents("/bundle/adr/0001-old.md");
    assert!(contents.contains(
        "> **SUPERSEDED — do not act on this record.** Replaced by [0002](0002-new-record.md)."
    ));
    assert!(contents.contains("# Old"));
}

#[test]
fn canonicalize_record_removes_a_stale_callout_from_an_active_record() {
    let stale = "---\ntype: ADR\ntitle: Active\ndescription: d\nstatus: Accepted\n---\n\n\
                 > **SUPERSEDED — do not act on this record.** Replaced by [0002](0002.md). \
                 Run `living-docs read` for what is in force.\n\n# Active\n\nBody text.\n";
    let store = MapStore::seeded(&[("/bundle/adr/0003-active.md", stale)]);

    let rewritten = canonicalize_record(&store, Path::new("/bundle/adr/0003-active.md"));

    assert!(rewritten);
    let contents = store.contents("/bundle/adr/0003-active.md");
    assert!(!contents.contains("SUPERSEDED"));
    assert!(contents.contains("# Active"));
}

#[test]
fn run_check_reports_a_retired_record_missing_its_callout_as_pending() {
    let root = real_temp_tree("check-callout", &["adr/0001-old.md"]);
    let target = root.join("adr/0001-old.md");
    let store = MapStore::seeded(&[(target.to_str().unwrap(), &superseded_record("Old"))]);

    let code = run(&store, &root, true);

    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::from(1)));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_second_canonicalize_after_reconciling_the_callout_rewrites_nothing() {
    let store = MapStore::seeded(&[
        ("/bundle/adr/0001-old.md", &superseded_record("Old")),
        (
            "/bundle/adr/0002-new-record.md",
            "---\ntype: ADR\ntitle: New\ndescription: d\n---\n\n# New\n",
        ),
    ]);

    canonicalize_record(&store, Path::new("/bundle/adr/0001-old.md"));
    let rewritten_again = canonicalize_record(&store, Path::new("/bundle/adr/0001-old.md"));

    assert!(!rewritten_again);
}

#[test]
fn run_check_on_a_canonical_bundle_exits_zero() {
    let canonical = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.\n---\n\n# Quokka Caching\n\nBody.\n";
    let root = real_temp_tree("check-canonical", &["adr/0001-doc.md"]);
    let target = root.join("adr/0001-doc.md");
    let store = MapStore::seeded(&[(target.to_str().unwrap(), canonical)]);

    let code = run(&store, &root, true);

    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
    assert_eq!(store.contents(target.to_str().unwrap()), canonical);
    let _ = std::fs::remove_dir_all(root);
}
