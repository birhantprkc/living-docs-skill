//! `new` verb wrapper: delegates to `living_docs_core::commands::new::write`,
//! rendering the created path as colored text or JSON (ADR 0060).

use crate::output::{self, OutputMode};
use crate::store::build_store;
use living_docs_core::commands;
use living_docs_core::commands::new::{NewOptions, BODY_ONLY_INSTRUCTION};
use serde::Serialize;
use std::path::Path;
use std::process::ExitCode;

/// The `new` verb's optional CLI arguments, bundled so the wrapper's
/// signature stays within the argument budget as flags accrue.
pub(crate) struct NewArgs<'a> {
    pub(crate) description: Option<&'a str>,
    pub(crate) kind: Option<&'a str>,
    pub(crate) owner: Option<&'a str>,
}

#[derive(Serialize)]
struct NewJson<'a> {
    path: &'a Path,
    instruction: &'a str,
}

pub(crate) fn run_new(
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    args: &NewArgs,
    mode: OutputMode,
) -> ExitCode {
    let opts = NewOptions {
        description: args.description,
        kind: args.kind,
        owner: args.owner,
    };
    match commands::new::write(build_store().as_ref(), docs_dir, doc_type, title, &opts) {
        Ok(path) => {
            if mode.is_json() {
                println!(
                    "{}",
                    output::to_json(&NewJson {
                        path: &path,
                        instruction: BODY_ONLY_INSTRUCTION,
                    })
                );
            } else {
                println!("{}", path.display());
                println!("{BODY_ONLY_INSTRUCTION}");
            }
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("living-docs new: {message}");
            ExitCode::from(2)
        }
    }
}
