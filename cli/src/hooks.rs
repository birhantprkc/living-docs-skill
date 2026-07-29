use crate::skill;
use serde_json::{json, Value};
use std::borrow::Cow;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::ExitCode;

const HOOK_ASSET_PATHS: [&str; 2] = [
    "living-docs/hooks/block-docs-handwrite.sh",
    "living-docs/hooks/session-context.sh",
];

const HOOKS_DEST_SUBDIR: &str = ".living-docs/hooks";
const SCRIPT_MODE: u32 = 0o755;
const SETTINGS_REL_PATH: &str = ".claude/settings.json";
const PRE_TOOL_USE_SECTION: &str = "PreToolUse";
const SESSION_START_SECTION: &str = "SessionStart";
const PRE_TOOL_USE_MATCHER: &str = "Write|Edit|MultiEdit";

struct HookScript {
    basename: &'static str,
    bytes: Cow<'static, [u8]>,
}

struct HookEntrySpec {
    script_basename: &'static str,
    section: &'static str,
    matcher: Option<&'static str>,
}

/// Materializes the corpus hook scripts into `<project_root>/.living-docs/hooks/`
/// at mode 0755, then wires them into `<project_root>/.claude/settings.json`
/// via a `serde_json::Value` parse/mutate/serialize merge — idempotently,
/// replacing any prior living-docs entry by identity rather than appending.
/// The bundle pinned into each generated command's `LIVING_DOCS_BUNDLE=` is
/// `docs_dir`, resolved against `project_root` and required to already
/// exist. Under `dry_run`, reports the same plan on stdout and changes
/// nothing on disk. A missing embedded asset, a non-existent `docs_dir`, or
/// a settings file that fails to parse as JSON are hard errors — named on
/// stderr, no file written, `ExitCode::from(2)`.
pub(crate) fn install(project_root: &Path, docs_dir: &Path, dry_run: bool) -> ExitCode {
    if let Err(message) = validate_docs_dir(project_root, docs_dir) {
        return report_failure(&message);
    }
    let scripts = match resolve_scripts() {
        Ok(scripts) => scripts,
        Err(message) => return report_failure(&message),
    };
    let settings_path = project_root.join(SETTINGS_REL_PATH);
    let mut settings = match load_settings(&settings_path) {
        Ok(settings) => settings,
        Err(message) => return report_failure(&message),
    };
    let bundle = docs_dir.to_string_lossy().into_owned();
    apply_hook_entries(&mut settings, &bundle);
    if dry_run {
        announce_dry_run(&scripts);
        announce_dry_run_wiring(&settings_path, &bundle);
        return ExitCode::SUCCESS;
    }
    if let Err(err) = write_scripts(project_root, &scripts) {
        return report_failure(&err.to_string());
    }
    match write_settings(&settings_path, &settings) {
        Ok(()) => {
            println!("wired {}", settings_path.display());
            ExitCode::SUCCESS
        }
        Err(err) => report_failure(&err.to_string()),
    }
}

fn validate_docs_dir(project_root: &Path, docs_dir: &Path) -> Result<(), String> {
    if project_root.join(docs_dir).is_dir() {
        return Ok(());
    }
    Err(format!(
        "--docs-dir {} does not name an existing directory under {}",
        docs_dir.display(),
        project_root.display()
    ))
}

fn resolve_scripts() -> Result<Vec<HookScript>, String> {
    HOOK_ASSET_PATHS.into_iter().map(resolve_one).collect()
}

fn resolve_one(path: &'static str) -> Result<HookScript, String> {
    let bytes = skill::asset(path).ok_or_else(|| format!("missing embedded asset: {path}"))?;
    Ok(HookScript {
        basename: basename_of(path),
        bytes,
    })
}

