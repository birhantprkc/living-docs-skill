use crate::commands::new::unsupported_type_message;
use crate::doc_type::{self, Identity, IndexPartition};
use crate::frontmatter;
use crate::store::DocStore;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Every Numbered-identity registry token, in [`doc_type::DOC_TYPES`] order —
/// the set `index` regenerates when invoked with no explicit type (ADR 0026).
/// A [`Identity::Singleton`] type has no directory to index, so the bare
/// sweep excludes it — regenerating it would need a directory that `new`
/// never creates for a singleton.
fn all_type_tokens() -> Vec<String> {
    doc_type::DOC_TYPES
        .iter()
        .filter(|spec| matches!(spec.identity, Identity::Numbered { .. }))
        .map(|spec| spec.token.to_string())
        .collect()
}

pub fn run(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: Option<String>,
    visibility_filter: Option<Vec<String>>,
) -> ExitCode {
    let types: Vec<String> = match doc_type {
        Some(t) => vec![t],
        None => all_type_tokens(),
    };

    for doc_type in &types {
        if let Err(message) = regenerate(store, docs_dir, doc_type, visibility_filter.as_deref()) {
            eprintln!("living-docs index: {message}");
            return ExitCode::from(2);
        }
    }

    ExitCode::SUCCESS
}

/// `index.md` itself is a reserved fs presentation artifact outside every
/// `DocStore` domain (ADR 0007: never synced to `db-store`), so it is always
/// read/written through `std::fs` regardless of the active backend — only
/// the records feeding its body are read through `store`, meaning a db-mode
/// run regenerates the filesystem `index.md` from the records in the
/// database.
///
/// `doc_type`'s directory coming into existence is `new`'s job, never
/// `index`'s (ADR 0026): a type with no directory yet is a successful no-op
/// here, both for the bare `index` sweep and for an explicit `index
/// <type>` naming a type the bundle doesn't use — otherwise a bare sweep
/// would materialize an empty `index.md` per registry token regardless of
/// whether the bundle carries that type, breaking invariant 3 (an
/// unreachable directory index) for every type the bundle never populated.
fn regenerate(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    visibility_filter: Option<&[String]>,
) -> Result<(), String> {
    let (index_path, content) = compute(store, docs_dir, doc_type, visibility_filter)?;
    let type_dir = index_path.parent().unwrap_or(docs_dir);
    if !type_dir.is_dir() {
        return Ok(());
    }
    fs::write(&index_path, content).map_err(|e| e.to_string())
}

/// Computes `doc_type`'s regenerated `index.md` path and full content,
/// reading the current on-disk file (if any) to preserve its preamble and
/// reading the records feeding its body through `store`, without touching
/// the filesystem itself — the pure step both [`regenerate`] (CLI `index`)
/// and `db-store`'s `write_checked` build on, the latter needing to inspect
/// and control the write/rollback timing itself.
pub fn compute(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    visibility_filter: Option<&[String]>,
) -> Result<(PathBuf, String), String> {
    let dir_name = numbered_dir_for(doc_type)?;
    let type_dir = docs_dir.join(dir_name);
    let records: Vec<Record> = collect_records(store, docs_dir, &type_dir)?
        .into_iter()
        .filter(|record| record_visible(record, visibility_filter))
        .collect();

    let index_path = type_dir.join("index.md");
    let existing = fs::read_to_string(&index_path).unwrap_or_default();
    let preamble = preamble_for(&existing, doc_type);
    let body = render_body(doc_type, &records);

    Ok((index_path, format!("{preamble}{body}")))
}

/// Resolves the numbered-series directory `index` regenerates for
/// `doc_type`: an unknown token gets [`unsupported_type_message`], but a
/// registered [`Identity::Singleton`] token gets its own message instead —
/// it IS supported, it simply has no directory index, and reusing the
/// unsupported-type message would list `doc_type` itself among the tokens
/// the caller is told to pick from.
fn numbered_dir_for(doc_type: &str) -> Result<&'static str, String> {
    let spec = doc_type::spec_for(doc_type).ok_or_else(|| unsupported_type_message(doc_type))?;
    match spec.identity {
        Identity::Numbered { dir } => Ok(dir),
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
    visibility: String,
}

