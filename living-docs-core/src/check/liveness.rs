//! Mechanical record liveness (ADR 0049): `check` derives whether an ADR/BDR
//! is still current from evidence the record already carries, emitting
//! advisories only — the exit code never moves, same posture as `SIZE` and
//! `MOVED-SOURCE`. Currency, unlike materiality, has a sound oracle: the
//! linked issue's status and the filesystem.
//!
//! Three classifications, all computed here and exported through [`classify`]
//! so the read verbs (effective view, `why`) exclude stale records from what
//! they serve without re-deriving the rule:
//!   - **stale-proposed** — a record still at its seed status
//!     (`Proposed`/`Draft`) that links a terminal or superseded issue.
//!   - **stale-impact** — an accepted record whose `Implementation impact:`
//!     names a literal repository path that no longer exists.
//!   - **contract vs narrative** — an accepted record with a `## Verification`
//!     block is a contract; without one it is narrative. A classification, not
//!     a finding: it is surfaced only in the `--liveness` summary and the API.

mod impact;

use super::links::{link_destinations, resolve_destination};
use super::records::{frontmatter_scalar, is_reserved};
use super::{file_name_str, Reporter};
use crate::doc_type;
use crate::store::DocStore;
use impact::has_dead_impact_path;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Whether a record's declared status still matches the world it describes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Liveness {
    Active,
    StaleProposed,
    StaleImpact,
}

/// Whether an accepted record is a checkable contract (carries a
/// `## Verification` block) or advisory narrative (does not).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    Contract,
    Narrative,
}

/// One ADR/BDR record's liveness classification.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RecordLiveness {
    pub liveness: Liveness,
    pub shape: Shape,
}

/// The four `--liveness` summary counts.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct LivenessCounts {
    pub stale_proposed: usize,
    pub stale_impact: usize,
    pub contract: usize,
    pub narrative: usize,
}

/// Liveness classification of every ADR/BDR record under a bundle, keyed by
/// the record's display path. Records of any other type are absent — callers
/// treat an absent path as [`Liveness::Active`].
#[derive(Clone, Debug, Default)]
pub struct LivenessReport {
    entries: BTreeMap<String, RecordLiveness>,
}

impl LivenessReport {
    /// True when `path`'s record is stale (proposed-but-terminal or
    /// dead-impact) and should be withheld from an agent-facing view. An
    /// absent path (a non-ADR/BDR record) is never stale.
    pub fn is_stale(&self, path: &Path) -> bool {
        matches!(
            self.entries.get(&path.display().to_string()).map(|r| r.liveness),
            Some(Liveness::StaleProposed | Liveness::StaleImpact)
        )
    }

    /// The record's shape, `None` for a non-ADR/BDR record.
    pub fn shape(&self, path: &Path) -> Option<Shape> {
        self.entries.get(&path.display().to_string()).map(|r| r.shape)
    }

    /// The four summary counts across every classified record.
    pub fn counts(&self) -> LivenessCounts {
        let mut counts = LivenessCounts::default();
        for record in self.entries.values() {
            match record.liveness {
                Liveness::StaleProposed => counts.stale_proposed += 1,
                Liveness::StaleImpact => counts.stale_impact += 1,
                Liveness::Active => {}
            }
            match record.shape {
                Shape::Contract => counts.contract += 1,
                Shape::Narrative => counts.narrative += 1,
            }
        }
        counts
    }
}

/// Classifies every ADR/BDR record under `bundle`. Pure over the store: reads
/// each record and the issues it links, resolves impact paths against the
/// bundle's parent, and touches nothing.
pub fn classify(store: &dyn DocStore, bundle: &Path, all_md: &[PathBuf]) -> LivenessReport {
    let bundle_str = bundle.to_string_lossy();
    let mut entries = BTreeMap::new();
    for f in all_md {
        if let Some(record) = classify_one(store, f, &bundle_str, all_md) {
            entries.insert(f.display().to_string(), record);
        }
    }
    LivenessReport { entries }
}

pub(crate) fn check_liveness(
    store: &dyn DocStore,
    bundle: &Path,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    let report = classify(store, bundle, all_md);
    for (path, record) in &report.entries {
        let message = match record.liveness {
            Liveness::StaleProposed => "LIVENESS stale-proposed: seed status but the linked issue is terminal — settle or supersede",
            Liveness::StaleImpact => "LIVENESS stale-impact: an Implementation-impact path no longer exists — update the Verification block",
            Liveness::Active => continue,
        };
        reporter.advise(Path::new(path), message);
    }
}

