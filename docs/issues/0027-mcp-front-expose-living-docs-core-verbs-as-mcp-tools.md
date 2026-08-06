---
type: Issue
title: "MCP front: expose living-docs core verbs as MCP tools"
description: A read-only MCP server crate in the workspace exposing search/show/list and, as they land, the graph verbs — agents consume the read-model through typed tools instead of parsing markdown.
status: open
timestamp: 2026-08-05T20:33:40Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## MCP front: expose living-docs core verbs as MCP tools

Agents are first-class consumers of living docs, and today they consume them by shelling out to the CLI or parsing markdown. An MCP server front makes the read-model and the knowledge graph directly consumable by any MCP-capable agent, with typed tools instead of ad-hoc parsing. Implements [ADR 0033](/adr/0033-new-consumers-are-fronts-in-the-workspace-never-new-repos-until-a-deploy-cadence-or-ownership-trigger-fires.md) (fronts in the workspace); consumes the graph verbs of [ADR 0031](/adr/0031-the-knowledge-graph-is-a-typed-bi-temporal-edge-set-in-the-existing-relational-store.md) as they land. Precedent: Graphiti ships its MCP server from the same repository as its core.

### Scope

- New workspace member `mcp`: an MCP server (stdio transport first) that depends on `living-docs-core` and the db-store read-model, never on `cli` or `web`.
- Initial read-only tool set mapping existing verbs: `search`, `show` (record by type+number), `list` (by type/status), plus graph verbs (`why`, `impact`, `as-of`) as ADR 0031 slices deliver them.
- Version site registered in `check-version.sh` in the same PR that creates the crate (ADR 0033 verification criterion).
- KEPT: the CLI as the authoring surface — the MCP front is read-only in its first iteration; authoring tools (new/status/supersede) are a later decision with the same optimistic-concurrency rules as ADR 0016.
- Out of scope: HTTP/SSE transport; authoring tools; any LLM-side logic (the server serves data, the consuming agent judges).

### Acceptance

- An MCP client lists the tools and gets typed schemas; `search` over a synced corpus returns the same hits as `living-docs search` for the same query.
- `show` returns a record's frontmatter fields and body for a type+number reference, without number-collision ambiguity (depends on issue 0025's qualified reference).
- The crate builds in the workspace, depends only on core + db-store, and its version site fails `check-version.sh` when out of sync (fitness).

### Plan

1. Crate skeleton + stdio MCP server + `search`/`show`/`list` over the existing read-model.
2. Graph tools (`why`/`impact`/`as-of`) as ADR 0031 query verbs land — same slice adds them to both `cli` and `mcp`.
