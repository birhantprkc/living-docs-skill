//! `export` verb wrapper: delegates to `living_docs_core::commands::export::export`.

use crate::config::{Backend, Engine};
use crate::store::{build_backend_store, report_failure};
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_export(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    out_dir: &Path,
    visibility: Option<Vec<String>>,
) -> ExitCode {
    match build_backend_store(backend, engine, docs_dir) {
        Ok(store) => living_docs_core::commands::export::export(
            store.as_ref(),
            docs_dir,
            out_dir,
            visibility,
        ),
        Err(err) => report_failure(&err),
    }
}
