//! Integration coverage for `check --liveness` (ADR 0049): the record-liveness
//! pass emits `stale-proposed`/`stale-impact` advisories and a summary line,
//! and never changes the exit code.

use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, stdout_of, write};

fn liveness_bundle(label: &str) -> std::path::PathBuf {
    common::temp_bundle("liveness", label)
}

fn run_check_liveness(bundle: &Path) -> Output {
    living_docs()
        .args(["check", bundle.to_str().unwrap(), "--liveness"])
        .output()
        .expect("failed to run living-docs check --liveness")
}

/// A `Proposed` ADR linking a `closed` issue and an `Accepted` ADR whose
/// Implementation impact names a missing path print both liveness advisories
/// and the summary line, and the bundle still exits 0 — liveness advises,
/// never gates.
#[test]
fn check_liveness_flags_stale_proposed_and_stale_impact_without_gating() {
    let bundle = liveness_bundle("mixed");
    write(
        &bundle,
        "index.md",
        "---\nokf_version: \"1.0\"\n---\n# Docs\n\n* [ADRs](adr/index.md)\n* [Issues](issues/index.md)\n",
    );
    write(
        &bundle,
        "adr/index.md",
        "# ADRs\n\n## Active\n\n* [0001 — Proposed](0001-proposed.md) - Proposed\n* [0002 — Accepted](0002-accepted.md) - Accepted\n",
    );
    write(
        &bundle,
        "adr/0001-proposed.md",
        "---\ntype: ADR\ntitle: Proposed\ndescription: d\nowner: x\nstatus: Proposed\n---\n\n# 0001. Proposed\n\nSee [issue](/issues/0001-task.md).\n",
    );
    write(
        &bundle,
        "adr/0002-accepted.md",
        "---\ntype: ADR\ntitle: Accepted\ndescription: d\nowner: x\nstatus: Accepted\n---\n\n# 0002. Accepted\n\n## Verification\n\n**Implementation impact:** `ghost/module/missing.rs`.\n",
    );
    write(
        &bundle,
        "issues/index.md",
        "# Issues\n\n## Closed\n\n* [0001 — Task](0001-task.md) - closed\n",
    );
    write(
        &bundle,
        "issues/0001-task.md",
        "---\ntype: Issue\ntitle: Task\ndescription: d\nstatus: closed\n---\n\n# 0001. Task\n",
    );

    let output = run_check_liveness(&bundle);
    let stdout = stdout_of(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "liveness must not gate; got:\n{stdout}"
    );
    assert!(
        stdout.contains("liveness — 1 stale-proposed, 1 stale-impact"),
        "got:\n{stdout}"
    );
    assert!(
        stdout.contains("0001-proposed.md") && stdout.contains("stale-proposed"),
        "got:\n{stdout}"
    );
    assert!(
        stdout.contains("0002-accepted.md") && stdout.contains("stale-impact"),
        "got:\n{stdout}"
    );
}
