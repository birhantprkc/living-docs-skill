//! Second-level subcommand enums for `seal`, `hooks`, `skill`, and `db`.

use crate::skill_install::Harness;
use clap::{Args, Subcommand, ValueEnum};
use std::path::PathBuf;

/// The `effective --tier` value, mapped to
/// `living_docs_core::commands::effective::Tier` by the command wrapper.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum TierArg {
    Index,
    Outline,
    Full,
}

/// Arguments for the `effective` verb (ADR 0050), factored into their own
/// `Args` struct so the top-level `Command` enum stays within the file-size
/// ratchet (issue 0028).
#[derive(Args)]
pub(crate) struct EffectiveArgs {
    /// Restrict the view to records whose title, description, or body
    /// contains this term (case-insensitive). Omitted: everything active.
    #[arg(long)]
    pub(crate) topic: Option<String>,
    /// How much of each record to show: `index` (the default), `outline`
    /// (headings), or `full` (bodies).
    #[arg(long, value_enum, default_value = "index")]
    pub(crate) tier: TierArg,
    /// Hard cap on output tokens (~4 chars each): truncation degrades the tier
    /// first, then drops the lowest-ranked records — never exceeded.
    #[arg(long)]
    pub(crate) budget: Option<usize>,
    /// Include records the liveness check (ADR 0049) flags stale, withheld by
    /// default.
    #[arg(long)]
    pub(crate) include_stale: bool,
}

/// Arguments for the `scorecard` verb, in their own `Args` struct for the
/// same file-size reason as [`EffectiveArgs`].
#[derive(Args)]
pub(crate) struct ScorecardArgs {
    /// Emits deterministic JSON instead of the human-readable table.
    #[arg(long)]
    pub(crate) json: bool,
    /// Consumption window, e.g. `7d` or `2w` (ADR 0053): summarize only doc
    /// reads within it, measured back from the newest captured read. Omitted
    /// spans the whole capture.
    #[arg(long)]
    pub(crate) since: Option<String>,
}

/// Arguments for the `why` verb (ADR 0051), in their own `Args` struct for the
/// same file-size reason as [`EffectiveArgs`].
#[derive(Args)]
pub(crate) struct WhyArgs {
    /// The repository path to trace. Omitted when `--from-diff` supplies the
    /// paths instead.
    pub(crate) path: Option<String>,
    /// A git range (e.g. `HEAD~1..HEAD`) whose touched files are traced —
    /// resolved by the front as `git diff --name-only <range>`.
    #[arg(long)]
    pub(crate) from_diff: Option<String>,
    /// Include records the liveness check (ADR 0049) flags stale, withheld by
    /// default.
    #[arg(long)]
    pub(crate) include_stale: bool,
}

#[derive(Subcommand)]
pub(crate) enum SealCmd {
    /// Generates a fresh per-clone key and seals the current bundle as the
    /// trusted baseline — run after clone, merge, or adoption.
    Init,
}

#[derive(Subcommand)]
pub(crate) enum HooksCmd {
    /// Writes the two corpus hook scripts into `<dir>/.living-docs/hooks/`
    /// at mode 0755, materializes the pre-commit doc-gate to
    /// `<dir>/.githooks/pre-commit` (pointing `core.hooksPath` at it), and
    /// wires the Claude Code hooks into `<dir>/.claude/settings.json`,
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
    /// Removes the artifacts `install` wrote — the two `.living-docs/hooks/`
    /// scripts, `.githooks/pre-commit`, and the living-docs entries in
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

#[derive(Subcommand)]
pub(crate) enum DbCmd {
    /// Rebuild the read-model from every doc `--docs-dir` lists, scoped to
    /// one named project (ADR 0005, issue 0005 slice 0005-B).
    Sync {
        /// The project slug to sync into. Defaults to a slug derived from
        /// `--docs-dir`: its own directory name, or its parent directory's
        /// name when the final component is literally `docs` — so every
        /// repo's `<repo>/docs` bundle gets a project unique to that repo
        /// instead of every repo colliding on the literal word `docs`.
        #[arg(long)]
        project: Option<String>,
    },
}
