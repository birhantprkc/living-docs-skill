//! `export` verb wrapper: delegates to `living_docs_core::commands::export::export`.

use crate::store::build_store;
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_export(
    docs_dir: &Path,
    out_dir: &Path,
    visibility: Option<Vec<String>>,
) -> ExitCode {
    living_docs_core::commands::export::export(
        build_store().as_ref(),
        docs_dir,
        out_dir,
        visibility,
    )
}
