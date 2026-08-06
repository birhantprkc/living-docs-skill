//! `living-docs brief` (issue 0008) — `new` plus deterministic pre-fill: the
//! frontmatter title, the numbered title heading, a trail comment naming the
//! records this type conventionally links, and every judgment section
//! collapsed to a byte-identical `<!-- judgment: <name> -->` marker an agent
//! can locate without re-reading the file. The tool derives facts only — it
//! never writes rationale prose (ADR 0001 determinism boundary).

use crate::commands::new::{
    fill_frontmatter, fill_frontmatter_title, now_iso8601, unsupported_type_message,
};
use crate::commands::next::next_number_from_store;
use crate::doc_type::{self, Identity};
use crate::paths;
use crate::store::DocStore;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The files a git range touched, resolved by the CLI front (`git diff
/// --name-only <range>`) so the core stays I/O-free.
pub struct DiffContext {
    pub range: String,
    pub files: Vec<String>,
}

pub fn run(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    diff: Option<&DiffContext>,
) -> ExitCode {
    match scaffold_brief(store, docs_dir, doc_type, title, &now_iso8601(), diff) {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("living-docs brief: {message}");
            ExitCode::from(2)
        }
    }
}

fn scaffold_brief(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    timestamp: &str,
    diff: Option<&DiffContext>,
) -> Result<PathBuf, String> {
    let spec = doc_type::spec_for(doc_type).ok_or_else(|| unsupported_type_message(doc_type))?;
    let (target_path, number) = brief_target_for(store, docs_dir, spec, title)?;

    if store.read(&target_path).is_ok() {
        return Err(format!("{} already exists", target_path.display()));
    }

    let content = brief_content(
        spec.template,
        doc_type,
        spec.frontmatter,
        timestamp,
        number,
        title,
        diff,
    );
    store
        .write(&target_path, &content)
        .map_err(|e| e.to_string())?;
    Ok(target_path)
}

/// Resolves `brief`'s target path and heading number for `spec`'s identity
/// shape, mirroring [`crate::commands::new::target_path_for`]: a
/// [`Identity::Numbered`] type allocates the next number and slugifies
/// `title`; a [`Identity::Singleton`] type resolves straight to
/// `docs_dir.join(file)` with no number allocated. The returned `0` for a
/// singleton is never rendered — [`is_title_heading_placeholder`] only
/// recognizes a `# NNNN. <...>` heading, which a singleton template (e.g.
/// `constitution.md`'s plain `# Product Constitution`) does not carry.
fn brief_target_for(
    store: &dyn DocStore,
    docs_dir: &Path,
    spec: &doc_type::DocTypeSpec,
    title: &str,
) -> Result<(PathBuf, u32), String> {
    match spec.identity {
        Identity::Numbered { dir: dir_name } => {
            let number =
                next_number_from_store(store, docs_dir, dir_name).map_err(|e| e.to_string())?;
            let target_path = docs_dir
                .join(dir_name)
                .join(format!("{number:04}-{}.md", paths::slugify(title)));
            Ok((target_path, number))
        }
        Identity::Singleton { file } => Ok((docs_dir.join(file), 0)),
    }
}

fn brief_content(
    template: &str,
    doc_type: &str,
    frontmatter_type: &str,
    timestamp: &str,
    number: u32,
    title: &str,
    diff: Option<&DiffContext>,
) -> String {
    let filled = fill_frontmatter(template, frontmatter_type, timestamp);
    let titled = fill_frontmatter_title(&filled, title);
    let slotted = replace_judgment_sections(&titled, slots_for(doc_type));
    let headed = fill_title_heading(&slotted, doc_type, number, title);
    match diff {
        Some(d) if !d.files.is_empty() => {
            insert_touched_files(&headed, context_marker_for(doc_type), d)
        }
        _ => headed,
    }
}

