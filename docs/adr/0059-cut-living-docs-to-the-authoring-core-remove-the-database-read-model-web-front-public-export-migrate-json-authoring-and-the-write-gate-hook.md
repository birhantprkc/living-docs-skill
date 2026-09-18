---
type: ADR
title: "Cut living-docs to the authoring core: remove the database read-model, web front, public export, migrate, JSON authoring and the write-gate hook"
description: "living-docs is cut back to nine verbs over the .md tree: the db-store read-model, search, the axum web front (Atlas), public export with its leak gate and PII catalog, migrate, new --json and the PreToolUse write-gate are deleted from the workspace, the constitution scope is amended, and the pre-commit check plus session teaching remain the only enforcement"
owner: Evaldo Klock
status: Accepted
timestamp: 2026-09-18T08:26:00Z
---

# 0059. Cut living-docs to the authoring core

## Context

Living docs is a decision log with a gate (ADR 0057). Everything the authoring loop needs is nine verbs over the `.md` tree: `new`, `set`, `supersede`, `index`, `check`, `fmt`, `effective`, `skill`, `hooks`. Measured on this repository, that core is roughly 11k lines of Rust. The rest of the workspace is a periphery nobody in the loop calls: the `db-store` read-model with `db sync` and `search` (about 5.7k lines, and no skill rule cites `search`), the axum web front behind PRD 0001 Atlas (2.3k lines, none of issues 0010 to 0013 delivered), public export with the leak gate and the PII catalog (5k lines), the `migrate` advisor, `new --json` section authoring, the `hmac`/`sha2` leftovers of the removed seal, and the PreToolUse write-gate hook. Together they are half the code, two thirds of the 352 locked dependencies, and the subject of 22 active ADRs that `effective` serves to every agent session.

The write-gate deserves its own line. It blocks the harness `Write`/`Edit` tools on record files and the observed effect is that the agent negotiates with the block rather than switching to the CLI; the gate also exists only on the one harness that exposes a pre-write hook. The deterministic gate that works on every harness is `check`, run by the pre-commit hook and CI.

Alternatives considered and rejected:

- **Keep the periphery behind feature flags or an unbuilt workspace member.** Rejected: code the CI does not compile rots on the next core refactor, and a flagged crate still costs clippy, the file-size ratchet and reading time. Git history already preserves it; the workspace should hold only what ships.
- **Keep the db read-model but drop the web front.** Rejected: the read-model has no consumer once the web is gone, since `effective` compiles the agent view from the tree without a database.
- **Keep `--json` beside scaffold-and-edit.** Rejected: two authoring paths for one act is the bloat this ADR removes, and the agent's native path is editing markdown. The scaffold path is made safe instead: templates carry only `{{PLACEHOLDER}}` slots, and an unfilled slot is already a `check` violation.
- **Keep the write-gate and tune its message.** Rejected: instructions and pre-write blocks do not change agent behaviour; a failing `check` in the same session does.

## Decision

We will cut the workspace to the authoring core and amend the constitution to match:

- **Crates.** `db-store` and `web` leave the workspace. `living-docs-core` loses `pii`, `commands/export`, `commands/leak_gate`, `commands/migrate`, `commands/new/sections`, the `SearchIndex` port, the `visibility` frontmatter field and the `hmac`/`sha2` dependencies.
- **Verbs and flags.** `db`, `search`, `export`, `leak-gate` and `migrate` are removed, together with the global `--backend` and `--engine` flags, `new --json` and `index --visibility`. `hooks install` materializes the session-teaching hook and the pre-commit doc-gate only; the PreToolUse write-gate, its plugin wiring and its fixtures are deleted.
- **Skill corpus.** The `public-export` skill and the `migration` topic are deleted; `procedure` teaches one authoring path, scaffold then edit the body; templates drop their HTML-comment guidance.
- **Constitution.** Scope is amended to the authoring core; the read-model, web, ParadeDB, multi-project catalog and db-mode authoring move to an explicit re-entry clause: they return as workspace fronts (ADR 0033) only when a consumer needs cross-project search or an independently deployed surface.
- **Records.** The ADRs whose subject is deleted are marked `Deprecated` (`supersede` is one-to-one, as ADR 0057 noted); PRD 0001 is withdrawn; the Atlas and artifact issues are closed.

## Consequences

**Easier / gained:**
- One binary, nine verbs, a workspace CI can build in a fraction of the time, and an `effective` view that no longer carries 22 records about surfaces that do not exist.

**Harder / accepted trade-offs:**
- Cross-project full-text search is gone until a front earns it back; `grep` and `effective --topic` are the interim answer.
- The seal, PII catalog and Atlas work are recoverable from git history only, not from the tree.

**Follow-ups:**
- The corpus audit that motivated this cut also found active ADRs without a rejected alternative and issues resolved but still open; those become `check` gates in a separate record.

## Verification

**Implementation impact:** `Cargo.toml` (workspace members), `cli/src/{args.rs,main.rs,config.rs,store.rs,hooks.rs}`, `cli/src/commands/`, `living-docs-core/src/{lib.rs,store.rs,commands/,pii/}`, `skills/living-docs/{hooks,rules,templates}`, `skills/public-export/`, `hooks/hooks.json`, `.claude/settings.json`, `Makefile`, `docker-compose.yml`, `scripts/*-baseline.txt`, `docs/constitution.md`.

**Verification criteria:**
- `living-docs --help` lists exactly `new`, `index`, `supersede`, `set`, `check`, `fmt`, `effective`, `skill`, `hooks`; `new --json`, `--backend` and `--engine` are rejected as unknown.
- `cargo test`, `cargo clippy -D warnings` and `make check` are green with `db-store` and `web` absent from the workspace, and `check docs` passes with the retired records deprecated or closed.
