use crate::commands::new::unsupported_type_message;
use crate::doc_type::{self, Identity};
use crate::frontmatter;
use crate::store::DocStore;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod named;
mod rows;

/// Every registry token with a directory to index (Numbered and Named
/// identities, ADR 0026/0036), in [`doc_type::DOC_TYPES`] order — what a
/// bare `index` regenerates. A [`Identity::Singleton`] has no directory, so
/// the sweep excludes it.
fn all_type_tokens() -> Vec<String> {
    doc_type::DOC_TYPES
        .iter()
        .filter(|spec| {
            matches!(
                spec.identity,
                Identity::Numbered { .. } | Identity::Named { .. }
            )
        })
        .map(|spec| spec.token.to_string())
        .collect()
}

pub fn run(store: &dyn DocStore, docs_dir: &Path, doc_type: Option<String>) -> ExitCode {
    match write(store, docs_dir, doc_type) {
        Ok(written) => {
            written.iter().for_each(|p| println!("{}", p.display()));
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("living-docs index: {message}");
            ExitCode::from(2)
        }
    }
}

/// Regenerates `doc_type`'s index(es), returning the rewritten paths (a
/// type with no directory yet is a silent no-op); [`run`] renders these as
/// plain text, the CLI front also as JSON (ADR 0060).
pub fn write(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: Option<String>,
) -> Result<Vec<PathBuf>, String> {
    let types: Vec<String> = match doc_type {
        Some(t) => vec![t],
        None => all_type_tokens(),
    };
    types
        .iter()
        .filter_map(|doc_type| regenerate(store, docs_dir, doc_type).transpose())
        .collect()
}

/// `index.md` is a reserved fs artifact outside every `DocStore` domain
/// (ADR 0007): always read/written through `std::fs`, even in db-mode,
/// where only the records feeding its body come from `store`.
///
/// A type with no directory yet is a silent no-op (ADR 0026) — `new` owns
/// directory creation, never `index`; regenerating one would materialize an
/// empty `index.md` for every unused registry token, breaking invariant 3.
fn regenerate(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
) -> Result<Option<PathBuf>, String> {
    let (index_path, content) = compute(store, docs_dir, doc_type)?;
    let type_dir = index_path.parent().unwrap_or(docs_dir);
    if !type_dir.is_dir() {
        return Ok(None);
    }
    fs::write(&index_path, content).map_err(|e| e.to_string())?;
    Ok(Some(index_path))
}

/// Computes `doc_type`'s regenerated `index.md` path and content — reading
/// the on-disk preamble and the records through `store` — without writing
/// anything: the pure step [`regenerate`] and `db-store`'s `write_checked`
/// both build on, the latter controlling its own write/rollback timing.
pub fn compute(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
) -> Result<(PathBuf, String), String> {
    let dir_name = numbered_dir_for(doc_type)?;
    let type_dir = docs_dir.join(dir_name);
    let index_path = type_dir.join("index.md");
    let existing = fs::read_to_string(&index_path).unwrap_or_default();
    let preamble = preamble_for(&existing, doc_type);
    let body = body_for(store, docs_dir, doc_type, &type_dir)?;

    Ok((index_path, format!("{preamble}{body}")))
}

/// Renders `doc_type`'s listing body along its identity shape: a Named type
/// delegates to [`named::render_body`] (kind-ranked view rows, ADR 0036),
/// a Numbered one collects `NNNN-*.md` records and renders along its
/// registry partition axis.
fn body_for(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    type_dir: &Path,
) -> Result<String, String> {
    let is_named = matches!(
        doc_type::spec_for(doc_type).map(|spec| spec.identity),
        Some(Identity::Named { .. })
    );
    if is_named {
        return named::render_body(store, docs_dir, type_dir);
    }
    let records = collect_records(store, docs_dir, type_dir)?;
    Ok(rows::render_body(doc_type, &records))
}

/// Resolves the numbered-series directory `index` regenerates for
/// `doc_type`: an unknown token gets [`unsupported_type_message`], but a
/// registered [`Identity::Singleton`] gets its own message — it IS
/// supported, it simply has no directory index.
fn numbered_dir_for(doc_type: &str) -> Result<&'static str, String> {
    let spec = doc_type::spec_for(doc_type).ok_or_else(|| unsupported_type_message(doc_type))?;
    match spec.identity {
        Identity::Numbered { dir } | Identity::Named { dir } => Ok(dir),
        Identity::Singleton { file } => {
            Err(singleton_has_no_directory_index_message(doc_type, file))
        }
    }
}

fn singleton_has_no_directory_index_message(doc_type: &str, file: &str) -> String {
    format!(
        "'{doc_type}' has no directory index — it writes a single {file} at the bundle root, not a numbered series"
    )
}

struct Record {
    number: u32,
    title: String,
    status: String,
    filename: String,
    superseded_by: Option<String>,
}

/// Every `NNNN-*.md` record directly under `type_dir`, sorted ascending by
/// `NNNN`, read through `store` (backend-faithful). `title`/`status` come
/// from each record's frontmatter; `NNNN` from the filename.
fn collect_records(
    store: &dyn DocStore,
    docs_dir: &Path,
    type_dir: &Path,
) -> Result<Vec<Record>, String> {
    let paths = store.list(docs_dir).map_err(|e| e.to_string())?;

    let mut records: Vec<Record> = paths
        .iter()
        .filter(|path| path.parent() == Some(type_dir))
        .filter_map(|path| record_from_path(store, path))
        .collect();

    records.sort_by_key(|record| record.number);
    Ok(records)
}

