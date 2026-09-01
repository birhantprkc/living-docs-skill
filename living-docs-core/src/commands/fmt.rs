//! `living-docs fmt` (ADR 0019 slice S2, ADR 0046): canonicalizes every
//! concept record's frontmatter in place, and unwraps hard-wrapped prose in
//! its body. Enumerates the bundle through the same `DocStore::list` call
//! `check::run` reads from — no second directory walker — then rewrites
//! each record through [`crate::record::extract_record`] ->
//! [`reflow::reflow_body`] -> [`crate::record::to_canonical_markdown`], the
//! round-trip primitives ADR 0019's canonical check (S3) validates against.
//! File-mode only: db-mode is canonical by construction on export (ADR
//! 0007), so this verb never runs against `--backend db`.

mod reflow;

use crate::record::{extract_record, to_canonical_markdown};
use crate::store::DocStore;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Rewrites every non-canonical record under `bundle` to its canonical
/// frontmatter form, printing each rewritten path followed by a summary
/// count. A record already in canonical form, or carrying no frontmatter
/// block at all, is left untouched — `fmt` never fabricates frontmatter
/// (the determinism boundary, CLAUDE.md rule 4).
pub fn run(store: &dyn DocStore, bundle: &Path) -> ExitCode {
    if !bundle.is_dir() {
        eprintln!(
            "living-docs fmt: bundle root not found: {}",
            bundle.display()
        );
        return ExitCode::from(2);
    }

    let all_md = store.list(bundle).unwrap_or_default();
    let rewritten = canonicalize_bundle(store, &all_md);

    println!("{rewritten} record(s) rewritten.");
    ExitCode::SUCCESS
}

/// Canonicalizes every non-reserved record in `all_md`, printing each
/// rewritten path as it happens, and returns how many were rewritten.
fn canonicalize_bundle(store: &dyn DocStore, all_md: &[PathBuf]) -> usize {
    let mut rewritten = 0;
    for path in all_md {
        if is_reserved_file(path) {
            continue;
        }
        if canonicalize_record(store, path) {
            println!("{}", path.display());
            rewritten += 1;
        }
    }
    rewritten
}

/// `index.md`/`log.md` carry no frontmatter and are never part of the
/// record domain (mirrors `check::records::is_reserved`).
fn is_reserved_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("index.md") | Some("log.md")
    )
}

/// Rewrites `path` to its canonical form when it already carries a
/// frontmatter block whose canonical re-serialization differs byte for
/// byte from its current contents. Returns whether a write happened.
fn canonicalize_record(store: &dyn DocStore, path: &Path) -> bool {
    let Ok(contents) = store.read(path) else {
        return false;
    };
    if !has_frontmatter(&contents) {
        return false;
    }
    let normalized = normalize_frontmatter_gap(&contents);
    let mut record = extract_record(path, &normalized);
    record.body = reflow::reflow_body(&record.body);
    let canonical = to_canonical_markdown(&record);
    if canonical == contents {
        return false;
    }
    store.write(path, &canonical).is_ok()
}

fn has_frontmatter(contents: &str) -> bool {
    contents.lines().next() == Some("---")
}

/// Collapses any blank line(s) between the closing frontmatter fence and
/// the body down to a single newline, before `contents` reaches
/// [`crate::record::extract_record`]. Without this, an already-canonical
/// file (whose fence is followed by exactly one blank line, since
/// [`crate::record::to_canonical_markdown`] always inserts one) would
/// extract a body carrying a leftover leading newline — `extract_record`
/// strips only one newline after the fence — and re-canonicalizing it would
/// grow the gap by one newline on every run instead of reproducing the same
/// file, breaking `fmt`'s idempotency. Reducing the gap to a single newline
/// first makes the extracted body identical to a freshly hand-written
/// record's, so re-serialization reproduces the file it read.
fn normalize_frontmatter_gap(contents: &str) -> String {
    let Some(rest) = contents.strip_prefix("---\n") else {
        return contents.to_owned();
    };
    let Some(close_at) = rest.find("\n---") else {
        return contents.to_owned();
    };
    let fence_end = "---\n".len() + close_at + "\n---".len();
    let tail = contents[fence_end..].trim_start_matches('\n');
    format!("{}\n{tail}", &contents[..fence_end])
}

#[cfg(test)]
mod tests;
