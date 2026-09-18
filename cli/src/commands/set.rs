//! `set` verb wrapper: delegates to `living_docs_core::commands::set::apply`,
//! rendering the written record as colored text or JSON (ADR 0060).

use crate::output::{self, OutputMode};
use crate::store::build_store;
use living_docs_core::commands;
use serde::Serialize;
use std::path::Path;
use std::process::ExitCode;

#[derive(Serialize)]
struct SetJson<'a> {
    path: &'a Path,
    key: &'a str,
    value: &'a str,
}

pub(crate) fn run_set(
    docs_dir: &Path,
    reference: &str,
    key: &str,
    value: &str,
    mode: OutputMode,
) -> ExitCode {
    match commands::set::apply(build_store().as_ref(), docs_dir, reference, key, value) {
        Ok(path) => {
            if mode.is_json() {
                println!(
                    "{}",
                    output::to_json(&SetJson {
                        path: &path,
                        key,
                        value,
                    })
                );
            } else {
                println!("{}", path.display());
            }
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("living-docs set: {message}");
            ExitCode::from(2)
        }
    }
}
