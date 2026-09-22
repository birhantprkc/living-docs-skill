use super::*;
use crate::record::format_scalar;
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

    fn contents(&self, path: &str) -> String {
        self.files
            .borrow()
            .get(&PathBuf::from(path))
            .cloned()
            .unwrap()
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

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let mut files = self.files.borrow_mut();
        let contents = files
            .remove(from)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))?;
        files.insert(to.to_path_buf(), contents);
        Ok(())
    }
}

const ADR: &str = "---\ntype: ADR\ntitle: A Decision\ndescription: <One sentence — the decision and its scope.>\nstatus: Proposed\nsupersedes:\nsuperseded_by:\n---\n\n# A Decision\n";

fn store() -> MapStore {
    MapStore::seeded(&[("/bundle/adr/0001-a-decision.md", ADR)])
}

#[test]
fn set_status_updates_the_field_within_the_type_vocabulary() {
    let store = store();
    apply(&store, Path::new("/bundle"), "0001", "status", "Accepted").expect("valid status");
    assert!(store
        .contents("/bundle/adr/0001-a-decision.md")
        .contains("status: Accepted\n"));
}

#[test]
fn set_status_rejects_a_value_outside_the_type_vocabulary_and_writes_nothing() {
    let store = store();
    let err = apply(&store, Path::new("/bundle"), "0001", "status", "Ratified")
        .expect_err("Ratified is not an ADR status");
    assert!(err.contains("not a valid status"), "got: {err}");
    assert_eq!(store.contents("/bundle/adr/0001-a-decision.md"), ADR);
}

#[test]
fn set_status_reserves_superseded_for_the_supersede_verb() {
    let store = store();
    let err = apply(&store, Path::new("/bundle"), "0001", "status", "Superseded")
        .expect_err("Superseded is set only via supersede");
    assert!(err.contains("living-docs supersede"), "got: {err}");
}

#[test]
fn set_status_deprecated_adds_the_deprecated_callout() {
    let store = store();
    apply(&store, Path::new("/bundle"), "0001", "status", "Deprecated").expect("valid status");
    let contents = store.contents("/bundle/adr/0001-a-decision.md");
    assert!(
        contents.contains(
            "> **DEPRECATED — do not act on this record.** It has no successor. \
             Run `living-docs read` for what is in force.\n\n# A Decision"
        ),
        "got: {contents}"
    );
}

#[test]
fn set_status_back_to_active_removes_the_callout_and_leaves_the_body_byte_identical() {
    let store = store();
    apply(&store, Path::new("/bundle"), "0001", "status", "Deprecated").expect("valid status");
    apply(&store, Path::new("/bundle"), "0001", "status", "Accepted").expect("valid status");

    let contents = store.contents("/bundle/adr/0001-a-decision.md");
    let original_body = extract_record(Path::new("/bundle/adr/0001-a-decision.md"), ADR).body;
    let final_body = extract_record(Path::new("/bundle/adr/0001-a-decision.md"), &contents).body;

    assert!(!contents.contains("DEPRECATED"), "got: {contents}");
    assert_eq!(final_body, original_body, "got: {contents}");
}

#[test]
fn set_description_quotes_and_replaces_the_placeholder() {
    let store = store();
    apply(
        &store,
        Path::new("/bundle"),
        "0001",
        "description",
        "Caching: a deep dive",
    )
    .expect("any sentence is accepted");
    assert!(store
        .contents("/bundle/adr/0001-a-decision.md")
        .contains(&format!(
            "description: {}\n",
            format_scalar("Caching: a deep dive")
        )));
}

#[test]
fn set_owner_accepts_any_string() {
    let store = store();
    apply(
        &store,
        Path::new("/bundle"),
        "0001",
        "owner",
        "carol@example.com",
    )
    .expect("any owner");
    assert!(store
        .contents("/bundle/adr/0001-a-decision.md")
        .contains("owner: carol@example.com\n"));
}

#[test]
fn set_rejects_an_unknown_field() {
    let store = store();
    let err = apply(&store, Path::new("/bundle"), "0001", "labels", "urgent")
        .expect_err("labels is not a settable field");
    assert!(err.contains("not a settable field"), "got: {err}");
    assert_eq!(store.contents("/bundle/adr/0001-a-decision.md"), ADR);
}

#[test]
fn set_fails_when_the_record_cannot_be_found_and_leaves_it_unchanged() {
    let store = store();
    let err = apply(&store, Path::new("/bundle"), "0099", "status", "Accepted")
        .expect_err("no such record");
    assert!(err.contains("no record found for 0099"), "got: {err}");
    assert_eq!(store.contents("/bundle/adr/0001-a-decision.md"), ADR);
}
