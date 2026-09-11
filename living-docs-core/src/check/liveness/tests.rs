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

fn adr(status: &str, body: &str) -> String {
    format!("---\ntype: ADR\ntitle: t\nstatus: {status}\n---\n\n{body}")
}

fn issue(status: &str) -> String {
    format!("---\ntype: Issue\ntitle: t\nstatus: {status}\n---\n\nbody")
}

fn stale_count(files: &[(&str, &str)]) -> usize {
    let store = store_of(files);
    let bundle = Path::new("docs");
    let all_md = store.list(bundle).unwrap();
    let mut reporter = Reporter::new();
    check_liveness(&store, bundle, &all_md, &mut reporter);
    reporter.advisories.len()
}

#[test]
fn a_proposed_adr_linking_a_closed_issue_is_stale_proposed() {
    let n = stale_count(&[
        (
            "docs/adr/0001-x.md",
            &adr("Proposed", "See [issue](/issues/0002-y.md)."),
        ),
        ("docs/issues/0002-y.md", &issue("closed")),
    ]);
    assert_eq!(n, 1);
}

#[test]
fn a_proposed_adr_linking_a_done_issue_is_stale_proposed() {
    let n = stale_count(&[
        (
            "docs/adr/0001-x.md",
            &adr("Proposed", "See [issue](/issues/0002-y.md)."),
        ),
        ("docs/issues/0002-y.md", &issue("done")),
    ]);
    assert_eq!(n, 1);
}

#[test]
fn a_proposed_adr_linking_an_open_issue_is_active() {
    let n = stale_count(&[
        (
            "docs/adr/0001-x.md",
            &adr("Proposed", "See [issue](/issues/0002-y.md)."),
        ),
        ("docs/issues/0002-y.md", &issue("open")),
    ]);
    assert_eq!(n, 0);
}

#[test]
fn a_proposed_adr_linking_no_issue_is_active() {
    let n = stale_count(&[("docs/adr/0001-x.md", &adr("Proposed", "no links here"))]);
    assert_eq!(n, 0);
}

#[test]
fn an_accepted_adr_is_never_stale_proposed() {
    let n = stale_count(&[
        (
            "docs/adr/0001-x.md",
            &adr("Accepted", "See [issue](/issues/0002-y.md)."),
        ),
        ("docs/issues/0002-y.md", &issue("closed")),
    ]);
    assert_eq!(n, 0);
}

#[test]
fn liveness_is_scoped_to_adr_records() {
    let n = stale_count(&[
        (
            "docs/prd/0001-x.md",
            "---\ntype: PRD\ntitle: t\nstatus: Draft\n---\n\nSee [issue](/issues/0002-y.md).",
        ),
        ("docs/issues/0002-y.md", &issue("closed")),
    ]);
    assert_eq!(n, 0);
}

#[test]
fn is_seed_status_reads_the_registry_seed_for_adr() {
    assert!(is_seed_status("ADR", "Proposed"));
    assert!(!is_seed_status("ADR", "Accepted"));
}

#[test]
fn terminal_issue_status_covers_closed_done_and_superseded_only() {
    assert!(is_terminal_issue_status("closed"));
    assert!(is_terminal_issue_status("done"));
    assert!(is_terminal_issue_status("Superseded"));
    assert!(!is_terminal_issue_status("open"));
    assert!(!is_terminal_issue_status("in-progress"));
}
