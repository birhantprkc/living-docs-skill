//! `status` verb wrapper: delegates to `living_docs_core::commands::status::run`.

use crate::config::{Backend, Engine};
use crate::store::{build_backend_store, report_failure};
use living_docs_core::commands;
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_status(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    number: &str,
    new_status: &str,
) -> ExitCode {
    match build_backend_store(backend, engine, docs_dir) {
        Ok(store) => commands::status::run(store.as_ref(), docs_dir, number, new_status),
        Err(err) => report_failure(&err),
    }
}
