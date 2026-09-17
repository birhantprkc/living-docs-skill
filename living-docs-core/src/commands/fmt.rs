//! `living-docs fmt` canonicalizes a concept record's frontmatter in
//! place, and reconciles the retired-record callout at the top of its
//! body to match the status that frontmatter carries — the rest of the
//! body below the closing `---` stays byte-for-byte unchanged. `target`
//! accepts either a bundle root (canonicalizes every record under it,
//! enumerated through the same `DocStore::list` call `check::run` reads
//! from — no second directory walker) or a single record path
//! (canonicalizes only that record). With `check_only`, no record is
//! written: the command reports which records are pending instead.
//! File-mode only: db-mode is canonical by construction on export, so
//! this verb never runs against `--backend db`.

use crate::callout;
use crate::record::{extract_record, to_canonical_markdown, ExtractedRecord};
use crate::store::DocStore;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Canonicalizes `target`'s frontmatter — a bundle root or a single record
/// path — printing each pending record and a summary count. With
/// `check_only`, prints the same pending records but writes nothing, and
/// returns a non-zero exit code when any record is pending.
pub fn run(store: &dyn DocStore, target: &Path, check_only: bool) -> ExitCode {
    let Some(paths) = resolve_targets(store, target) else {
        eprintln!(
            "living-docs fmt: bundle root not found: {}",
            target.display()
        );
        return ExitCode::from(2);
    };

    if check_only {
        return run_check(store, &paths);
    }

    let rewritten = canonicalize_bundle(store, &paths);
    println!("{rewritten} record(s) rewritten.");
    ExitCode::SUCCESS
}

/// Resolves `target` to the record paths `fmt` should consider: a single
/// path when `target` is a file, every listed `.md` file when it is a
/// directory, or `None` when it is neither.
fn resolve_targets(store: &dyn DocStore, target: &Path) -> Option<Vec<PathBuf>> {
    if target.is_file() {
        Some(vec![target.to_path_buf()])
    } else if target.is_dir() {
        Some(store.list(target).unwrap_or_default())
    } else {
        None
    }
}

/// Reports which of `paths` are non-canonical without writing any of them,
/// printing each pending path and a summary count. Returns
/// [`ExitCode::SUCCESS`] when none are pending, `ExitCode::from(1)`
/// otherwise.
fn run_check(store: &dyn DocStore, paths: &[PathBuf]) -> ExitCode {
    let mut pending = 0;
    for path in paths {
        if is_reserved_file(path) {
            continue;
        }
        if record_is_pending(store, path) {
            println!("{}", path.display());
            pending += 1;
        }
    }
    println!("{pending} record(s) would change.");
    if pending == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
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
    let Some(canonical) = canonical_form_if_pending(store, path) else {
        return false;
    };
    store.write(path, &canonical).is_ok()
}

/// Returns whether `path` carries a frontmatter block whose canonical
/// re-serialization differs from its current contents, without writing it.
fn record_is_pending(store: &dyn DocStore, path: &Path) -> bool {
    canonical_form_if_pending(store, path).is_some()
}

/// Reads `path` and returns its canonical re-serialization when that
/// differs from the current contents. Returns `None` when `path` cannot be
/// read, carries no frontmatter, or is already canonical.
fn canonical_form_if_pending(store: &dyn DocStore, path: &Path) -> Option<String> {
    let contents = store.read(path).ok()?;
    if !has_frontmatter(&contents) {
        return None;
    }
    let normalized = callout::normalize_frontmatter_gap(&contents);
    let mut record = extract_record(path, &normalized);
    record.body = reconciled_body(store, path, &record);
    let canonical = to_canonical_markdown(&record);
    if canonical == contents {
        None
    } else {
        Some(canonical)
    }
}

/// The body `record` should carry once its retired-record callout matches
/// its status: unchanged when `record` is active and carries none, gains
/// or loses one otherwise.
fn reconciled_body(store: &dyn DocStore, path: &Path, record: &ExtractedRecord) -> String {
    let successor = record
        .superseded_by
        .as_deref()
        .map(|superseded_by| callout::successor_filename(store, path, superseded_by));
    let expected = callout::expected(record.status.as_deref(), successor.as_deref());
    callout::reconcile_body(&record.body, expected.as_deref())
}

fn has_frontmatter(contents: &str) -> bool {
    contents.lines().next() == Some("---")
}

#[cfg(test)]
mod tests;
