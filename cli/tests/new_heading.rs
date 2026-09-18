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
    let dir = std::env::temp_dir().join(format!("living-docs-new-heading-{label}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_new(docs: &Path, doc_type: &str, title: &str) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "new", doc_type, title])
        .output()
        .expect("failed to run living-docs")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn new_fills_the_title_heading_of_a_numbered_record() {
    let docs = temp_dir("heading-numbered");

    let output = run_new(&docs, "adr", "Pick The Queue");
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));

    let contents = fs::read_to_string(docs.join("adr/0001-pick-the-queue.md")).unwrap();
    assert!(
        contents.contains("\n# 0001. Pick The Queue\n"),
        "got:\n{contents}"
    );
    assert!(
        !contents.contains("<Short decision title>"),
        "got:\n{contents}"
    );

    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn new_fills_the_bare_title_heading_of_a_named_record() {
    let docs = temp_dir("heading-named");

    let output = run_new(&docs, "view", "Request Flow");
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));

    let contents = fs::read_to_string(docs.join("architecture/request-flow.md")).unwrap();
    assert!(contents.contains("\n# Request Flow\n"), "got:\n{contents}");

    let _ = fs::remove_dir_all(&docs);
}
