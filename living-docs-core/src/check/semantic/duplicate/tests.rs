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
    check(&store, &all, &mut reporter);
    reporter
        .into_findings()
        .1
        .into_iter()
        .map(|(_, m)| m)
        .collect()
}

fn adr(number: &str, prose: &str) -> (String, String) {
    (
        format!("docs/adr/{number}-x.md"),
        format!("---\ntype: ADR\ntitle: t\nstatus: Accepted\n---\n\n## Decision\n\n{prose}\n"),
    )
}

#[test]
fn two_near_identical_records_of_the_same_type_are_flagged() {
    let prose = "the store port stays stable across every backend so callers never depend on a concrete adapter implementation detail";
    let a = adr("0001", prose);
    let b = adr("0002", prose);
    let out = advisories(&[(&a.0, &a.1), (&b.0, &b.1)]);
    assert!(out.iter().any(|m| m.contains("DUPLICATE")), "got: {out:?}");
}

#[test]
fn two_distinct_records_are_not_flagged() {
    let a = adr(
        "0001",
        "the storage backend is config-selected and mutually exclusive between file and database",
    );
    let b = adr(
        "0002",
        "mermaid validation runs in process via a pure rust parser rather than a docker shell out",
    );
    assert!(advisories(&[(&a.0, &a.1), (&b.0, &b.1)]).is_empty());
}

#[test]
fn records_of_different_types_are_never_compared() {
    let prose = "identical prose repeated across two records of different declared types entirely";
    let a = adr("0001", prose);
    let bdr = (
        "docs/bdr/0001-x.md".to_string(),
        format!("---\ntype: BDR\ntitle: t\nstatus: Accepted\n---\n\n## Scenarios\n\n{prose}\n"),
    );
    assert!(advisories(&[(&a.0, &a.1), (&bdr.0, &bdr.1)]).is_empty());
}

#[test]
fn shared_madr_headings_alone_do_not_make_records_duplicates() {
    let a = adr(
        "0001",
        "we chose sqlite for the local embedded read model because it needs no server",
    );
    let b = adr(
        "0002",
        "we chose axum for the web front because it reuses the one rust domain crate",
    );
    assert!(
        advisories(&[(&a.0, &a.1), (&b.0, &b.1)]).is_empty(),
        "boilerplate must be stripped"
    );
}
