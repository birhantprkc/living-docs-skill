//! `completions` verb wrapper (ADR 0060): prints a shell completion script
//! generated straight from the `clap` command tree, so it never drifts from
//! the verbs and flags `--help` documents.

use crate::args::Cli;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;
use std::process::ExitCode;

pub(crate) fn run_completions(shell: Shell) -> ExitCode {
    let mut command = Cli::command();
    let name = command.get_name().to_owned();
    generate(shell, &mut command, name, &mut io::stdout());
    ExitCode::SUCCESS
}