fn basename_of(path: &'static str) -> &'static str {
    path.rsplit('/').next().unwrap_or(path)
}

fn announce_dry_run(scripts: &[HookScript]) {
    for script in scripts {
        println!("[dry-run] would write {}", dest_display(script.basename));
    }
}

fn announce_dry_run_wiring(settings_path: &Path, bundle: &str) {
    println!(
        "[dry-run] would wire {} (LIVING_DOCS_BUNDLE={bundle})",
        settings_path.display()
    );
}

fn dest_display(basename: &str) -> String {
    format!("{HOOKS_DEST_SUBDIR}/{basename}")
}

fn write_scripts(project_root: &Path, scripts: &[HookScript]) -> io::Result<()> {
    let dest_dir = project_root.join(HOOKS_DEST_SUBDIR);
    fs::create_dir_all(&dest_dir)?;
    scripts
        .iter()
        .try_for_each(|script| write_one(&dest_dir, script))
}

fn write_one(dest_dir: &Path, script: &HookScript) -> io::Result<()> {
    let dest = dest_dir.join(script.basename);
    fs::write(&dest, script.bytes.as_ref())?;
    fs::set_permissions(&dest, fs::Permissions::from_mode(SCRIPT_MODE))?;
    println!("wrote {}", dest.display());
    Ok(())
}

fn hook_entry_specs() -> [HookEntrySpec; 2] {
    [
        HookEntrySpec {
            script_basename: basename_of(HOOK_ASSET_PATHS[0]),
            section: PRE_TOOL_USE_SECTION,
            matcher: Some(PRE_TOOL_USE_MATCHER),
        },
        HookEntrySpec {
            script_basename: basename_of(HOOK_ASSET_PATHS[1]),
            section: SESSION_START_SECTION,
            matcher: None,
        },
    ]
}

fn living_docs_hook_marker() -> String {
    format!("{HOOKS_DEST_SUBDIR}/")
}

fn build_entry(spec: &HookEntrySpec, bundle: &str) -> Value {
    let command = format!(
        "LIVING_DOCS_BUNDLE={bundle} \"$CLAUDE_PROJECT_DIR\"/{HOOKS_DEST_SUBDIR}/{}",
        spec.script_basename
    );
    let mut entry = json!({ "hooks": [ { "type": "command", "command": command } ] });
    if let Some(matcher) = spec.matcher {
        entry["matcher"] = json!(matcher);
    }
    entry
}

fn apply_hook_entries(settings: &mut Value, bundle: &str) {
    let marker = living_docs_hook_marker();
    for spec in hook_entry_specs() {
        let entry = build_entry(&spec, bundle);
        let section = ensure_hooks_section(settings, spec.section);
        section.retain(|existing| !is_living_docs_entry(existing, &marker));
        section.push(entry);
    }
}

fn load_settings(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let value: Value = serde_json::from_str(&raw)
        .map_err(|err| format!("{} is not valid JSON: {err}", path.display()))?;
    if !value.is_object() {
        return Err(format!("{} does not contain a JSON object", path.display()));
    }
    Ok(value)
}

fn ensure_hooks_section<'a>(settings: &'a mut Value, section: &str) -> &'a mut Vec<Value> {
    let hooks = ensure_child_object(settings, "hooks");
    ensure_child_array(hooks, section)
}

fn ensure_child_object<'a>(value: &'a mut Value, key: &str) -> &'a mut Value {
    let object = value
        .as_object_mut()
        .expect("settings value is validated as an object before this call");
    let child = object.entry(key).or_insert_with(|| json!({}));
    if !child.is_object() {
        *child = json!({});
    }
    object.get_mut(key).expect("key was just inserted")
}

fn ensure_child_array<'a>(value: &'a mut Value, key: &str) -> &'a mut Vec<Value> {
    let object = value
        .as_object_mut()
        .expect("settings value is validated as an object before this call");
    let child = object.entry(key).or_insert_with(|| json!([]));
    if !child.is_array() {
        *child = json!([]);
    }
    object
        .get_mut(key)
        .and_then(Value::as_array_mut)
        .expect("key was just normalized to an array")
}

fn is_living_docs_entry(entry: &Value, marker: &str) -> bool {
    entry
        .get("hooks")
        .and_then(Value::as_array)
        .is_some_and(|hooks| hooks.iter().any(|hook| hook_command_contains(hook, marker)))
}

fn hook_command_contains(hook: &Value, marker: &str) -> bool {
    hook.get("command")
        .and_then(Value::as_str)
        .is_some_and(|command| command.contains(marker))
}

fn write_settings(path: &Path, settings: &Value) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut rendered = serde_json::to_string_pretty(settings)
        .expect("a settings value built from string literals always serializes");
    rendered.push('\n');
    fs::write(path, rendered)
}

fn report_failure(message: &str) -> ExitCode {
    eprintln!("living-docs hooks install: {message}");
    ExitCode::from(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_scripts_finds_both_corpus_assets() {
        let scripts = resolve_scripts().expect("both hook scripts are embedded");
        let basenames: Vec<&str> = scripts.iter().map(|script| script.basename).collect();
        assert_eq!(
            basenames,
            vec!["block-docs-handwrite.sh", "session-context.sh"]
        );
        assert!(scripts.iter().all(|script| !script.bytes.is_empty()));
    }

    #[test]
    fn is_living_docs_entry_true_when_any_nested_command_contains_the_marker() {
        let marker = living_docs_hook_marker();
        let entry = json!({
            "hooks": [
                { "type": "command", "command": "echo unrelated" },
                {
                    "type": "command",
                    "command": format!(
                        "LIVING_DOCS_BUNDLE=docs \"$CLAUDE_PROJECT_DIR\"/{marker}session-context.sh"
                    )
                }
            ]
        });
        assert!(is_living_docs_entry(&entry, &marker));
    }

    #[test]
    fn is_living_docs_entry_false_when_no_command_mentions_the_marker() {
        let marker = living_docs_hook_marker();
        let entry = json!({ "hooks": [ { "type": "command", "command": "echo unrelated" } ] });
        assert!(!is_living_docs_entry(&entry, &marker));
    }

    #[test]
    fn is_living_docs_entry_false_when_the_entry_carries_no_hooks_array() {
        let marker = living_docs_hook_marker();
        assert!(!is_living_docs_entry(&json!({ "matcher": "X" }), &marker));
    }

    #[test]
    fn ensure_child_array_normalizes_a_non_array_value_instead_of_panicking() {
        let mut settings = json!({ "hooks": "not-an-object" });
        let section = ensure_hooks_section(&mut settings, PRE_TOOL_USE_SECTION);
        assert!(section.is_empty());
    }
}
