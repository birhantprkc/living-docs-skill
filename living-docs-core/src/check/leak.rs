//! `LEAK` advisories (ADR 0054): content that landed in the wrong record
//! type, detected by regex and structure only — no LLM. Advisory, never
//! gating (same posture as `SIZE`/`LIVENESS`). Each finding names the record
//! and the type the content belongs in. The undetectable leaks stay in the
//! `doc-trail` leak table and never become a hard stop.
//!
//! Five detectable cases:
//!   - Given/When/Then scenario in an ADR or PRD body → belongs in a BDR.
//!   - A JSON/data fenced block in an ADR outside `## Verification` → an issue.
//!   - An unfilled `{{PLACEHOLDER}}` in any record.
//!   - A BDR carrying a G/W/T scenario with no `Proves:` line.
//!   - A deferred "Needs an ADR"/"Needs a BDR" inside an issue body.

use super::{file_name_str, records, Reporter};
use crate::record;
use crate::store::DocStore;
use regex::Regex;
use std::path::PathBuf;

pub(crate) fn check_leak(store: &dyn DocStore, all_md: &[PathBuf], reporter: &mut Reporter) {
    for f in all_md {
        if records::is_reserved(&file_name_str(f)) {
            continue;
        }
        let Ok(contents) = store.read(f) else {
            continue;
        };
        let rec = record::extract_record(f, &contents);
        for message in leaks(&rec.doc_type, &rec.body) {
            reporter.advise(f, message);
        }
    }
}

fn leaks(doc_type: &str, body: &str) -> Vec<String> {
    let mut out = Vec::new();
    if has_unfilled_placeholder(body) {
        out.push("LEAK unfilled {{PLACEHOLDER}} — fill it or remove the slot".to_string());
    }
    match doc_type {
        "ADR" => push_adr_leaks(body, &mut out),
        "PRD" if has_gwt_scenario(&without_code(body)) => out.push(gwt_message()),
        "BDR" if has_gwt_scenario(&without_code(body)) && !mentions_proves(body) => {
            out.push("LEAK BDR scenario has no `Proves:` line — cite the requirement each proves".to_string());
        }
        "Issue" if defers_a_decision(body) => out.push(
            "LEAK deferred decision (\"Needs an ADR/BDR\") — decide in the issue now, or open the record now".to_string(),
        ),
        _ => {}
    }
    out
}

fn push_adr_leaks(body: &str, out: &mut Vec<String>) {
    let prose = decision_prose(body);
    if has_gwt_scenario(&without_code(&prose)) {
        out.push(gwt_message());
    }
    if has_data_fence(&prose) {
        out.push("LEAK data/JSON block in an ADR — test output and benchmarks belong in an issue or research".to_string());
    }
}

fn gwt_message() -> String {
    "LEAK Given/When/Then scenario — target behavior belongs in a BDR, not an ADR/PRD".to_string()
}

/// The body with the `## Verification` section removed (from that heading to
/// the next `## ` / `# References` / end) — Implementation-impact paths and
/// checkable criteria legitimately carry code and "Given" prose, so they are
/// out of scope for the ADR leak checks.
fn decision_prose(body: &str) -> String {
    let mut out = Vec::new();
    let mut skipping = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## Verification") {
            skipping = true;
            continue;
        }
        if skipping && (trimmed.starts_with("## ") || trimmed == "# References") {
            skipping = false;
        }
        if !skipping {
            out.push(line);
        }
    }
    out.join("\n")
}

/// `text` with fenced ``` blocks and inline `code` spans removed, so a doc that
/// legitimately *shows* placeholder or scenario syntax inside code formatting
/// is not mistaken for one that leaked it into prose.
fn without_code(text: &str) -> String {
    let mut out = String::new();
    let mut in_fence = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            out.push_str(&strip_inline_code(line));
            out.push('\n');
        }
    }
    out
}

fn strip_inline_code(line: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    for ch in line.chars() {
        if ch == '`' {
            in_code = !in_code;
        } else if !in_code {
            out.push(ch);
        }
    }
    out
}

fn has_unfilled_placeholder(body: &str) -> bool {
    placeholder_re().is_match(&without_code(body))
}

/// A full Gherkin triple as step lines (`Given` … `When` … `Then`), tolerating
/// a leading list/number/bold marker — one stray "Given" in prose is not a
/// scenario.
fn has_gwt_scenario(text: &str) -> bool {
    let step = |kw: &str| {
        Regex::new(&format!(r"(?im)^\s*(?:[-*]\s*|\d+\.\s*)?\*{{0,2}}{kw}\b"))
            .is_ok_and(|re| re.is_match(text))
    };
    step("Given") && step("When") && step("Then")
}

fn has_data_fence(prose: &str) -> bool {
    let mut lines = prose.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```json") {
            return true;
        }
        if trimmed.starts_with("```") {
            let first = lines.peek().map(|l| l.trim_start()).unwrap_or("");
            if first.starts_with('{') || first.starts_with('[') {
                return true;
            }
        }
    }
    false
}

fn mentions_proves(body: &str) -> bool {
    body.to_lowercase().contains("proves:")
}

fn defers_a_decision(body: &str) -> bool {
    Regex::new(r"(?i)needs an?\s+(adr|bdr)").is_ok_and(|re| re.is_match(body))
}

fn placeholder_re() -> Regex {
    Regex::new(r"\{\{[A-Za-z0-9_ ]+\}\}").expect("static placeholder regex is valid")
}

#[cfg(test)]
mod tests;
