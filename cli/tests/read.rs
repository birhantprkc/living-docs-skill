//! End-to-end wiring for the `read` verb (ADR 0050, renamed by ADR 0060): arg parsing, the
//! `--docs-dir` bundle, and the compiled output reaching stdout.

use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, stdout_of, write};

fn read(docs_dir: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["--docs-dir", docs_dir.to_str().unwrap(), "read", "--plain"];
    args.extend_from_slice(extra);
    living_docs()
        .args(&args)
        .output()
        .expect("failed to run living-docs read")
}

fn write_chain_bundle(bundle: &Path) {
    write(
        bundle,
        "adr/0001-old.md",
        "---\ntype: ADR\ntitle: Old\ndescription: d\nstatus: Superseded\nsuperseded_by: \"0002\"\n---\n\n# 0001. Old\n\nabout widgets\n",
    );
    write(
        bundle,
        "adr/0002-new.md",
        "---\ntype: ADR\ntitle: New\ndescription: the widget rule in force\nstatus: Accepted\nsupersedes: \"0001\"\n---\n\n# 0002. New\n\nabout widgets\n",
    );
}

#[test]
fn read_topic_returns_the_head_only_with_a_lineage_line() {
    let bundle = common::temp_bundle("read", "chain");
    write_chain_bundle(&bundle);

    let stdout = stdout_of(&read(&bundle, &["--topic", "widget"]));

    assert!(stdout.contains("[ADR 0002]"), "head present:\n{stdout}");
    assert!(
        stdout.contains("supersedes 0001"),
        "lineage present:\n{stdout}"
    );
    assert!(
        !stdout.contains("[ADR 0001]"),
        "superseded record absent:\n{stdout}"
    );
}

#[test]
fn effective_alias_still_runs_hidden_from_help() {
    let bundle = common::temp_bundle("read", "alias");
    write_chain_bundle(&bundle);

    let mut args = vec![
        "--docs-dir",
        bundle.to_str().unwrap(),
        "effective",
        "--plain",
    ];
    args.push("--topic");
    args.push("widget");
    let output = living_docs()
        .args(&args)
        .output()
        .expect("failed to run living-docs effective");
    let stdout = stdout_of(&output);

    assert!(output.status.success());
    assert!(stdout.contains("[ADR 0002]"), "head present:\n{stdout}");

    let help = stdout_of(
        &living_docs()
            .arg("--help")
            .output()
            .expect("failed to run living-docs --help"),
    );
    let lists_effective_as_a_command = help
        .lines()
        .any(|line| line.trim() == "effective" || line.trim_start().starts_with("effective  "));
    assert!(
        !lists_effective_as_a_command,
        "hidden alias must not be listed as a command in help:\n{help}"
    );
}
