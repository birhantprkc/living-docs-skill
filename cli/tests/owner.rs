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
    let dir = std::env::temp_dir().join(format!("living-docs-owner-test-{label}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_new(docs: &Path, doc_type: &str, title: &str, owner: Option<&str>) -> Output {
    let mut args = vec!["--docs-dir", docs.to_str().unwrap(), "new", doc_type, title];
    if let Some(owner) = owner {
        args.push("--owner");
        args.push(owner);
    }
    living_docs()
        .args(args)
        .output()
        .expect("failed to run living-docs new")
}

fn run_owner(docs: &Path, number: &str, value: &str) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "owner", number, value])
        .output()
        .expect("failed to run living-docs owner")
}

fn run_check(docs: &Path, require_owner: bool) -> Output {
    let mut args = vec![docs.to_str().unwrap().to_string()];
    if require_owner {
        args.push("--require-owner".to_string());
    }
    living_docs()
        .arg("check")
        .args(&args)
        .output()
        .expect("failed to run living-docs check")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// A minimal, already-canonical, check-clean bundle carrying exactly one
/// record: a bundle-root `index.md` and the type's own directory `index.md`
/// both link the record, and the record's frontmatter matches its own
/// canonical re-serialization — so `check`'s only possible finding is the
/// owner ratchet these tests exercise, never an unrelated invariant (a
/// missing index, an orphan record, or a non-canonical frontmatter block).
fn seed_check_clean_bundle(docs: &Path, type_dir: &str, doc_type: &str, owner_line: &str) {
    let filename = "0001-title.md";
    fs::create_dir_all(docs.join(type_dir)).unwrap();
    fs::write(
        docs.join("index.md"),
        format!("# Index\n\n- [{type_dir}](/{type_dir}/index.md)\n"),
    )
    .unwrap();
    fs::write(
        docs.join(type_dir).join("index.md"),
        format!("# Index\n\n- [Title](/{type_dir}/{filename})\n"),
    )
    .unwrap();
    fs::write(
        docs.join(type_dir).join(filename),
        format!("---\ntype: {doc_type}\ntitle: Title\ndescription: \"\"\n{owner_line}---\n\n# Title\n\nBody.\n"),
    )
    .unwrap();
}

/// AC1: `new adr "x" --owner alice` emits `owner: alice` in canonical
/// position, immediately after `description:`.
#[test]
fn new_with_owner_emits_the_field_in_canonical_position() {
    let docs = temp_dir("new-owner");
    let output = run_new(&docs, "adr", "Owned Decision", Some("alice"));
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));

    let contents = fs::read_to_string(docs.join("adr/0001-owned-decision.md")).unwrap();
    let description_index = contents.find("description:").unwrap();
    let owner_index = contents.find("owner: alice").unwrap();
    let status_index = contents.find("status:").unwrap();
    assert!(
        description_index < owner_index && owner_index < status_index,
        "got: {contents}"
    );

    let _ = fs::remove_dir_all(&docs);
}

/// AC1: `new` without `--owner` still succeeds and carries no `owner:` field.
#[test]
fn new_without_owner_succeeds_and_carries_no_owner_field() {
    let docs = temp_dir("new-no-owner");
    let output = run_new(&docs, "adr", "Unowned Decision", None);
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));

    let contents = fs::read_to_string(docs.join("adr/0001-unowned-decision.md")).unwrap();
    assert!(!contents.contains("owner:"), "got: {contents}");

    let _ = fs::remove_dir_all(&docs);
}

/// A freshly created record carrying `--owner` still passes `check`,
/// proving the insertion lands in the canonical-frontmatter fixed point.
#[test]
fn new_with_owner_still_passes_check() {
    let docs = temp_dir("new-owner-check");
    seed_check_clean_bundle(&docs, "adr", "ADR", "owner: alice\n");

    let output = run_check(&docs, false);
    assert!(output.status.success(), "stdout: {}", stdout_of(&output));

    let _ = fs::remove_dir_all(&docs);
}

/// AC2: `owner <ref> bob` rewrites only the `owner` field, preserves the
/// rest of the record, and the record still passes `check`.
#[test]
fn owner_verb_rewrites_only_the_owner_field_and_the_record_still_passes_check() {
    let docs = temp_dir("owner-verb");
    seed_check_clean_bundle(&docs, "adr", "ADR", "owner: alice\n");

    let output = run_owner(&docs, "0001", "bob");
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));

    let contents = fs::read_to_string(docs.join("adr/0001-title.md")).unwrap();
    assert!(contents.contains("owner: bob\n"), "got: {contents}");
    assert!(!contents.contains("owner: alice"), "got: {contents}");
    assert!(contents.contains("title: Title"), "got: {contents}");

    let check_output = run_check(&docs, false);
    assert!(
        check_output.status.success(),
        "stdout: {}",
        stdout_of(&check_output)
    );

    let _ = fs::remove_dir_all(&docs);
}

/// AC2: an unresolvable reference fails cleanly and leaves the store
/// unchanged.
#[test]
fn owner_verb_rejects_an_unknown_record_number_leaving_the_store_unchanged() {
    let docs = temp_dir("owner-unknown");
    assert!(run_new(&docs, "adr", "A Decision", None).status.success());
    let before = fs::read_to_string(docs.join("adr/0001-a-decision.md")).unwrap();

    let output = run_owner(&docs, "0099", "bob");

    assert!(!output.status.success());
    let stderr = stderr_of(&output);
    assert!(stderr.contains("no record found for 0099"), "got: {stderr}");

    let after = fs::read_to_string(docs.join("adr/0001-a-decision.md")).unwrap();
    assert_eq!(before, after, "file must be left unchanged");

    let _ = fs::remove_dir_all(&docs);
}

/// AC3: `check` warns on an ADR without `owner` and exits zero by default —
/// the existing corpus must never break CI before the backfill.
#[test]
fn check_warns_on_an_adr_without_owner_and_exits_zero_by_default() {
    let docs = temp_dir("check-warn");
    seed_check_clean_bundle(&docs, "adr", "ADR", "");

    let output = run_check(&docs, false);
    assert!(output.status.success(), "stdout: {}", stdout_of(&output));
    assert!(
        stdout_of(&output).contains("ADR record has no owner"),
        "got: {}",
        stdout_of(&output)
    );

    let _ = fs::remove_dir_all(&docs);
}

/// AC3: `check --require-owner` promotes the same finding to a violation.
#[test]
fn check_require_owner_fails_on_the_same_tree() {
    let docs = temp_dir("check-require");
    seed_check_clean_bundle(&docs, "adr", "ADR", "");

    let output = run_check(&docs, true);
    assert!(!output.status.success(), "stdout: {}", stdout_of(&output));

    let _ = fs::remove_dir_all(&docs);
}

/// AC3: a non-ADR/BDR type (Issue) missing `owner` never warns, even under
/// `--require-owner`.
#[test]
fn check_never_flags_a_type_that_does_not_require_owner() {
    let docs = temp_dir("check-not-required");
    seed_check_clean_bundle(&docs, "issues", "Issue", "");

    let output = run_check(&docs, true);
    assert!(output.status.success(), "stdout: {}", stdout_of(&output));
    assert!(
        !stdout_of(&output).contains("Issue record has no owner"),
        "got: {}",
        stdout_of(&output)
    );

    let _ = fs::remove_dir_all(&docs);
}
