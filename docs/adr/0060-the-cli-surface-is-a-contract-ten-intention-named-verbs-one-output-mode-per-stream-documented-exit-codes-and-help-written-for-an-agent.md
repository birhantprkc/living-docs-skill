---
type: ADR
title: "The CLI surface is a contract: ten intention-named verbs, one output mode per stream, documented exit codes, and help written for an agent"
description: "Fixes the CLI's public surface: effective becomes read, skill becomes guide, skill install and hooks install fold into install and uninstall, with hidden aliases for one release; every data verb prints JSON when stdout is not a TTY and plain text with color when it is, honoring NO_COLOR and --color; exit codes are 0 success, 1 gate failed, 2 usage; --help is grouped, one line per verb, with examples; shell completions ship from clap"
owner: Evaldo Klock
status: Accepted
timestamp: 2026-09-18T09:11:58Z
---

# 0060. The CLI surface is a contract: ten intention-named verbs, one output mode per stream, documented exit codes, and help written for an agent

## Context

Two audiences call the binary: a human at a terminal and an agent that parses stdout. Today the surface serves neither well. `--help` prints each verb's Rust docblock, so the root help is a wall of ADR numbers and script names; three verbs are named after internals rather than intent (`effective` for "read what is in force", `skill` for "load the guide for a record type", and skill placement split between `skill install` and `hooks install`); only `skill` detects a TTY and switches between plain text and JSON, every other verb prints the same prose to a pipe as to a person; and the exit codes (0, 1 for a failed gate, 2 for usage) are consistent in practice but written nowhere. The README's disclaimer states that verbs may change between releases; this record is where they change once, so that afterwards a verb name is a contract.

## Decision

We will fix the surface along the command-line interface guidelines (clig.dev) and keep it there.

- **Names say the intention.** `effective` becomes `read` ("read what governs X now"). `skill` becomes `guide` with the topic positional (`guide adr`, `guide --list`, `--skill <name>` for a corpus other than living-docs). `skill install` and `hooks install|uninstall` fold into `install skills|hooks` and `uninstall skills|hooks`. The surface is ten verbs: `new`, `set`, `supersede`, `index`, `check`, `fmt`, `read`, `guide`, `install`, `uninstall`. `effective` and `skill` survive as hidden aliases for exactly one release and are removed in the next.
- **One output mode per stream.** Data goes to stdout, diagnostics and progress to stderr. When stdout is not a TTY, every data verb (`read`, `guide`, `check`, `index`, `new`, `set`, `supersede`) prints minified JSON with a stable shape; when it is a TTY it prints plain text with color. `--json` and `--plain` force either; `--color=auto|always|never` and the `NO_COLOR` environment variable govern color; `--quiet` silences informational stderr.
- **Exit codes are documented and stable.** 0 success, 1 a gate or a verb's own check failed, 2 invalid usage or a missing input. Every verb's `--help` ends with that table.
- **Help is written for an agent.** The root help lists one line per verb, grouped as authoring (`new`, `set`, `supersede`, `index`, `fmt`), gate (`check`), reading (`read`, `guide`) and distribution (`install`, `uninstall`), and carries the body-only rule as its about line. Each verb's help has a `Examples` section with two or three real invocations. Docblocks stay in the code; they are no longer the help text.
- **Completions ship from clap.** `living-docs completions <shell>` prints the script for bash, zsh and fish.

Rejected alternatives:

- **Keep the names and only fix help.** Rejected: the names are the part an agent has to guess. `effective` and `skill living-docs --topic` were the two verbs consumers most often failed to find in the skill router.
- **Rename without aliases.** Rejected: the session hook, hard-rules template and consumers' scripts name the verbs; one release of hidden aliases costs three lines and avoids a breaking release for a rename.
- **A global `--output json|text` flag instead of TTY detection.** Rejected as the only mechanism: the detection `skill` already does is what makes the binary correct by default in an agent's pipe; the explicit flags stay for the cases where the default is wrong.

## Consequences

**Easier / gained:**
- An agent can read any verb's output as JSON without a flag, a human gets colored, scannable text, and both find the verb by what they want to do.

**Harder / accepted trade-offs:**
- Every document that names a verb (SKILL.md, CLAUDE.md, the session hook, the hard-rules template, README, the rules topics) changes in the same release; retired ADR bodies keep the old names as history.
- The JSON shapes become a contract to keep; a field is added, never renamed or removed, from this release on.

**Follow-ups:**
- Remove the `effective` and `skill` aliases in the release after v0.18.0.

## Verification

**Implementation impact:** `cli/src/args.rs`, `cli/src/args/sub.rs`, `cli/src/main.rs`, `cli/src/commands/*`, a new `cli/src/output.rs` (mode resolution, color, JSON envelope), `living-docs-core/src/check/mod.rs` (the reporter renders through the output mode), `skills/living-docs/SKILL.md`, `skills/living-docs/hooks/session-context.sh`, `skills/living-docs/templates/claude-hard-rules.md`, `skills/living-docs/rules/*.md`, `CLAUDE.md`, `README.md`, `cli/tests/*`.

**Verification criteria:**
- `living-docs --help` lists exactly the ten verbs in four groups; `living-docs effective` and `living-docs skill` still run and are absent from the help.
- `living-docs check docs | cat` prints one JSON document and `living-docs check docs` on a TTY prints colored text; `NO_COLOR=1` and `--color=never` remove the color; a failing gate exits 1 and an unknown verb exits 2, asserted by tests that capture stdout and stderr separately.
