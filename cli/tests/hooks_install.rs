use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const SCRIPT_BASENAMES: [&str; 2] = ["block-docs-handwrite.sh", "session-context.sh"];
const LIVING_DOCS_MARKER: &str = ".living-docs/hooks/";

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

fn project_with_bundle(label: &str, bundle: &str) -> PathBuf {
    let project = temp_dir(label);
    fs::create_dir_all(project.join(bundle)).unwrap();
    project
}

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("skills/living-docs/hooks")
}

fn run_hooks_install(args: &[&str]) -> Output {
    living_docs()
        .arg("hooks")
        .arg("install")
        .args(args)
        .output()
        .expect("failed to run living-docs hooks install")
}

fn run_install(dir: &Path, dry_run: bool) -> Output {
    let dir_str = dir.to_str().unwrap();
    if dry_run {
        run_hooks_install(&["--dir", dir_str, "--dry-run"])
    } else {
        run_hooks_install(&["--dir", dir_str])
    }
}

fn read_settings(project: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(project.join(".claude/settings.json"))
        .expect("settings.json was written");
    serde_json::from_str(&raw).expect("settings.json parses as JSON")
}

fn entries(settings: &serde_json::Value, section: &str) -> Vec<serde_json::Value> {
    settings["hooks"][section]
        .as_array()
        .unwrap_or_else(|| panic!("hooks.{section} is an array"))
        .clone()
}

fn living_docs_entries(settings: &serde_json::Value, section: &str) -> Vec<serde_json::Value> {
    entries(settings, section)
        .into_iter()
        .filter(|entry| {
            entry["hooks"].as_array().into_iter().flatten().any(|hook| {
                hook["command"]
                    .as_str()
                    .is_some_and(|command| command.contains(LIVING_DOCS_MARKER))
            })
        })
        .collect()
}

#[test]
fn install_materializes_both_scripts_byte_identical_at_mode_0755() {
    let project = project_with_bundle("materialize", "docs");

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
    let project = project_with_bundle("idempotent", "docs");
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
    let project = project_with_bundle("dry-run", "docs");

    let output = run_install(&project, true);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    for basename in SCRIPT_BASENAMES {
        assert!(
            stdout.contains(basename),
            "stdout missing {basename}: {stdout}"
        );
    }
    assert!(stdout.contains(".claude/settings.json"), "got: {stdout}");
    assert!(!project.join(".living-docs").exists());
    assert!(!project.join(".claude").exists());

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_defaults_dir_to_the_current_directory_when_omitted() {
    let project = project_with_bundle("default-dir", "docs");

    let output = living_docs()
        .args(["hooks", "install", "--dry-run"])
        .current_dir(&project)
        .output()
        .expect("failed to run living-docs hooks install");

    assert!(output.status.success());
    assert!(!project.join(".living-docs").exists());

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_wires_a_fresh_settings_json_with_one_pretooluse_and_one_sessionstart_entry() {
    let project = project_with_bundle("fresh-wire", "docs");

    let output = run_install(&project, false);
    assert!(output.status.success());

    let settings = read_settings(&project);
    let pre_tool_use = entries(&settings, "PreToolUse");
    assert_eq!(pre_tool_use.len(), 1, "got: {pre_tool_use:?}");
    assert_eq!(pre_tool_use[0]["matcher"], "Write|Edit|MultiEdit");
    let pre_command = pre_tool_use[0]["hooks"][0]["command"]
        .as_str()
        .expect("PreToolUse entry carries a command string");
    assert!(pre_command.contains(".living-docs/hooks/block-docs-handwrite.sh"));
    assert!(pre_command.starts_with("LIVING_DOCS_BUNDLE=docs "));

    let session_start = entries(&settings, "SessionStart");
    assert_eq!(session_start.len(), 1, "got: {session_start:?}");
    let session_command = session_start[0]["hooks"][0]["command"]
        .as_str()
        .expect("SessionStart entry carries a command string");
    assert!(session_command.contains(".living-docs/hooks/session-context.sh"));
    assert!(session_command.starts_with("LIVING_DOCS_BUNDLE=docs "));

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_preserves_unrelated_settings_entries_and_top_level_keys() {
    let project = project_with_bundle("preserve", "docs");
    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::write(
        project.join(".claude/settings.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "unrelatedTopLevelKey": "keep-me",
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [ { "type": "command", "command": "echo custom-guard" } ]
                    }
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let output = run_install(&project, false);
    assert!(output.status.success());

    let settings = read_settings(&project);
    assert_eq!(settings["unrelatedTopLevelKey"], "keep-me");

    let pre_tool_use = entries(&settings, "PreToolUse");
    assert_eq!(pre_tool_use.len(), 2, "got: {pre_tool_use:?}");
    let unrelated_still_present = pre_tool_use.iter().any(|entry| {
        entry["hooks"][0]["command"] == "echo custom-guard" && entry["matcher"] == "Bash"
    });
    assert!(unrelated_still_present, "got: {pre_tool_use:?}");

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_replaces_living_docs_entries_on_reinstall_without_duplicating() {
    let project = project_with_bundle("reinstall", "docs");

    let first = run_install(&project, false);
    assert!(first.status.success());
    let second = run_install(&project, false);
    assert!(second.status.success());

    let settings = read_settings(&project);
    assert_eq!(
        living_docs_entries(&settings, "PreToolUse").len(),
        1,
        "got: {settings:?}"
    );
    assert_eq!(
        living_docs_entries(&settings, "SessionStart").len(),
        1,
        "got: {settings:?}"
    );

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_pins_a_custom_docs_dir_verbatim_into_the_generated_commands() {
    let project = project_with_bundle("custom-bundle", "handbook");

    let output = run_hooks_install(&["--docs-dir", "handbook", "--dir", project.to_str().unwrap()]);
    assert!(output.status.success());

    let settings = read_settings(&project);
    let pre_command = entries(&settings, "PreToolUse")[0]["hooks"][0]["command"]
        .as_str()
        .unwrap()
        .to_string();
    let session_command = entries(&settings, "SessionStart")[0]["hooks"][0]["command"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        pre_command.starts_with("LIVING_DOCS_BUNDLE=handbook "),
        "got: {pre_command}"
    );
    assert!(
        session_command.starts_with("LIVING_DOCS_BUNDLE=handbook "),
        "got: {session_command}"
    );

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_fails_with_exit_2_when_docs_dir_does_not_exist_and_writes_nothing() {
    let project = temp_dir("missing-bundle");

    let output = run_hooks_install(&["--docs-dir", "nope", "--dir", project.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--docs-dir"), "got: {stderr}");
    assert!(!project.join(".living-docs").exists());
    assert!(!project.join(".claude").exists());

    let _ = fs::remove_dir_all(&project);
}

#[test]
fn install_fails_with_exit_2_when_existing_settings_json_is_not_valid_json_and_never_overwrites_it()
{
    let project = project_with_bundle("invalid-json", "docs");
    fs::create_dir_all(project.join(".claude")).unwrap();
    let settings_path = project.join(".claude/settings.json");
    fs::write(&settings_path, "{ not valid json").unwrap();

    let output = run_install(&project, false);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("settings.json"), "got: {stderr}");
    let raw = fs::read_to_string(&settings_path).unwrap();
    assert_eq!(
        raw, "{ not valid json",
        "settings.json must never be overwritten on parse failure"
    );
    assert!(!project
        .join(".living-docs/hooks/block-docs-handwrite.sh")
        .exists());

    let _ = fs::remove_dir_all(&project);
}
