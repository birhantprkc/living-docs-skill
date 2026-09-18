//! Second-level subcommand enums for `hooks` and `skill`.

use crate::skill_install::Harness;
use clap::{Args, Subcommand};
use std::path::PathBuf;

/// Arguments for the `effective` verb (ADR 0050), factored into their own
/// `Args` struct so the top-level `Command` enum stays within the file-size
/// ratchet (issue 0028).
#[derive(Args)]
pub(crate) struct EffectiveArgs {
    /// Restrict the view to records whose title, description, or body
    /// contains this term (case-insensitive). Omitted: everything active.
    #[arg(long)]
    pub(crate) topic: Option<String>,
    /// Print each record's full body instead of a one-line index entry.
    #[arg(long)]
    pub(crate) full: bool,
}

#[derive(Subcommand)]
pub(crate) enum HooksCmd {
    /// Writes the session-teaching hook into `<dir>/.living-docs/hooks/`
    /// at mode 0755, materializes the pre-commit doc-gate to
    /// `<dir>/.githooks/pre-commit` (pointing `core.hooksPath` at it), and
    /// wires the `SessionStart` hook into `<dir>/.claude/settings.json`,
    /// idempotently — re-running replaces the living-docs entries by
    /// identity rather than appending. The generated commands pin the
    /// resolved `--docs-dir` bundle as a `LIVING_DOCS_BUNDLE=` prefix.
    /// `--dry-run` reports the same plan without writing anything.
    Install {
        /// Target project root; defaults to the current directory.
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Report the plan without writing any file or directory.
        #[arg(long)]
        dry_run: bool,
    },
    /// Removes the artifacts `install` wrote — the `.living-docs/hooks/`
    /// script, `.githooks/pre-commit`, and the living-docs entries in
    /// `<dir>/.claude/settings.json` — leaving unrelated entries and
    /// `core.hooksPath` untouched. A clean no-op when nothing was installed.
    /// `--dry-run` reports the same removal plan without deleting anything.
    Uninstall {
        /// Target project root; defaults to the current directory.
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Report the plan without removing any file.
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum SkillCmd {
    /// Places the three skill directories from the embedded corpus into a
    /// harness's skills directory (ADR 0028) — no working tree involved.
    /// `--project` scopes the destination to the current project instead of
    /// the harness's global, `$HOME`-rooted directory; `--dir` overrides the
    /// destination outright.
    Install {
        #[arg(long, value_enum, default_value = "claude")]
        harness: Harness,
        #[arg(long)]
        project: bool,
        /// Destination root for the skill directories, overriding both
        /// `--harness` and `--project` outright. When given, `--harness`
        /// still parses but no longer affects where anything is placed.
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Report the plan without writing any file.
        #[arg(long)]
        dry_run: bool,
    },
}
