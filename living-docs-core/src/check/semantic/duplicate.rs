//! `DUPLICATE` advisory (ADR 0055): the instrument behind the "duplicate home"
//! refusal trigger. Near-duplicate detection over same-type record bodies via
//! shingled Jaccard similarity above a high threshold, on prose with headings
//! and bold labels stripped so the shared MADR skeleton does not inflate the
//! score. Advisory only — it names both files and never gates.

use crate::check::records::is_reserved;
use crate::check::{file_name_str, Reporter};
use crate::record;
use crate::store::DocStore;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const SIMILARITY_THRESHOLD: f64 = 0.8;
const SHINGLE: usize = 3;
/// A record with fewer prose tokens than this carries too little signal to
/// compare — a stub or near-empty body is never a meaningful duplicate.
const MIN_TOKENS: usize = 10;

struct Doc {
    path: PathBuf,
    doc_type: String,
    shingles: BTreeSet<String>,
}

pub(super) fn check(store: &dyn DocStore, all_md: &[PathBuf], reporter: &mut Reporter) {
    let docs: Vec<Doc> = all_md.iter().filter_map(|p| doc_of(store, p)).collect();
    for (i, a) in docs.iter().enumerate() {
        for b in &docs[i + 1..] {
            if a.doc_type != b.doc_type {
                continue;
            }
            let score = jaccard(&a.shingles, &b.shingles);
            if score >= SIMILARITY_THRESHOLD {
                reporter.advise(&b.path, duplicate_message(&a.path, score));
            }
        }
    }
}

fn doc_of(store: &dyn DocStore, path: &Path) -> Option<Doc> {
    if is_reserved(&file_name_str(path)) {
        return None;
    }
    let contents = store.read(path).ok()?;
    let record = record::extract_record(path, &contents);
    if record.doc_type.is_empty() {
        return None;
    }
    let tokens = prose_tokens(&record.body);
    if tokens.len() < MIN_TOKENS {
        return None;
    }
    let shingles = shingles(&tokens);
    (!shingles.is_empty()).then(|| Doc {
        path: path.to_path_buf(),
        doc_type: record.doc_type,
        shingles,
    })
}

/// Lowercased word tokens of the record's prose, with heading lines, bold
/// labels, and list markers dropped — the MADR skeleton is shared by every
/// record of a type, so counting it would make unrelated records look alike.
fn prose_tokens(body: &str) -> Vec<String> {
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with("**"))
        .flat_map(|line| line.trim_start_matches(['-', '*', ' ']).split_whitespace())
        .map(|word| word.to_lowercase())
        .collect()
}

fn shingles(tokens: &[String]) -> BTreeSet<String> {
    if tokens.len() < SHINGLE {
        return tokens.iter().cloned().collect();
    }
    tokens.windows(SHINGLE).map(|w| w.join(" ")).collect()
}

fn jaccard(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        return 0.0;
    }
    intersection as f64 / union as f64
}

fn duplicate_message(other: &Path, score: f64) -> String {
    format!(
        "DUPLICATE near-duplicate of {} (similarity {score:.2}) — one home per fact; cross-reference instead",
        file_name_str(other)
    )
}

#[cfg(test)]
mod tests;
