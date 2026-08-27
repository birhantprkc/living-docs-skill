use crate::commands::supersede::{find_record, set_frontmatter_fields};
use crate::record::format_scalar;
use crate::store::DocStore;
use std::path::Path;
use std::process::ExitCode;

pub fn run(store: &dyn DocStore, docs_dir: &Path, reference: &str, value: &str) -> ExitCode {
    match owner(store, docs_dir, reference, value) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("living-docs owner: {message}");
            ExitCode::from(2)
        }
    }
}

/// Sets a record's `owner:` frontmatter field, reusing `supersede`'s
/// collision-aware record-resolution ([`find_record`], accepting either a
/// bare `NNNN` or a type-qualified `TYPE/NNNN` reference) and
/// frontmatter-mutation ([`set_frontmatter_fields`]) helpers rather than
/// duplicating them, exactly as `describe.rs` does. Any string is accepted:
/// the tool never validates an owner value against an identity directory,
/// so the only failure mode is an unresolvable `reference`, surfaced by
/// `find_record` before any write is attempted. `value` is quoted via
/// [`format_scalar`] exactly as `describe.rs` quotes a description.
fn owner(
    store: &dyn DocStore,
    docs_dir: &Path,
    reference: &str,
    value: &str,
) -> Result<(), String> {
    let path = find_record(store, docs_dir, reference)?;
    set_frontmatter_fields(store, &path, &[("owner", format_scalar(value))])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::io;
    use std::path::PathBuf;

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

    const RECORD: &str = "---\ntype: ADR\ntitle: A Decision\ndescription: A one sentence summary.\nowner: alice\nstatus: Proposed\nsupersedes:\nsuperseded_by:\n---\n\n# A Decision\n";

    #[test]
    fn owner_sets_the_owner_field_and_preserves_the_rest_of_the_record() {
        let store = MapStore::seeded(&[("/bundle/adr/0001-a-decision.md", RECORD)]);

        owner(&store, Path::new("/bundle"), "0001", "bob").expect("owner should succeed");

        let updated = store
            .read(Path::new("/bundle/adr/0001-a-decision.md"))
            .unwrap();
        assert!(updated.contains("owner: bob\n"), "got: {updated}");
        assert!(updated.contains("title: A Decision"), "got: {updated}");
        assert!(updated.contains("status: Proposed"), "got: {updated}");
        assert!(updated.contains("supersedes:\n"), "got: {updated}");
        assert!(updated.contains("# A Decision\n"), "got: {updated}");
    }

    #[test]
    fn owner_quotes_a_value_containing_a_colon_space_and_round_trips_through_format_scalar() {
        let store = MapStore::seeded(&[("/bundle/adr/0001-a-decision.md", RECORD)]);

        owner(&store, Path::new("/bundle"), "0001", "Team: Platform")
            .expect("owner should succeed");

        let updated = store
            .read(Path::new("/bundle/adr/0001-a-decision.md"))
            .unwrap();
        assert!(
            updated.contains(&format!("owner: {}\n", format_scalar("Team: Platform"))),
            "got: {updated}"
        );
    }

    #[test]
    fn owner_accepts_any_string_with_no_vocabulary_constraint() {
        let store = MapStore::seeded(&[("/bundle/adr/0001-a-decision.md", RECORD)]);

        owner(&store, Path::new("/bundle"), "0001", "carol@example.com")
            .expect("owner places no constraint on the value's content");

        let updated = store
            .read(Path::new("/bundle/adr/0001-a-decision.md"))
            .unwrap();
        assert!(updated.contains("owner: carol@example.com\n"));
    }

    #[test]
    fn owner_fails_when_the_store_lists_no_record_for_a_number_and_leaves_it_unchanged() {
        let store = MapStore::seeded(&[("/bundle/adr/0001-a-decision.md", RECORD)]);

        let err = owner(&store, Path::new("/bundle"), "0099", "bob")
            .expect_err("owner must fail when the record cannot be found");

        assert!(err.contains("no record found for 0099"), "got: {err}");
        let unchanged = store
            .read(Path::new("/bundle/adr/0001-a-decision.md"))
            .unwrap();
        assert_eq!(unchanged, RECORD);
    }

    const ADR_RECORD_0029: &str = "---\ntype: ADR\ntitle: A Decision\ndescription: A one sentence summary.\nowner: alice\nstatus: Proposed\nsupersedes:\nsuperseded_by:\n---\n\n# A Decision\n";
    const ISSUE_RECORD_0029: &str =
        "---\ntype: Issue\ntitle: A Bug\ndescription: <One sentence.>\nstatus: open\n---\n\n# A Bug\n";

    #[test]
    fn owner_resolves_a_type_qualified_reference_to_only_that_type_across_a_number_collision() {
        let store = MapStore::seeded(&[
            ("/bundle/adr/0029-collision.md", ADR_RECORD_0029),
            ("/bundle/issues/0029-collision.md", ISSUE_RECORD_0029),
        ]);

        owner(&store, Path::new("/bundle"), "adr/0029", "bob")
            .expect("a type-qualified reference must resolve");

        let adr = store
            .read(Path::new("/bundle/adr/0029-collision.md"))
            .unwrap();
        let issue = store
            .read(Path::new("/bundle/issues/0029-collision.md"))
            .unwrap();
        assert!(adr.contains("owner: bob\n"), "got: {adr}");
        assert_eq!(
            issue, ISSUE_RECORD_0029,
            "the colliding Issue must stay byte-identical"
        );
    }

    #[test]
    fn owner_fails_loud_on_an_unqualified_cross_type_collision_and_writes_no_file() {
        let store = MapStore::seeded(&[
            ("/bundle/adr/0029-collision.md", ADR_RECORD_0029),
            ("/bundle/issues/0029-collision.md", ISSUE_RECORD_0029),
        ]);

        let err = owner(&store, Path::new("/bundle"), "0029", "bob")
            .expect_err("an unqualified cross-type collision must be rejected");

        assert!(err.contains("/bundle/adr/0029-collision.md"), "got: {err}");
        assert!(
            err.contains("/bundle/issues/0029-collision.md"),
            "got: {err}"
        );
        let adr = store
            .read(Path::new("/bundle/adr/0029-collision.md"))
            .unwrap();
        let issue = store
            .read(Path::new("/bundle/issues/0029-collision.md"))
            .unwrap();
        assert_eq!(
            adr, ADR_RECORD_0029,
            "no file may be written on an ambiguous reference"
        );
        assert_eq!(
            issue, ISSUE_RECORD_0029,
            "no file may be written on an ambiguous reference"
        );
    }

    #[test]
    fn run_returns_the_success_exit_code_when_owner_is_set() {
        let store = MapStore::seeded(&[("/bundle/adr/0001-a-decision.md", RECORD)]);

        let code = run(&store, Path::new("/bundle"), "0001", "bob");

        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
    }

    #[test]
    fn run_returns_a_non_success_exit_code_for_an_unknown_record() {
        let store = MapStore::seeded(&[("/bundle/adr/0001-a-decision.md", RECORD)]);

        let code = run(&store, Path::new("/bundle"), "0099", "bob");

        assert_ne!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
    }
}
