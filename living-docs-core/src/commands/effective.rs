//! `living-docs effective` (ADR 0050, simplified by ADR 0057): the agent-facing
//! read of the bundle — active records only (superseded/deprecated withheld),
//! supersede chains collapsed to the head with a one-line lineage, grouped by
//! kind then number. `--topic` restricts to records mentioning a term
//! (case-insensitive, title/description/body); `--full` prints bodies instead
//! of the one-line index. Derived, never committed: the records stay the SSOT.

use crate::record::{self, ExtractedRecord};
use crate::store::DocStore;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod render;

/// Everything `effective` needs from the CLI front.
pub struct Options {
    pub topic: Option<String>,
    pub full: bool,
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

/// A read record paired with its `bundle`-prefixed path.
type Entry = (PathBuf, ExtractedRecord);

pub fn run(store: &dyn DocStore, bundle: &Path, options: &Options) -> ExitCode {
    print!("{}", compile(store, bundle, options));
    ExitCode::SUCCESS
}

/// Compiles the effective view as a string — the pure core, so tests assert on
/// the compiled text without capturing stdout.
pub fn compile(store: &dyn DocStore, bundle: &Path, options: &Options) -> String {
    let all_md = store.list(bundle).unwrap_or_default();
    let records = read_records(store, &all_md);
    let topic = options.topic.as_deref().map(str::to_lowercase);

    let mut active: Vec<&Entry> = records
        .iter()
        .filter(|(_, record)| is_in_force(record.status.as_deref()))
        .filter(|(_, record)| matches_topic(record, topic.as_deref()))
        .collect();
    active.sort_by_key(|(_, record)| base_rank(record));

    let views: Vec<View> = active
        .iter()
        .map(|(_, record)| view_of(record, &records))
        .collect();
    render::render(&views, options.full)
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

fn is_in_force(status: Option<&str>) -> bool {
    !matches!(
        status.map(|s| s.to_ascii_lowercase()).as_deref(),
        Some("superseded") | Some("deprecated")
    )
}

/// Whether a record mentions `topic` (already lowercased) anywhere in its
/// title, description, or body. `None` matches every record.
fn matches_topic(record: &ExtractedRecord, topic: Option<&str>) -> bool {
    let Some(topic) = topic else {
        return true;
    };
    [&record.title, &record.description, &record.body]
        .iter()
        .any(|field| field.to_lowercase().contains(topic))
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

/// Reading order: constitution and PRDs first, then ADRs, then everything
/// else, each by number. A grouping for orientation, not a relevance rank.
fn base_rank(record: &ExtractedRecord) -> (u8, i32) {
    let group = match record.doc_type.as_str() {
        "Constitution" => 0,
        "PRD" => 1,
        "ADR" => 2,
        _ => 3,
    };
    (group, record.number.unwrap_or(i32::MAX))
}

#[cfg(test)]
mod tests;
