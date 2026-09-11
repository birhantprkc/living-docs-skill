use super::*;
use crate::test_support::MapStore;
use std::collections::BTreeMap;

fn store_with(files: Vec<(&str, &str)>) -> MapStore {
    MapStore {
        files: files
            .into_iter()
            .map(|(path, contents)| (PathBuf::from(path), contents.to_string()))
            .collect::<BTreeMap<_, _>>(),
    }
}

fn current_bundle() -> Vec<(&'static str, &'static str)> {
    vec![("/d/index.md", "# Docs\n")]
}

#[test]
fn a_current_bundle_yields_an_empty_plan() {
    let store = store_with(current_bundle());

    assert!(plan(&store, Path::new("/d")).is_empty());
}

#[test]
fn a_missing_bundle_yields_the_adopt_sequence_and_nothing_else() {
    let store = store_with(vec![]);

    let steps = plan(&store, Path::new("/d"));

    assert_eq!(steps.len(), 7);
    assert!(steps.iter().all(|step| step.starts_with("ADOPT ")));
    assert!(steps[0].contains("no docs bundle at /d"));
    assert!(steps[2].contains("new constitution"));
    assert!(steps[4].contains("new adr"));
    assert!(steps[5].contains("--kind context"));
}

#[test]
fn a_root_architecture_file_is_an_author_step_naming_the_split() {
    let mut files = current_bundle();
    files.push(("/d/architecture.md", "# Architecture\n"));
    let store = store_with(files);

    let steps = plan(&store, Path::new("/d"));

    assert!(steps[0].starts_with("AUTHOR /d/architecture.md"));
    assert!(steps[0].contains("new view"));
    assert!(steps[0].contains("ADR 0036"));
}

#[test]
fn a_view_without_kind_is_an_author_step_naming_the_vocabulary() {
    let mut files = current_bundle();
    files.push((
        "/d/architecture/context.md",
        "---\ntype: Architecture View\ntitle: Context\n---\n\n# Context\n",
    ));
    let store = store_with(files);

    let steps = plan(&store, Path::new("/d"));

    assert!(steps[0].starts_with("AUTHOR /d/architecture/context.md"));
    assert!(steps[0].contains("kind"));
    assert!(steps[0].contains("context|container|component"));
}

#[test]
fn a_view_with_kind_and_the_architecture_index_are_not_findings() {
    let mut files = current_bundle();
    files.push((
        "/d/architecture/context.md",
        "---\ntype: Architecture View\ntitle: Context\nkind: context\n---\n\n# Context\n",
    ));
    files.push(("/d/architecture/index.md", "# Architecture\n"));
    let store = store_with(files);

    assert!(plan(&store, Path::new("/d")).is_empty());
}

#[test]
fn a_hand_maintained_table_index_in_a_registry_dir_is_a_run_step() {
    let mut files = current_bundle();
    files.push((
        "/d/adr/index.md",
        "# ADRs\n\n| # | Decision | Status |\n|---|---|---|\n| [0001](0001-x.md) | X | Accepted |\n",
    ));
    let store = store_with(files);

    let steps = plan(&store, Path::new("/d"));

    assert!(steps[0].starts_with("RUN living-docs index"));
    assert!(steps[0].contains("/d/adr/index.md"));
}

#[test]
fn a_table_index_outside_registry_dirs_is_ignored() {
    let mut files = current_bundle();
    files.push((
        "/d/context/index.md",
        "| # | Term | Home |\n|---|---|---|\n| [0001](x.md) | X | Y |\n",
    ));
    let store = store_with(files);

    assert!(plan(&store, Path::new("/d")).is_empty());
}

#[test]
fn any_finding_appends_the_closing_fmt_and_check_run_steps_once() {
    let mut files = current_bundle();
    files.push(("/d/architecture.md", "# Architecture\n"));
    let store = store_with(files);

    let steps = plan(&store, Path::new("/d"));

    assert_eq!(steps.len(), 3);
    assert_eq!(steps[1], "RUN living-docs fmt");
    assert_eq!(steps[2], "RUN living-docs check /d");
}

#[test]
fn every_step_carries_a_parseable_prefix() {
    let mut files = current_bundle();
    files.push(("/d/architecture.md", "# Architecture\n"));
    files.push((
        "/d/prd/0001-p.md",
        "---\ntype: PRD\ntitle: P\nstatus: Accepted\n---\n\n1. x\n",
    ));
    let store = store_with(files);

    for step in plan(&store, Path::new("/d")) {
        assert!(
            step.starts_with("RUN ") || step.starts_with("AUTHOR ") || step.starts_with("ADOPT "),
            "unprefixed step: {step}"
        );
    }
}
