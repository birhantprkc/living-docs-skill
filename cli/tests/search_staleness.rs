use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("living-docs-search-staleness-{label}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_record(docs: &Path, filename: &str, title: &str, body: &str) {
    let type_dir = docs.join("adr");
    fs::create_dir_all(&type_dir).unwrap();
    let contents = format!(
        "---\ntype: ADR\ntitle: {title}\ndescription: d.\nstatus: Accepted\n---\n\n# {title}\n\n{body}\n"
    );
    fs::write(type_dir.join(filename), contents).unwrap();
}

fn run(cwd: &Path, docs: &Path, args: &[&str]) -> Output {
    let mut full_args = vec!["--docs-dir", docs.to_str().unwrap()];
    full_args.extend_from_slice(args);
    living_docs()
        .current_dir(cwd)
        .args(full_args)
        .output()
        .expect("failed to run living-docs")
}

fn sync(cwd: &Path, docs: &Path) -> Output {
    run(cwd, docs, &["--engine", "sqlite", "db", "sync"])
}

fn search(cwd: &Path, docs: &Path, strict: bool) -> Output {
    let mut args = vec!["--engine", "sqlite", "search", "quokka"];
    if strict {
        args.push("--strict");
    }
    run(cwd, docs, &args)
}

#[test]
fn search_strict_on_an_unchanged_tree_after_sync_exits_zero_with_no_warning() {
    let docs = temp_dir("fresh-docs");
    let cwd = temp_dir("fresh-cwd");
    write_record(
        &docs,
        "0001-quokka.md",
        "Quokka Strategy",
        "Body about quokka caching.",
    );

    assert!(sync(&cwd, &docs).status.success());

    let output = search(&cwd, &docs, true);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());
    assert!(!String::from_utf8_lossy(&output.stdout).trim().is_empty());

    let _ = fs::remove_dir_all(&docs);
    let _ = fs::remove_dir_all(&cwd);
}

#[test]
fn search_after_an_unsynced_edit_warns_on_stderr_and_still_returns_results() {
    let docs = temp_dir("edited-docs");
    let cwd = temp_dir("edited-cwd");
    write_record(
        &docs,
        "0001-quokka.md",
        "Quokka Strategy",
        "Body about quokka caching.",
    );
    assert!(sync(&cwd, &docs).status.success());

    write_record(
        &docs,
        "0002-second.md",
        "Second Record",
        "More quokka content added.",
    );

    let output = search(&cwd, &docs, false);

    assert!(
        output.status.success(),
        "a non-strict stale search still succeeds, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("db sync"), "got: {stderr}");
    assert!(!String::from_utf8_lossy(&output.stdout).trim().is_empty());

    let _ = fs::remove_dir_all(&docs);
    let _ = fs::remove_dir_all(&cwd);
}

#[test]
fn search_strict_after_an_unsynced_edit_exits_nonzero_with_no_results() {
    let docs = temp_dir("strict-edited-docs");
    let cwd = temp_dir("strict-edited-cwd");
    write_record(
        &docs,
        "0001-quokka.md",
        "Quokka Strategy",
        "Body about quokka caching.",
    );
    assert!(sync(&cwd, &docs).status.success());

    write_record(
        &docs,
        "0002-second.md",
        "Second Record",
        "More quokka content added.",
    );

    let output = search(&cwd, &docs, true);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("db sync"), "got: {stderr}");
    assert!(String::from_utf8_lossy(&output.stdout).trim().is_empty());

    let _ = fs::remove_dir_all(&docs);
    let _ = fs::remove_dir_all(&cwd);
}

#[test]
fn search_strict_is_fresh_again_after_a_resync() {
    let docs = temp_dir("resync-docs");
    let cwd = temp_dir("resync-cwd");
    write_record(
        &docs,
        "0001-quokka.md",
        "Quokka Strategy",
        "Body about quokka caching.",
    );
    assert!(sync(&cwd, &docs).status.success());
    write_record(
        &docs,
        "0002-second.md",
        "Second Record",
        "More quokka content added.",
    );
    assert!(!search(&cwd, &docs, true).status.success());

    assert!(sync(&cwd, &docs).status.success());
    let output = search(&cwd, &docs, true);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).is_empty());

    let _ = fs::remove_dir_all(&docs);
    let _ = fs::remove_dir_all(&cwd);
}

#[test]
fn search_strict_treats_a_project_with_no_sync_meta_row_as_stale() {
    let docs = temp_dir("no-row-docs");
    let cwd = temp_dir("no-row-cwd");
    write_record(
        &docs,
        "0001-quokka.md",
        "Quokka Strategy",
        "Body about quokka caching.",
    );
    assert!(sync(&cwd, &docs).status.success());

    let output = run(
        &cwd,
        &docs,
        &[
            "--engine",
            "sqlite",
            "search",
            "quokka",
            "--project",
            "never-synced",
            "--strict",
        ],
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("db sync"), "got: {stderr}");

    let _ = fs::remove_dir_all(&docs);
    let _ = fs::remove_dir_all(&cwd);
}
