use crate::skill;
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

struct HookScript {
    basename: &'static str,
    bytes: Cow<'static, [u8]>,
}

/// Materializes the corpus hook scripts into `<project_root>/.living-docs/hooks/`
/// at mode 0755, idempotently. Under `dry_run`, reports the same plan on
/// stdout and writes nothing. A missing embedded asset is a hard error —
/// named on stderr, no file written, `ExitCode::from(2)`.
pub(crate) fn install(project_root: &Path, dry_run: bool) -> ExitCode {
    let scripts = match resolve_scripts() {
        Ok(scripts) => scripts,
        Err(message) => return report_failure(&message),
    };
    if dry_run {
        announce_dry_run(&scripts);
        return ExitCode::SUCCESS;
    }
    match write_scripts(project_root, &scripts) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => report_failure(&err.to_string()),
    }
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
}
