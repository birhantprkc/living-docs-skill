//! Advisory doc-trail inflation signals (issue #53 proposal C): make the cost
//! of getting materiality wrong observable before it reaches the corpus.
//! Read-only, never gating — the signature signal is an ADR:BDR pairing ratio
//! near 1.0 (records paired by rule, not by need), alongside the supersession
//! rate, the recent-ADR count, and the stale-`Proposed` count (ADR 0049).

use crate::check::liveness;
use crate::record::{self, ExtractedRecord};
use crate::store::DocStore;
use std::collections::BTreeSet;
use std::path::Path;

const RECENT_WINDOW_DAYS: i64 = 30;

/// The four inflation counts, all derived from the records and the liveness
/// pass — none needs a wall clock (the recency window is measured back from
/// the newest record's own timestamp).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Inflation {
    pub adrs: usize,
    pub adrs_with_matching_bdr: usize,
    pub adrs_recent: usize,
    pub superseded: usize,
    pub proposed_stale: usize,
}

impl Inflation {
    /// The share of ADRs carrying a same-numbered BDR; `None` when there are
    /// no ADRs. A ratio near 1.0 is the signature of pairing-by-rule.
    pub fn pairing_ratio(&self) -> Option<f64> {
        (self.adrs > 0).then(|| self.adrs_with_matching_bdr as f64 / self.adrs as f64)
    }
}

pub fn compute(store: &dyn DocStore, bundle: &Path) -> Inflation {
    let all_md = store.list(bundle).unwrap_or_default();
    let records: Vec<ExtractedRecord> = all_md
        .iter()
        .filter_map(|path| read_record(store, path))
        .collect();

    let adr_numbers = numbers_of(&records, "ADR");
    let bdr_numbers = numbers_of(&records, "BDR");
    let newest = records.iter().filter_map(day_number).max();

    Inflation {
        adrs: adr_numbers.len(),
        adrs_with_matching_bdr: adr_numbers.intersection(&bdr_numbers).count(),
        adrs_recent: recent_adr_count(&records, newest),
        superseded: records.iter().filter(|r| is_superseded(r)).count(),
        proposed_stale: liveness::classify(store, bundle, &all_md)
            .counts()
            .stale_proposed,
    }
}

fn read_record(store: &dyn DocStore, path: &Path) -> Option<ExtractedRecord> {
    if matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("index.md") | Some("log.md")
    ) {
        return None;
    }
    let contents = store.read(path).ok()?;
    let record = record::extract_record(path, &contents);
    (!record.doc_type.is_empty()).then_some(record)
}

fn numbers_of(records: &[ExtractedRecord], doc_type: &str) -> BTreeSet<i32> {
    records
        .iter()
        .filter(|r| r.doc_type == doc_type)
        .filter_map(|r| r.number)
        .collect()
}

fn is_superseded(record: &ExtractedRecord) -> bool {
    record
        .status
        .as_deref()
        .is_some_and(|s| s.eq_ignore_ascii_case("superseded"))
}

/// Count of ADRs whose timestamp falls within the recency window ending at the
/// newest record's timestamp. Zero when no timestamps parse.
fn recent_adr_count(records: &[ExtractedRecord], newest: Option<i64>) -> usize {
    let Some(newest) = newest else {
        return 0;
    };
    records
        .iter()
        .filter(|r| r.doc_type == "ADR")
        .filter_map(day_number)
        .filter(|day| newest - day <= RECENT_WINDOW_DAYS && newest - day >= 0)
        .count()
}

/// A coarse day number from a record's `timestamp` frontmatter (`YYYY-MM-DD…`),
/// good enough to bucket a 30-day window: `year*372 + month*31 + day`. `None`
/// when the record carries no parseable timestamp.
fn day_number(record: &ExtractedRecord) -> Option<i64> {
    let stamp = record
        .frontmatter_tail
        .iter()
        .find(|(k, _)| k == "timestamp")
        .and_then(|(_, v)| scalar(v))?;
    let date = stamp.get(0..10)?;
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    Some(y * 372 + m * 31 + d)
}

fn scalar(value: &record::TailValue) -> Option<String> {
    match value {
        record::TailValue::Scalar(s) => Some(s.clone()),
        record::TailValue::Sequence(_) => None,
    }
}

pub fn render_line(inflation: &Inflation) -> String {
    let pairing = inflation
        .pairing_ratio()
        .map(|r| format!("{r:.2}"))
        .unwrap_or_else(|| "n/a".to_string());
    format!(
        "inflation — {} ADRs ({} in last {RECENT_WINDOW_DAYS}d), ADR:BDR pairing {pairing}, {} superseded, {} proposed-stale",
        inflation.adrs, inflation.adrs_recent, inflation.superseded, inflation.proposed_stale
    )
}

#[cfg(test)]
mod tests;
