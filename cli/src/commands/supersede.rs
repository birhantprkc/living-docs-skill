//! `supersede` verb wrapper: delegates to `living_docs_core::commands::supersede::run`.

use crate::config::{Backend, Engine};
use crate::store::{build_backend_store, report_failure};
use living_docs_core::commands;
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_supersede(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    old: &str,
    new: &str,
) -> ExitCode {
    match build_backend_store(backend, engine, docs_dir) {
        Ok(store) => commands::supersede::run(store.as_ref(), docs_dir, old, new),
        Err(err) => report_failure(&err),
    }
}
