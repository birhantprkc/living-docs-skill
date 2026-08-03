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
    let dir = std::env::temp_dir().join(format!("living-docs-describe-test-{label}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn temp_sqlite_url(label: &str) -> (PathBuf, String) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir()
        .join(format!("living-docs-describe-test-db-{label}-{nanos}"))
        .join("index.db");
    let url = format!("sqlite://{}?mode=rwc", path.display());
    (path, url)
}

fn run_new(docs: &Path, doc_type: &str, title: &str) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "new", doc_type, title])
        .output()
        .expect("failed to run living-docs new")
}

fn run_describe(docs: &Path, number: &str, description: &str) -> Output {
    living_docs()
        .args([
            "--docs-dir",
            docs.to_str().unwrap(),
            "describe",
            number,
            description,
        ])
        .output()
        .expect("failed to run living-docs describe")
}

fn run_db(db_url: &str, docs: &Path, args: &[&str]) -> Output {
    let mut full_args = vec!["--backend", "db", "--docs-dir", docs.to_str().unwrap()];
    full_args.extend_from_slice(args);
    living_docs()
        .env("DATABASE_URL", db_url)
        .args(full_args)
        .output()
        .expect("failed to run living-docs")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn cleanup(docs: &Path, db_path: &Path) {
    let _ = fs::remove_dir_all(docs);
    let _ = fs::remove_file(db_path);
    let _ = fs::remove_dir(db_path.parent().unwrap());
}

/// A bundle-root `index.md` linking to the type's `index.md`, with the
/// type directory's own `index.md` deliberately absent — the freshly
/// bootstrapped shape `--backend db new` needs before its first record
/// (mirrors `db_authoring.rs::seed_root_index_only`).
fn seed_root_index_only(docs: &Path) {
    fs::create_dir_all(docs.join("adr")).unwrap();
    fs::write(docs.join("index.md"), "# Index\n\n- [ADRs](adr/index.md)\n").unwrap();
}

/// Every "fill this in" example link the ADR template embeds verbatim,
/// broken by design until an author replaces them — required for
/// `--backend db new`'s `write_checked` link-validity invariant to pass
/// (mirrors `db_authoring.rs::seed_adr_placeholder_link_targets`).
fn seed_adr_placeholder_link_targets(docs: &Path) {
    fs::create_dir_all(docs.join("research").join("NNNN-<slug>.md")).unwrap();
    fs::create_dir_all(docs.join("prd").join("NNNN-<slug>.md")).unwrap();
    fs::create_dir_all(docs.join("adr")).unwrap();
    fs::write(docs.join("adr").join("{{URL}}"), "").unwrap();
}

/// AC1: `describe` updates only `description:`, leaving every other
/// frontmatter key and the body untouched.
#[test]
fn describe_sets_only_the_description_field_and_preserves_body_and_other_frontmatter() {
    let docs = temp_dir("fs-set");
    assert!(run_new(&docs, "adr", "A Decision").status.success());

    let output = run_describe(&docs, "0001", "A concise sentence describing the change.");
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));

    let contents = fs::read_to_string(docs.join("adr/0001-a-decision.md")).unwrap();
    assert!(
        contents.contains("description: A concise sentence describing the change.\n"),
        "got: {contents}"
    );
    assert!(contents.contains("status: Proposed"), "got: {contents}");
    assert!(contents.contains("title: A Decision"), "got: {contents}");
    assert!(
        contents.contains("We will {{DECISION}}"),
        "body lost: {contents}"
    );
    assert!(
        contents.contains("Proposed | Accepted | Deprecated"),
        "comment lost: {contents}"
    );

    let _ = fs::remove_dir_all(&docs);
}

/// AC2: a description containing a colon+space is quoted via `format_scalar`
/// exactly as `title`/`new --description` already are.
#[test]
fn describe_quotes_a_description_containing_a_colon_space_exactly_as_the_canonical_serializer_would(
) {
    let docs = temp_dir("fs-quoted");
    assert!(run_new(&docs, "adr", "Quoted Decision").status.success());

    let output = run_describe(&docs, "0001", "Caching: a deep dive");
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));

    let contents = fs::read_to_string(docs.join("adr/0001-quoted-decision.md")).unwrap();
    assert!(
        contents.contains("description: \"Caching: a deep dive\"\n"),
        "got: {contents}"
    );

    let _ = fs::remove_dir_all(&docs);
}

