---
type: Issue
title: "Projection freshness SLA: search and web warn or refuse on a stale db-store projection"
description: search and web must warn or refuse when the db-store projection is behind the records tree, keyed to the last successful sync
status: closed
timestamp: 2026-08-27T19:04:16Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## Projection freshness SLA: search and web warn or refuse on a stale db-store projection

The db-store projection is rebuilt by an explicit `sync` step (ADR 0003). Nothing stops `living-docs search` or the web front from answering over a projection that is behind the `.md` records. An agent that consumes a stale search result acts on it with confidence — the stale-index failure mode described in "Making Your Data Ready for Agentic AI" (Sadalage & Chandrasekaran, 2026).

The SLA clock measures the last SUCCESSFUL sync, not the last content change. A sync that silently stopped must read as stale, even when no record appears to have changed.

### Scope

- Record a `last_sync_completed_at` timestamp (and the source-tree fingerprint at that moment) in the db-store on every successful `sync`.
- `living-docs search` compares the fingerprint of the current records tree against the stored one. On mismatch it prints a staleness warning to stderr and still answers. A `--strict` flag turns the warning into a refusal with a nonzero exit.
- The web front surfaces the same staleness state on its responses.
- KEPT: `sync` stays explicit. No automatic re-sync, no file watcher.

### Acceptance

- After `sync`, `search` answers with no warning.
- After a record is added or edited without a `sync`, `search` warns on stderr and `search --strict` exits nonzero with a message that names `sync` as the fix.
- A db-store with no recorded sync timestamp is treated as stale.
- Fitness function: `sync` followed by `search --strict` always exits zero on an unchanged tree.

### Plan

1. ADR first: staleness detection contract (timestamp + fingerprint, where it lives in the schema).
2. Implement in db-store + `search`, with tests for the three acceptance states.
3. Web front surfaces the staleness flag.
