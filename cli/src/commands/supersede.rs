//! `supersede` verb wrapper: delegates to `living_docs_core::commands::supersede::run`.

use crate::store::build_store;
use living_docs_core::commands;
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_supersede(docs_dir: &Path, old: &str, new: &str) -> ExitCode {
    commands::supersede::run(build_store().as_ref(), docs_dir, old, new)
}
