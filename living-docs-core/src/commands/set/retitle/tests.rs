use super::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;

struct MapStore {
    files: RefCell<BTreeMap<PathBuf, String>>,
}

impl MapStore {
    fn seeded(seed: &[(&str, &str)]) -> Self {
        Self {
            files: RefCell::new(
                seed.iter()
                    .map(|(path, contents)| (PathBuf::from(path), (*contents).to_string()))
                    .collect(),
            ),
        }
    }

    fn contents(&self, path: &str) -> Option<String> {
        self.files.borrow().get(&PathBuf::from(path)).cloned()
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

fn adr(status: &str) -> String {
    format!("---\ntype: ADR\ntitle: A Decision\ndescription: A decision.\nstatus: {status}\n---\n\n# 0001. A Decision\n\nBody.\n")
}

const ADR_PATH: &str = "/bundle/adr/0001-a-decision.md";

fn bundle() -> &'static Path {
    Path::new("/bundle")
}

fn store(status: &str) -> MapStore {
    MapStore::seeded(&[(ADR_PATH, &adr(status))])
}

#[test]
fn retitle_rewrites_frontmatter_heading_and_filename() {
    let store = store("Proposed");
    let retitled =
        apply(&store, bundle(), Path::new(ADR_PATH), "Cache is write-through").expect("proposed");

    let contents = store
        .contents("/bundle/adr/0001-cache-is-write-through.md")
        .expect("renamed file");
    assert_eq!(
        retitled.path,
        PathBuf::from("/bundle/adr/0001-cache-is-write-through.md")
    );
    assert!(contents.contains("title: Cache is write-through\n"), "{contents}");
    assert!(contents.contains("# 0001. Cache is write-through\n"), "{contents}");
    assert!(store.contents(ADR_PATH).is_none(), "old file still present");
}

#[test]
fn retitle_is_allowed_on_an_accepted_record() {
    let store = store("Accepted");
    apply(&store, bundle(), Path::new(ADR_PATH), "Still revisable").expect("accepted is not closed");
    assert!(store
        .contents("/bundle/adr/0001-still-revisable.md")
        .is_some());
}

#[test]
fn retitle_is_refused_on_a_terminal_record_and_writes_nothing() {
    let store = store("Deprecated");
    let err = apply(&store, bundle(), Path::new(ADR_PATH), "Too late").expect_err("terminal status");

    assert!(err.contains("living-docs supersede"), "got: {err}");
    assert_eq!(store.contents(ADR_PATH), Some(adr("Deprecated")));
}

#[test]
fn retitle_is_refused_on_a_superseded_record() {
    let store = store("Superseded");
    let err = apply(&store, bundle(), Path::new(ADR_PATH), "Too late").expect_err("superseded");
    assert!(err.contains("living-docs supersede"), "got: {err}");
}

#[test]
fn retitle_is_refused_when_the_new_slug_is_taken_and_writes_nothing() {
    let taken = "---\ntype: Architecture View\ntitle: System Context\nkind: context\n---\n\n# System Context\n";
    let moving = "---\ntype: Architecture View\ntitle: Container View\nkind: container\n---\n\n# Container View\n";
    let store = MapStore::seeded(&[
        ("/bundle/architecture/system-context.md", taken),
        ("/bundle/architecture/container-view.md", moving),
    ]);

    let err = apply(
        &store,
        bundle(),
        Path::new("/bundle/architecture/container-view.md"),
        "System Context",
    )
    .expect_err("a named record's slug is its whole filename");

    assert!(err.contains("already holds that slug"), "got: {err}");
    assert_eq!(
        store.contents("/bundle/architecture/container-view.md"),
        Some(moving.to_string())
    );
}

#[test]
fn retitle_refuses_a_title_with_no_slug() {
    let store = store("Proposed");
    let err = apply(&store, bundle(), Path::new(ADR_PATH), "!!!").expect_err("empty slug");
    assert!(err.contains("has no slug"), "got: {err}");
    assert_eq!(store.contents(ADR_PATH), Some(adr("Proposed")));
}

#[test]
fn retitle_repoints_every_in_bundle_reference_to_the_old_filename() {
    let citing = "---\ntype: Issue\ntitle: Cites\nstatus: open\n---\n\n# 0009. Cites\n\nSee [ADR 0001](/adr/0001-a-decision.md) and `0001-a-decision.md`.\n";
    let store = MapStore::seeded(&[(ADR_PATH, &adr("Accepted")), ("/bundle/issues/0009-cites.md", citing)]);

    let retitled = apply(&store, bundle(), Path::new(ADR_PATH), "Write-through cache").expect("live");

    let cites = store.contents("/bundle/issues/0009-cites.md").expect("citing record");
    assert!(cites.contains("/adr/0001-write-through-cache.md"), "{cites}");
    assert!(!cites.contains("0001-a-decision.md"), "{cites}");
    assert_eq!(
        retitled.rewritten,
        vec![PathBuf::from("/bundle/issues/0009-cites.md")]
    );
}

#[test]
fn retitle_of_a_named_record_renames_it_to_the_bare_slug() {
    let view = "---\ntype: Architecture View\ntitle: Context View\nkind: context\n---\n\n# Context View\n\nBody.\n";
    let store = MapStore::seeded(&[("/bundle/architecture/context-view.md", view)]);

    apply(
        &store,
        bundle(),
        Path::new("/bundle/architecture/context-view.md"),
        "System Context",
    )
    .expect("a view has no status vocabulary");

    let contents = store
        .contents("/bundle/architecture/system-context.md")
        .expect("renamed view");
    assert!(contents.contains("# System Context\n"), "{contents}");
}

#[test]
fn retitle_of_a_singleton_rewrites_the_record_without_renaming_it() {
    let constitution =
        "---\ntype: Constitution\ntitle: Old Name\n---\n\n# Old Name\n\nBody.\n";
    let store = MapStore::seeded(&[("/bundle/constitution.md", constitution)]);

    let retitled = apply(
        &store,
        bundle(),
        Path::new("/bundle/constitution.md"),
        "Product Constitution",
    )
    .expect("a singleton keeps its filename");

    let contents = store.contents("/bundle/constitution.md").expect("same path");
    assert_eq!(retitled.path, PathBuf::from("/bundle/constitution.md"));
    assert!(contents.contains("title: Product Constitution\n"), "{contents}");
    assert!(contents.contains("# Product Constitution\n"), "{contents}");
}
