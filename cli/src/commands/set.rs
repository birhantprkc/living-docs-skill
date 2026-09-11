//! `set` verb wrapper: delegates to `living_docs_core::commands::set::run`.

use crate::config::{Backend, Engine};
use crate::store::{build_backend_store, report_failure};
use living_docs_core::commands;
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_set(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    reference: &str,
    key: &str,
    value: &str,
) -> ExitCode {
    match build_backend_store(backend, engine, docs_dir) {
        Ok(store) => commands::set::run(store.as_ref(), docs_dir, reference, key, value),
        Err(err) => report_failure(&err),
    }
}
