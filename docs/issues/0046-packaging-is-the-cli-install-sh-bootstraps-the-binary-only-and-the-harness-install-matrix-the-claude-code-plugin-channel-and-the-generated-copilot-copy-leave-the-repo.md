---
type: Issue
title: "Packaging is the CLI: install.sh bootstraps the binary only, and the harness install matrix, the Claude Code plugin channel and the generated Copilot copy leave the repo"
description: "Executes ADR 0028's packaging decision: the binary is the unit of distribution, skill install and hooks install are the only placement channels, and the shell installer, Makefile install targets, plugin manifest and generated instruction copies are removed"
status: closed
timestamp: 2026-09-18T08:59:34Z
---

## Packaging is the CLI

ADR 0028 decided that the release binary is the unit of distribution, that `install.sh` only bootstraps it, and that every skill and hook placement is a CLI verb. The repo never finished executing that: `install.sh` still carries a six-harness copy matrix plus a companion-skills clone, the Makefile mirrors it in fourteen targets, a Claude Code plugin manifest duplicates `hooks install` for one harness, and a generated Copilot instruction file is committed as if it were source. ADR 0059 keeps the engine and the CLI and cuts the accessories; this issue finishes the packaging half.

### Scope

Kept: `living-docs skill install --harness <claude|opencode|codex|pi> [--project] [--dir]`, `living-docs hooks install|uninstall`, `install.sh cli` (release-asset download with checksum verification and the cargo-build fallback, ADR 0041), `make build`, `make cli-install`, `make check`, the Dockerfile.dev toolchain image, `examples/linkly`, `references/`, `ATTRIBUTION.md`.

Removed: every non-`cli` branch of `install.sh` (claude, opencode, codex, pi, cursor, copilot, all, pocock) and the skill-copy helpers; the Makefile `install-*`, `project-*`, `uninstall*` targets; `.claude-plugin/` and `hooks/hooks.json` with their manifest test; `.github/instructions/living-docs.instructions.md`; the `.se-core/` baseline of a tool the repo does not run; README, CONTRIBUTING and skill-rule text that describe the removed channels.

### Decision

Cursor and Copilot lose their generated rule file rather than gaining a CLI verb: both tools read a plain markdown instruction, so a project points them at `living-docs skill living-docs --plain` output or at the installed `SKILL.md`, and a placement verb for two one-file harnesses is not worth its maintenance. Matt Pocock's companion skills stay referenced from ATTRIBUTION.md, never cloned by this repo. Both are reversible in one commit.

### Acceptance

- `./install.sh --help` documents only the `cli` target and its flags; `./install.sh claude` exits 1 with a usage error.
- `make help` lists no `install-*`, `project-*` or `uninstall*` target; `make check` is green and runs `install.sh cli --dry-run` in place of the harness dry-runs.
- `cli/tests/plugin_manifest.rs`, `.claude-plugin/`, `hooks/hooks.json`, `.github/instructions/` and `.se-core/` no longer exist; `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` are green.
- README's installation section describes two steps, install the binary and run `living-docs skill install` plus `living-docs hooks install`, and `living-docs check docs` passes.

### Plan

1. Trim `install.sh` to the `cli` path and its tests; trim the Makefile; update `.github/workflows/*` and `scripts/verify-release-assets.sh` references.
2. Delete the plugin channel, the Copilot copy and the stale baseline, with their tests.
3. Rewrite the README installation and enforcement sections, CONTRIBUTING's layout, and the hard-rules template's channel list.
