//! The `stale-impact` half of record liveness (ADR 0049): does an accepted
//! record's `**Implementation impact:**` still name paths that exist? The
//! impact line is free-form prose, so extraction is deliberately conservative
//! — only backtick-quoted tokens that look like a real repository path (a
//! `/`-bearing token with a file extension), not annotated `(removed)`, are
//! resolved. A descriptive impact phrase never trips the advisory.

use std::path::Path;

/// True when the record's `**Implementation impact:**` names a literal
/// repository path that no longer exists on disk, resolved against the
/// bundle's parent (the repository root).
pub(super) fn has_dead_impact_path(contents: &str, bundle: &str) -> bool {
    let root = Path::new(bundle).parent().unwrap_or(Path::new("."));
    impact_paths(&impact_segment(contents))
        .iter()
        .any(|rel| !root.join(rel).exists())
}

/// The text of the `**Implementation impact:**` block: from that emphasized
/// label to the next bold label, heading, or blank line. The label must be
/// the emphasized form so the template's `<!-- … Implementation impact: … -->`
/// guidance comment (which carries an `e.g. \`src/store.py\`` example) is never
/// mistaken for the record's real impact line.
fn impact_segment(contents: &str) -> String {
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

/// Every backtick-quoted token in `segment` that is path-shaped (contains `/`
/// and a file extension) and not annotated as removed.
fn impact_paths(segment: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let bytes = segment.as_bytes();
    let mut cursor = 0;
    while let Some(open) = segment[cursor..].find('`') {
        let start = cursor + open + 1;
        let Some(rel_close) = segment[start..].find('`') else {
            break;
        };
        let end = start + rel_close;
        let token = &segment[start..end];
        if is_path_shaped(token) && !annotated_removed(segment, end + 1, bytes) {
            paths.push(token.to_string());
        }
        cursor = end + 1;
    }
    paths
}

fn is_path_shaped(token: &str) -> bool {
    token.contains('/')
        && Path::new(token)
            .extension()
            .is_some_and(|ext| !ext.is_empty())
}

/// Whether the ~16 chars following a token's closing backtick carry a
/// `(removed)`/`(deleted)`/`(gone)` annotation — a path a decision deliberately
/// records as gone is not a stale impact.
fn annotated_removed(segment: &str, after: usize, bytes: &[u8]) -> bool {
    if after > bytes.len() {
        return false;
    }
    let end = (after + 16).min(bytes.len());
    let tail = segment[after..end].to_lowercase();
    tail.contains("(removed") || tail.contains("(deleted") || tail.contains("(gone")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impact_paths_takes_only_path_shaped_tokens_and_skips_descriptive_ones() {
        let segment = "**Implementation impact:** `living-docs-core/src/a.rs`, the `check` pass, `living-docs-core` crate, `.github/workflows/x.yml`.";
        assert_eq!(
            impact_paths(segment),
            vec![
                "living-docs-core/src/a.rs".to_string(),
                ".github/workflows/x.yml".to_string()
            ]
        );
    }

    #[test]
    fn impact_paths_skips_a_path_annotated_as_removed() {
        let segment = "**Implementation impact:** `src/old.rs` (removed), `src/new.rs`.";
        assert_eq!(impact_paths(segment), vec!["src/new.rs".to_string()]);
    }

    #[test]
    fn impact_segment_stops_at_the_next_bold_label() {
        let contents = "## Verification\n\n**Implementation impact:** `src/a.rs`.\n\n**Verification criteria:**\n- `src/should-not-appear.rs`\n";
        let segment = impact_segment(contents);
        assert!(segment.contains("src/a.rs"));
        assert!(!segment.contains("should-not-appear"));
    }

    #[test]
    fn impact_segment_ignores_the_template_guidance_comment_example() {
        let contents = "<!-- Implementation impact: files, e.g. `src/store.py`. -->\n\n**Implementation impact:** `src/real.rs`.";
        assert_eq!(impact_paths(&impact_segment(contents)), vec!["src/real.rs".to_string()]);
    }

    #[test]
    fn has_dead_impact_path_flags_a_missing_path_but_not_an_existing_one() {
        let root = std::env::temp_dir().join(format!("ld-liveness-{}", std::process::id()));
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("real.rs"), "fn main() {}").unwrap();

        let bundle = root.join("docs");
        let bundle_str = bundle.to_string_lossy();

        assert!(!has_dead_impact_path("**Implementation impact:** `src/real.rs`.", &bundle_str));
        assert!(has_dead_impact_path("**Implementation impact:** `src/ghost.rs`.", &bundle_str));

        std::fs::remove_dir_all(&root).ok();
    }
}
