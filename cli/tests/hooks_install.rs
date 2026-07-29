use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const SCRIPT_BASENAMES: [&str; 2] = ["block-docs-handwrite.sh", "session-context.sh"];

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("living-docs-hooks-install-test-{label}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("skills/living-docs/hooks")
}

fn run_install(dir: &Path, dry_run: bool) -> Output {
    let mut command = living_docs();
    command.args(["hooks", "install", "--dir", dir.to_str().unwrap()]);
    if dry_run {
        command.arg("--dry-run");
    }
    command
        .output()
        .expect("failed to run living-docs hooks install")
}

#[test]
fn install_materializes_both_scripts_byte_identical_at_mode_0755() {
    let project = temp_dir("materialize");

    let output = run_install(&project, false);

    assert!(output.status.success());
    let hooks_dir = project.join(".living-docs/hooks");
    for basename in SCRIPT_BASENAMES {
        let installed = hooks_dir.join(basename);
        let expected = fs::read(corpus_root().join(basename))
            .unwrap_or_else(|e| panic!("failed to read corpus {basename}: {e}"));
        let actual = fs::read(&installed)
            .unwrap_or_else(|e| panic!("failed to read installed {basename}: {e}"));
        assert_eq!(actual, expected, "{basename} bytes differ from the corpus");

        let mode = fs::metadata(&installed).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755, "{basename} unexpected mode: {mode:o}");
    }

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_is_idempotent_across_repeated_runs() {
    let project = temp_dir("idempotent");
    let hooks_dir = project.join(".living-docs/hooks");

    let first = run_install(&project, false);
    assert!(first.status.success());
    let first_bytes: Vec<Vec<u8>> = SCRIPT_BASENAMES
        .iter()
        .map(|basename| fs::read(hooks_dir.join(basename)).unwrap())
        .collect();

    let second = run_install(&project, false);
    assert!(second.status.success());
    let second_bytes: Vec<Vec<u8>> = SCRIPT_BASENAMES
        .iter()
        .map(|basename| fs::read(hooks_dir.join(basename)).unwrap())
        .collect();

    assert_eq!(
        first_bytes, second_bytes,
        "repeated install must leave byte-identical files"
    );

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn dry_run_reports_the_plan_and_writes_nothing() {
    let project = temp_dir("dry-run");

    let output = run_install(&project, true);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    for basename in SCRIPT_BASENAMES {
        assert!(
            stdout.contains(basename),
            "stdout missing {basename}: {stdout}"
        );
    }
    assert!(!project.join(".living-docs").exists());

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_defaults_dir_to_the_current_directory_when_omitted() {
    let project = temp_dir("default-dir");

    let output = living_docs()
        .args(["hooks", "install", "--dry-run"])
        .current_dir(&project)
        .output()
        .expect("failed to run living-docs hooks install");

    assert!(output.status.success());
    assert!(!project.join(".living-docs").exists());

    let _ = fs::remove_dir_all(&project);
}
