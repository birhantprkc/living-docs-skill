//! `living-docs fmt` canonicalizes a concept record's frontmatter in
//! place, and reconciles the retired-record callout at the top of its
//! body to match the status that frontmatter carries — the rest of the
//! body below the closing `---` stays byte-for-byte unchanged. `target`
//! accepts either a bundle root (canonicalizes every record under it,
//! enumerated through the same `DocStore::list` call `check::compile` reads
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
    match outcome(store, target, check_only) {
        Ok(Outcome::Rewritten(paths)) => {
            for path in &paths {
                println!("{}", path.display());
            }
            println!("{} record(s) rewritten.", paths.len());
            ExitCode::SUCCESS
        }
        Ok(Outcome::Pending(paths)) => {
            for path in &paths {
                println!("{}", path.display());
            }
            println!("{} record(s) would change.", paths.len());
            if paths.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(message) => {
            eprintln!("living-docs fmt: {message}");
            ExitCode::from(2)
        }
    }
}

/// `fmt`'s outcome: which records were rewritten, or (`--check`) which are
/// pending. Kept as one type so a caller matches on the mode it asked for
/// rather than juggling two return values.
pub enum Outcome {
    Rewritten(Vec<PathBuf>),
    Pending(Vec<PathBuf>),
}

/// Canonicalizes `target`'s frontmatter (or, under `check_only`, reports
/// which records would change) without printing anything — the CLI front
/// renders this as colored text or JSON (ADR 0060); [`run`] is the
/// plain-text-always convenience wrapper over it.
pub fn outcome(store: &dyn DocStore, target: &Path, check_only: bool) -> Result<Outcome, String> {
    let Some(paths) = resolve_targets(store, target) else {
        return Err(format!("bundle root not found: {}", target.display()));
    };
    if check_only {
        return Ok(Outcome::Pending(pending_records(store, &paths)));
    }
    Ok(Outcome::Rewritten(canonicalize_bundle(store, &paths)))
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

/// Returns which of `paths` are non-canonical, without writing any of them.
fn pending_records(store: &dyn DocStore, paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .filter(|path| !is_reserved_file(path))
        .filter(|path| record_is_pending(store, path))
        .cloned()
        .collect()
}

/// Canonicalizes every non-reserved record in `all_md`, returning the paths
/// actually rewritten.
fn canonicalize_bundle(store: &dyn DocStore, all_md: &[PathBuf]) -> Vec<PathBuf> {
    all_md
        .iter()
        .filter(|path| !is_reserved_file(path))
        .filter(|path| canonicalize_record(store, path))
        .cloned()
        .collect()
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
