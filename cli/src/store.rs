//! Store construction over the `.md` tree and the shared failure reporter.

use living_docs_core::store::DocStore;
use std::process::ExitCode;

pub(crate) fn build_store() -> Box<dyn DocStore> {
    Box::new(fs_store::FsStore::new())
}

/// Prints a usage-level failure (an invalid or unresolvable argument, e.g.
/// an unknown `guide` skill/topic) and returns exit code 2, per the
/// documented exit-code table (ADR 0060: 0 success, 1 gate failed, 2 usage).
pub(crate) fn report_failure(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(2)
}
