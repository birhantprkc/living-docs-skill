//! Advisory body-size check (issue 0009) — decision/execution records aim for
//! ~100 body lines; past 120 the check prints a `SIZE` note. Advisory only:
//! it never affects the exit code. Which doc types the target applies to is
//! decided per-row by `doc_type::DocTypeSpec::body_size` (ADR 0027), not by
//! this module.

use super::{file_name_str, records, Reporter};
use crate::doc_type::{self, BodySize};
use crate::frontmatter;
use crate::store::DocStore;
use std::path::PathBuf;

const AIM_LINES: usize = 100;
const WARN_LINES: usize = 120;

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
    }
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
