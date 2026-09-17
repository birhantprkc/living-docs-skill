//! Native core of `living-docs check` — ports `skills/living-docs/scripts/lint-docs.sh`.
//!
//! Covers the mechanical invariants: OKF frontmatter/type, index-format,
//! directory-index membership, bundle-root reachability, supersede-chain
//! integrity, local link/image validity via `pulldown-cmark`, unfilled
//! placeholders, and ```mermaid``` fence validation (ADR 0013). Record
//! liveness (ADR 0049, `stale-proposed` only) is advisory.
//!
//! Every record's content (`records`, `links`) is read through
//! `DocStore::read`, so `check` validates whichever backend `run` is given.
//! `index.md`/`log.md` are excluded from the record domain by design (never
//! synced to `db-store`); `check::graph` reads them straight from disk.

mod callout;
pub(crate) mod canonical;
mod graph;
pub(crate) mod links;
mod liveness;
mod mermaid;
mod moved_source;
mod placeholder;
mod records;
mod size;

use crate::doc_type::{self, Identity};
use crate::store::DocStore;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// `check --mermaid-only [paths...]` — validates ONLY the mermaid fences under
/// `paths`, skipping every other invariant. See `mermaid::run_mermaid_only`.
pub fn run_mermaid_only(paths: &[PathBuf]) -> ExitCode {
    mermaid::run_mermaid_only(paths)
}

pub fn run(store: &dyn DocStore, bundle: &Path) -> ExitCode {
    run_require_owner(store, bundle, false)
}

/// `check --require-owner`: promotes a missing `owner` on a doctype whose
/// registry row requires it from an advisory to an invariant violation.
/// Every other invariant behaves exactly as [`run`].
pub fn run_require_owner(store: &dyn DocStore, bundle: &Path, require_owner: bool) -> ExitCode {
    if !bundle.is_dir() {
        eprintln!(
            "living-docs check: bundle root not found: {}",
            bundle.display()
        );
        eprintln!(
            "       run from the repo root, or pass the docs directory: living-docs check path/to/docs"
        );
        return ExitCode::from(2);
    }

    println!("Living Docs lint — bundle: {}", bundle.display());
    println!();

    let mut reporter = Reporter::new();
    let doc_count = run_all_checks(store, bundle, &mut reporter, require_owner, true);
    reporter.finish(doc_count)
}

/// Every invariant `check` validates, without the surrounding
/// `bundle.is_dir()` guard, header, or verdict rendering — shared by
/// [`run`] (which prints the verdict) and [`check_violations`] (which
/// returns the raw list, for a caller like `db_store::DbDocStore::write_checked`
/// that gates a write on the same invariants without printing anything).
/// Returns the number of docs `store` enumerated under `bundle`.
fn run_all_checks(
    store: &dyn DocStore,
    bundle: &Path,
    reporter: &mut Reporter,
    require_owner: bool,
    check_placeholders: bool,
) -> usize {
    let all_md = store.list(bundle).unwrap_or_default();
    let root_index = bundle.join("index.md");
    if !root_index.is_file() {
        reporter.report(&root_index, "missing bundle-root index.md (invariant 3)");
    }
    records::check_frontmatter_and_format(store, &all_md, &root_index, reporter);
    graph::check_directory_membership(bundle, &all_md, reporter);
    graph::check_reachability(bundle, &root_index, &all_md, reporter);
    links::check_links(store, bundle, &all_md, reporter);
    records::check_supersede_chain(store, &all_md, reporter);
    callout::check_callouts(store, &all_md, reporter);
    moved_source::check_moved_source(store, bundle, &all_md, reporter);
    records::check_owner_requirement(store, &all_md, require_owner, reporter);
    canonical::check_canonical_frontmatter(store, bundle, &all_md, reporter);
    mermaid::check_bundle(&all_md, reporter);
    size::check_body_size(store, &all_md, reporter);
    if check_placeholders {
        placeholder::check_placeholders(store, &all_md, reporter);
    }
    liveness::check_liveness(store, bundle, &all_md, reporter);

    all_md.len()
}

/// The invariants [`run`] validates, minus the unfilled-placeholder pass,
/// returned as a plain violation list rather than printed — the mechanism a
/// per-write gate (e.g. `db_store::DbDocStore::write_checked`) uses to reject a
/// structurally invalid record. The placeholder pass is deliberately excluded
/// here: db-mode authoring is create-then-edit, so a freshly created record
/// legitimately still carries its `{{PLACEHOLDER}}` slots until the author
/// fills them in the edit view. Unfilled placeholders are caught at the commit
/// boundary by the `check` command (which runs the pass), not by the per-write
/// gate.
pub fn check_violations(store: &dyn DocStore, bundle: &Path) -> Vec<(String, String)> {
    let mut reporter = Reporter::new();
    run_all_checks(store, bundle, &mut reporter, false, false);
    reporter.into_violations()
}

pub(crate) fn file_name_str(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

pub(crate) fn collect_md_files(bundle: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_md_files(bundle, &mut out);
    out.sort();
    out
}

fn walk_md_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            walk_md_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

/// True when `path` is exactly the bundle-root file of some registry
/// [`Identity::Singleton`] row — the single place the check layer learns
/// what a singleton is, so a second singleton row is handled without an
/// edit here.
///
/// The comparison is a plain path equality, not a filesystem lookup, so it
/// only stays correct because every [`DocStore`] enumerates a bundle's paths
/// rooted at the same `bundle` it was given (see [`collect_md_files`]) —
/// `read_dir` never resolves symlinks, so this holds even when the caller
/// reaches the bundle through one. Reach for `canonicalize` here instead and
/// this pure predicate starts touching disk, breaking every `MapStore`
/// fixture whose paths never exist on disk.
pub(crate) fn is_bundle_singleton(bundle: &Path, path: &Path) -> bool {
    doc_type::DOC_TYPES.iter().any(|spec| match spec.identity {
        Identity::Singleton { file } => path == bundle.join(file),
        Identity::Numbered { .. } | Identity::Named { .. } => false,
    })
}

/// Collects violations and renders the final report + exit code, mirroring
/// `report()` and the verdict block of `lint-docs.sh`. Advisories (issue
/// 0009) print alongside violations but never touch the exit code.
pub(crate) struct Reporter {
    violations: Vec<(String, String)>,
    advisories: Vec<(String, String)>,
}

impl Reporter {
    fn new() -> Self {
        Self {
            violations: Vec::new(),
            advisories: Vec::new(),
        }
    }

    pub(crate) fn report(&mut self, file: &Path, message: impl Into<String>) {
        self.violations
            .push((file.display().to_string(), message.into()));
    }

    pub(crate) fn advise(&mut self, file: &Path, message: impl Into<String>) {
        self.advisories
            .push((file.display().to_string(), message.into()));
    }

    fn into_violations(self) -> Vec<(String, String)> {
        self.violations
    }

    fn finish(self, doc_count: usize) -> ExitCode {
        for (file, message) in &self.advisories {
            println!("  {file:<44} {message}");
        }
        if !self.advisories.is_empty() {
            println!();
        }
        for (file, message) in &self.violations {
            println!("  {file:<44} {message}");
        }
        println!();
        if self.violations.is_empty() {
            println!("OK — {doc_count} docs, no invariant violations.");
            ExitCode::SUCCESS
        } else {
            println!(
                "FAIL — {} violation(s) across {doc_count} docs.",
                self.violations.len()
            );
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests;
