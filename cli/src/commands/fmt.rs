//! `fmt` verb wrapper: reuses `check::check_bundle` for bundle resolution.

use crate::commands::check::check_bundle;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub(crate) fn run_fmt(docs_dir: &Path, paths: Vec<PathBuf>, check_only: bool) -> ExitCode {
    let target = check_bundle(docs_dir, paths);
    living_docs_core::commands::fmt::run(&fs_store::FsStore::new(), &target, check_only)
}
