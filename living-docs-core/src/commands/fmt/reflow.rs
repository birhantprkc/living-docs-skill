//! Body-only prose reflow for `living-docs fmt` (ADR 0046). Joins each
//! hard-wrapped prose block -- a paragraph, or a list item plus its
//! indented continuation lines -- into one line with a single space at
//! each former line break. Every non-prose construct (fenced and indented
//! code, tables, headings, blockquotes, thematic breaks, setext
//! underlines, HTML blocks and multi-line HTML comments, link-reference
//! definitions, and lines ending in a hard-break marker) stays byte
//! identical. Applied only to [`crate::record::ExtractedRecord::body`];
//! frontmatter, `check`, and the seal never see this pass.

use regex::Regex;
use std::sync::OnceLock;

fn list_item_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"^\s*(?:[-*+]|\d+[.)])\s+").expect("list-item pattern is a valid literal")
    })
}

fn link_reference_definition_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"^\s{0,3}\[[^\]]+\]:").expect("link-reference pattern is a valid literal")
    })
}

/// Joins every hard-wrapped prose block in `body` into one line, leaving
/// every non-prose construct byte identical. Idempotent: reflowing an
/// already-reflowed body returns it unchanged.
pub(super) fn reflow_body(body: &str) -> String {
    let ends_with_newline = body.ends_with('\n');
    let mut reflower = Reflower::default();
    for line in body.lines() {
        reflower.push_line(line);
    }
    reflower.finish(ends_with_newline)
}

#[derive(Default)]
struct Reflower {
    out: Vec<String>,
    pending: Vec<String>,
    fence: Option<String>,
    in_comment: bool,
    in_indented_code: bool,
}

impl Reflower {
    fn push_line(&mut self, line: &str) {
        if self.push_open_construct(line) {
            return;
        }
        self.push_content(line);
    }

    /// Handles the four constructs that can already be open, or that a
    /// line opens: an open fence, an open comment, a new fence marker, and
    /// a new multi-line comment opener. Returns whether one of them
    /// consumed `line`.
    fn push_open_construct(&mut self, line: &str) -> bool {
        self.emit_inside_fence(line)
            || self.emit_inside_comment(line)
            || self.open_fence(line)
            || self.open_comment(line)
    }

    /// Handles ordinary content once no fence or comment is open or
    /// opening: blank lines, indented code, standalone constructs, and
    /// prose or list items.
    fn push_content(&mut self, line: &str) {
        if line.trim().is_empty() {
            self.flush();
            self.in_indented_code = false;
            self.out.push(String::new());
            return;
        }
        if self.in_indented_code {
            self.out.push(line.to_owned());
            return;
        }
        if self.starts_indented_code(line) {
            self.in_indented_code = true;
            self.out.push(line.to_owned());
            return;
        }
        if is_standalone(line) {
            self.flush();
            self.out.push(line.to_owned());
            return;
        }
        self.push_prose_or_list(line);
    }

    /// Emits `line` as-is when a fence is open, closing the fence when
    /// `line` carries a matching closing marker. Returns whether a fence
    /// was open.
    fn emit_inside_fence(&mut self, line: &str) -> bool {
        let Some(marker) = &self.fence else {
            return false;
        };
        let closes = line.trim_start().starts_with(marker.as_str());
        self.out.push(line.to_owned());
        if closes {
            self.fence = None;
        }
        true
    }

    /// Emits `line` as-is when an HTML comment is open, closing it when
    /// `line` carries `-->`. Returns whether a comment was open.
    fn emit_inside_comment(&mut self, line: &str) -> bool {
        if !self.in_comment {
            return false;
        }
        self.out.push(line.to_owned());
        if line.contains("-->") {
            self.in_comment = false;
        }
        true
    }

