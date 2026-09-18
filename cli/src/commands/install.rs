//! `install skills`/`install hooks` verb wrappers (ADR 0060, folding the
//! retired `skill install` and `hooks install` verbs): delegate to
//! `crate::skill_install` and `crate::hooks` respectively.

use crate::hooks;
use crate::skill_install::{self, Harness};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub(crate) fn run_install_skills(
    harness: Harness,
    project: bool,
    dir: Option<PathBuf>,
    dry_run: bool,
) -> ExitCode {
    skill_install::install(harness, project, dir, dry_run)
}

/// Defaults `--dir` to the current directory when omitted, matching every
/// other subcommand's cwd-relative default. `docs_dir` is the CLI's global
/// `--docs-dir` flag, resolved at install time and pinned into the
/// generated `LIVING_DOCS_BUNDLE=` commands (ADR 0020 scope, resolved once
/// here rather than at hook run time).
pub(crate) fn run_install_hooks(dir: Option<PathBuf>, dry_run: bool, docs_dir: &Path) -> ExitCode {
    let project_root = dir.unwrap_or_else(|| PathBuf::from("."));
    hooks::install(&project_root, docs_dir, dry_run)
}
