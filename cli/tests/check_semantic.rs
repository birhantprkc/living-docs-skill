//! Integration coverage for the semantic instruments (ADR 0055): `check`
//! prints `DIAGRAM` and `DUPLICATE` advisories on built fixtures and the exit
//! code is unchanged — both are advisory, never gating.

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

fn root_index(bundle: &Path) {
    write(
        bundle,
        "index.md",
        "---\nokf_version: \"1.0\"\n---\n# Docs\n\n* [ADRs](adr/index.md)\n",
    );
}

fn accepted_adr(bundle: &Path, n: &str, prose: &str) {
    write(
        bundle,
        &format!("adr/{n}-x.md"),
        &format!("---\ntype: ADR\ntitle: A{n}\ndescription: d\nowner: x\nstatus: Accepted\n---\n\n# {n}. A{n}\n\n## Decision\n\n{prose}\n"),
    );
}

#[test]
fn near_duplicate_records_print_a_duplicate_advisory_without_gating() {
    let bundle = common::temp_bundle("semantic", "dup");
    root_index(&bundle);
    write(&bundle, "adr/index.md", "# ADRs\n\n## Active\n\n* [0001 — A0001](0001-x.md) - Accepted\n* [0002 — A0002](0002-x.md) - Accepted\n");
    let prose = "the store port stays stable across every backend so callers never depend on a concrete adapter implementation detail";
    accepted_adr(&bundle, "0001", prose);
    accepted_adr(&bundle, "0002", prose);

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "advisory must not gate; got:\n{stdout}"
    );
    assert!(stdout.contains("DUPLICATE"), "got:\n{stdout}");
}

#[test]
fn a_diagram_scope_mismatch_prints_a_diagram_advisory_without_gating() {
    let bundle = common::temp_bundle("semantic", "diagram");
    write(
        &bundle,
        "index.md",
        "---\nokf_version: \"1.0\"\n---\n# Docs\n\n* [Architecture](architecture/index.md)\n",
    );
    write(
        &bundle,
        "architecture/index.md",
        "# Architecture\n\n* [Context](context.md)\n",
    );
    write(&bundle, "architecture/diagram-scope.txt", "core\nweb\n");
    write(
        &bundle,
        "architecture/context.md",
        "---\ntype: Architecture View\ntitle: Context\ndescription: d\n---\n\n# Context\n\n```mermaid\nflowchart LR\n  A[core] --> B[ghost]\n```\n",
    );

    let output = run_check(&bundle);
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "advisory must not gate; got:\n{stdout}"
    );
    assert!(
        stdout.contains("DIAGRAM") && stdout.contains("ghost"),
        "got:\n{stdout}"
    );
}