/// Judgment sections per doc type: heading line → marker name. Everything a
/// slot heading opens (until the next heading) is judgment the authoring
/// model owns; the structural sections (BDR Behavior/Contract/Test Design,
/// PRD NFR table, ADR Verification) keep their template scaffolding.
#[allow(clippy::too_many_lines)]
fn slots_for(doc_type: &str) -> &'static [(&'static str, &'static str)] {
    match doc_type {
        "adr" => &[
            ("## Context", "context"),
            ("## Decision", "decision"),
            ("## Consequences", "consequences"),
            ("# References", "references"),
        ],
        "bdr" => &[
            ("## Context", "context"),
            ("## Textual Description", "textual-description"),
            ("## Scenarios", "scenarios"),
            ("## Related", "related"),
        ],
        "prd" => &[
            ("## Problem / Motivation", "problem-motivation"),
            ("## Goals", "goals"),
            ("## Non-goals", "non-goals"),
            ("## Requirements", "requirements"),
            ("## Acceptance criteria", "acceptance-criteria"),
            ("## Success metrics", "success-metrics"),
            ("## Behavior (BDRs)", "behavior-bdrs"),
            ("## Open questions", "open-questions"),
            ("## Decision log", "decision-log"),
            ("## Related", "related"),
        ],
        "issue" => &[
            ("## <Issue title>", "context"),
            ("### Scope", "scope"),
            ("### Acceptance", "acceptance"),
            ("### Plan", "plan"),
        ],
        "research" => &[
            ("## Question", "question"),
            ("## Method", "method"),
            ("## Implications", "implications"),
            ("## Open Questions", "open-questions"),
            ("# References", "references"),
        ],
        "constitution" => &[
            ("## Product", "product"),
            ("## Scope Boundaries", "scope-boundaries"),
            ("## Non-negotiables", "non-negotiables"),
        ],
        _ => &[],
    }
}

fn context_marker_for(doc_type: &str) -> &'static str {
    match doc_type {
        "prd" => "problem-motivation",
        "research" => "question",
        "constitution" => "product",
        _ => "context",
    }
}

/// Trail stubs live inside a comment so an unfilled scaffold carries no
/// dangling markdown links — `check` stays green on the raw `brief` output.
fn trail_comment_for(doc_type: &str) -> &'static str {
    match doc_type {
        "adr" => "<!-- trail: motivated-by /research/NNNN-<slug>.md · /prd/NNNN-<slug>.md · tracked-by /issues/NNNN-<slug>.md -->",
        "bdr" => "<!-- trail: spawned-by /prd/NNNN-<slug>.md · /adr/NNNN-<slug>.md · tracked-by /issues/NNNN-<slug>.md -->",
        "prd" => "<!-- trail: constitution /constitution.md · behavior /bdr/NNNN-<slug>.md · tracked-by /issues/NNNN-<slug>.md -->",
        "issue" => "<!-- trail: implements /adr/NNNN-<slug>.md · part-of /prd/NNNN-<slug>.md -->",
        "research" => "<!-- trail: motivates /adr/NNNN-<slug>.md · /prd/NNNN-<slug>.md · tracked-by /issues/NNNN-<slug>.md -->",
        _ => "",
    }
}

fn replace_judgment_sections(content: &str, slots: &[(&str, &str)]) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        out.push(lines[i].to_string());
        let Some(marker) = marker_for_heading(lines[i], slots) else {
            i += 1;
            continue;
        };
        out.push(String::new());
        out.push(format!("<!-- judgment: {marker} -->"));
        i = next_heading_index(&lines, i + 1);
        if i < lines.len() {
            out.push(String::new());
        }
    }
    out.join("\n") + "\n"
}

fn marker_for_heading<'a>(line: &str, slots: &[(&str, &'a str)]) -> Option<&'a str> {
    slots
        .iter()
        .find(|(heading, _)| *heading == line)
        .map(|(_, marker)| *marker)
}

fn next_heading_index(lines: &[&str], from: usize) -> usize {
    (from..lines.len())
        .find(|&i| lines[i].starts_with('#'))
        .unwrap_or(lines.len())
}

fn fill_title_heading(content: &str, doc_type: &str, number: u32, title: &str) -> String {
    let filled: Vec<String> = content
        .lines()
        .map(|line| {
            if is_title_heading_placeholder(line, doc_type) {
                filled_heading_with_trail(doc_type, number, title)
            } else {
                line.to_string()
            }
        })
        .collect();
    filled.join("\n") + "\n"
}

fn is_title_heading_placeholder(line: &str, doc_type: &str) -> bool {
    match doc_type {
        "issue" => line == "## <Issue title>",
        _ => line.starts_with("# NNNN. <"),
    }
}

