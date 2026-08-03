use crate::commands::supersede::{find_record, parse_record_number, set_frontmatter_fields};
use crate::doc_type::{self, DocTypeSpec};
use crate::record::extract_record;
use crate::store::DocStore;
use std::path::Path;
use std::process::ExitCode;

pub fn run(store: &dyn DocStore, docs_dir: &Path, number: &str, new_status: &str) -> ExitCode {
    match status(store, docs_dir, number, new_status) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("living-docs status: {message}");
            ExitCode::from(2)
        }
    }
}

/// Sets a record's `status:` frontmatter field, reusing `supersede`'s
/// record-resolution ([`find_record`]) and frontmatter-mutation
/// ([`set_frontmatter_fields`]) helpers rather than duplicating them (lesson
/// 3717). `new_status` is validated against the record's own resolved
/// [`DocTypeSpec::status_vocabulary`] (ADR 0029) before any write, so an
/// invalid value never reaches the frontmatter writer or partially mutates a
/// file.
fn status(
    store: &dyn DocStore,
    docs_dir: &Path,
    number: &str,
    new_status: &str,
) -> Result<(), String> {
    let record_number = parse_record_number(number)?;
    let path = find_record(store, docs_dir, record_number)?;
    let contents = store.read(&path).map_err(|e| e.to_string())?;
    let spec = resolve_spec(&path, &contents)?;
    validate_status(new_status, spec)?;
    set_frontmatter_fields(store, &path, &[("status", new_status.to_string())])
}

/// Resolves the [`DocTypeSpec`] the record at `path` belongs to, from its own
/// `type:` frontmatter — never a fixed global assumption — so `validate_status`
/// checks a record against its own type's vocabulary (ADR 0029).
fn resolve_spec(path: &Path, contents: &str) -> Result<&'static DocTypeSpec, String> {
    let doc_type = extract_record(path, contents).doc_type;
    doc_type::spec_for_frontmatter(&doc_type).ok_or_else(|| {
        format!(
            "{}: unrecognized 'type: {doc_type}' frontmatter",
            path.display()
        )
    })
}

