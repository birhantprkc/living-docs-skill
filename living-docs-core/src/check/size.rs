//! Advisory body-size check (issue 0009) — decision/execution records aim for
//! ~100 body lines; past 120 the check prints a per-record `SIZE` note. It
//! also carries a tighter word budget for ADR/BDR decision prose (ADR 0052):
//! ADR/BDR bodies past ~300 words across Context/Decision/Consequences
//! (Verification and References excluded) surface as **one aggregated `SIZE`
//! summary** (count + worst offenders), not a line per record, so a corpus of
//! long legacy records does not flood `check`. Advisory only: it never affects
//! the exit code. Which doc types the line target applies to is decided per-row
//! by `doc_type::DocTypeSpec::body_size` (ADR 0027), not by this module.

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
    }
}

/// Every ADR/BDR whose decision prose exceeds the word budget, paired with its
/// word count, sorted worst-first. The decision-prose budget is reported as a
/// single aggregated summary (see [`word_budget_summary`]) rather than one
/// advisory per record, so a corpus of long legacy records surfaces as one
/// line instead of flooding `check` (ADR 0052; noise fix).
pub(crate) fn word_budget_offenders(
    store: &dyn DocStore,
    all_md: &[PathBuf],
) -> Vec<(PathBuf, usize)> {
    let mut offenders: Vec<(PathBuf, usize)> = all_md
        .iter()
        .filter(|f| !records::is_reserved(&file_name_str(f)))
        .filter_map(|f| {
            let content = store.read(f).ok()?;
            over_word_budget(&content).map(|words| (f.clone(), words))
        })
        .collect();
    offenders.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    offenders
}

/// A one-line summary of the decision-prose word budget, or `None` when every
/// ADR/BDR is within budget. Names the count and the three worst offenders so
/// the signal survives without a line per record.
pub(crate) fn word_budget_summary(store: &dyn DocStore, all_md: &[PathBuf]) -> Option<String> {
    let offenders = word_budget_offenders(store, all_md);
    if offenders.is_empty() {
        return None;
    }
    let worst: Vec<String> = offenders
        .iter()
        .take(3)
        .map(|(path, words)| format!("{} {words}w", file_name_str(path)))
        .collect();
    Some(format!(
        "SIZE {} ADR/BDR decision bodies exceed the ~{WORD_BUDGET}-word budget (Context+Decision+Consequences); worst: {} — length is not decision quality (ADR 0052)",
        offenders.len(),
        worst.join(", ")
    ))
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
mod tests;