/// The default-deny fallback effective visibility for a record whose
/// frontmatter carries no `visibility` key at all.
const DEFAULT_VISIBILITY: &str = "private";

/// True when `record` belongs in the rendered index under `filter`: every
/// record passes when `filter` is `None` (today's unfiltered dev view, ADR
/// 0009), otherwise only a record whose effective visibility is a member of
/// `filter` passes — default-deny, so an absent-visibility record is only
/// included when `filter` explicitly names `"private"`.
fn record_visible(record: &Record, filter: Option<&[String]>) -> bool {
    match filter {
        None => true,
        Some(allowed) => allowed.contains(&record.visibility),
    }
}

/// Every `NNNN-*.md` record directly under `type_dir`, sorted ascending by
/// `NNNN`, read through `store` (backend-faithful: a db-mode run sees
/// exactly the records the database lists, not whatever happens to sit on
/// disk). `title`/`status` come from each record's frontmatter (S1's
/// reader); `NNNN` comes from the filename, matching how `next`/`new`
/// allocate it.
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
    let visibility = frontmatter::read_scalar_from_str(&contents, "visibility")
        .unwrap_or_else(|| DEFAULT_VISIBILITY.to_string());
    Some(Record {
        number,
        title,
        status,
        filename,
        visibility,
    })
}

/// The record's rendered title: its frontmatter `title:` when present and
/// parseable, otherwise its first `# ` H1 heading with a leading numbering
/// prefix stripped (issue 0021 cause 2 — legacy records that only ever
/// carried the title in their heading). A stderr warning names `path`
/// whenever the fallback fires, since a blank/substituted title is otherwise
/// invisible in the rendered index; only when the H1 is also absent does the
/// title stay empty (still warned).
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

/// Renders `records` along the partition axis `doc_type`'s registry spec
/// declares (ADR 0026): [`IndexPartition::OpenClosed`] for work-in-progress
/// types, [`IndexPartition::ActiveSuperseded`] for types that track what is
/// in force, and [`IndexPartition::Flat`] — also the fallback for an
/// unrecognized `doc_type`, unreachable in practice since every caller
/// already validated it — as a single flat listing (`render_flat_body`).
fn render_body(doc_type: &str, records: &[Record]) -> String {
    match doc_type::spec_for(doc_type).map(|spec| &spec.index_partition) {
        Some(IndexPartition::OpenClosed) => {
            render_partitioned(records, "Open", "Closed", is_open_status)
        }
        Some(IndexPartition::ActiveSuperseded) => {
            render_partitioned(records, "Active", "Superseded", is_active_status)
        }
        Some(IndexPartition::Flat) | None => render_flat_body(records),
    }
}

fn render_flat_body(records: &[Record]) -> String {
    if records.is_empty() {
        return String::new();
    }
    render_rows(records) + "\n"
}

/// Splits records into a `first_heading` section above a `second_heading`
/// section, keyed by `in_first`, so a reader sees what matters now without
/// scrolling through history — see
/// `skills/living-docs/rules/adr-conventions.md` rule 7 for the decision-type
/// case this generalizes from. The first heading is always emitted; either
/// section's rows are omitted (heading only) when that bucket is empty.
fn render_partitioned(
    records: &[Record],
    first_heading: &str,
    second_heading: &str,
    in_first: fn(&str) -> bool,
) -> String {
    let (first, second): (Vec<&Record>, Vec<&Record>) =
        records.iter().partition(|record| in_first(&record.status));

    let mut body = format!("## {first_heading}\n");
    if !first.is_empty() {
        body.push('\n');
        body.push_str(&render_rows_ref(&first));
        body.push('\n');
    }

    if !second.is_empty() {
        body.push_str(&format!("\n## {second_heading}\n\n"));
        body.push_str(&render_rows_ref(&second));
        body.push('\n');
    }

    body
}

/// The decision-type axis (adr/bdr/prd): everything not explicitly retired
/// is still in force, so new decision statuses (e.g. a future vocabulary
/// entry) default to Active without special-casing each type's own words.
fn is_active_status(status: &str) -> bool {
    !matches!(status, "Superseded" | "Deprecated")
}

