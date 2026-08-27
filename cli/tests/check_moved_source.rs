use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn run_check(bundle: &Path) -> Output {
    living_docs()
        .args(["check", bundle.to_str().unwrap()])
        .output()
        .expect("failed to run living-docs check")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn temp_bundle(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir()
        .join(format!("living-docs-moved-source-test-{label}-{nanos}"))
        .join("docs");
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(bundle: &Path, rel: &str, contents: &str) {
    let path = bundle.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn write_moved_source_tree(bundle: &Path, dependent_body: &str) {
    write(
        bundle,
        "index.md",
        "# Index\n\n- [A](a.md)\n- [B](b.md)\n- [C](c.md)\n",
    );
    write(
        bundle,
        "a.md",
        &format!("---\ntype: Reference\ntitle: A\ndescription: Dependent record.\n---\n# A\n\n{dependent_body}\n"),
    );
    write(
        bundle,
        "b.md",
        "---\ntype: Reference\ntitle: B\ndescription: Moved source.\nstatus: Superseded\nsuperseded_by: c\n---\n# B\n",
    );
    write(
        bundle,
        "c.md",
        "---\ntype: Reference\ntitle: C\ndescription: Successor.\n---\n# C\n",
    );
}

#[test]
fn moved_source_finding_names_dependent_source_status_and_successor_and_stays_exit_zero() {
    let bundle = temp_bundle("finding");
    write_moved_source_tree(&bundle, "[b](./b.md)");

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "advisory-only finding must not change the exit code, got:\n{stdout}"
    );
    assert!(stdout.contains("MOVED-SOURCE"), "got:\n{stdout}");
    assert!(stdout.contains("a.md"), "got:\n{stdout}");
    assert!(stdout.contains("b.md"), "got:\n{stdout}");
    assert!(stdout.contains("Superseded"), "got:\n{stdout}");
    assert!(stdout.contains("superseded by c"), "got:\n{stdout}");

    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn linking_the_successor_clears_the_moved_source_finding() {
    let bundle = temp_bundle("cleared");
    write_moved_source_tree(&bundle, "[b](./b.md) [c](./c.md)");

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected clean, got:\n{stdout}"
    );
    assert!(!stdout.contains("MOVED-SOURCE"), "got:\n{stdout}");

    let _ = fs::remove_dir_all(&bundle);
}

#[test]
fn a_broken_link_alongside_a_moved_source_candidate_keeps_its_own_error_class_and_exit_code() {
    let bundle = temp_bundle("broken");
    write(
        &bundle,
        "index.md",
        "# Index\n\n- [A](a.md)\n- [B](b.md)\n- [C](c.md)\n",
    );
    write(
        &bundle,
        "a.md",
        "---\ntype: Reference\ntitle: A\ndescription: Dependent record.\n---\n# A\n\n[b](./b.md)\n[missing](./no-such.md)\n",
    );
    write(
        &bundle,
        "b.md",
        "---\ntype: Reference\ntitle: B\ndescription: Moved source.\nstatus: Superseded\nsuperseded_by: c\n---\n# B\n",
    );
    write(
        &bundle,
        "c.md",
        "---\ntype: Reference\ntitle: C\ndescription: Successor.\n---\n# C\n",
    );

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(1), "got:\n{stdout}");
    assert!(
        stdout.contains("broken link"),
        "expected the existing broken-link error class, got:\n{stdout}"
    );
    assert!(stdout.contains("MOVED-SOURCE"), "got:\n{stdout}");
    for line in stdout.lines().filter(|line| line.contains("broken link")) {
        assert!(
            !line.contains("MOVED-SOURCE"),
            "a broken link must never be labelled MOVED-SOURCE, got:\n{line}"
        );
    }

    let _ = fs::remove_dir_all(&bundle);
}