    /// Flushes pending prose and opens a fence when `line` carries a
    /// fenced-code marker. Returns whether a fence was opened.
    fn open_fence(&mut self, line: &str) -> bool {
        let Some(marker) = fence_marker(line) else {
            return false;
        };
        self.flush();
        self.fence = Some(marker.to_owned());
        self.out.push(line.to_owned());
        true
    }

    /// Flushes pending prose and opens a multi-line HTML comment when
    /// `line` opens one without closing it on the same line. Returns
    /// whether a comment was opened.
    fn open_comment(&mut self, line: &str) -> bool {
        if !line.contains("<!--") || line.contains("-->") {
            return false;
        }
        self.flush();
        self.in_comment = true;
        self.out.push(line.to_owned());
        true
    }

    /// True for an indented line that opens an indented code block: it
    /// follows a blank line (or starts the body) and no prose block is
    /// already pending.
    fn starts_indented_code(&self, line: &str) -> bool {
        let after_blank_or_start = self.out.last().map(|last| last.is_empty()).unwrap_or(true);
        is_indented(line) && self.pending.is_empty() && after_blank_or_start
    }

    /// Appends `line` to the pending prose block, starting a new list
    /// item block when `line` opens one, then flushes immediately when
    /// `line` carries a hard-break marker.
    fn push_prose_or_list(&mut self, line: &str) {
        let hard_break = ends_with_hard_break(line);
        if is_list_item(line) {
            self.flush();
            self.pending.push(store_line(line, true, hard_break));
        } else if self.pending.is_empty() {
            self.pending.push(store_line(line, true, hard_break));
        } else {
            self.pending.push(store_line(line, false, hard_break));
        }
        if hard_break {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        self.out.push(self.pending.join(" "));
        self.pending.clear();
    }

    fn finish(mut self, ends_with_newline: bool) -> String {
        self.flush();
        let mut result = self.out.join("\n");
        if ends_with_newline {
            result.push('\n');
        }
        result
    }
}

/// Stores a prose line for the pending block: the first line of a block
/// keeps its leading indentation, a continuation line has its leading
/// indentation trimmed. Trailing whitespace is trimmed unless `line`
/// carries a hard-break marker, which stays exact.
fn store_line(line: &str, is_first: bool, hard_break: bool) -> String {
    let leading_kept = if is_first { line } else { line.trim_start() };
    if hard_break {
        leading_kept.to_owned()
    } else {
        leading_kept.trim_end().to_owned()
    }
}

fn fence_marker(line: &str) -> Option<&'static str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("```") {
        Some("```")
    } else if trimmed.starts_with("~~~") {
        Some("~~~")
    } else {
        None
    }
}

fn is_standalone(line: &str) -> bool {
    is_heading(line)
        || is_table_row(line)
        || is_blockquote(line)
        || is_thematic_or_setext(line)
        || is_link_reference_definition(line)
        || is_html_block_start(line)
}

fn is_heading(line: &str) -> bool {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    indent <= 3 && trimmed.starts_with('#')
}

fn is_table_row(line: &str) -> bool {
    line.trim_start().starts_with('|')
}

fn is_blockquote(line: &str) -> bool {
    line.trim_start().starts_with('>')
}

fn is_thematic_or_setext(line: &str) -> bool {
    let trimmed = line.trim();
    let significant: Vec<char> = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    if significant.len() < 3 {
        return false;
    }
    let first = significant[0];
    matches!(first, '-' | '=' | '*' | '_') && significant.iter().all(|&c| c == first)
}

fn is_link_reference_definition(line: &str) -> bool {
    link_reference_definition_pattern().is_match(line)
}

fn is_html_block_start(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with('<') && !trimmed.starts_with("<!--")
}

fn is_list_item(line: &str) -> bool {
    list_item_pattern().is_match(line)
}

fn ends_with_hard_break(line: &str) -> bool {
    line.ends_with("  ") || line.ends_with('\\')
}

fn is_indented(line: &str) -> bool {
    line.starts_with("    ") || line.starts_with('\t')
}

#[cfg(test)]
mod tests;
