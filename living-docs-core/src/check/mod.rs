//! Native core of `living-docs check` — ports `skills/living-docs/scripts/lint-docs.sh`.
//!
//! Covers the mechanical invariants: OKF frontmatter/type, index-format,
//! directory-index membership, bundle-root reachability, supersede-chain
//! integrity, local link/image validity via `pulldown-cmark`, requirement
//! traceability (ADR 0035), record liveness (ADR 0049), and (ADR 0013)
//! ```mermaid``` fence validation in-process via `merman-core`.
//!
//! Every record's content (`records`, `links`) is read through
//! `DocStore::read`, so `check` validates whichever backend `run` is given.
//! `index.md`/`log.md` are excluded from the record domain by design (never
//! synced to `db-store`, see `db_store::record::is_reserved`), so
//! `check::graph`'s directory-index parsing reads them straight from disk.

pub(crate) mod canonical;
mod graph;
mod leak;
pub mod liveness;
pub(crate) mod links;
mod mermaid;
mod moved_source;
mod records;
mod seal;
mod size;
pub(crate) mod traceability;

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
    run_configured(store, bundle, require_owner, false)
}

/// `check --liveness`: every invariant [`run_require_owner`] validates, plus a
/// trailing summary of the four record-liveness counts (ADR 0049).
pub fn run_liveness(store: &dyn DocStore, bundle: &Path, require_owner: bool) -> ExitCode {
    run_configured(store, bundle, require_owner, true)
}

fn run_configured(
    store: &dyn DocStore,
    bundle: &Path,
    require_owner: bool,
    liveness_summary: bool,
) -> ExitCode {
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
    let doc_count = run_all_checks(store, bundle, &mut reporter, require_owner);

    if liveness_summary {
        liveness::print_summary(store, bundle);
    }

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
    moved_source::check_moved_source(store, bundle, &all_md, reporter);
    records::check_owner_requirement(store, &all_md, require_owner, reporter);
    canonical::check_canonical_frontmatter(store, bundle, &all_md, reporter);

    mermaid::check_bundle(&all_md, reporter);
    size::check_body_size(store, &all_md, reporter);
    seal::check_seals(store, bundle, &all_md, reporter);
    traceability::check_requirement_traceability(store, &all_md, reporter);
    liveness::check_liveness(store, bundle, &all_md, reporter);
    leak::check_leak(store, &all_md, reporter);

    all_md.len()
}

/// The same invariants [`run`] validates, returned as a plain violation list
/// rather than printed and turned into an [`ExitCode`] — the mechanism a
/// transactional write+check verb (e.g. `db_store::DbDocStore::write_checked`)
/// needs to gate a commit on `check` passing without emitting `check`'s own
/// stdout report.
pub fn check_violations(store: &dyn DocStore, bundle: &Path) -> Vec<(String, String)> {
    let mut reporter = Reporter::new();
    run_all_checks(store, bundle, &mut reporter, false);
    reporter.into_violations()
}

/// One check finding: the record's display path paired with the finding's
/// message.
pub type Finding = (String, String);

/// Every violation and advisory [`run`] would print, without the printing
/// or the [`ExitCode`] — the read-only aggregation surface a summarizing
/// verb (e.g. `scorecard`) runs its own classification over. [`run`] and
/// [`run_require_owner`] keep their own output and exit code unchanged;
/// this is an additive view over the same [`run_all_checks`] pass.
pub struct Findings {
    pub violations: Vec<Finding>,
    pub advisories: Vec<Finding>,
}

/// Runs every invariant [`run`] validates and returns the full finding set,
/// printing nothing.
pub fn findings(store: &dyn DocStore, bundle: &Path) -> Findings {
    let mut reporter = Reporter::new();
    run_all_checks(store, bundle, &mut reporter, false);
    let (violations, advisories) = reporter.into_findings();
    Findings {
        violations,
        advisories,
    }
}

/// Owner coverage over the records whose doctype registry row requires an
/// owner, as `(owned, required)`. `None` when no record under `bundle`
/// requires one — there is nothing to grade a ratio over.
pub fn owner_coverage(store: &dyn DocStore, bundle: &Path) -> Option<(usize, usize)> {
    let all_md = store.list(bundle).unwrap_or_default();
    let (mut owned, mut required) = (0usize, 0usize);
    for f in &all_md {
        count_owner_coverage(store, f, &mut owned, &mut required);
    }
    (required > 0).then_some((owned, required))
}

fn count_owner_coverage(store: &dyn DocStore, f: &Path, owned: &mut usize, required: &mut usize) {
    if records::is_reserved(&file_name_str(f)) {
        return;
    }
    let Ok(contents) = store.read(f) else {
        return;
    };
    let Some(doc_type) = records::frontmatter_scalar(&contents, "type") else {
        return;
    };
    let Some(spec) = doc_type::spec_for_frontmatter(&doc_type) else {
        return;
    };
    if !spec.requires_owner {
        return;
    }
    *required += 1;
    if records::frontmatter_scalar(&contents, "owner").is_some() {
        *owned += 1;
    }
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

    fn into_findings(self) -> (Vec<Finding>, Vec<Finding>) {
        (self.violations, self.advisories)
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
