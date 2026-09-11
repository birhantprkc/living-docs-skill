use super::*;
use crate::check::Reporter;
use crate::test_support::MapStore;
use std::collections::BTreeMap;

fn store_of(files: &[(&str, &str)]) -> MapStore {
    let mut map = BTreeMap::new();
    for (path, contents) in files {
        map.insert(PathBuf::from(path), (*contents).to_string());
    }
    MapStore { files: map }
}

fn advisories(files: &[(&str, &str)]) -> Vec<String> {
    let store = store_of(files);
    let all: Vec<PathBuf> = store.files.keys().cloned().collect();
    let mut reporter = Reporter::new();
    check(&store, Path::new("docs"), &all, &mut reporter);
    reporter
        .into_findings()
        .1
        .into_iter()
        .map(|(_, m)| m)
        .collect()
}

fn view(nodes: &str) -> (String, String) {
    (
        "docs/architecture/context.md".to_string(),
        format!("---\ntype: Architecture View\ntitle: t\n---\n\n```mermaid\nflowchart LR\n{nodes}\n```\n"),
    )
}

const SCOPE: (&str, &str) = ("docs/architecture/diagram-scope.txt", "core\ncli\nweb\n");

#[test]
fn a_node_with_no_module_is_flagged() {
    let v = view("  A[core] --> B[ghost]\n");
    let out = advisories(&[SCOPE, (&v.0, &v.1)]);
    assert!(
        out.iter().any(|m| m.contains("node 'ghost'")),
        "got: {out:?}"
    );
}

#[test]
fn a_module_with_no_node_is_flagged() {
    let v = view("  A[core] --> B[cli]\n");
    let out = advisories(&[SCOPE, (&v.0, &v.1)]);
    assert!(
        out.iter().any(|m| m.contains("module 'web'")),
        "got: {out:?}"
    );
    assert!(!out.iter().any(|m| m.contains("module 'core'")));
}

#[test]
fn every_module_present_and_every_node_mapped_is_clean() {
    let v = view("  A[core] --> B[cli]\n  A --> C[web]\n");
    assert!(advisories(&[SCOPE, (&v.0, &v.1)]).is_empty());
}

#[test]
fn no_scope_file_is_a_no_op() {
    let v = view("  A[core] --> B[ghost]\n");
    assert!(
        advisories(&[(&v.0, &v.1)]).is_empty(),
        "without a scope file there is nothing to compare"
    );
}

#[test]
fn a_participant_declaration_is_a_node() {
    let seq = (
        "docs/architecture/flow.md".to_string(),
        "---\ntype: Architecture View\ntitle: t\n---\n\n```mermaid\nsequenceDiagram\nparticipant core\nparticipant ghost\n```\n".to_string(),
    );
    let out = advisories(&[SCOPE, (&seq.0, &seq.1)]);
    assert!(
        out.iter().any(|m| m.contains("node 'ghost'")),
        "got: {out:?}"
    );
}
