use super::*;
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

fn classify_of(files: &[(&str, &str)]) -> LivenessReport {
    let store = store_of(files);
    let bundle = Path::new("docs");
    let all_md = store.list(bundle).unwrap();
    classify(&store, bundle, &all_md)
}

#[test]
fn a_proposed_adr_linking_a_closed_issue_is_stale_proposed() {
    let report = classify_of(&[
        (
            "docs/adr/0001-x.md",
            &adr("Proposed", "See [issue](/issues/0002-y.md)."),
        ),
        ("docs/issues/0002-y.md", &issue("closed")),
    ]);
    assert!(report.is_stale(Path::new("docs/adr/0001-x.md")));
    assert_eq!(report.counts().stale_proposed, 1);
}

#[test]
fn a_proposed_adr_linking_a_done_issue_is_stale_proposed() {
    let report = classify_of(&[
        (
            "docs/adr/0001-x.md",
            &adr("Proposed", "See [issue](/issues/0002-y.md)."),
        ),
        ("docs/issues/0002-y.md", &issue("done")),
    ]);
    assert!(report.is_stale(Path::new("docs/adr/0001-x.md")));
}

#[test]
fn a_proposed_adr_linking_an_open_issue_is_active() {
    let report = classify_of(&[
        (
            "docs/adr/0001-x.md",
            &adr("Proposed", "See [issue](/issues/0002-y.md)."),
        ),
        ("docs/issues/0002-y.md", &issue("open")),
    ]);
    assert!(!report.is_stale(Path::new("docs/adr/0001-x.md")));
    assert_eq!(report.counts().stale_proposed, 0);
}

#[test]
fn a_proposed_adr_linking_no_issue_is_active() {
    let report = classify_of(&[("docs/adr/0001-x.md", &adr("Proposed", "no links here"))]);
    assert!(!report.is_stale(Path::new("docs/adr/0001-x.md")));
}

#[test]
fn an_accepted_adr_with_a_verification_block_is_a_contract() {
    let report = classify_of(&[(
        "docs/adr/0001-x.md",
        &adr("Accepted", "## Decision\nwe will\n\n## Verification\n**x**"),
    )]);
    assert_eq!(
        report.shape(Path::new("docs/adr/0001-x.md")),
        Some(Shape::Contract)
    );
    assert_eq!(report.counts().contract, 1);
}

#[test]
fn an_accepted_adr_without_a_verification_block_is_narrative() {
    let report = classify_of(&[(
        "docs/adr/0001-x.md",
        &adr("Accepted", "## Decision\nwe will, and that is all"),
    )]);
    assert_eq!(
        report.shape(Path::new("docs/adr/0001-x.md")),
        Some(Shape::Narrative)
    );
    assert_eq!(report.counts().narrative, 1);
}

#[test]
fn liveness_is_scoped_to_adr_and_bdr_records() {
    let report = classify_of(&[(
        "docs/prd/0001-x.md",
        "---\ntype: PRD\nstatus: Draft\n---\n\nb",
    )]);
    assert!(report.shape(Path::new("docs/prd/0001-x.md")).is_none());
}

#[test]
fn an_absent_path_is_never_stale() {
    let report = LivenessReport::default();
    assert!(!report.is_stale(Path::new("docs/adr/9999-absent.md")));
}

#[test]
fn is_seed_status_reads_the_registry_seed_per_type() {
    assert!(is_seed_status("ADR", "Proposed"));
    assert!(is_seed_status("BDR", "Draft"));
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
