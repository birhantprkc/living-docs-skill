use super::*;
use crate::test_support::MapStore;
use std::collections::BTreeMap;

fn store_of(files: &[(&str, String)]) -> MapStore {
    let mut map = BTreeMap::new();
    for (path, contents) in files {
        map.insert(PathBuf::from(*path), contents.clone());
    }
    MapStore { files: map }
}

fn adr_words(n: usize) -> String {
    format!("---\ntype: ADR\n---\n\n## Context\n{}", "word ".repeat(n))
}

#[test]
fn word_budget_summary_aggregates_offenders_worst_first() {
    let store = store_of(&[
        ("docs/adr/0001-a.md", adr_words(400)),
        ("docs/adr/0002-b.md", adr_words(500)),
        ("docs/adr/0003-c.md", adr_words(10)),
    ]);
    let all: Vec<PathBuf> = store.files.keys().cloned().collect();
    let summary = word_budget_summary(&store, &all).expect("offenders present");
    assert!(
        summary.contains("2 ADR/BDR decision bodies"),
        "got: {summary}"
    );
    assert!(
        summary.find("0002-b.md").unwrap() < summary.find("0001-a.md").unwrap(),
        "worst offender first: {summary}"
    );
}

#[test]
fn word_budget_summary_is_none_when_all_within_budget() {
    let store = store_of(&[("docs/adr/0001-a.md", adr_words(10))]);
    let all: Vec<PathBuf> = store.files.keys().cloned().collect();
    assert!(word_budget_summary(&store, &all).is_none());
}

fn doc_with_body_lines(doc_type: &str, body_lines: usize) -> String {
    let body = (0..body_lines)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("---\ntype: {doc_type}\n---\n{body}")
}

#[test]
fn body_line_count_excludes_the_frontmatter_block() {
    assert_eq!(body_line_count("---\ntype: ADR\n---\none\ntwo\n"), 2);
}

#[test]
fn body_line_count_without_frontmatter_counts_every_line() {
    assert_eq!(body_line_count("one\ntwo\nthree\n"), 3);
}

#[test]
fn a_body_at_exactly_the_warn_threshold_is_not_flagged() {
    assert_eq!(
        over_target_body_lines(&doc_with_body_lines("ADR", 120)),
        None
    );
}

#[test]
fn a_body_one_line_over_the_warn_threshold_is_flagged_with_its_count() {
    assert_eq!(
        over_target_body_lines(&doc_with_body_lines("ADR", 121)),
        Some(121)
    );
}

#[test]
fn research_is_exempt_regardless_of_length() {
    assert_eq!(
        over_target_body_lines(&doc_with_body_lines("Research", 400)),
        None
    );
}

#[test]
fn a_short_adr_is_within_the_word_budget() {
    assert_eq!(
        over_word_budget("---\ntype: ADR\n---\n\n## Context\nA brief decision.\n"),
        None
    );
}

#[test]
fn a_long_adr_exceeds_the_word_budget() {
    let prose = "word ".repeat(400);
    let doc = format!("---\ntype: ADR\n---\n\n## Context\n{prose}");
    assert!(over_word_budget(&doc).is_some_and(|w| w >= 400));
}

#[test]
fn verification_and_references_prose_is_excluded_from_the_word_budget() {
    let prose = "word ".repeat(400);
    let doc = format!(
        "---\ntype: ADR\n---\n\n## Context\nshort.\n\n## Verification\n{prose}\n\n# References\n{prose}"
    );
    assert_eq!(over_word_budget(&doc), None);
}

#[test]
fn only_adr_and_bdr_carry_the_word_budget() {
    let prose = "word ".repeat(400);
    assert_eq!(
        over_word_budget(&format!("---\ntype: PRD\n---\n\n{prose}")),
        None
    );
    assert!(over_word_budget(&format!("---\ntype: BDR\n---\n\n{prose}")).is_some());
}

#[test]
fn a_type_absent_from_the_registry_is_exempt_regardless_of_length() {
    assert!(
        doc_type::spec_for_frontmatter("Context").is_none(),
        "fixture premise broken: `Context` is now a registered frontmatter value — pick another unregistered type",
    );
    assert_eq!(
        over_target_body_lines(&doc_with_body_lines("Context", 400)),
        None
    );
}

/// Proves `check::size` reads `doc_type::DOC_TYPES` rather than a
/// hardcoded list of which types get the size target — it does not, and
/// cannot, prove any single row's `body_size` verdict is *correct*. That
/// is held by the pinned tests beside it: `a_body_one_line_over_...`
/// pins ADR = Targeted, `research_is_exempt_regardless_of_length` pins
/// Research = Exempt.
#[test]
fn every_registry_row_is_flagged_exactly_when_its_body_size_is_targeted() {
    let mut saw_targeted = false;
    let mut saw_exempt = false;
    for spec in doc_type::DOC_TYPES {
        let over_target = over_target_body_lines(&doc_with_body_lines(spec.frontmatter, 121));
        match spec.body_size {
            BodySize::Targeted => {
                saw_targeted = true;
                assert_eq!(
                    over_target,
                    Some(121),
                    "{} is Targeted so it should carry the size target",
                    spec.frontmatter
                );
            }
            BodySize::Exempt => {
                saw_exempt = true;
                assert_eq!(
                    over_target, None,
                    "{} is Exempt so it should not carry the size target",
                    spec.frontmatter
                );
            }
        }
    }
    assert!(saw_targeted, "no Targeted row was exercised");
    assert!(saw_exempt, "no Exempt row was exercised");
}
