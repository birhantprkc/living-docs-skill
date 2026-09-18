//! `new` verb wrapper: delegates to `living_docs_core::commands::new::run`.

use crate::store::build_store;
use living_docs_core::commands;
use living_docs_core::commands::new::NewOptions;
use std::path::Path;
use std::process::ExitCode;

/// The `new` verb's optional CLI arguments, bundled so the wrapper's
/// signature stays within the argument budget as flags accrue.
pub(crate) struct NewArgs<'a> {
    pub(crate) description: Option<&'a str>,
    pub(crate) kind: Option<&'a str>,
    pub(crate) owner: Option<&'a str>,
}

pub(crate) fn run_new(docs_dir: &Path, doc_type: &str, title: &str, args: &NewArgs) -> ExitCode {
    let opts = NewOptions {
        description: args.description,
        kind: args.kind,
        owner: args.owner,
    };
    commands::new::run(build_store().as_ref(), docs_dir, doc_type, title, &opts)
}
