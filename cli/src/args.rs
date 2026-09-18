//! Clap argument and subcommand definitions for the `living-docs` CLI.

use clap::{Parser, Subcommand};

mod sub;
use std::path::PathBuf;
pub(crate) use sub::{EffectiveArgs, HooksCmd, SkillCmd};

#[derive(Parser)]
#[command(
    name = "living-docs",
    version,
    about = "Deterministic layer of Living Docs authoring. Write ONLY the body below the closing ---. Frontmatter and indexes are CLI-owned: `living-docs set` / `supersede` / `index`."
)]
pub(crate) struct Cli {
    /// Root of the docs bundle. Overridable so tests can point at a temp tree.
    #[arg(long, global = true, default_value = "docs")]
    pub(crate) docs_dir: PathBuf,

    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    New {
        doc_type: String,
        title: String,
        /// Seeds the frontmatter `description:` field with this sentence
        /// instead of the template's placeholder (issue 0021).
        #[arg(long)]
        description: Option<String>,
        /// Architecture views only (ADR 0036): seeds the frontmatter
        /// `kind:` field from the C4/arc42 vocabulary (context, container,
        /// component, flow, sequence, state, data-model, deployment) that
        /// orders the generated architecture index.
        #[arg(long)]
        kind: Option<String>,
        /// Seeds the frontmatter `owner:` field with this value — a
        /// free-form name or email, inserted in canonical position
        /// immediately after `description:`. Never validated against an
        /// identity directory.
        #[arg(long)]
        owner: Option<String>,
    },
    Index {
        doc_type: Option<String>,
    },
    /// `old` and `new` each accept a bare `NNNN` or a type-qualified
    /// `TYPE/NNNN` reference (e.g. `issue/0028`) — required when the same
    /// number exists in more than one doc-type directory, since a bare
    /// `NNNN` fails loudly on that collision instead of guessing (issue
    /// 0029/0025).
    Supersede {
        old: String,
        new: String,
    },
    /// Sets one CLI-owned frontmatter field on a record: `status`,
    /// `description`, or `owner`. `status` is validated against the record's
    /// own type vocabulary (`Superseded` is reserved for `supersede`);
    /// `description`/`owner` accept any string. `reference` accepts a bare
    /// `NNNN` or a type-qualified `TYPE/NNNN` reference (e.g. `issue/0028`),
    /// required when the same number exists in more than one doc-type
    /// directory (issue 0029/0025).
    Set {
        reference: String,
        key: String,
        value: String,
    },
    /// Validate the mechanical Living Docs invariants on a docs bundle, matching
    /// `lint-docs.sh`'s `[BUNDLE_ROOT]` argument (default `docs`) rather than the
    /// global `--docs-dir`. With `--mermaid-only`, `paths` instead lists the
    /// file(s)/directory(ies) to sweep for ```mermaid``` fences (default:
    /// git-tracked `*.md`, fixtures dir excluded), matching `lint-mermaid.sh`.
    Check {
        paths: Vec<PathBuf>,
        /// Validate only ```mermaid``` fences over `paths`, skipping every other invariant.
        #[arg(long)]
        mermaid_only: bool,
        /// Promotes a missing `owner` on a doctype whose registry row
        /// requires it (ADR) from a warning to an invariant violation.
        #[arg(long)]
        require_owner: bool,
    },
    /// Canonicalizes a concept record's frontmatter in place, leaving its
    /// body untouched — the remediation verb for `check`'s
    /// canonical-frontmatter invariant. `paths` accepts a bundle root or a
    /// single record path, matching `check`'s own `[BUNDLE_ROOT]` argument
    /// rather than the global `--docs-dir`; fs-backend only, since db-mode
    /// is canonical by construction on export.
    Fmt {
        paths: Vec<PathBuf>,
        /// Reports which records would change without writing any of them;
        /// exits non-zero when at least one record is pending.
        #[arg(long)]
        check: bool,
    },
    /// Compiles the agent-facing effective view of the bundle (ADR 0050):
    /// active records only (superseded/deprecated withheld), supersede chains
    /// collapsed to the head with a one-line lineage, grouped by kind. Read
    /// this instead of `index.md`. `--topic` filters by a substring; `--full`
    /// prints bodies.
    Effective(EffectiveArgs),
    /// Serves skill content embedded in the binary at compile time (ADR
    /// 0014): list embedded skills and their topics, print a skill's full
    /// `SKILL.md` body, or print one topic's detail. `skill install` (ADR
    /// 0028) places the corpus into a harness's skills directory instead.
    Skill {
        /// The skill to query, e.g. `living-docs`. Required unless `--list`.
        name: Option<String>,
        /// Print only this topic's detail instead of the full `SKILL.md`
        /// body; maps to a `rules/`/`templates/` basename.
        #[arg(long)]
        topic: Option<String>,
        /// List every embedded skill and its available topics instead of
        /// printing a single skill's content.
        #[arg(long)]
        list: bool,
        /// Emit minified single-line JSON instead of plain text, for
        /// consumption by other agents. Only changes the success-output
        /// shape; errors still print to stderr as plain text. Overrides TTY
        /// autodetection; mutually exclusive with `--plain`.
        #[arg(long)]
        json: bool,
        /// Force human-readable plain text, overriding TTY autodetection.
        /// Mutually exclusive with `--json`.
        #[arg(long, conflicts_with = "json")]
        plain: bool,
        #[command(subcommand)]
        action: Option<SkillCmd>,
    },
    /// Materializes the corpus hook scripts into a target project (ADR 0023).
    Hooks {
        #[command(subcommand)]
        cmd: HooksCmd,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use living_docs_core::commands;

    #[test]
    fn root_help_about_carries_the_same_body_only_instruction_new_prints() {
        let about = Cli::command()
            .get_about()
            .expect("the root command carries an about string")
            .to_string();
        assert!(
            about.contains(commands::new::BODY_ONLY_INSTRUCTION),
            "got: {about}"
        );
    }
}
