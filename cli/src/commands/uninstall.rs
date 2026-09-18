//! `uninstall hooks` verb wrapper (ADR 0060, folding the retired `hooks
//! uninstall` verb): delegates to `crate::hooks`.

use crate::hooks;
use std::path::PathBuf;
use std::process::ExitCode;

/// Defaults `--dir` to the current directory, mirroring `install hooks`.
pub(crate) fn run_uninstall_hooks(dir: Option<PathBuf>, dry_run: bool) -> ExitCode {
    let project_root = dir.unwrap_or_else(|| PathBuf::from("."));
    hooks::uninstall(&project_root, dry_run)
}
