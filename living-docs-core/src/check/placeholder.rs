//! Unfilled-placeholder check (ADR 0057): a template slot left unfilled is a
//! mechanical defect, so `check` reports `{{PLACEHOLDER}}` as an invariant
//! violation. A slot may carry its authoring hint after the name
//! (`{{CONTEXT: the forces at play}}`), so the guidance lives in the slot
//! and disappears with it. Placeholders shown inside code formatting are
//! ignored, so a record that documents the slot syntax is not flagged.

use super::{file_name_str, records, Reporter};
use crate::record;
use crate::store::DocStore;
use regex::Regex;
use std::path::PathBuf;

pub(crate) fn check_placeholders(
    store: &dyn DocStore,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    for f in all_md {
        if records::is_reserved(&file_name_str(f)) {
            continue;
        }
        let Ok(contents) = store.read(f) else {
            continue;
        };
        let body = record::extract_record(f, &contents).body;
        if placeholder_re().is_match(&without_code(&body)) {
            reporter.report(f, "unfilled {{PLACEHOLDER}} — fill the slot or remove it");
        }
    }
}

/// `text` with fenced ``` blocks and inline `code` spans removed, so a record
/// that legitimately *shows* placeholder syntax inside code formatting is not
/// mistaken for one that left a slot unfilled.
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

fn placeholder_re() -> Regex {
    Regex::new(r"\{\{[^{}\n]+\}\}").expect("static placeholder regex is valid")
}

#[cfg(test)]
mod tests;
