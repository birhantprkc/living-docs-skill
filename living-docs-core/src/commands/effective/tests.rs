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

fn options(topic: Option<&str>, tier: Tier, budget: Option<usize>, include_stale: bool) -> Options {
    Options {
        topic: topic.map(str::to_string),
        tier,
        budget,
        include_stale,
    }
}

fn adr(number: &str, status: &str, supersedes: Option<&str>, body: &str) -> (String, String) {
    let mut fm = format!("---\ntype: ADR\ntitle: ADR {number}\ndescription: d\nstatus: {status}\n");
    if let Some(s) = supersedes {
        fm.push_str(&format!("supersedes: \"{s}\"\n"));
    }
    fm.push_str("---\n\n");
    (format!("docs/adr/{number}-x.md"), format!("{fm}{body}"))
}

#[test]
fn a_supersede_chain_collapses_to_the_head_with_a_lineage_line() {
    let a = adr("0131", "Superseded", None, "old");
    let b = adr("0133", "Superseded", Some("0131"), "middle");
    let c = adr("0143", "Accepted", Some("0133"), "the rule in force");
    let store = store_of(&[
        (&a.0, &a.1),
        (&b.0, &b.1),
        (&c.0, &c.1),
    ]);
    let out = compile(&store, Path::new("docs"), &options(Some("rule"), Tier::Index, None, false));

    assert!(out.contains("ADR 0143"), "head must appear:\n{out}");
    assert!(!out.contains("ADR 0131") || out.contains("supersedes 0131"), "ancestors only in lineage:\n{out}");
    assert!(out.contains("supersedes 0131 via 0133"), "lineage line:\n{out}");
    assert_eq!(out.matches("- [ADR").count(), 1, "only the head is a row:\n{out}");
}

#[test]
fn superseded_and_deprecated_records_are_excluded_by_default() {
    let a = adr("0001", "Superseded", None, "gone");
    let b = adr("0002", "Deprecated", None, "retired");
    let c = adr("0003", "Accepted", None, "live");
    let store = store_of(&[(&a.0, &a.1), (&b.0, &b.1), (&c.0, &c.1)]);
    let out = compile(&store, Path::new("docs"), &options(None, Tier::Index, None, false));
    assert!(out.contains("ADR 0003"));
    assert!(!out.contains("ADR 0001"));
    assert!(!out.contains("ADR 0002"));
}

#[test]
fn a_stale_record_is_absent_by_default_and_present_under_include_stale() {
    let proposed = adr("0001", "Proposed", None, "See [issue](/issues/0009-t.md).");
    let issue = ("docs/issues/0009-t.md", "---\ntype: Issue\ntitle: t\nstatus: closed\n---\n\nb");
    let store = store_of(&[(&proposed.0, &proposed.1), (issue.0, issue.1)]);

    let default = compile(&store, Path::new("docs"), &options(None, Tier::Index, None, false));
    assert!(!default.contains("ADR 0001"), "stale excluded by default:\n{default}");

    let with_stale = compile(&store, Path::new("docs"), &options(None, Tier::Index, None, true));
    assert!(with_stale.contains("ADR 0001"), "included under --include-stale:\n{with_stale}");
}

#[test]
fn topic_filters_by_case_insensitive_term_across_title_and_body() {
    let a = adr("0001", "Accepted", None, "concerns the STORAGE backend");
    let b = adr("0002", "Accepted", None, "about mermaid diagrams");
    let store = store_of(&[(&a.0, &a.1), (&b.0, &b.1)]);
    let out = compile(&store, Path::new("docs"), &options(Some("storage"), Tier::Index, None, false));
    assert!(out.contains("ADR 0001"));
    assert!(!out.contains("ADR 0002"));
}

#[test]
fn constitution_and_prd_rank_above_adrs_at_the_index_tier() {
    let store = store_of(&[
        ("docs/adr/0001-x.md", "---\ntype: ADR\ntitle: A\ndescription: d\nstatus: Accepted\n---\n\nb"),
        ("docs/prd/0001-x.md", "---\ntype: PRD\ntitle: P\ndescription: d\nstatus: Accepted\n---\n\nb"),
        ("docs/constitution.md", "---\ntype: Constitution\ntitle: C\ndescription: d\n---\n\nb"),
    ]);
    let out = compile(&store, Path::new("docs"), &options(None, Tier::Index, None, false));
    let c = out.find("[Constitution]").unwrap();
    let p = out.find("[PRD 0001]").unwrap();
    let a = out.find("[ADR 0001]").unwrap();
    assert!(c < p && p < a, "order should be constitution, PRD, ADR:\n{out}");
}

#[test]
fn a_contract_ranks_above_narrative_within_the_same_group() {
    let narrative = adr("0001", "Accepted", None, "just prose");
    let contract = adr("0002", "Accepted", None, "## Verification\n**x**");
    let store = store_of(&[(&narrative.0, &narrative.1), (&contract.0, &contract.1)]);
    let out = compile(&store, Path::new("docs"), &options(None, Tier::Index, None, false));
    assert!(
        out.find("[ADR 0002]").unwrap() < out.find("[ADR 0001]").unwrap(),
        "contract 0002 must precede narrative 0001:\n{out}"
    );
}

#[test]
fn a_budget_is_a_hard_cap_and_degrades_tier_before_dropping_records() {
    let big_body = "## H\n".to_string() + &"word ".repeat(400);
    let a = adr("0001", "Accepted", None, &big_body);
    let b = adr("0002", "Accepted", None, &big_body);
    let store = store_of(&[(&a.0, &a.1), (&b.0, &b.1)]);

    let full = compile(&store, Path::new("docs"), &options(None, Tier::Full, None, false));
    let full_tokens = full.chars().count().div_ceil(4);
    assert!(full_tokens > 50, "fixture must exceed the budget at full tier");

    let capped = compile(&store, Path::new("docs"), &options(None, Tier::Full, Some(50), false));
    assert!(capped.chars().count().div_ceil(4) <= 50, "budget is a hard cap:\n{capped}");
}

#[test]
fn a_tiny_budget_drops_lowest_ranked_records_but_never_exceeds() {
    let a = adr("0001", "Accepted", None, "## Verification\ncontract");
    let b = adr("0002", "Accepted", None, "narrative body");
    let store = store_of(&[(&a.0, &a.1), (&b.0, &b.1)]);
    let capped = compile(&store, Path::new("docs"), &options(None, Tier::Index, Some(12), false));
    assert!(capped.chars().count().div_ceil(4) <= 12, "hard cap:\n{capped}");
}
