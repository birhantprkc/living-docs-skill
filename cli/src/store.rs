//! Store construction over the `.md` tree and the shared failure reporter.

use living_docs_core::store::DocStore;
use std::process::ExitCode;

pub(crate) fn build_store() -> Box<dyn DocStore> {
    Box::new(fs_store::FsStore::new())
}

pub(crate) fn report_failure(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::FAILURE
}
