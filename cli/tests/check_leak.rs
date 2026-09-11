//! Integration coverage for the `LEAK` advisory (ADR 0054): a leaky record
//! prints a `LEAK` line and the bundle still exits 0 — advisory, never a gate.

use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, stdout_of, write};

fn run_check(bundle: &Path) -> Output {
    living_docs()
        .args(["check", bundle.to_str().unwrap()])
        .output()
        .expect("failed to run living-docs check")
}

fn write_leaky_bundle(bundle: &Path) {
    write(
        bundle,
        "index.md",
        "---\nokf_version: \"1.0\"\n---\n# Docs\n\n* [ADRs](adr/index.md)\n",
    );
    write(
        bundle,
        "adr/index.md",
        "# ADRs\n\n## Active\n\n* [0001 — Leaky](0001-leaky.md) - Accepted\n",
    );
    write(
        bundle,
        "adr/0001-leaky.md",
        "---\ntype: ADR\ntitle: Leaky\ndescription: d\nowner: x\nstatus: Accepted\n---\n\n# 0001. Leaky\n\n## Decision\n\nGiven a record\nWhen it changes\nThen check reports it\n",
    );
}

#[test]
fn a_given_when_then_scenario_in_an_adr_prints_a_leak_advisory_without_gating() {
    let bundle = common::temp_bundle("leak", "gwt");
    write_leaky_bundle(&bundle);

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(0), "LEAK must not gate; got:\n{stdout}");
    assert!(stdout.contains("LEAK") && stdout.contains("Given/When/Then"), "got:\n{stdout}");
}
