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
    /// Record paths (`bundle`-prefixed, matching [`DocStore::list`]) in FTS5
    /// relevance order, when the front resolved `--topic` against the search
    /// read-model (ADR 0056). When `Some`, the view is restricted to these
    /// records in this order; when `None`, `--topic` falls back to a
    /// deterministic relevance rank over the store.
    pub ranked_topic: Option<Vec<String>>,
}

/// One record that survived selection, flattened to what the renderer needs.
pub(crate) struct View {
    pub doc_type: String,
    pub number: Option<i32>,
    pub title: String,
    pub description: String,
    pub body: String,
    pub lineage: Option<String>,
}

/// A read record paired with its `bundle`-prefixed path — the unit the
/// ordering helpers pass around.
type Entry = (PathBuf, ExtractedRecord);

/// The base sort key `(group, not_contract, number)`.
type Rank = (u8, u8, i32);

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

    let active: Vec<&Entry> = records
        .iter()
        .filter(|(path, record)| is_selected(record, options, stale.is_stale(path)))
        .collect();
    let ordered = order_for_topic(active, options);
    let views: Vec<View> = ordered
        .iter()
        .map(|(_, record)| view_of(record, &records))
        .collect();

    render::render(&views, options.tier, options.budget)
}

/// Orders (and, for a topic, filters) the active records: by the FTS5 ranked
/// set when the front supplied one, else by a deterministic relevance rank for
/// a `--topic`, else by the base constitution/PRD/contract rank (ADR 0056).
fn order_for_topic<'a>(active: Vec<&'a Entry>, options: &Options) -> Vec<&'a Entry> {
    if let Some(ranked) = &options.ranked_topic {
        return order_by_ranked_set(active, ranked);
    }
    match options.topic.as_deref() {
        Some(topic) => order_by_relevance(active, topic),
        None => order_by_base_rank(active),
    }
}

/// Keeps only records whose `bundle`-prefixed path is in `ranked` and orders
/// them by their position there — the FTS5 relevance order.
fn order_by_ranked_set<'a>(active: Vec<&'a Entry>, ranked: &[String]) -> Vec<&'a Entry> {
    let position: BTreeMap<&str, usize> = ranked
        .iter()
        .enumerate()
        .map(|(i, path)| (path.as_str(), i))
        .collect();
    let mut kept: Vec<(usize, &Entry)> = active
        .into_iter()
        .filter_map(|entry| {
            position
                .get(entry.0.display().to_string().as_str())
                .map(|rank| (*rank, entry))
        })
        .collect();
    kept.sort_by_key(|(rank, _)| *rank);
    kept.into_iter().map(|(_, entry)| entry).collect()
}

/// Keeps records that mention `topic` (title/description/body) and orders them
/// by a deterministic relevance score, tie-broken by the base rank — the
/// no-projection fallback for `--topic`.
fn order_by_relevance<'a>(active: Vec<&'a Entry>, topic: &str) -> Vec<&'a Entry> {
    let needle = topic.to_lowercase();
    let mut scored: Vec<(u32, Rank, &Entry)> = active
        .into_iter()
        .filter_map(|entry| {
            let score = relevance_score(&entry.1, &needle);
            (score > 0).then(|| (score, base_rank(&entry.1), entry))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    scored.into_iter().map(|(_, _, entry)| entry).collect()
}

fn order_by_base_rank(active: Vec<&Entry>) -> Vec<&Entry> {
    let mut ordered = active;
    ordered.sort_by_key(|entry| base_rank(&entry.1));
    ordered
}

/// Weighted term-frequency of `needle` across a record: title counts triple,
/// description double, body single — enough to order topic matches without a
/// search index.
fn relevance_score(record: &ExtractedRecord, needle: &str) -> u32 {
    let count = |haystack: &str| haystack.to_lowercase().matches(needle).count() as u32;
    count(&record.title) * 3 + count(&record.description) * 2 + count(&record.body)
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

fn view_of(record: &ExtractedRecord, all: &[(PathBuf, ExtractedRecord)]) -> View {
    View {
        doc_type: record.doc_type.clone(),
        number: record.number,
        title: record.title.clone(),
        description: record.description.clone(),
        body: record.body.clone(),
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
    while let Some(number) = current
        .as_deref()
        .and_then(|s| s.trim().parse::<i32>().ok())
    {
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

/// Base sort key: constitution and PRDs first, then contracts (a
/// `## Verification` block) above narrative, then by number.
/// `(group, not_contract, number)`.
fn base_rank(record: &ExtractedRecord) -> Rank {
    let group = match record.doc_type.as_str() {
        "Constitution" => 0,
        "PRD" => 1,
        _ => 2,
    };
    let not_contract = u8::from(!record.body.contains("## Verification"));
    (group, not_contract, record.number.unwrap_or(i32::MAX))
}

#[cfg(test)]
mod tests;
