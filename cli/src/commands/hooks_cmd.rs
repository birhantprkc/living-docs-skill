//! `hooks install`/`hooks uninstall` verb wrappers: delegate to the `hooks` module.

use crate::hooks;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Defaults `--dir` to the current directory when omitted, matching every
/// other subcommand's cwd-relative default. `docs_dir` is the CLI's global
/// `--docs-dir` flag, resolved at install time and pinned into the
/// generated `LIVING_DOCS_BUNDLE=` commands (ADR 0020 scope, resolved once
/// here rather than at hook run time).
pub(crate) fn run_hooks_install(dir: Option<PathBuf>, dry_run: bool, docs_dir: &Path) -> ExitCode {
    let project_root = dir.unwrap_or_else(|| PathBuf::from("."));
    hooks::install(&project_root, docs_dir, dry_run)
}

/// Defaults `--dir` to the current directory, mirroring [`run_hooks_install`].
pub(crate) fn run_hooks_uninstall(dir: Option<PathBuf>, dry_run: bool) -> ExitCode {
    let project_root = dir.unwrap_or_else(|| PathBuf::from("."));
    hooks::uninstall(&project_root, dry_run)
}
