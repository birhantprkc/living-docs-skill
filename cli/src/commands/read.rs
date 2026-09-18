//! `read` verb wrapper (renamed from `effective` by ADR 0060): delegates to
//! `living_docs_core::commands::effective::run` to compile and print the
//! agent-facing view.

use crate::args::ReadArgs;
use crate::store::build_store;
use living_docs_core::commands::effective::{self, Options};
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_read(docs_dir: &Path, args: ReadArgs) -> ExitCode {
    let options = Options {
        topic: args.topic,
        full: args.full,
    };
    effective::run(build_store().as_ref(), docs_dir, &options)
}
