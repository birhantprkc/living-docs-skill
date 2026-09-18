//! `set` verb wrapper: delegates to `living_docs_core::commands::set::run`.

use crate::store::build_store;
use living_docs_core::commands;
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_set(docs_dir: &Path, reference: &str, key: &str, value: &str) -> ExitCode {
    commands::set::run(build_store().as_ref(), docs_dir, reference, key, value)
}
