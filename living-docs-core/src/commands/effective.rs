//! `living-docs effective` (ADR 0050): compiles the agent-facing view of the
//! bundle — active records only, supersede chains collapsed to the head with a
//! one-line lineage, ranked constitution/PRD/contract-first, and rendered at a
//! progressive tier under a hard token budget. Derived, never committed: the
//! records stay the SSOT, exactly as the generated indexes do.

use crate::check::liveness;
use crate::record::{self, ExtractedRecord};
use crate::store::DocStore;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod render;

/// Progressive disclosure tier: how much of each surviving record the view
/// carries. `Index` is the default — enough to orient, not to drill in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tier {
    Index,
    Outline,
    Full,
}

/// Everything `effective` needs from the CLI front.
pub struct Options {
    pub topic: Option<String>,
    pub tier: Tier,
    pub budget: Option<usize>,
    pub include_stale: bool,
}

/// One record that survived selection, flattened to what the renderer needs.
pub(crate) struct View {
    pub doc_type: String,
    pub number: Option<i32>,
    pub title: String,
    pub description: String,
    pub body: String,
    pub is_contract: bool,
    pub lineage: Option<String>,
}

pub fn run(store: &dyn DocStore, bundle: &Path, options: &Options) -> ExitCode {
    print!("{}", compile(store, bundle, options));
    ExitCode::SUCCESS
}

/// Compiles the effective view as a string — the pure core, so tests assert on
/// the compiled text without capturing stdout.
pub fn compile(store: &dyn DocStore, bundle: &Path, options: &Options) -> String {
    let all_md = store.list(bundle).unwrap_or_default();
    let stale = liveness::classify(store, bundle, &all_md);
    let records = read_records(store, &all_md);

    let mut views: Vec<View> = records
        .iter()
        .filter(|(path, record)| {
            is_selected(record, options, stale.is_stale(path)) && matches_topic(record, options)
        })
        .map(|(_, record)| view_of(record, &records))
        .collect();

    views.sort_by_key(rank_key);
    render::render(&views, options.tier, options.budget)
}

fn read_records(store: &dyn DocStore, all_md: &[PathBuf]) -> Vec<(PathBuf, ExtractedRecord)> {
    all_md
        .iter()
        .filter(|path| !is_reserved(path))
        .filter_map(|path| {
            let contents = store.read(path).ok()?;
            let record = record::extract_record(path, &contents);
            (!record.doc_type.is_empty()).then(|| (path.clone(), record))
        })
        .collect()
}

fn is_reserved(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("index.md") | Some("log.md")
    )
}

/// A record is in the effective view when it is in force (not `Superseded`
/// /`Deprecated`) and either not stale or explicitly requested via
/// `--include-stale`.
fn is_selected(record: &ExtractedRecord, options: &Options, is_stale: bool) -> bool {
    is_in_force(record.status.as_deref()) && (options.include_stale || !is_stale)
}

fn is_in_force(status: Option<&str>) -> bool {
    !matches!(
        status.map(|s| s.to_ascii_lowercase()).as_deref(),
        Some("superseded") | Some("deprecated")
    )
}

fn matches_topic(record: &ExtractedRecord, options: &Options) -> bool {
    let Some(topic) = options.topic.as_deref() else {
        return true;
    };
    let needle = topic.to_lowercase();
    record.title.to_lowercase().contains(&needle)
        || record.description.to_lowercase().contains(&needle)
        || record.body.to_lowercase().contains(&needle)
}

fn view_of(record: &ExtractedRecord, all: &[(PathBuf, ExtractedRecord)]) -> View {
    View {
        doc_type: record.doc_type.clone(),
        number: record.number,
        title: record.title.clone(),
        description: record.description.clone(),
        body: record.body.clone(),
        is_contract: record.body.contains("## Verification"),
        lineage: lineage_of(record, all),
    }
}

/// The lineage line for a head record: `supersedes 0131 via 0133` when the
/// chain is `0131 → 0133 → head`, or `supersedes 0133` for a single ancestor.
/// `None` when the record supersedes nothing.
fn lineage_of(record: &ExtractedRecord, all: &[(PathBuf, ExtractedRecord)]) -> Option<String> {
    let chain = supersede_chain(record, all);
    match chain.split_last() {
        None => None,
        Some((oldest, [])) => Some(format!("supersedes {oldest}")),
        Some((oldest, middle)) => Some(format!("supersedes {oldest} via {}", middle.join(", "))),
    }
}

/// Walks `supersedes` from `record` through the full record set (superseded
/// ancestors included), returning their zero-padded numbers direct-first. A
/// cycle or missing link stops the walk.
fn supersede_chain(record: &ExtractedRecord, all: &[(PathBuf, ExtractedRecord)]) -> Vec<String> {
    let by_key: BTreeMap<(String, i32), &ExtractedRecord> = all
        .iter()
        .filter_map(|(_, r)| r.number.map(|n| ((r.doc_type.clone(), n), r)))
        .collect();
    let mut chain = Vec::new();
    let mut current = record.supersedes.clone();
    while let Some(number) = current.as_deref().and_then(|s| s.trim().parse::<i32>().ok()) {
        if chain.contains(&format!("{number:04}")) {
            break;
        }
        chain.push(format!("{number:04}"));
        current = by_key
            .get(&(record.doc_type.clone(), number))
            .and_then(|ancestor| ancestor.supersedes.clone());
    }
    chain
}

/// Sort key: constitution and PRDs first, then contracts above narrative,
/// then by number. `(group, not_contract, number)`.
fn rank_key(view: &View) -> (u8, u8, i32) {
    let group = match view.doc_type.as_str() {
        "Constitution" => 0,
        "PRD" => 1,
        _ => 2,
    };
    let not_contract = u8::from(!view.is_contract);
    (group, not_contract, view.number.unwrap_or(i32::MAX))
}

#[cfg(test)]
mod tests;
