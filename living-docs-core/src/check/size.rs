//! Advisory body-size check (issue 0009) — decision/execution records aim for
//! ~100 body lines; past 120 the check prints a `SIZE` note. It also carries
//! a tighter word budget for ADR/BDR decision prose (ADR 0052): past ~300
//! words across Context/Decision/Consequences (Verification and References
//! excluded) a `SIZE` note fires, because decision quality comes from the
//! grilling step, not from length. Advisory only: it never affects the exit
//! code. Which doc types the line target applies to is decided per-row by
//! `doc_type::DocTypeSpec::body_size` (ADR 0027), not by this module.

use super::{file_name_str, records, Reporter};
use crate::doc_type::{self, BodySize};
use crate::frontmatter;
use crate::store::DocStore;
use std::path::PathBuf;

const AIM_LINES: usize = 100;
const WARN_LINES: usize = 120;
const WORD_BUDGET: usize = 300;

pub(crate) fn check_body_size(store: &dyn DocStore, all_md: &[PathBuf], reporter: &mut Reporter) {
    for f in all_md {
        if records::is_reserved(&file_name_str(f)) {
            continue;
        }
        let Ok(content) = store.read(f) else {
            continue;
        };
        if let Some(lines) = over_target_body_lines(&content) {
            reporter.advise(
                f,
                format!("SIZE body {lines} lines exceeds the {WARN_LINES}-line advisory target (aim ~{AIM_LINES})"),
            );
        }
        if let Some(words) = over_word_budget(&content) {
            reporter.advise(
                f,
                format!("SIZE decision prose {words} words exceeds the ~{WORD_BUDGET}-word budget (Context+Decision+Consequences); length is not decision quality (ADR 0052)"),
            );
        }
    }
}

/// The word count of an ADR/BDR's decision prose when it exceeds the budget.
/// `None` for any other type or a within-budget record.
fn over_word_budget(content: &str) -> Option<usize> {
    let doc_type = frontmatter::read_scalar_from_str(content, "type")?;
    if !matches!(doc_type.as_str(), "ADR" | "BDR") {
        return None;
    }
    let words = decision_prose(content).split_whitespace().count();
    (words > WORD_BUDGET).then_some(words)
}

/// The body with the Verification block, the References section, and HTML
/// comments removed — the Context/Decision/Consequences prose the word budget
/// governs.
fn decision_prose(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let start = body_start_index(&lines);
    let mut out: Vec<&str> = Vec::new();
    let mut in_comment = false;
    for line in &lines[start..] {
        let trimmed = line.trim();
        if trimmed.starts_with("## Verification") || trimmed == "# References" {
            break;
        }
        if in_comment {
            in_comment = !trimmed.contains("-->");
            continue;
        }
        if trimmed.starts_with("<!--") {
            in_comment = !trimmed.contains("-->");
            continue;
        }
        out.push(line);
    }
    out.join(" ")
}

fn over_target_body_lines(content: &str) -> Option<usize> {
    let doc_type = frontmatter::read_scalar_from_str(content, "type")?;
    let spec = doc_type::spec_for_frontmatter(&doc_type)?;
    (spec.body_size == BodySize::Targeted)
        .then(|| body_line_count(content))
        .filter(|lines| *lines > WARN_LINES)
}

fn body_line_count(content: &str) -> usize {
    let lines: Vec<&str> = content.lines().collect();
    lines.len() - body_start_index(&lines)
}

fn body_start_index(lines: &[&str]) -> usize {
    if lines.first() != Some(&"---") {
        return 0;
    }
    lines
        .iter()
        .skip(1)
        .position(|&l| l == "---")
        .map_or(0, |close| close + 2)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
