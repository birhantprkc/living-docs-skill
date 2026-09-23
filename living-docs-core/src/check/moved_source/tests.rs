use super::*;
use crate::test_support::MapStore;
use std::collections::BTreeMap;

fn run_over(files: Vec<(&str, &str)>) -> Reporter {
    let store = MapStore {
        files: files
            .iter()
            .map(|(path, contents)| (PathBuf::from(path), contents.to_string()))
            .collect::<BTreeMap<_, _>>(),
    };
    let all_md: Vec<PathBuf> = files.iter().map(|(path, _)| PathBuf::from(path)).collect();
    let mut reporter = Reporter::new();
    check_moved_source(&store, Path::new("/bundle"), &all_md, &mut reporter);
    reporter
}

fn advisory_messages(reporter: &Reporter) -> Vec<&str> {
    reporter
        .advisories
        .iter()
        .map(|advisory| advisory.message.as_str())
        .collect()
}

#[test]
fn reports_a_finding_naming_dependent_source_status_and_successor() {
    let reporter = run_over(vec![
        (
            "/bundle/adr/0001-a.md",
            "---\ntype: ADR\nstatus: Accepted\n---\n\n[b](./0002-b.md)\n",
        ),
        (
            "/bundle/adr/0002-b.md",
            "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0003\n---\n# B\n",
        ),
        (
            "/bundle/adr/0003-c.md",
            "---\ntype: ADR\nstatus: Accepted\n---\n# C\n",
        ),
    ]);

    let messages = advisory_messages(&reporter);
    assert_eq!(messages.len(), 1);
    assert!(messages[0].contains("MOVED-SOURCE"));
    assert!(messages[0].contains("0001-a.md"));
    assert!(messages[0].contains("0002-b.md"));
    assert!(messages[0].contains("Superseded"));
    assert!(messages[0].contains("superseded by 0003"));
}

#[test]
fn clears_when_the_dependent_links_the_successor_anywhere_in_its_body() {
    let reporter = run_over(vec![
        (
            "/bundle/adr/0001-a.md",
            "---\ntype: ADR\nstatus: Accepted\n---\n\n[b](./0002-b.md) and [c](./0003-c.md)\n",
        ),
        (
            "/bundle/adr/0002-b.md",
            "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0003\n---\n# B\n",
        ),
        (
            "/bundle/adr/0003-c.md",
            "---\ntype: ADR\nstatus: Accepted\n---\n# C\n",
        ),
    ]);

    assert!(advisory_messages(&reporter).is_empty());
}

#[test]
fn clears_when_the_dependent_itself_is_superseded() {
    let reporter = run_over(vec![
        (
            "/bundle/adr/0001-a.md",
            "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0004\n---\n\n[b](./0002-b.md)\n",
        ),
        (
            "/bundle/adr/0002-b.md",
            "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0003\n---\n# B\n",
        ),
    ]);

    assert!(advisory_messages(&reporter).is_empty());
}

#[test]
fn clears_when_the_dependent_itself_is_closed() {
    let reporter = run_over(vec![
        (
            "/bundle/issues/0001-a.md",
            "---\ntype: Issue\nstatus: closed\n---\n\n[b](./0002-b.md)\n",
        ),
        (
            "/bundle/issues/0002-b.md",
            "---\ntype: Issue\nstatus: Deprecated\n---\n# B\n",
        ),
    ]);

    assert!(advisory_messages(&reporter).is_empty());
}

#[test]
fn clears_when_the_dependent_issue_is_done() {
    let reporter = run_over(vec![
        (
            "/bundle/issues/0001-a.md",
            "---\ntype: Issue\nstatus: done\n---\n\n[b](./0002-b.md)\n",
        ),
        (
            "/bundle/issues/0002-b.md",
            "---\ntype: Issue\nstatus: Deprecated\n---\n# B\n",
        ),
    ]);

    assert!(advisory_messages(&reporter).is_empty());
}

#[test]
fn clears_when_the_resolved_successor_is_the_dependent_record_itself() {
    let reporter = run_over(vec![
        (
            "/bundle/adr/0016-atlas.md",
            "---\ntype: ADR\nstatus: Accepted\nsupersedes: 0006\n---\n\n[the old approach](./0006-web-read-only.md)\n",
        ),
        (
            "/bundle/adr/0006-web-read-only.md",
            "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0016\n---\n# Web read-only\n",
        ),
    ]);

    assert!(advisory_messages(&reporter).is_empty());
}

#[test]
fn clears_when_the_dependent_is_superseded_with_no_registered_type() {
    let reporter = run_over(vec![
        (
            "/bundle/misc/0001-a.md",
            "---\ntype: Playbook\nstatus: Superseded\n---\n\n[b](./0002-b.md)\n",
        ),
        (
            "/bundle/misc/0002-b.md",
            "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0003\n---\n# B\n",
        ),
    ]);

    assert!(advisory_messages(&reporter).is_empty());
}

#[test]
fn never_flags_accepted_proposed_or_open_targets_or_non_record_links() {
    let reporter = run_over(vec![
        (
            "/bundle/adr/0001-a.md",
            "---\ntype: ADR\nstatus: Accepted\n---\n\n[b](./0002-b.md) [c](./0003-c.md) [ext](https://example.com)\n",
        ),
        (
            "/bundle/adr/0002-b.md",
            "---\ntype: ADR\nstatus: Accepted\n---\n# B\n",
        ),
        (
            "/bundle/adr/0003-c.md",
            "---\ntype: ADR\nstatus: Proposed\n---\n# C\n",
        ),
    ]);

    assert!(advisory_messages(&reporter).is_empty());
}

#[test]
fn a_deprecated_target_without_superseded_by_reports_without_a_successor_clause() {
    let reporter = run_over(vec![
        (
            "/bundle/adr/0001-a.md",
            "---\ntype: ADR\nstatus: Accepted\n---\n\n[b](./0002-b.md)\n",
        ),
        (
            "/bundle/adr/0002-b.md",
            "---\ntype: ADR\nstatus: Deprecated\n---\n# B\n",
        ),
    ]);

    let messages = advisory_messages(&reporter);
    assert_eq!(messages.len(), 1);
    assert!(messages[0].contains("Deprecated"));
    assert!(!messages[0].contains("superseded by"));
}

#[test]
fn a_rejected_target_produces_the_finding() {
    let reporter = run_over(vec![
        (
            "/bundle/adr/0001-a.md",
            "---\ntype: ADR\nstatus: Accepted\n---\n\n[b](./0002-b.md)\n",
        ),
        (
            "/bundle/adr/0002-b.md",
            "---\ntype: ADR\nstatus: Rejected\n---\n# B\n",
        ),
    ]);

    let messages = advisory_messages(&reporter);
    assert_eq!(messages.len(), 1);
    assert!(messages[0].contains("Rejected"));
}

#[test]
fn a_moved_source_finding_never_touches_the_exit_code() {
    let reporter = run_over(vec![
        (
            "/bundle/adr/0001-a.md",
            "---\ntype: ADR\nstatus: Accepted\n---\n\n[b](./0002-b.md)\n",
        ),
        (
            "/bundle/adr/0002-b.md",
            "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0003\n---\n# B\n",
        ),
    ]);

    assert!(!advisory_messages(&reporter).is_empty());
    assert!(reporter.violations.is_empty());
}