fn filled_heading_with_trail(doc_type: &str, number: u32, title: &str) -> String {
    let heading = match doc_type {
        "issue" => format!("## {title}"),
        _ => format!("# {number:04}. {title}"),
    };
    format!("{heading}\n\n{}", trail_comment_for(doc_type))
}

fn insert_touched_files(content: &str, context_marker: &str, diff: &DiffContext) -> String {
    let marker_line = format!("<!-- judgment: {context_marker} -->");
    let mut out: Vec<String> = Vec::new();
    for line in content.lines() {
        out.push(line.to_string());
        if line == marker_line {
            out.push(String::new());
            out.push(format!(
                "Touched files (`git diff --name-only {}`):",
                diff.range
            ));
            out.push(String::new());
            out.extend(diff.files.iter().map(|file| format!("- `{file}`")));
        }
    }
    out.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::io;

    const TEMPLATE: &str = "---\ntype: ADR\ntitle: <Short decision title>\nstatus: Proposed\ntimestamp: <ISO 8601 datetime>\n---\n\n# NNNN. <Short decision title>\n\n## Context\n\n<guidance with a [link](/research/NNNN-<slug>.md)>\n\n## Decision\n\nWe will <the choice>.\n\n## Consequences\n\n- <what this unlocks>\n\n# References\n\n[1] [<source>](<url>)\n";

    fn briefed(diff: Option<&DiffContext>) -> String {
        brief_content(
            TEMPLATE,
            "adr",
            "ADR",
            "2026-07-19T00:00:00Z",
            7,
            "Choose X",
            diff,
        )
    }

    fn briefed_with_title(title: &str) -> String {
        brief_content(
            TEMPLATE,
            "adr",
            "ADR",
            "2026-07-19T00:00:00Z",
            7,
            title,
            None,
        )
    }

    #[test]
    fn every_judgment_section_collapses_to_exactly_its_marker() {
        let content = briefed(None);
        assert!(content.contains("## Context\n\n<!-- judgment: context -->\n"));
        assert!(content.contains("## Decision\n\n<!-- judgment: decision -->\n"));
        assert!(content.contains("## Consequences\n\n<!-- judgment: consequences -->\n"));
        assert!(content.contains("# References\n\n<!-- judgment: references -->\n"));
        assert!(!content.contains("We will"));
        assert!(!content.contains("guidance with"));
    }

    #[test]
    fn the_frontmatter_title_and_the_numbered_heading_are_filled() {
        let content = briefed(None);
        assert!(content.contains("title: Choose X\n"));
        assert!(content.contains("# 0007. Choose X\n"));
        assert!(!content.contains("<Short decision title>"));
    }

    #[test]
    fn the_frontmatter_title_is_quoted_exactly_when_the_canonical_serializer_would_quote_it() {
        let content = briefed_with_title("Caching: A Deep Dive");
        assert!(content.contains(&format!(
            "title: {}\n",
            crate::record::format_scalar("Caching: A Deep Dive")
        )));
    }

    #[test]
    fn the_trail_comment_sits_under_the_title_heading() {
        let content = briefed(None);
        assert!(content
            .contains("# 0007. Choose X\n\n<!-- trail: motivated-by /research/NNNN-<slug>.md"));
    }

    #[test]
    fn touched_files_land_verbatim_under_the_context_marker() {
        let diff = DiffContext {
            range: "HEAD~1..HEAD".to_string(),
            files: vec!["src/a.rs".to_string(), "docs/b.md".to_string()],
        };
        let content = briefed(Some(&diff));
        assert!(content.contains(
            "<!-- judgment: context -->\n\nTouched files (`git diff --name-only HEAD~1..HEAD`):\n\n- `src/a.rs`\n- `docs/b.md`"
        ));
    }

    #[test]
    fn an_empty_diff_inserts_nothing() {
        let diff = DiffContext {
            range: "HEAD~1..HEAD".to_string(),
            files: Vec::new(),
        };
        assert_eq!(briefed(Some(&diff)), briefed(None));
    }

    #[test]
    fn the_issue_intro_heading_is_both_a_slot_and_the_filled_title() {
        let template = "---\ntype: Issue\ntitle: <Issue title>\nstatus: open\ntimestamp: <ISO 8601 datetime>\n---\n\n## <Issue title>\n\n<intro guidance>\n\n### Scope\n\n<scope guidance>\n";
        let content = brief_content(
            template,
            "issue",
            "Issue",
            "2026-07-19T00:00:00Z",
            3,
            "Fix It",
            None,
        );
        assert!(content.contains("## Fix It\n\n<!-- trail: implements"));
        assert!(content.contains("<!-- judgment: context -->"));
        assert!(content.contains("### Scope\n\n<!-- judgment: scope -->"));
        assert!(!content.contains("intro guidance"));
    }

    #[test]
    fn constitution_judgment_sections_collapse_while_structural_sections_stay_intact() {
        let template = doc_type::spec_for("constitution")
            .expect("constitution must be registered")
            .template;
        let content = brief_content(
            template,
            "constitution",
            "Constitution",
            "2026-07-19T00:00:00Z",
            0,
            "Acme Constitution",
            None,
        );

        assert!(content.contains("## Product\n\n<!-- judgment: product -->\n"));
        assert!(content.contains("## Scope Boundaries\n\n<!-- judgment: scope-boundaries -->\n"));
        assert!(content.contains("## Non-negotiables\n\n<!-- judgment: non-negotiables -->\n"));
        assert!(content.contains("erDiagram"));
        assert!(content.contains("ENTITY_A ||--o{ ENTITY_B"));
        assert!(content.contains("<!-- Append amendments here"));
        assert!(!content.contains("<What the product is"));
        assert!(!content.contains("<Capability or domain"));
        assert!(!content.contains("<Non-negotiable 1>"));
    }

    #[test]
    fn constitution_has_no_trail_comment_and_the_empty_trail_does_not_break_the_output() {
        let template = doc_type::spec_for("constitution")
            .expect("constitution must be registered")
            .template;
        let content = brief_content(
            template,
            "constitution",
            "Constitution",
            "2026-07-19T00:00:00Z",
            0,
            "Acme Constitution",
            None,
        );

        assert!(!content.contains("<!-- trail:"));
        assert!(content.contains("# Product Constitution\n"));
    }

    /// A minimal in-memory [`DocStore`] test double, mirroring the one in
    /// `commands::new`'s tests, so `scaffold_brief`'s singleton branch needs
    /// no filesystem.
    struct MapStore {
        files: RefCell<BTreeMap<PathBuf, String>>,
    }

    impl MapStore {
        fn new() -> Self {
            Self {
                files: RefCell::new(BTreeMap::new()),
            }
        }

        fn seeded(seed: &[(&str, &str)]) -> Self {
            let files = seed
                .iter()
                .map(|(path, contents)| (PathBuf::from(path), (*contents).to_string()))
                .collect();
            Self {
                files: RefCell::new(files),
            }
        }
    }

    impl DocStore for MapStore {
        fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
            Ok(self
                .files
                .borrow()
                .keys()
                .filter(|path| path.starts_with(root))
                .cloned()
                .collect())
        }

        fn read(&self, path: &Path) -> io::Result<String> {
            self.files
                .borrow()
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))
        }

        fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
            self.files
                .borrow_mut()
                .insert(path.to_path_buf(), contents.to_string());
            Ok(())
        }
    }

    #[test]
    fn scaffold_brief_writes_a_singleton_constitution_with_no_number_or_slug() {
        let store = MapStore::new();

        let target = scaffold_brief(
            &store,
            Path::new("/bundle"),
            "constitution",
            "Acme Constitution",
            "2026-07-19T00:00:00Z",
            None,
        )
        .expect("scaffold_brief should succeed");

        assert_eq!(target, PathBuf::from("/bundle/constitution.md"));
        let persisted = store
            .read(&target)
            .expect("scaffold_brief must persist through DocStore::write");
        assert!(persisted.contains("type: Constitution"));
        assert!(persisted.contains("<!-- judgment: product -->"));
    }

    #[test]
    fn scaffold_brief_refuses_a_second_constitution() {
        let store = MapStore::seeded(&[("/bundle/constitution.md", "existing content")]);

        let err = scaffold_brief(
            &store,
            Path::new("/bundle"),
            "constitution",
            "Acme Constitution",
            "2026-07-19T00:00:00Z",
            None,
        )
        .expect_err("a second constitution must be refused");

        assert!(err.contains("already exists"), "got: {err}");
        assert_eq!(
            store.read(Path::new("/bundle/constitution.md")).unwrap(),
            "existing content"
        );
    }
}
