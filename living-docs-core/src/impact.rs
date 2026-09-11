//! Shared parsing of a record's `**Implementation impact:**` block — the list
//! that both the stale-impact liveness check (ADR 0049) and the `why` reverse
//! index (ADR 0051) read. [`segment`] slices the block text; [`tokens`]
//! returns its backtick-quoted tokens, each carrying whether a removal
//! annotation follows it. Neither touches the filesystem.

/// One backtick-quoted token from an Implementation-impact block, with whether
/// a `(removed)`/`(deleted)`/`(gone)` annotation follows its closing backtick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub text: String,
    pub removed: bool,
}

/// The text of the `**Implementation impact:**` block: from that emphasized
/// label to the next bold label, heading, or blank line. The label must be
/// the emphasized form so the template's `<!-- … Implementation impact: … -->`
/// guidance comment (which carries an `e.g. \`src/store.py\`` example) is never
/// mistaken for the record's real impact line.
pub fn segment(contents: &str) -> String {
    let mut out = Vec::new();
    let mut in_block = false;
    for line in contents.lines() {
        if line.contains("**Implementation impact:") {
            in_block = true;
            out.push(line);
            continue;
        }
        if !in_block {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("**") || trimmed.starts_with('#') {
            break;
        }
        out.push(line);
    }
    out.join("\n")
}

/// Every backtick-quoted token in `segment`, in document order, each carrying
/// whether a removal annotation follows its closing backtick.
pub fn tokens(segment: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while let Some(open) = segment[cursor..].find('`') {
        let start = cursor + open + 1;
        let Some(rel_close) = segment[start..].find('`') else {
            break;
        };
        let end = start + rel_close;
        tokens.push(Token {
            text: segment[start..end].to_string(),
            removed: annotated_removed(segment, end + 1),
        });
        cursor = end + 1;
    }
    tokens
}

/// Whether the ~16 chars following a token's closing backtick carry a
/// `(removed)`/`(deleted)`/`(gone)` annotation — a path a decision deliberately
/// records as gone is not a stale impact.
fn annotated_removed(segment: &str, after: usize) -> bool {
    if after > segment.len() {
        return false;
    }
    let end = (after + 16).min(segment.len());
    let tail = segment[after..end].to_lowercase();
    tail.contains("(removed") || tail.contains("(deleted") || tail.contains("(gone")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_ignores_the_template_guidance_comment_example() {
        let contents = "<!-- Implementation impact: files, e.g. `src/store.py`. -->\n\n**Implementation impact:** `src/real.rs`.";
        let texts: Vec<String> = tokens(&segment(contents))
            .into_iter()
            .map(|t| t.text)
            .collect();
        assert_eq!(texts, vec!["src/real.rs".to_string()]);
    }

    #[test]
    fn segment_stops_at_the_next_bold_label() {
        let contents = "**Implementation impact:** `src/a.rs`.\n\n**Verification criteria:**\n- `src/nope.rs`\n";
        let seg = segment(contents);
        assert!(seg.contains("src/a.rs"));
        assert!(!seg.contains("nope"));
    }

    #[test]
    fn tokens_flag_a_removed_annotation() {
        let toks = tokens("**Implementation impact:** `src/old.rs` (removed), `src/new.rs`.");
        assert_eq!(toks[0].text, "src/old.rs");
        assert!(toks[0].removed);
        assert_eq!(toks[1].text, "src/new.rs");
        assert!(!toks[1].removed);
    }
}
