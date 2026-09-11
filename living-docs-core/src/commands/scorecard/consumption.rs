//! Consumption metrics (issue #58): what agents actually read from docs, not
//! only what the corpus contains. Fed by a lightweight capture (the
//! `observe-docs-read.sh` PostToolUse hook appends one JSONL line per doc
//! read) and an optional review-findings JSONL. Pure over the captured text —
//! the CLI front reads the files and passes their contents here. Missing
//! capture is "not measured", never an error: `scorecard` still never fails.

mod classifier;

pub use classifier::{classify, Category};

/// The three consumption metrics over a capture window.
pub struct Consumption {
    pub reads: usize,
    pub tokens_sum: u64,
    pub tokens_median: u64,
    pub tokens_p90: u64,
    pub stale_reads: usize,
    pub findings: Option<FindingShare>,
}

/// The doc-trail share of review findings — `None` when no findings capture
/// was supplied.
pub struct FindingShare {
    pub total: usize,
    pub doc_trail: usize,
    pub unclassified: usize,
}

struct Read {
    tokens: u64,
    stale: bool,
    day: Option<i64>,
}

/// Summarizes a doc-read JSONL (and optional findings JSONL) over the window
/// `since_days` — measured back from the newest captured read, so the summary
/// is deterministic without a wall clock.
pub fn summarize(reads_jsonl: &str, findings_jsonl: Option<&str>, since_days: Option<i64>) -> Consumption {
    let reads = within_window(parse_reads(reads_jsonl), since_days);
    let mut tokens: Vec<u64> = reads.iter().map(|r| r.tokens).collect();
    tokens.sort_unstable();
    Consumption {
        reads: reads.len(),
        tokens_sum: tokens.iter().sum(),
        tokens_median: percentile(&tokens, 50),
        tokens_p90: percentile(&tokens, 90),
        stale_reads: reads.iter().filter(|r| r.stale).count(),
        findings: findings_jsonl.map(summarize_findings),
    }
}

fn parse_reads(jsonl: &str) -> Vec<Read> {
    jsonl.lines().filter_map(parse_read_line).collect()
}

fn parse_read_line(line: &str) -> Option<Read> {
    let value: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    let tokens = value.get("tokens").and_then(serde_json::Value::as_u64).unwrap_or(0);
    let status = value.get("status").and_then(serde_json::Value::as_str).unwrap_or("");
    Some(Read {
        tokens,
        stale: is_stale_status(status),
        day: value.get("ts").and_then(serde_json::Value::as_str).and_then(day_number),
    })
}

fn is_stale_status(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "superseded" | "deprecated"
    )
}

/// Keeps reads within `since_days` of the newest dated read. With no window,
/// or no parseable dates, every read is kept.
fn within_window(reads: Vec<Read>, since_days: Option<i64>) -> Vec<Read> {
    let Some(since) = since_days else {
        return reads;
    };
    let Some(newest) = reads.iter().filter_map(|r| r.day).max() else {
        return reads;
    };
    reads
        .into_iter()
        .filter(|r| r.day.is_none_or(|day| newest - day <= since && newest - day >= 0))
        .collect()
}

/// Nearest-rank percentile of a sorted slice; 0 for an empty slice.
fn percentile(sorted: &[u64], p: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = (p * (sorted.len() - 1) + 50) / 100;
    sorted[idx.min(sorted.len() - 1)]
}

fn summarize_findings(jsonl: &str) -> FindingShare {
    let categories: Vec<Category> = jsonl
        .lines()
        .filter_map(finding_text)
        .map(|text| classify(&text))
        .collect();
    FindingShare {
        total: categories.len(),
        doc_trail: categories.iter().filter(|c| c.is_doc_trail()).count(),
        unclassified: categories
            .iter()
            .filter(|c| **c == Category::Unclassified)
            .count(),
    }
}

fn finding_text(line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    value
        .get("text")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// A coarse day number from an ISO `YYYY-MM-DD…` timestamp, matching
/// `inflation`'s bucketing.
fn day_number(ts: &str) -> Option<i64> {
    let date = ts.get(0..10)?;
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    Some(y * 372 + m * 31 + d)
}

pub fn render_block(consumption: &Consumption) -> String {
    let findings = match &consumption.findings {
        Some(share) => format!(
            "findings: {}/{} doc-trail ({} unclassified)",
            share.doc_trail, share.total, share.unclassified
        ),
        None => "findings: not measured".to_string(),
    };
    format!(
        "consumption — {} doc reads, tokens sum {} median {} p90 {}, {} stale reads\n{:<13}{findings}",
        consumption.reads,
        consumption.tokens_sum,
        consumption.tokens_median,
        consumption.tokens_p90,
        consumption.stale_reads,
        ""
    )
}

#[cfg(test)]
mod tests;