fn record_from_path(store: &dyn DocStore, path: &Path) -> Option<Record> {
    let filename = path.file_name()?.to_str()?.to_string();
    let number = numbered_prefix(&filename)?;
    let contents = store.read(path).ok()?;
    let title = title_for_record(&contents, path, number);
    let status = frontmatter::read_scalar_from_str(&contents, "status").unwrap_or_default();
    let superseded_by = frontmatter::read_scalar_from_str(&contents, "superseded_by");
    Some(Record {
        number,
        title,
        status,
        filename,
        superseded_by,
    })
}

/// The record's rendered title: its frontmatter `title:` when present and
/// parseable, otherwise its first `# ` H1 heading with a leading numbering
/// prefix stripped (issue 0021 cause 2). A stderr warning names `path`
/// whenever the fallback fires; the title stays empty (still warned) only
/// when the H1 is also absent.
fn title_for_record(contents: &str, path: &Path, number: u32) -> String {
    if let Some(title) = frontmatter::read_scalar_from_str(contents, "title") {
        return title;
    }
    let fallback = first_heading(contents)
        .map(|heading| strip_heading_number_prefix(&heading, number))
        .unwrap_or_default();
    warn_missing_title(path, &fallback);
    fallback
}

fn first_heading(contents: &str) -> Option<String> {
    contents
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(|title| title.trim().to_owned()))
}

/// Strips a leading `ADR NNNN — `, `NNNN. `, or `NNNN — ` numbering prefix
/// (`NNNN` being `number` zero-padded to four digits) from `heading`, in
/// that order, leaving it untouched when none matches.
fn strip_heading_number_prefix(heading: &str, number: u32) -> String {
    let padded = format!("{number:04}");
    [
        format!("ADR {padded} — "),
        format!("{padded}. "),
        format!("{padded} — "),
    ]
    .into_iter()
    .find_map(|prefix| heading.strip_prefix(prefix.as_str()).map(str::to_owned))
    .unwrap_or_else(|| heading.to_owned())
}

fn warn_missing_title(path: &Path, fallback: &str) {
    if fallback.is_empty() {
        eprintln!(
            "living-docs index: {} has no parseable 'title' frontmatter and no H1 heading; rendering an empty title",
            path.display()
        );
    } else {
        eprintln!(
            "living-docs index: {} has no parseable 'title' frontmatter; using its H1 heading {fallback:?}",
            path.display()
        );
    }
}

fn numbered_prefix(filename: &str) -> Option<u32> {
    if !filename.ends_with(".md") || filename.as_bytes().get(4) != Some(&b'-') {
        return None;
    }
    let prefix = filename.get(0..4)?;
    if !prefix.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    prefix.parse().ok()
}

/// Everything above the first generator-managed heading survives byte-for-byte —
/// this is what makes `index` idempotent on the second run, since the boundary is
/// found at the same offset both times. A fresh (or marker-less) file falls back to
/// a minimal `# <Title>` preamble.
fn preamble_for(existing: &str, doc_type: &str) -> String {
    match find_boundary_offset(existing) {
        Some(offset) => existing[..offset].to_string(),
        None => fallback_preamble(existing, doc_type),
    }
}

fn find_boundary_offset(existing: &str) -> Option<usize> {
    let mut offset = 0;
    for line in existing.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\n', '\r']);
        if is_boundary_line(trimmed) {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

/// Any generator-managed heading (`## `, whatever its text), bullet listing
/// row, or hand-maintained Markdown table listing row is a boundary,
/// whichever comes first — letting a legacy `## Done`/`## Open` index or a
/// hand-maintained table (issue 0021) migrate in a single pass rather than
/// a silent append below it.
fn is_boundary_line(line: &str) -> bool {
    line.starts_with("## ") || line.starts_with("* [") || is_table_listing_row(line)
}

/// True for a Markdown table row (`| cell | cell | ... |`) whose first cell
/// is either a numbered-listing header (`| # |`) or a record link
/// (`| [NNNN](...)` / `| [NNNN-...`) — the two shapes a hand-maintained
/// record table uses in place of the generator's bullet format.
pub(crate) fn is_table_listing_row(line: &str) -> bool {
    let Some(first_cell) = first_table_cell(line) else {
        return false;
    };
    first_cell == "#" || is_record_link_cell(first_cell)
}

fn first_table_cell(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix('|')?;
    let cell = rest.split('|').next()?;
    Some(cell.trim())
}

fn is_record_link_cell(cell: &str) -> bool {
    let Some(after_bracket) = cell.strip_prefix('[') else {
        return false;
    };
    after_bracket.len() >= 4
        && after_bracket
            .get(..4)
            .is_some_and(|digits| digits.chars().all(|c| c.is_ascii_digit()))
}

fn fallback_preamble(existing: &str, doc_type: &str) -> String {
    let trimmed = existing.trim();
    if trimmed.is_empty() {
        format!("# {}\n\n", heading_title_for(doc_type))
    } else {
        format!("{trimmed}\n\n")
    }
}

fn heading_title_for(doc_type: &str) -> &'static str {
    doc_type::spec_for(doc_type)
        .map(|spec| spec.index_heading)
        .unwrap_or("Index")
}

#[cfg(test)]
mod store_tests;
#[cfg(test)]
mod tests;
