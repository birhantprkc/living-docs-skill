//! The `stale-impact` half of record liveness (ADR 0049): does an accepted
//! record's `**Implementation impact:**` still name paths that exist? The
//! impact line is free-form prose, so this is deliberately conservative —
//! only tokens that look like a real repository path (a `/`-bearing token with
//! a file extension), not annotated `(removed)`, are resolved. Block parsing
//! and tokenization are shared with `why` via `crate::impact`.

use crate::impact;
use std::path::Path;

/// True when the record's `**Implementation impact:**` names a literal
/// repository path that no longer exists on disk, resolved against the
/// bundle's parent (the repository root).
pub(super) fn has_dead_impact_path(contents: &str, bundle: &str) -> bool {
    let root = Path::new(bundle).parent().unwrap_or(Path::new("."));
    impact::tokens(&impact::segment(contents))
        .iter()
        .filter(|token| !token.removed && is_path_shaped(&token.text))
        .any(|token| !root.join(&token.text).exists())
}

fn is_path_shaped(token: &str) -> bool {
    token.contains('/')
        && Path::new(token)
            .extension()
            .is_some_and(|ext| !ext.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_path_shaped_requires_a_slash_and_an_extension() {
        assert!(is_path_shaped("living-docs-core/src/a.rs"));
        assert!(is_path_shaped(".github/workflows/x.yml"));
        assert!(!is_path_shaped("living-docs-core"));
        assert!(!is_path_shaped("check"));
    }

    #[test]
    fn has_dead_impact_path_flags_a_missing_path_but_not_an_existing_one() {
        let root = std::env::temp_dir().join(format!("ld-liveness-{}", std::process::id()));
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("real.rs"), "fn main() {}").unwrap();

        let bundle = root.join("docs");
        let bundle_str = bundle.to_string_lossy();

        assert!(!has_dead_impact_path(
            "**Implementation impact:** `src/real.rs`.",
            &bundle_str
        ));
        assert!(has_dead_impact_path(
            "**Implementation impact:** `src/ghost.rs`.",
            &bundle_str
        ));
        assert!(!has_dead_impact_path(
            "**Implementation impact:** `src/ghost.rs` (removed).",
            &bundle_str
        ));

        std::fs::remove_dir_all(&root).ok();
    }
}
