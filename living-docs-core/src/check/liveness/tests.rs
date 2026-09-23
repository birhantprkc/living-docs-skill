use super::*;
use crate::check::Reporter;
use crate::test_support::MapStore;

fn adr(status: &str, body: &str) -> String {
    format!("---\ntype: ADR\ntitle: t\nstatus: {status}\n---\n\n{body}")
}

fn issue(status: &str) -> String {
    format!("---\ntype: Issue\ntitle: t\nstatus: {status}\n---\n\nbody")
}

fn stale_count(files: &[(&str, &str)]) -> usize {
    let (store, all_md) = MapStore::seeded(files);
    let bundle = Path::new("docs");
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
fn a_proposed_adr_linking_a_superseded_issue_is_stale_proposed() {
    let n = stale_count(&[
        (
            "docs/adr/0001-x.md",
            &adr("Proposed", "See [issue](/issues/0002-y.md)."),
        ),
        ("docs/issues/0002-y.md", &issue("Superseded")),
    ]);
    assert_eq!(n, 1);
}

#[test]
fn a_proposed_adr_linking_an_in_progress_issue_is_active() {
    let n = stale_count(&[
        (
            "docs/adr/0001-x.md",
            &adr("Proposed", "See [issue](/issues/0002-y.md)."),
        ),
        ("docs/issues/0002-y.md", &issue("in-progress")),
    ]);
    assert_eq!(n, 0);
}
