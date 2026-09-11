//! `effective` verb wrapper: builds the active-backend store, then delegates
//! to `living_docs_core::commands::effective::run` to compile and print the
//! agent-facing view.

use crate::args::EffectiveArgs;
use crate::config::{Backend, Engine};
use crate::store::{build_backend_store, report_failure};
use living_docs_core::commands::effective::{self, Options};
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_effective(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    args: EffectiveArgs,
) -> ExitCode {
    let options = Options {
        topic: args.topic,
        full: args.full,
    };
    match build_backend_store(backend, engine, docs_dir) {
        Ok(store) => effective::run(store.as_ref(), docs_dir, &options),
        Err(err) => report_failure(&err),
    }
}
