//! `new` verb wrapper: resolves the `--json` payload, then delegates to core.

use crate::store::build_store;
use living_docs_core::commands;
use living_docs_core::commands::new::NewOptions;
use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

/// Resolves the `--json` argument's payload forms (ADR 0038): `-` reads
/// stdin, `@<path>` reads a file, anything else is the literal JSON.
fn resolve_json_payload(json: Option<&str>) -> Result<Option<String>, String> {
    match json {
        None => Ok(None),
        Some("-") => {
            let mut payload = String::new();
            std::io::stdin()
                .read_to_string(&mut payload)
                .map_err(|e| format!("--json -: reading stdin failed: {e}"))?;
            Ok(Some(payload))
        }
        Some(arg) => match arg.strip_prefix('@') {
            Some(path) => std::fs::read_to_string(path)
                .map(Some)
                .map_err(|e| format!("--json @{path}: {e}")),
            None => Ok(Some(arg.to_string())),
        },
    }
}

/// The `new` verb's optional CLI arguments, bundled so the wrapper's
/// signature stays within the argument budget as flags accrue.
pub(crate) struct NewArgs<'a> {
    pub(crate) description: Option<&'a str>,
    pub(crate) kind: Option<&'a str>,
    pub(crate) json: Option<&'a str>,
    pub(crate) owner: Option<&'a str>,
}

pub(crate) fn run_new(docs_dir: &Path, doc_type: &str, title: &str, args: &NewArgs) -> ExitCode {
    let payload = match resolve_json_payload(args.json) {
        Ok(payload) => payload,
        Err(err) => {
            eprintln!("living-docs new: {err}");
            return ExitCode::from(2);
        }
    };
    let opts = NewOptions {
        description: args.description,
        kind: args.kind,
        sections_json: payload.as_deref(),
        owner: args.owner,
    };
    commands::new::run(build_store().as_ref(), docs_dir, doc_type, title, &opts)
}
