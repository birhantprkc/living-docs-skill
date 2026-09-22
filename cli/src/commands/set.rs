//! `set` verb wrapper: delegates to `living_docs_core::commands::set::apply`,
//! rendering the written record as colored text or JSON (ADR 0060).

use crate::output::{self, OutputMode};
use crate::store::build_store;
use living_docs_core::commands;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Serialize)]
struct SetJson<'a> {
    path: &'a Path,
    key: &'a str,
    value: &'a str,
    /// The other bundle records whose reference to the renamed file was
    /// rewritten by a retitle (ADR 0061); empty for every other key.
    rewritten: &'a [PathBuf],
}

/// Names what a retitle rewrote and the search for what it could not reach:
/// a reference outside the bundle — another repo's README, a code comment, a
/// published URL — is beyond any scan the tool owns (ADR 0061).
fn report_rewritten(applied: &commands::set::Applied) {
    let Some(previous) = applied.previous.as_ref().and_then(|path| path.file_name()) else {
        return;
    };
    let previous = previous.to_string_lossy();
    for path in &applied.rewritten {
        println!("{}", path.display());
    }
    eprintln!(
        "living-docs set: rewrote {} in-bundle reference(s) to {previous}; references outside the bundle are not rewritten — run `grep -rn {previous} .`",
        applied.rewritten.len()
    );
}

pub(crate) fn run_set(
    docs_dir: &Path,
    reference: &str,
    key: &str,
    value: &str,
    mode: OutputMode,
) -> ExitCode {
    match commands::set::apply(build_store().as_ref(), docs_dir, reference, key, value) {
        Ok(applied) => {
            if mode.is_json() {
                println!(
                    "{}",
                    output::to_json(&SetJson {
                        path: &applied.path,
                        key,
                        value,
                        rewritten: &applied.rewritten,
                    })
                );
            } else {
                println!("{}", applied.path.display());
                report_rewritten(&applied);
            }
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("living-docs set: {message}");
            ExitCode::from(2)
        }
    }
}
