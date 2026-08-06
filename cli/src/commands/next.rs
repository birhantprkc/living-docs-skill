//! `next` verb wrapper: delegates straight to `living_docs_core::commands::next::run`.

use living_docs_core::commands;
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_next(docs_dir: &Path, doc_type: &str) -> ExitCode {
    commands::next::run(docs_dir, doc_type)
}
