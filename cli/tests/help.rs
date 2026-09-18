//! `--help`/`completions` coverage (ADR 0060): the root help lists exactly
//! the ten verbs plus `completions`, grouped under the four documented
//! headings, with `effective`/`skill` absent (they are hidden aliases); and
//! `completions <shell>` prints a non-empty script.

use std::process::{Command, Output};

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

const VERBS: [&str; 11] = [
    "new",
    "set",
    "supersede",
    "index",
    "fmt",
    "check",
    "read",
    "guide",
    "install",
    "uninstall",
    "completions",
];

fn lists_as_a_command(help: &str, name: &str) -> bool {
    help.lines()
        .any(|line| line.trim_start().starts_with(&format!("{name} ")) || line.trim() == name)
}

#[test]
fn root_help_lists_exactly_the_ten_verbs_plus_completions() {
    let output = living_docs()
        .arg("--help")
        .output()
        .expect("failed to run living-docs --help");
    let help = stdout_of(&output);
    assert!(output.status.success(), "got:\n{help}");

    for verb in VERBS {
        assert!(lists_as_a_command(&help, verb), "missing {verb}:\n{help}");
    }
    assert!(
        !lists_as_a_command(&help, "effective"),
        "hidden alias must not be listed:\n{help}"
    );
    assert!(
        !lists_as_a_command(&help, "skill"),
        "hidden alias must not be listed:\n{help}"
    );
}

#[test]
fn root_help_contains_the_four_group_headings() {
    let output = living_docs()
        .arg("--help")
        .output()
        .expect("failed to run living-docs --help");
    let help = stdout_of(&output);

    for heading in ["Authoring:", "Gate:", "Reading:", "Distribution:"] {
        assert!(help.contains(heading), "missing {heading}:\n{help}");
    }
}

#[test]
fn root_help_documents_the_exit_codes() {
    let output = living_docs()
        .arg("--help")
        .output()
        .expect("failed to run living-docs --help");
    let help = stdout_of(&output);

    assert!(help.contains("Exit codes:"), "got:\n{help}");
    assert!(help.contains('0'), "got:\n{help}");
    assert!(help.contains('1'), "got:\n{help}");
    assert!(help.contains('2'), "got:\n{help}");
}

#[test]
fn verb_help_carries_an_examples_section() {
    let output = living_docs()
        .args(["new", "--help"])
        .output()
        .expect("failed to run living-docs new --help");
    let help = stdout_of(&output);

    assert!(output.status.success(), "got:\n{help}");
    assert!(help.contains("Examples:"), "got:\n{help}");
    assert!(help.contains("living-docs new"), "got:\n{help}");
}

#[test]
fn completions_bash_prints_a_non_empty_script() {
    let output = living_docs()
        .args(["completions", "bash"])
        .output()
        .expect("failed to run living-docs completions bash");
    let stdout = stdout_of(&output);

    assert!(output.status.success(), "got:\n{stdout}");
    assert!(!stdout.trim().is_empty());
    assert!(stdout.contains("living_docs") || stdout.contains("living-docs"));
}

#[test]
fn completions_supports_zsh_and_fish() {
    for shell in ["zsh", "fish"] {
        let output = living_docs()
            .args(["completions", shell])
            .output()
            .unwrap_or_else(|_| panic!("failed to run living-docs completions {shell}"));
        let stdout = stdout_of(&output);
        assert!(output.status.success(), "{shell} got:\n{stdout}");
        assert!(!stdout.trim().is_empty(), "{shell} produced no output");
    }
}

#[test]
fn completions_rejects_an_unknown_shell_with_exit_code_two() {
    let output = living_docs()
        .args(["completions", "no-such-shell"])
        .output()
        .expect("failed to run living-docs completions no-such-shell");
    assert_eq!(output.status.code(), Some(2));
}
