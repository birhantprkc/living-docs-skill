use super::*;
use crate::test_support::MapStore;
use std::collections::BTreeMap;

fn store_of(files: &[(&str, &str)]) -> MapStore {
    let mut map = BTreeMap::new();
    for (path, contents) in files {
        map.insert(std::path::PathBuf::from(path), (*contents).to_string());
    }
    MapStore { files: map }
}

fn record(doc_type: &str, dir: &str, number: &str, status: &str, timestamp: &str) -> (String, String) {
    (
        format!("docs/{dir}/{number}-x.md"),
        format!("---\ntype: {doc_type}\ntitle: t\ndescription: d\nstatus: {status}\ntimestamp: {timestamp}\n---\n\nbody"),
    )
}

fn compute_of(files: &[(&str, &str)]) -> Inflation {
    compute(&store_of(files), std::path::Path::new("docs"))
}

#[test]
fn pairing_ratio_is_the_share_of_adrs_with_a_same_numbered_bdr() {
    let a1 = record("ADR", "adr", "0001", "Accepted", "2026-01-01T00:00:00Z");
    let a2 = record("ADR", "adr", "0002", "Accepted", "2026-01-01T00:00:00Z");
    let b1 = record("BDR", "bdr", "0001", "Accepted", "2026-01-01T00:00:00Z");
    let inflation = compute_of(&[(&a1.0, &a1.1), (&a2.0, &a2.1), (&b1.0, &b1.1)]);
    assert_eq!(inflation.adrs, 2);
    assert_eq!(inflation.adrs_with_matching_bdr, 1);
    assert_eq!(inflation.pairing_ratio(), Some(0.5));
}

#[test]
fn pairing_ratio_is_none_without_any_adr() {
    let inflation = compute_of(&[]);
    assert_eq!(inflation.pairing_ratio(), None);
}

#[test]
fn superseded_records_are_counted() {
    let a1 = record("ADR", "adr", "0001", "Superseded", "2026-01-01T00:00:00Z");
    let a2 = record("ADR", "adr", "0002", "Accepted", "2026-01-01T00:00:00Z");
    let inflation = compute_of(&[(&a1.0, &a1.1), (&a2.0, &a2.1)]);
    assert_eq!(inflation.superseded, 1);
}

#[test]
fn recent_adrs_are_counted_within_thirty_days_of_the_newest() {
    let old = record("ADR", "adr", "0001", "Accepted", "2026-01-01T00:00:00Z");
    let recent = record("ADR", "adr", "0002", "Accepted", "2026-03-01T00:00:00Z");
    let newest = record("ADR", "adr", "0003", "Accepted", "2026-03-20T00:00:00Z");
    let inflation = compute_of(&[(&old.0, &old.1), (&recent.0, &recent.1), (&newest.0, &newest.1)]);
    assert_eq!(inflation.adrs, 3);
    assert_eq!(inflation.adrs_recent, 2, "0002 and 0003 fall within 30d of 0003");
}

#[test]
fn proposed_stale_reflects_liveness() {
    let adr = (
        "docs/adr/0001-x.md",
        "---\ntype: ADR\ntitle: t\ndescription: d\nstatus: Proposed\ntimestamp: 2026-01-01T00:00:00Z\n---\n\nSee [issue](/issues/0009-t.md).",
    );
    let issue = ("docs/issues/0009-t.md", "---\ntype: Issue\ntitle: t\nstatus: closed\n---\n\nb");
    let inflation = compute_of(&[adr, issue]);
    assert_eq!(inflation.proposed_stale, 1);
}

#[test]
fn render_line_names_every_signal() {
    let inflation = Inflation {
        adrs: 4,
        adrs_with_matching_bdr: 4,
        adrs_recent: 2,
        superseded: 1,
        proposed_stale: 0,
    };
    let line = render_line(&inflation);
    assert!(line.contains("4 ADRs"));
    assert!(line.contains("ADR:BDR pairing 1.00"));
    assert!(line.contains("1 superseded"));
    assert!(line.contains("0 proposed-stale"));
}