fn classify_one(
    store: &dyn DocStore,
    f: &Path,
    bundle: &str,
    all_md: &[PathBuf],
) -> Option<RecordLiveness> {
    if is_reserved(&file_name_str(f)) {
        return None;
    }
    let contents = store.read(f).ok()?;
    let doc_type = frontmatter_scalar(&contents, "type")?;
    if !is_liveness_type(&doc_type) {
        return None;
    }
    let status = frontmatter_scalar(&contents, "status")?;
    let liveness = liveness_of(store, f, &contents, &doc_type, &status, bundle, all_md);
    Some(RecordLiveness {
        liveness,
        shape: shape_of(&contents),
    })
}

fn liveness_of(
    store: &dyn DocStore,
    f: &Path,
    contents: &str,
    doc_type: &str,
    status: &str,
    bundle: &str,
    all_md: &[PathBuf],
) -> Liveness {
    if is_seed_status(doc_type, status) {
        return if links_terminal_issue(store, f, contents, bundle, all_md) {
            Liveness::StaleProposed
        } else {
            Liveness::Active
        };
    }
    if is_accepted_status(doc_type, status) && has_dead_impact_path(contents, bundle) {
        return Liveness::StaleImpact;
    }
    Liveness::Active
}

/// Only ADR and BDR carry a liveness signal (ADR 0049): a decision record's
/// currency is checkable against its linked issue and its impact paths.
fn is_liveness_type(doc_type: &str) -> bool {
    matches!(doc_type, "ADR" | "BDR")
}

/// The seed status is `status_vocabulary[0]` — `Proposed` for ADR, `Draft`
/// for BDR — the state a record is born in and should leave once its work
/// lands (registry-sourced, ADR 0029).
fn is_seed_status(doc_type: &str, status: &str) -> bool {
    doc_type::spec_for_frontmatter(doc_type)
        .and_then(|spec| spec.status_vocabulary.first())
        .is_some_and(|seed| seed.eq_ignore_ascii_case(status))
}

/// An accepted state is any vocabulary member past the seed that the registry
/// does not mark terminal — `Accepted` for ADR, `Accepted`/`Implemented` for
/// BDR. Impact paths are only meaningful once a decision is in force.
fn is_accepted_status(doc_type: &str, status: &str) -> bool {
    let Some(spec) = doc_type::spec_for_frontmatter(doc_type) else {
        return false;
    };
    if is_seed_status(doc_type, status) {
        return false;
    }
    let is_terminal = spec
        .terminal_statuses
        .iter()
        .any(|t| t.eq_ignore_ascii_case(status));
    !is_terminal
        && spec
            .status_vocabulary
            .iter()
            .any(|s| s.eq_ignore_ascii_case(status))
}

fn shape_of(contents: &str) -> Shape {
    if has_verification_block(contents) {
        Shape::Contract
    } else {
        Shape::Narrative
    }
}

fn has_verification_block(contents: &str) -> bool {
    contents
        .lines()
        .any(|line| line.trim_start().eq_ignore_ascii_case("## verification"))
}

/// True when `f` links a bundle-relative issue record whose status is terminal
/// for its type (`closed`/`done`) or `Superseded`.
fn links_terminal_issue(
    store: &dyn DocStore,
    f: &Path,
    contents: &str,
    bundle: &str,
    all_md: &[PathBuf],
) -> bool {
    let file_str = f.to_string_lossy();
    link_destinations(contents)
        .iter()
        .filter_map(|dest| resolve_destination(&file_str, dest, bundle))
        .any(|target| issue_target_is_terminal(store, &PathBuf::from(target), all_md))
}

fn issue_target_is_terminal(store: &dyn DocStore, target: &Path, all_md: &[PathBuf]) -> bool {
    if !all_md.iter().any(|p| p == target) {
        return false;
    }
    let Ok(contents) = store.read(target) else {
        return false;
    };
    if frontmatter_scalar(&contents, "type").as_deref() != Some("Issue") {
        return false;
    }
    let Some(status) = frontmatter_scalar(&contents, "status") else {
        return false;
    };
    is_terminal_issue_status(&status)
}

fn is_terminal_issue_status(status: &str) -> bool {
    if status.eq_ignore_ascii_case("superseded") {
        return true;
    }
    doc_type::spec_for_frontmatter("Issue").is_some_and(|spec| {
        spec.terminal_statuses
            .iter()
            .any(|t| t.eq_ignore_ascii_case(status))
    })
}

/// Prints the four `--liveness` summary counts for the bundle. Lives here
/// because the summary is the liveness pass's own output, not the check
/// orchestrator's.
pub(crate) fn print_summary(store: &dyn DocStore, bundle: &Path) {
    let counts = classify(store, bundle, &store.list(bundle).unwrap_or_default()).counts();
    println!(
        "liveness — {} stale-proposed, {} stale-impact, {} contract, {} narrative",
        counts.stale_proposed, counts.stale_impact, counts.contract, counts.narrative
    );
    println!();
}

#[cfg(test)]
mod tests;