/// AC3: an unknown record number fails cleanly with a "no record found"
/// style error and leaves the store completely unchanged.
#[test]
fn describe_rejects_an_unknown_record_number_leaving_the_store_unchanged() {
    let docs = temp_dir("fs-unknown");
    assert!(run_new(&docs, "adr", "A Decision").status.success());
    let before = fs::read_to_string(docs.join("adr/0001-a-decision.md")).unwrap();

    let output = run_describe(&docs, "0099", "A sentence nobody will see.");

    assert!(!output.status.success());
    let stderr = stderr_of(&output);
    assert!(stderr.contains("no record found for 0099"), "got: {stderr}");

    let after = fs::read_to_string(docs.join("adr/0001-a-decision.md")).unwrap();
    assert_eq!(before, after, "file must be left unchanged");

    let _ = fs::remove_dir_all(&docs);
}

/// Re-describing a record is a targeted single-key edit — the rest of the
/// file, byte for byte, comes along unchanged.
#[test]
fn describe_setting_a_new_description_leaves_every_other_byte_of_the_record_unchanged() {
    let docs = temp_dir("fs-idempotent-shape");
    assert!(run_new(&docs, "adr", "A Decision").status.success());
    let before = fs::read_to_string(docs.join("adr/0001-a-decision.md")).unwrap();
    let before_lines: Vec<&str> = before
        .lines()
        .filter(|line| !line.starts_with("description:"))
        .collect();

    let output = run_describe(&docs, "0001", "Replaces the placeholder sentence.");
    assert!(output.status.success(), "stderr: {}", stderr_of(&output));

    let after = fs::read_to_string(docs.join("adr/0001-a-decision.md")).unwrap();
    let after_lines: Vec<&str> = after
        .lines()
        .filter(|line| !line.starts_with("description:"))
        .collect();
    assert_eq!(before_lines, after_lines, "only description: may change");

    let _ = fs::remove_dir_all(&docs);
}

/// AC1/AC4: the db backend (`build_backend_store`/`commands::describe::run`)
/// also updates `description:` end to end, not just the fs backend. The
/// record only lives in the db-store, so it is materialized to disk through
/// `export` before its `description:` line is inspected, matching
/// `db_authoring.rs`'s own export-then-read pattern.
#[test]
fn backend_db_describe_updates_the_description_field_and_round_trips_on_export() {
    let docs = temp_dir("db-set");
    let (db_path, db_url) = temp_sqlite_url("db-set");
    seed_root_index_only(&docs);
    seed_adr_placeholder_link_targets(&docs);

    let new_output = run_db(&db_url, &docs, &["new", "adr", "Db Described Decision"]);
    assert!(
        new_output.status.success(),
        "stderr: {}",
        stderr_of(&new_output)
    );

    let describe_output = run_db(
        &db_url,
        &docs,
        &["describe", "0001", "A db-backed sentence."],
    );
    assert!(
        describe_output.status.success(),
        "stderr: {}",
        stderr_of(&describe_output)
    );

    let out_dir = temp_dir("db-set-out");
    fs::remove_dir_all(&out_dir).unwrap();
    let export_output = run_db(&db_url, &docs, &["export", out_dir.to_str().unwrap()]);
    assert!(
        export_output.status.success(),
        "stderr: {}",
        stderr_of(&export_output)
    );

    let contents = fs::read_to_string(out_dir.join("adr/0001-db-described-decision.md")).unwrap();
    assert!(
        contents.contains("description: A db-backed sentence.\n"),
        "got: {contents}"
    );

    let _ = fs::remove_dir_all(&out_dir);
    cleanup(&docs, &db_path);
}

/// AC3: the db backend fails the same way the fs backend does for an
/// unknown record number.
#[test]
fn backend_db_describe_fails_when_a_record_number_does_not_exist() {
    let docs = temp_dir("db-unknown");
    let (db_path, db_url) = temp_sqlite_url("db-unknown");
    seed_root_index_only(&docs);
    seed_adr_placeholder_link_targets(&docs);

    let new_output = run_db(&db_url, &docs, &["new", "adr", "Only Decision"]);
    assert!(
        new_output.status.success(),
        "stderr: {}",
        stderr_of(&new_output)
    );

    let output = run_db(
        &db_url,
        &docs,
        &["describe", "0099", "Nobody will see this."],
    );

    assert!(!output.status.success());
    let stderr = stderr_of(&output);
    assert!(stderr.contains("no record found"), "got: {stderr}");

    cleanup(&docs, &db_path);
}
