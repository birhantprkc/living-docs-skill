use super::*;
use crate::record::{extract_record, to_canonical_markdown};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct RealFsStore;

impl DocStore for RealFsStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        let mut found = Vec::new();
        collect_md(root, &mut found);
        found.sort();
        Ok(found)
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)
    }
}

fn collect_md(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_md(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

struct TempBundle {
    root: PathBuf,
}

impl TempBundle {
    fn new(label: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("living-docs-scorecard-{label}-{nanos}"));
        fs::create_dir_all(&root).expect("create temp bundle root");
        Self { root }
    }

    fn bundle(&self) -> PathBuf {
        self.root.join("docs")
    }
}

impl Drop for TempBundle {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_record(bundle: &Path, rel: &str, contents: &str) {
    let path = bundle.join(rel);
    fs::create_dir_all(path.parent().expect("record has a parent dir"))
        .expect("create record parent dir");
    fs::write(path, contents).expect("write record");
}

fn canonical(path: &Path, hand_written: &str) -> String {
    to_canonical_markdown(&extract_record(path, hand_written))
}

fn adr_body(title: &str, owner: Option<&str>, superseded_by: Option<&str>) -> String {
    let owner_line = owner.map(|o| format!("owner: {o}\n")).unwrap_or_default();
    let (status, superseded_by_line) = match superseded_by {
        Some(target) => ("Superseded", format!("superseded_by: \"{target}\"\n")),
        None => ("Accepted", String::new()),
    };
    format!(
        "---\ntype: ADR\ntitle: {title}\ndescription: A decision.\n{owner_line}status: {status}\n{superseded_by_line}---\n\n# {title}\n\nBody.\n"
    )
}

const ROOT_INDEX: &str =
    "# Docs Index\n\n- [ADRs](/adr/index.md)\n- [Context](/context/index.md)\n";
const CONTEXT_INDEX: &str = "# Context Index\n\n- [Glossary](/context/glossary.md)\n";
const GLOSSARY: &str =
    "---\ntype: Context\ntitle: Glossary\n---\n\n# Glossary\n\nTerm — definition.\n";

/// Builds a bundle that clears every `check` invariant: a reachable root and
/// directory indexes, a glossary, a canonical constitution singleton, and one
/// owner-carrying ADR — the shared base every degradation test starts from
/// and layers exactly one change onto.
fn build_conformant_bundle(label: &str) -> TempBundle {
    let temp = TempBundle::new(label);
    let bundle = temp.bundle();

    write_record(&bundle, "index.md", ROOT_INDEX);
    write_record(
        &bundle,
        "adr/index.md",
        "# ADR Index\n\n- [First Decision](/adr/0001-first-decision.md)\n",
    );
    write_record(&bundle, "context/index.md", CONTEXT_INDEX);
    write_record(&bundle, "context/glossary.md", GLOSSARY);

    let constitution_path = bundle.join("constitution.md");
    write_record(
        &bundle,
        "constitution.md",
        &canonical(
            &constitution_path,
            "---\ntype: Constitution\ntitle: Project Constitution\n---\n\n# Project Constitution\n\nBody.\n",
        ),
    );

    let adr_path = bundle.join("adr").join("0001-first-decision.md");
    write_record(
        &bundle,
        "adr/0001-first-decision.md",
        &canonical(&adr_path, &adr_body("First Decision", Some("alice"), None)),
    );

    temp
}

#[test]
fn a_conformant_tree_grades_every_measured_attribute_agent_ready_and_overall_agent_ready() {
    let temp = build_conformant_bundle("conformant");
    let store = RealFsStore;

    let scorecard = compute(&store, &temp.bundle(), None);

    assert_eq!(scorecard.trusted, Grade::AgentReady);
    assert_eq!(scorecard.contextual, Grade::AgentReady);
    assert_eq!(scorecard.traceable, Grade::AgentReady);
    assert_eq!(scorecard.governed, Grade::AgentReady);
    assert_eq!(scorecard.overall, Grade::AgentReady);
}

#[test]
fn removing_the_owner_from_an_owner_requiring_record_drops_only_governed() {
    let temp = build_conformant_bundle("owner-removed");
    let bundle = temp.bundle();
    let adr_path = bundle.join("adr").join("0001-first-decision.md");
    write_record(
        &bundle,
        "adr/0001-first-decision.md",
        &canonical(&adr_path, &adr_body("First Decision", None, None)),
    );
    let store = RealFsStore;

    let scorecard = compute(&store, &bundle, None);

    assert_eq!(scorecard.governed, Grade::HumanEra);
    assert_eq!(scorecard.trusted, Grade::AgentReady);
    assert_eq!(scorecard.contextual, Grade::AgentReady);
    assert_eq!(scorecard.traceable, Grade::AgentReady);
    assert_eq!(scorecard.overall, Grade::HumanEra);
}

#[test]
fn an_open_moved_source_advisory_drops_only_traceable_to_in_transition() {
    let temp = build_conformant_bundle("moved-source");
    let bundle = temp.bundle();

    let old_path = bundle.join("adr").join("0002-old-decision.md");
    write_record(
        &bundle,
        "adr/0002-old-decision.md",
        &canonical(
            &old_path,
            &adr_body("Old Decision", Some("alice"), Some("0003")),
        ),
    );
    let new_path = bundle.join("adr").join("0003-new-decision.md");
    write_record(
        &bundle,
        "adr/0003-new-decision.md",
        &canonical(&new_path, &adr_body("New Decision", Some("alice"), None)),
    );
    write_record(
        &bundle,
        "adr/index.md",
        "# ADR Index\n\n- [First Decision](/adr/0001-first-decision.md)\n- [Old Decision](/adr/0002-old-decision.md)\n- [New Decision](/adr/0003-new-decision.md)\n",
    );
    write_record(
        &bundle,
        "context/glossary.md",
        "---\ntype: Context\ntitle: Glossary\n---\n\n# Glossary\n\nSee the [old decision](/adr/0002-old-decision.md).\n",
    );
    let store = RealFsStore;

    let scorecard = compute(&store, &bundle, None);

    assert_eq!(scorecard.traceable, Grade::InTransition);
    assert_eq!(scorecard.trusted, Grade::AgentReady);
    assert_eq!(scorecard.contextual, Grade::AgentReady);
    assert_eq!(scorecard.governed, Grade::AgentReady);
    assert_eq!(scorecard.overall, Grade::InTransition);
}

#[test]
fn an_absent_freshness_signal_renders_not_measured_and_never_drags_down_trusted() {
    let temp = build_conformant_bundle("freshness-absent");
    let store = RealFsStore;

    let scorecard = compute(&store, &temp.bundle(), None);

    assert!(scorecard.freshness.is_none());
    assert_eq!(scorecard.trusted, Grade::AgentReady);
    assert_eq!(scorecard.overall, Grade::AgentReady);
    assert!(render_table(&scorecard).contains("freshness: not measured"));
    assert!(render_json(&scorecard).contains("\"freshness\":\"not measured\""));
}

#[test]
fn a_stale_freshness_signal_demotes_trusted_to_in_transition() {
    let temp = build_conformant_bundle("freshness-stale");
    let store = RealFsStore;

    let scorecard = compute(&store, &temp.bundle(), Some(Freshness::Stale));

    assert_eq!(scorecard.trusted, Grade::InTransition);
    assert_eq!(scorecard.overall, Grade::InTransition);
}

#[test]
fn two_consecutive_computations_over_the_same_tree_are_byte_identical() {
    let temp = build_conformant_bundle("determinism");
    let store = RealFsStore;

    let first = render_json(&compute(&store, &temp.bundle(), None));
    let second = render_json(&compute(&store, &temp.bundle(), None));

    assert_eq!(first, second);
}
