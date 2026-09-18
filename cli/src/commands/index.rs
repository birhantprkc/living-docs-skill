//! `index` verb wrapper: delegates to `living_docs_core::commands::index::run`.

use crate::store::build_store;
use living_docs_core::commands;
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_index(
    docs_dir: &Path,
    doc_type: Option<String>,
    visibility: Option<Vec<String>>,
) -> ExitCode {
    commands::index::run(build_store().as_ref(), docs_dir, doc_type, visibility)
}
