use living_docs_core::record::{extract_record, to_canonical_markdown};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn temp_bundle(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir()
        .join(format!("living-docs-scorecard-cli-{label}-{nanos}"))
        .join("docs");
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_record(bundle: &Path, rel: &str, contents: &str) {
    let path = bundle.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn canonical(path: &Path, hand_written: &str) -> String {
    to_canonical_markdown(&extract_record(path, hand_written))
}

fn adr_body(title: &str, owner: Option<&str>) -> String {
    let owner_line = owner.map(|o| format!("owner: {o}\n")).unwrap_or_default();
    format!(
        "---\ntype: ADR\ntitle: {title}\ndescription: A decision.\n{owner_line}status: Accepted\n---\n\n# {title}\n\nBody.\n"
    )
}

const ROOT_INDEX: &str =
    "# Docs Index\n\n- [ADRs](/adr/index.md)\n- [Context](/context/index.md)\n";
const ADR_INDEX: &str = "# ADR Index\n\n- [First Decision](/adr/0001-first-decision.md)\n";
const CONTEXT_INDEX: &str = "# Context Index\n\n- [Glossary](/context/glossary.md)\n";
const GLOSSARY: &str =
    "---\ntype: Context\ntitle: Glossary\n---\n\n# Glossary\n\nTerm — definition.\n";

fn build_conformant_bundle(label: &str) -> PathBuf {
    let bundle = temp_bundle(label);

    write_record(&bundle, "index.md", ROOT_INDEX);
    write_record(&bundle, "adr/index.md", ADR_INDEX);
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
        &canonical(&adr_path, &adr_body("First Decision", Some("alice"))),
    );

    bundle
}

fn run_scorecard(docs_dir: &Path, json: bool) -> Output {
    let mut args = vec!["--docs-dir", docs_dir.to_str().unwrap(), "scorecard"];
    if json {
        args.push("--json");
    }
    living_docs()
        .args(args)
        .output()
        .expect("failed to run living-docs scorecard")
}

fn snapshot(bundle: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    collect_md(bundle, &mut out);
    out.sort();
    out
}

fn collect_md(dir: &Path, out: &mut Vec<(PathBuf, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_md(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            let contents = fs::read_to_string(&path).unwrap();
            out.push((path, contents));
        }
    }
}

#[test]
fn scorecard_on_a_conformant_tree_exits_zero_and_names_every_attribute() {
    let bundle = build_conformant_bundle("conformant");

    let output = run_scorecard(&bundle, false);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    assert_eq!(output.status.code(), Some(0), "got:\n{stdout}");
    for label in ["Trusted", "Contextual", "Traceable", "Governed", "Overall"] {
        assert!(stdout.contains(label), "missing {label} in:\n{stdout}");
    }
    assert!(stdout.contains("agent-ready"), "got:\n{stdout}");

    let _ = fs::remove_dir_all(bundle.parent().unwrap());
}

#[test]
fn scorecard_still_exits_zero_and_grades_trusted_human_era_when_a_violation_is_present() {
    let bundle = build_conformant_bundle("violation");
    let broken_path = bundle.join("adr").join("0002-broken-link.md");
    let hand_written = "---\ntype: ADR\ntitle: Broken Link\ndescription: A decision.\nowner: alice\nstatus: Accepted\n---\n\n# Broken Link\n\nSee [missing](/adr/0099-missing.md).\n";
    write_record(
        &bundle,
        "adr/0002-broken-link.md",
        &canonical(&broken_path, hand_written),
    );
    write_record(
        &bundle,
        "adr/index.md",
        "# ADR Index\n\n- [First Decision](/adr/0001-first-decision.md)\n- [Broken Link](/adr/0002-broken-link.md)\n",
    );

    let output = run_scorecard(&bundle, false);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    assert_eq!(output.status.code(), Some(0), "got:\n{stdout}");
    assert!(
        stdout
            .lines()
            .any(|line| line.starts_with("Trusted") && line.contains("human-era")),
        "got:\n{stdout}"
    );

    let _ = fs::remove_dir_all(bundle.parent().unwrap());
}

#[test]
fn scorecard_json_output_is_byte_identical_across_two_consecutive_runs() {
    let bundle = build_conformant_bundle("json-determinism");

    let first = run_scorecard(&bundle, true);
    let second = run_scorecard(&bundle, true);

    assert_eq!(first.status.code(), Some(0));
    assert_eq!(second.status.code(), Some(0));
    assert_eq!(first.stdout, second.stdout);
    let stdout = String::from_utf8_lossy(&first.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("scorecard --json must emit valid JSON");
    assert!(parsed.get("overall").is_some(), "got:\n{stdout}");

    let _ = fs::remove_dir_all(bundle.parent().unwrap());
}

#[test]
fn scorecard_never_mutates_the_tree() {
    let bundle = build_conformant_bundle("read-only");
    let before = snapshot(&bundle);

    let output = run_scorecard(&bundle, false);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(before, snapshot(&bundle));

    let _ = fs::remove_dir_all(bundle.parent().unwrap());
}

#[test]
fn scorecard_without_a_db_projection_reports_freshness_not_measured() {
    let bundle = build_conformant_bundle("no-projection");

    let output = run_scorecard(&bundle, true);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    assert_eq!(output.status.code(), Some(0), "got:\n{stdout}");
    assert!(
        stdout.contains("\"freshness\":\"not measured\""),
        "got:\n{stdout}"
    );

    let _ = fs::remove_dir_all(bundle.parent().unwrap());
}