fn validate_status(new_status: &str, spec: &DocTypeSpec) -> Result<(), String> {
    if spec.status_vocabulary.contains(&new_status) {
        return Ok(());
    }
    if new_status.eq_ignore_ascii_case("superseded") {
        return Err(
            "'Superseded' must be set via `living-docs supersede <old> <new>`, which also wires the supersedes/superseded_by links".to_string(),
        );
    }
    Err(format!(
        "'{new_status}' is not a valid status for {}; expected one of {}",
        spec.frontmatter,
        spec.status_vocabulary.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::io;
    use std::path::PathBuf;

    #[test]
    fn validate_status_accepts_every_type_s_own_vocabulary() {
        for spec in doc_type::DOC_TYPES {
            for value in spec.status_vocabulary {
                assert!(
                    validate_status(value, spec).is_ok(),
                    "expected {value} to be valid for {}",
                    spec.token
                );
            }
        }
    }

    #[test]
    fn validate_status_rejects_a_value_from_a_different_type_s_vocabulary() {
        let issue_spec = doc_type::spec_for("issue").unwrap();

        let err = validate_status("Proposed", issue_spec)
            .expect_err("an ADR-only value must be rejected for an Issue");

        assert!(err.contains("open"), "got: {err}");
        assert!(err.contains("in-progress"), "got: {err}");
        assert!(err.contains("closed"), "got: {err}");
        assert!(!err.contains("Proposed, Accepted"), "got: {err}");
    }

    #[test]
    fn validate_status_rejects_superseded_case_insensitively_with_a_supersede_hint_for_every_type()
    {
        for spec in doc_type::DOC_TYPES {
            for value in ["Superseded", "superseded", "SUPERSEDED"] {
                let err = validate_status(value, spec)
                    .expect_err("Superseded must be rejected for every type");
                assert!(err.contains("living-docs supersede"), "got: {err}");
            }
        }
    }

    #[test]
    fn validate_status_rejects_an_unknown_value_and_names_the_records_own_type_s_vocabulary() {
        let adr_spec = doc_type::spec_for("adr").unwrap();

        let err = validate_status("Acepted", adr_spec).expect_err("typo status must be rejected");

        assert!(err.contains("Proposed"), "got: {err}");
        assert!(err.contains("Accepted"), "got: {err}");
        assert!(err.contains("Deprecated"), "got: {err}");
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

    const RECORD: &str =
        "---\ntype: ADR\nstatus: Proposed\nsupersedes:\nsuperseded_by:\n---\n\n# Record\n";

    #[test]
    fn status_sets_the_status_field_and_preserves_the_rest_of_the_record() {
        let store = MapStore::seeded(&[("/bundle/adr/0001-record.md", RECORD)]);

        status(&store, Path::new("/bundle"), "0001", "Accepted").expect("status should succeed");

        let updated = store.read(Path::new("/bundle/adr/0001-record.md")).unwrap();
        assert!(updated.contains("status: Accepted"), "got: {updated}");
        assert!(updated.contains("# Record\n"), "got: {updated}");
        assert!(updated.contains("supersedes:\n"), "got: {updated}");
    }

    #[test]
    fn status_rejects_superseded_without_touching_the_store() {
        let store = MapStore::seeded(&[("/bundle/adr/0001-record.md", RECORD)]);

        let err = status(&store, Path::new("/bundle"), "0001", "Superseded")
            .expect_err("Superseded must be rejected");

        assert!(err.contains("living-docs supersede"), "got: {err}");
        let unchanged = store.read(Path::new("/bundle/adr/0001-record.md")).unwrap();
        assert_eq!(unchanged, RECORD);
    }

    #[test]
    fn status_fails_when_the_store_lists_no_record_for_a_number() {
        let store = MapStore::seeded(&[("/bundle/adr/0001-record.md", RECORD)]);

        let err = status(&store, Path::new("/bundle"), "0099", "Accepted")
            .expect_err("status must fail when the record cannot be found");

        assert!(err.contains("no record found for 0099"), "got: {err}");
    }

    const ISSUE_RECORD: &str = "---\ntype: Issue\nstatus: open\n---\n\n# Record\n";

    #[test]
    fn status_validates_against_the_records_own_type_not_a_fixed_global_list() {
        let store = MapStore::seeded(&[("/bundle/issues/0001-record.md", ISSUE_RECORD)]);

        status(&store, Path::new("/bundle"), "0001", "in-progress")
            .expect("in-progress is a valid Issue status");

        let updated = store
            .read(Path::new("/bundle/issues/0001-record.md"))
            .unwrap();
        assert!(updated.contains("status: in-progress"), "got: {updated}");
    }

    #[test]
    fn status_rejects_an_adr_only_value_for_an_issue_record() {
        let store = MapStore::seeded(&[("/bundle/issues/0001-record.md", ISSUE_RECORD)]);

        let err = status(&store, Path::new("/bundle"), "0001", "Proposed")
            .expect_err("Proposed is not a valid Issue status");

        assert!(err.contains("open"), "got: {err}");
        assert!(err.contains("in-progress"), "got: {err}");
        assert!(err.contains("closed"), "got: {err}");
        let unchanged = store
            .read(Path::new("/bundle/issues/0001-record.md"))
            .unwrap();
        assert_eq!(unchanged, ISSUE_RECORD);
    }

    #[test]
    fn status_fails_when_the_records_type_frontmatter_is_unrecognized() {
        let record = "---\ntype: Glossary\nstatus: Active\n---\n\n# Record\n";
        let store = MapStore::seeded(&[("/bundle/adr/0001-record.md", record)]);

        let err = status(&store, Path::new("/bundle"), "0001", "Accepted")
            .expect_err("an unrecognized type must be rejected");

        assert!(err.contains("Glossary"), "got: {err}");
    }

    #[test]
    fn run_returns_the_success_exit_code_when_status_is_set() {
        let store = MapStore::seeded(&[("/bundle/adr/0001-record.md", RECORD)]);

        let code = run(&store, Path::new("/bundle"), "0001", "Accepted");

        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
    }

    #[test]
    fn run_returns_a_non_success_exit_code_for_an_unknown_status() {
        let store = MapStore::seeded(&[("/bundle/adr/0001-record.md", RECORD)]);

        let code = run(&store, Path::new("/bundle"), "0001", "Acepted");

        assert_ne!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
    }
}
