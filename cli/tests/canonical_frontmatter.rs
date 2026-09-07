//! ADR 0048 (issue 0041 defect #2): every CLI frontmatter-mutation verb
//! (`supersede`, `status`, `describe`, `owner`) routes through the canonical
//! serializer, so a record it touches — whether it inserts a
//! previously-absent key or edits an existing one — stays a canonical
//! round-trip fixed point. A dedicated file, mirroring the existing
//! per-verb split (`describe.rs`, `owner.rs`), keeps this coverage out of
//! `check_core.rs`/`index_supersede.rs`, both already at their file-size
//! ratchet baseline.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

mod common;
use common::{living_docs, new_two_adrs_and_supersede, run_check, run_new, run_supersede};

fn temp_dir(label: &str) -> PathBuf {
    common::temp_bundle("canonical-frontmatter", label)
}

fn run_status(docs: &Path, reference: &str, new_status: &str) -> Output {
    living_docs()
        .args([
            "--docs-dir",
            docs.to_str().unwrap(),
            "status",
            reference,
            new_status,
        ])
        .output()
        .expect("failed to run living-docs status")
}

fn run_describe(docs: &Path, reference: &str, description: &str) -> Output {
    living_docs()
        .args([
            "--docs-dir",
            docs.to_str().unwrap(),
            "describe",
            reference,
            description,
        ])
        .output()
        .expect("failed to run living-docs describe")
}

fn run_owner_verb(docs: &Path, reference: &str, value: &str) -> Output {
    living_docs()
        .args([
            "--docs-dir",
            docs.to_str().unwrap(),
            "owner",
            reference,
            value,
        ])
        .output()
        .expect("failed to run living-docs owner")
}

fn body_of(path: &Path) -> String {
    let contents = fs::read_to_string(path).unwrap();
    let after_open = contents.strip_prefix("---\n").unwrap();
    let close = after_open.find("\n---").unwrap();
    after_open[close..].to_string()
}

/// AC1: `supersede` inserting the previously-absent `supersedes`/
/// `superseded_by` keys leaves both records a canonical round-trip fixed
/// point — `check` reports no non-canonical-frontmatter finding.
#[test]
fn supersede_leaves_both_records_with_no_non_canonical_finding() {
    let docs = temp_dir("supersede-check-clean");
    new_two_adrs_and_supersede(&docs);

    let check = run_check(&docs);
    let stdout = String::from_utf8_lossy(&check.stdout);
    assert!(
        !stdout.contains("living-docs fmt"),
        "supersede left a non-canonical-frontmatter finding, stdout: {stdout}"
    );

    let _ = fs::remove_dir_all(&docs);
}

/// AC2: `status`, `describe`, and `owner` each editing an already-present
/// key also leave the record canonical, with no `fmt` pass required.
#[test]
fn status_describe_and_owner_each_leave_the_record_with_no_non_canonical_finding() {
    let docs = temp_dir("mutation-check-clean");
    assert!(run_new(&docs, "adr", "A Decision").status.success());

    assert!(run_status(&docs, "0001", "Accepted").status.success());
    assert!(run_describe(&docs, "0001", "A refreshed decision.")
        .status
        .success());
    assert!(run_owner_verb(&docs, "0001", "alice").status.success());

    let check = run_check(&docs);
    let stdout = String::from_utf8_lossy(&check.stdout);
    assert!(
        !stdout.contains("living-docs fmt"),
        "a mutation verb left a non-canonical-frontmatter finding, stdout: {stdout}"
    );

    let _ = fs::remove_dir_all(&docs);
}

/// AC3: for `supersede`, `status`, `describe`, and `owner`, the body below
/// the closing `---` stays byte-identical before and after the verb runs —
/// only the frontmatter block may change.
#[test]
fn mutation_verbs_leave_the_body_below_the_frontmatter_byte_identical() {
    let docs = temp_dir("mutation-body-preserved");
    assert!(run_new(&docs, "adr", "Old Decision").status.success());
    assert!(run_new(&docs, "adr", "New Decision").status.success());
    let old_path = docs.join("adr/0001-old-decision.md");
    let new_path = docs.join("adr/0002-new-decision.md");
    let old_body_before = body_of(&old_path);
    let new_body_before = body_of(&new_path);

    assert!(run_supersede(&docs, "0001", "0002").status.success());
    assert!(run_status(&docs, "0002", "Accepted").status.success());
    assert!(run_describe(&docs, "0002", "A refreshed decision.")
        .status
        .success());
    assert!(run_owner_verb(&docs, "0002", "alice").status.success());

    assert_eq!(
        body_of(&old_path),
        old_body_before,
        "supersede must not touch the body"
    );
    assert_eq!(
        body_of(&new_path),
        new_body_before,
        "status/describe/owner must not touch the body"
    );

    let _ = fs::remove_dir_all(&docs);
}
