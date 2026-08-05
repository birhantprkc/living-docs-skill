//! `index` verb wrapper: delegates to `living_docs_core::commands::index::run`.

use crate::config::{Backend, Engine};
use crate::store::{build_backend_store, report_failure};
use living_docs_core::commands;
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_index(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    doc_type: Option<String>,
    visibility: Option<Vec<String>>,
) -> ExitCode {
    match build_backend_store(backend, engine, docs_dir) {
        Ok(store) => commands::index::run(store.as_ref(), docs_dir, doc_type, visibility),
        Err(err) => report_failure(&err),
    }
}
