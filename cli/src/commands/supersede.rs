//! `supersede` verb wrapper: delegates to
//! `living_docs_core::commands::supersede::supersede`, rendering the two
//! resolved paths as colored text or JSON (ADR 0060).

use crate::output::{self, OutputMode};
use crate::store::build_store;
use living_docs_core::commands;
use serde::Serialize;
use std::path::Path;
use std::process::ExitCode;

#[derive(Serialize)]
struct SupersedeJson<'a> {
    old: &'a Path,
    new: &'a Path,
}

pub(crate) fn run_supersede(docs_dir: &Path, old: &str, new: &str, mode: OutputMode) -> ExitCode {
    match commands::supersede::supersede(build_store().as_ref(), docs_dir, old, new) {
        Ok((old_path, new_path)) => {
            if mode.is_json() {
                println!(
                    "{}",
                    output::to_json(&SupersedeJson {
                        old: &old_path,
                        new: &new_path,
                    })
                );
            } else {
                println!("{}", old_path.display());
                println!("{}", new_path.display());
            }
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("living-docs supersede: {message}");
            ExitCode::from(2)
        }
    }
}