/// The issue work axis: matched case-insensitively so `done` and `Done` both
/// land in Closed alongside `closed`/`superseded` — the repo's real tracker
/// uses `done` as its closed value. An unknown/empty status is presumed not
/// done yet, so it defaults to Open.
fn is_open_status(status: &str) -> bool {
    !matches!(
        status.to_ascii_lowercase().as_str(),
        "closed" | "done" | "superseded"
    )
}

fn render_rows(records: &[Record]) -> String {
    render_rows_ref(&records.iter().collect::<Vec<_>>())
}

fn render_rows_ref(records: &[&Record]) -> String {
    records
        .iter()
        .map(|record| render_row(record))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_row(record: &Record) -> String {
    let Record {
        number,
        title,
        filename,
        status,
        visibility: _,
    } = record;
    format!("* [{number:04} — {title}]({filename}) - {status}")
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
/// whichever comes first. A single prefix check — rather than pinning the
/// exact heading text per type — is what lets a legacy issues index still
/// carrying `## Done`/`## Open` sections migrate cleanly: the first `## `
/// line is found and replaced, regardless of its old wording. Recognizing a
/// table listing row too (issue 0021 cause 1) is what turns a hand-maintained
/// table-format index into a single migration pass instead of a silent
/// append below it.
fn is_boundary_line(line: &str) -> bool {
    line.starts_with("## ") || line.starts_with("* [") || is_table_listing_row(line)
}

/// True for a Markdown table row (`| cell | cell | ... |`) whose first cell
/// is either a numbered-listing header (`| # |`) or a record link
/// (`| [NNNN](...)` / `| [NNNN-...`) — the two shapes a hand-maintained
/// record table uses in place of the generator's bullet format.
fn is_table_listing_row(line: &str) -> bool {
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
mod tests {
    use super::*;

    /// ADR 0026 fitness function B (index half): the token set `index`
    /// regenerates with no explicit type equals the registry's
    /// Identity::Numbered token set, in the registry's own order.
    #[test]
    fn all_type_tokens_matches_every_numbered_registry_token_in_order() {
        assert_eq!(
            all_type_tokens(),
            vec!["adr", "bdr", "prd", "issue", "research"]
        );
    }

    /// The `constitution` row is a deliberate exclusion, not an oversight:
    /// a singleton has no directory index for the bare sweep to regenerate.
    #[test]
    fn all_type_tokens_excludes_the_constitution_singleton() {
        assert!(!all_type_tokens().contains(&"constitution".to_string()));
    }

    #[test]
    fn numbered_prefix_accepts_four_digit_dash_form() {
        assert_eq!(numbered_prefix("0007-old.md"), Some(7));
    }

    #[test]
    fn numbered_prefix_rejects_index_and_malformed_names() {
        assert_eq!(numbered_prefix("index.md"), None);
        assert_eq!(numbered_prefix("12-old.md"), None);
        assert_eq!(numbered_prefix("abcd-old.md"), None);
    }

    #[test]
    fn render_row_matches_the_locked_row_format() {
        let record = Record {
            number: 7,
            title: "My Title".to_string(),
            status: "Proposed".to_string(),
            filename: "0007-my-title.md".to_string(),
            visibility: "private".to_string(),
        };
        assert_eq!(
            render_row(&record),
            "* [0007 — My Title](0007-my-title.md) - Proposed"
        );
    }

    #[test]
    fn fallback_preamble_is_minimal_for_a_fresh_file() {
        assert_eq!(fallback_preamble("", "adr"), "# ADRs\n\n");
    }

    #[test]
    fn fallback_preamble_wraps_unmarked_existing_content() {
        assert_eq!(
            fallback_preamble("Custom intro.\n", "prd"),
            "Custom intro.\n\n"
        );
    }

    #[test]
    fn find_boundary_offset_locates_the_adr_active_heading() {
        let existing = "# ADRs\n\nIntro.\n\n## Active\n\n* [0001 — X](0001-x.md) - Proposed\n";
        let offset = find_boundary_offset(existing).unwrap();
        assert_eq!(
            &existing[offset..],
            "## Active\n\n* [0001 — X](0001-x.md) - Proposed\n"
        );
    }

    #[test]
    fn find_boundary_offset_locates_the_first_row_for_non_adr_types() {
        let existing = "# PRDs\n\nIntro.\n\n* [0001 — X](0001-x.md) - Draft\n";
        let offset = find_boundary_offset(existing).unwrap();
        assert_eq!(&existing[offset..], "* [0001 — X](0001-x.md) - Draft\n");
    }

    #[test]
    fn find_boundary_offset_locates_a_legacy_heading_regardless_of_its_text() {
        let existing = "# Issues\n\nIntro.\n\n## Done\n\n* [0001 — X](0001-x.md) - closed\n";
        let offset = find_boundary_offset(existing).unwrap();
        assert_eq!(
            &existing[offset..],
            "## Done\n\n* [0001 — X](0001-x.md) - closed\n"
        );
    }

    #[test]
    fn find_boundary_offset_locates_a_hand_maintained_table_header_row() {
        let existing = "# ADRs\n\nIntro.\n\n| # | Decision | Status |\n|---|---|---|\n| [0001](0001-x.md) | X | Accepted |\n";
        let offset = find_boundary_offset(existing).unwrap();
        assert_eq!(
            &existing[offset..],
            "| # | Decision | Status |\n|---|---|---|\n| [0001](0001-x.md) | X | Accepted |\n"
        );
    }

    #[test]
    fn is_boundary_line_detects_a_numbered_listing_table_header() {
        assert!(is_boundary_line("| # | Decision | Status |"));
    }

    #[test]
    fn is_boundary_line_detects_a_table_row_whose_first_cell_is_a_record_link() {
        assert!(is_boundary_line("| [0001](0001-x.md) | X | Accepted |"));
        assert!(is_boundary_line("| [0007-legacy-row | X | Accepted |"));
    }

    #[test]
    fn is_boundary_line_ignores_an_unrelated_table_row() {
        assert!(!is_boundary_line("| Some | Other | Row |"));
        assert!(!is_boundary_line("Just prose, not a table at all."));
    }

    #[test]
    fn is_open_status_treats_closed_done_and_superseded_case_insensitively_as_closed() {
        assert!(!is_open_status("closed"));
        assert!(!is_open_status("Closed"));
        assert!(!is_open_status("done"));
        assert!(!is_open_status("Done"));
        assert!(!is_open_status("Superseded"));
    }

    #[test]
    fn is_open_status_treats_open_in_progress_and_unknown_as_open() {
        assert!(is_open_status("open"));
        assert!(is_open_status("in-progress"));
        assert!(is_open_status("Mystery"));
        assert!(is_open_status(""));
    }

    #[test]
    fn is_active_status_treats_superseded_and_deprecated_as_not_active() {
        assert!(!is_active_status("Superseded"));
        assert!(!is_active_status("Deprecated"));
    }

    #[test]
    fn is_active_status_treats_draft_accepted_and_implemented_as_active() {
        assert!(is_active_status("Draft"));
        assert!(is_active_status("Accepted"));
        assert!(is_active_status("Implemented"));
        assert!(is_active_status("Proposed"));
    }

    #[test]
    fn render_partitioned_pins_the_adr_active_superseded_byte_shape() {
        let records = vec![
            Record {
                number: 1,
                title: "Old".to_string(),
                status: "Superseded".to_string(),
                filename: "0001-old.md".to_string(),
                visibility: "private".to_string(),
            },
            Record {
                number: 2,
                title: "Current".to_string(),
                status: "Accepted".to_string(),
                filename: "0002-current.md".to_string(),
                visibility: "private".to_string(),
            },
        ];

        let body = render_partitioned(&records, "Active", "Superseded", is_active_status);

        assert_eq!(
            body,
            "## Active\n\n* [0002 — Current](0002-current.md) - Accepted\n\n## Superseded\n\n* [0001 — Old](0001-old.md) - Superseded\n"
        );
    }

    #[test]
    fn render_partitioned_emits_only_the_first_heading_when_the_second_bucket_is_empty() {
        let records = vec![Record {
            number: 1,
            title: "Only".to_string(),
            status: "open".to_string(),
            filename: "0001-only.md".to_string(),
            visibility: "private".to_string(),
        }];

        let body = render_partitioned(&records, "Open", "Closed", is_open_status);

        assert_eq!(body, "## Open\n\n* [0001 — Only](0001-only.md) - open\n");
    }

    use std::collections::BTreeMap;
    use std::io;
    use std::path::PathBuf;

    /// A minimal in-memory [`DocStore`] test double, proving `collect_records`
    /// reads a record's title/status through the port rather than the
    /// filesystem — the same double pattern used by `export.rs`/`new.rs`.
    struct MapStore {
        files: BTreeMap<PathBuf, String>,
    }

    impl DocStore for MapStore {
        fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
            Ok(self
                .files
                .keys()
                .filter(|path| path.starts_with(root))
                .cloned()
                .collect())
        }

        fn read(&self, path: &Path) -> io::Result<String> {
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))
        }

        fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn compute_returns_the_index_path_and_content_regenerate_would_write_without_touching_disk() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-first.md"),
            "---\ntype: ADR\ntitle: First\nstatus: Accepted\n---\n# First\n".to_string(),
        );
        let store = MapStore { files };

        let (index_path, content) =
            compute(&store, Path::new("/bundle"), "adr", None).expect("compute should succeed");

        assert_eq!(index_path, PathBuf::from("/bundle/adr/index.md"));
        assert_eq!(
            content,
            "# ADRs\n\n## Active\n\n* [0001 — First](0001-first.md) - Accepted\n"
        );
        assert!(
            !index_path.exists(),
            "compute must not write anything to disk"
        );
    }

    #[test]
    fn regenerate_is_a_no_op_when_the_type_directory_does_not_exist() {
        let store = MapStore {
            files: BTreeMap::new(),
        };
        let docs_dir = std::env::temp_dir().join(format!(
            "living-docs-index-regenerate-noop-{}",
            std::process::id()
        ));
        let type_dir = docs_dir.join("research");
        assert!(!type_dir.exists());

        let result = regenerate(&store, &docs_dir, "research", None);

        assert!(result.is_ok());
        assert!(
            !type_dir.exists(),
            "regenerate must not create the type directory when it is absent"
        );
    }

    #[test]
    fn compute_rejects_an_unsupported_doc_type() {
        let store = MapStore {
            files: BTreeMap::new(),
        };

        let result = compute(&store, Path::new("/bundle"), "glossary", None);

        assert!(result.is_err());
    }

    /// `index constitution` gets its own message, not the unsupported-type
    /// one — the type IS supported, it just has no directory index, and the
    /// unsupported-type message would list the very token the caller used.
    #[test]
    fn compute_rejects_an_explicit_constitution_index_with_its_own_message() {
        let store = MapStore {
            files: BTreeMap::new(),
        };

        let err = compute(&store, Path::new("/bundle"), "constitution", None)
            .expect_err("constitution has no directory index");

        assert!(err.contains("constitution.md"), "got: {err}");
        assert!(
            !err.contains("expected one of"),
            "must not reuse the unsupported-type message: {err}"
        );
    }

    #[test]
    fn title_for_record_prefers_a_present_frontmatter_title_over_the_h1_heading() {
        let contents = "---\ntitle: Frontmatter Title\n---\n# ADR 0007 — Heading Title\n";
        let title = title_for_record(contents, Path::new("adr/0007-x.md"), 7);
        assert_eq!(title, "Frontmatter Title");
    }

    #[test]
    fn title_for_record_falls_back_to_the_h1_heading_stripping_the_adr_number_prefix() {
        let contents = "---\ntype: ADR\n---\n# ADR 0007 — Heading Title\n";
        let title = title_for_record(contents, Path::new("adr/0007-x.md"), 7);
        assert_eq!(title, "Heading Title");
    }

    #[test]
    fn title_for_record_falls_back_to_the_h1_heading_stripping_a_bare_numbered_dot_prefix() {
        let contents = "---\ntype: ADR\n---\n# 0007. Heading Title\n";
        let title = title_for_record(contents, Path::new("adr/0007-x.md"), 7);
        assert_eq!(title, "Heading Title");
    }

    #[test]
    fn title_for_record_falls_back_to_the_h1_heading_stripping_a_bare_numbered_dash_prefix() {
        let contents = "---\ntype: ADR\n---\n# 0007 — Heading Title\n";
        let title = title_for_record(contents, Path::new("adr/0007-x.md"), 7);
        assert_eq!(title, "Heading Title");
    }

    #[test]
    fn title_for_record_is_empty_when_neither_frontmatter_nor_h1_carry_a_title() {
        let contents = "---\ntype: ADR\n---\nBody with no heading.\n";
        let title = title_for_record(contents, Path::new("adr/0007-x.md"), 7);
        assert_eq!(title, "");
    }

    #[test]
    fn collect_records_reads_title_and_status_through_the_store() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-first.md"),
            "---\ntype: ADR\ntitle: First\nstatus: Accepted\n---\n# First\n".to_string(),
        );
        let store = MapStore { files };

        let records = collect_records(&store, Path::new("/bundle"), &PathBuf::from("/bundle/adr"))
            .expect("collect_records should succeed");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].title, "First");
        assert_eq!(records[0].status, "Accepted");
    }

    #[test]
    fn collect_records_ignores_paths_the_store_lists_outside_the_type_directory() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-in-scope.md"),
            "---\ntype: ADR\ntitle: In Scope\nstatus: Proposed\n---\n# In Scope\n".to_string(),
        );
        files.insert(
            PathBuf::from("/bundle/bdr/0001-other-type.md"),
            "---\ntype: BDR\ntitle: Other Type\nstatus: Draft\n---\n# Other Type\n".to_string(),
        );
        let store = MapStore { files };

        let records = collect_records(&store, Path::new("/bundle"), &PathBuf::from("/bundle/adr"))
            .expect("collect_records should succeed");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].filename, "0001-in-scope.md");
    }

    #[test]
    fn collect_records_on_an_empty_store_returns_no_records() {
        let store = MapStore {
            files: BTreeMap::new(),
        };

        let records = collect_records(&store, Path::new("/bundle"), &PathBuf::from("/bundle/adr"))
            .expect("collect_records should succeed on an empty store");

        assert!(records.is_empty());
    }

    #[test]
    fn collect_records_defaults_to_private_when_visibility_is_absent() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-first.md"),
            "---\ntype: ADR\ntitle: First\nstatus: Accepted\n---\n# First\n".to_string(),
        );
        let store = MapStore { files };

        let records = collect_records(&store, Path::new("/bundle"), &PathBuf::from("/bundle/adr"))
            .expect("collect_records should succeed");

        assert_eq!(records[0].visibility, "private");
    }

    #[test]
    fn collect_records_reads_an_explicit_visibility_value() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-first.md"),
            "---\ntype: ADR\ntitle: First\nstatus: Accepted\nvisibility: public\n---\n# First\n"
                .to_string(),
        );
        let store = MapStore { files };

        let records = collect_records(&store, Path::new("/bundle"), &PathBuf::from("/bundle/adr"))
            .expect("collect_records should succeed");

        assert_eq!(records[0].visibility, "public");
    }

    fn record_with_visibility(visibility: &str) -> Record {
        Record {
            number: 1,
            title: "Title".to_string(),
            status: "Accepted".to_string(),
            filename: "0001-title.md".to_string(),
            visibility: visibility.to_string(),
        }
    }

    #[test]
    fn record_visible_passes_every_record_when_the_filter_is_none() {
        assert!(record_visible(&record_with_visibility("private"), None));
        assert!(record_visible(&record_with_visibility("public"), None));
    }

    #[test]
    fn record_visible_excludes_a_record_outside_the_filter_set() {
        let filter = vec!["public".to_string(), "showcase".to_string()];
        assert!(!record_visible(
            &record_with_visibility("private"),
            Some(&filter)
        ));
    }

    #[test]
    fn record_visible_includes_a_record_inside_the_filter_set() {
        let filter = vec!["public".to_string(), "showcase".to_string()];
        assert!(record_visible(
            &record_with_visibility("public"),
            Some(&filter)
        ));
        assert!(record_visible(
            &record_with_visibility("showcase"),
            Some(&filter)
        ));
    }

    #[test]
    fn record_visible_default_deny_only_admits_private_when_explicitly_requested() {
        let private_filter = vec!["private".to_string()];
        let public_filter = vec!["public".to_string()];
        let absent_visibility = record_with_visibility(DEFAULT_VISIBILITY);

        assert!(record_visible(&absent_visibility, Some(&private_filter)));
        assert!(!record_visible(&absent_visibility, Some(&public_filter)));
    }
}
