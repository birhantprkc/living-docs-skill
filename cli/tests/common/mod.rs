#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

pub fn run_check(bundle: &Path) -> Output {
    living_docs()
        .args(["check", bundle.to_str().unwrap()])
        .output()
        .expect("failed to run living-docs check")
}

pub fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Builds a scratch bundle directory unique to this process run, scoped by
/// `suite` (the calling test file) and `label` (the individual test case).
pub fn temp_bundle(suite: &str, label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir()
        .join(format!("living-docs-{suite}-test-{label}-{nanos}"))
        .join("docs");
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn write(bundle: &Path, rel: &str, contents: &str) {
    let path = bundle.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

/// Fixtures live under `skills/living-docs/tests/fixtures` relative to the repo
/// root; `CARGO_MANIFEST_DIR` anchors this at compile time regardless of the
/// working directory `cargo test` is invoked from.
pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .join("skills/living-docs/tests/fixtures")
        .join(name)
        .join("docs")
}
