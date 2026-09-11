//! `living-docs why <path>` (ADR 0051): inverts the `**Implementation
//! impact:**` lists so an agent answers "which records govern this file" by
//! query rather than by a citation rotting in a code comment. Matches a query
//! path against each impact entry as exact, directory-prefix, or glob, ranked
//! most-specific first, over active records (superseded excluded; stale per
//! ADR 0049 excluded unless `--include-stale`).

use crate::check::liveness;
use crate::impact;
use crate::record;
use crate::store::DocStore;
use regex::Regex;
use std::path::Path;
use std::process::ExitCode;

/// The query: one or more paths (a positional path, or the files a
/// `--from-diff` range touched, resolved by the CLI front) and whether stale
/// records are included.
pub struct Query {
    pub paths: Vec<String>,
    pub include_stale: bool,
}

struct Match {
    doc_type: String,
    number: Option<i32>,
    title: String,
    status: String,
    criteria: Vec<String>,
    specificity: u8,
}

pub fn run(store: &dyn DocStore, bundle: &Path, query: &Query) -> ExitCode {
    print!("{}", compile(store, bundle, query));
    ExitCode::SUCCESS
}

/// Compiles the `why` result as a string — the pure core, so tests assert on
/// the text. Empty when nothing matches.
pub fn compile(store: &dyn DocStore, bundle: &Path, query: &Query) -> String {
    let all_md = store.list(bundle).unwrap_or_default();
    let stale = liveness::classify(store, bundle, &all_md);

    let mut matches: Vec<Match> = all_md
        .iter()
        .filter(|path| !is_reserved(path))
        .filter(|path| query.include_stale || !stale.is_stale(path))
        .filter_map(|path| match_for(store, path, &query.paths))
        .collect();

    matches.sort_by_key(rank);
    render(&matches)
}

fn match_for(store: &dyn DocStore, path: &Path, query_paths: &[String]) -> Option<Match> {
    let contents = store.read(path).ok()?;
    let record = record::extract_record(path, &contents);
    if record.doc_type.is_empty() || !is_in_force(record.status.as_deref()) {
        return None;
    }
    let entries = path_entries(&record.body);
    let specificity = best_specificity(&entries, query_paths)?;
    Some(Match {
        doc_type: record.doc_type,
        number: record.number,
        title: record.title,
        status: record.status.unwrap_or_default(),
        criteria: verification_criteria(&contents),
        specificity,
    })
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

/// The path-shaped impact entries of a record: backtick tokens that carry a
/// `/` or a `*` and are not annotated removed — dropping descriptive words
/// like `check` that are not paths.
fn path_entries(body: &str) -> Vec<String> {
    impact::tokens(&impact::segment(body))
        .into_iter()
        .filter(|token| !token.removed && (token.text.contains('/') || token.text.contains('*')))
        .map(|token| token.text)
        .collect()
}

/// The most specific match of any entry against any query path: exact (0),
/// directory prefix (1), glob (2). `None` when nothing matches.
fn best_specificity(entries: &[String], query_paths: &[String]) -> Option<u8> {
    entries
        .iter()
        .flat_map(|entry| query_paths.iter().map(move |path| (entry, path)))
        .filter_map(|(entry, path)| match_specificity(entry, path))
        .min()
}

fn match_specificity(entry: &str, path: &str) -> Option<u8> {
    if entry.contains('*') {
        return glob_matches(entry, path).then_some(2);
    }
    let entry = entry.trim_end_matches('/');
    if entry == path {
        return Some(0);
    }
    path.starts_with(&format!("{entry}/")).then_some(1)
}

fn glob_matches(glob: &str, path: &str) -> bool {
    Regex::new(&glob_to_regex(glob)).is_ok_and(|re| re.is_match(path))
}

fn glob_to_regex(glob: &str) -> String {
    let mut re = String::from("^");
    let mut chars = glob.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                re.push_str(".*");
            }
            '*' => re.push_str("[^/]*"),
            other => re.push_str(&regex::escape(&other.to_string())),
        }
    }
    re.push('$');
    re
}

/// The record's `**Verification criteria:**` bullet lines, in order, until the
/// next bold label, heading, or blank line.
fn verification_criteria(contents: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_block = false;
    for line in contents.lines() {
        if line.contains("**Verification criteria:") {
            in_block = true;
            continue;
        }
        if !in_block {
            continue;
        }
        let trimmed = line.trim();
        if let Some(item) = trimmed.strip_prefix("- ") {
            out.push(item.to_string());
        } else if trimmed.is_empty() || trimmed.starts_with("**") || trimmed.starts_with('#') {
            break;
        }
    }
    out
}

fn rank(m: &Match) -> (u8, u8, i32) {
    let type_order = match m.doc_type.as_str() {
        "PRD" => 0,
        "ADR" => 1,
        "BDR" => 2,
        _ => 3,
    };
    (m.specificity, type_order, m.number.unwrap_or(i32::MAX))
}

fn render(matches: &[Match]) -> String {
    if matches.is_empty() {
        return String::new();
    }
    matches
        .iter()
        .map(render_match)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn render_match(m: &Match) -> String {
    let label = match m.number {
        Some(number) => format!("{} {number:04}", m.doc_type),
        None => m.doc_type.clone(),
    };
    let mut block = format!("[{label}] {} ({})", m.title, m.status);
    for criterion in &m.criteria {
        block.push_str(&format!("\n    - {criterion}"));
    }
    block
}

#[cfg(test)]
mod tests;
