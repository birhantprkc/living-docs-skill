use args::{Cli, Command, HooksCmd, SkillCmd};
use clap::Parser;
use living_docs_core::check;
use std::process::ExitCode;

mod args;
mod commands;
mod hooks;
mod skill;
mod skill_install;
mod store;

#[allow(clippy::too_many_lines)]
fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::New {
            doc_type,
            title,
            description,
            kind,
            json,
            owner,
        } => commands::new::run_new(
            &cli.docs_dir,
            &doc_type,
            &title,
            &commands::new::NewArgs {
                description: description.as_deref(),
                kind: kind.as_deref(),
                json: json.as_deref(),
                owner: owner.as_deref(),
            },
        ),
        Command::Index { doc_type } => commands::index::run_index(&cli.docs_dir, doc_type),
        Command::Supersede { old, new } => {
            commands::supersede::run_supersede(&cli.docs_dir, &old, &new)
        }
        Command::Set {
            reference,
            key,
            value,
        } => commands::set::run_set(&cli.docs_dir, &reference, &key, &value),
        Command::Check {
            paths,
            mermaid_only,
            ..
        } if mermaid_only => check::run_mermaid_only(&paths),
        Command::Check {
            paths,
            require_owner,
            ..
        } => commands::check::run_check(&cli.docs_dir, paths, require_owner),
        Command::Fmt { paths, check } => commands::fmt::run_fmt(&cli.docs_dir, paths, check),
        Command::Migrate { paths, apply } => {
            commands::migrate::run_migrate(&cli.docs_dir, paths, apply)
        }
        Command::Effective(args) => commands::effective::run_effective(&cli.docs_dir, args),
        Command::Skill {
            action:
                Some(SkillCmd::Install {
                    harness,
                    project,
                    dir,
                    dry_run,
                }),
            ..
        } => skill_install::install(harness, project, dir, dry_run),
        Command::Skill {
            name,
            topic,
            list,
            json,
            plain,
            ..
        } => commands::skill_cmd::run_skill(name, topic, list, json, plain),
        Command::Hooks {
            cmd: HooksCmd::Install { dir, dry_run },
        } => commands::hooks_cmd::run_hooks_install(dir, dry_run, &cli.docs_dir),
        Command::Hooks {
            cmd: HooksCmd::Uninstall { dir, dry_run },
        } => commands::hooks_cmd::run_hooks_uninstall(dir, dry_run),
    }
}
