---
type: Issue
title: "Doc-readiness scorecard: a check subreport that grades a docs tree from human-era to agent-ready"
description: a deterministic per-attribute scorecard that grades a docs tree from human-era to agent-ready, so investment goes to the weakest row
status: open
timestamp: 2026-08-27T19:04:25Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## Doc-readiness scorecard: a check subreport that grades a docs tree from human-era to agent-ready

Agents are the primary consumers of a living-docs tree. A single pass/fail from `check` hides WHERE a corpus is weak. A scorecard grades each readiness attribute separately, so the next investment goes to the weakest row — readiness is capped by the weakest attribute, never averaged.

Every signal must be deterministic and computable from the tree alone. The tool grades structure; it never judges prose quality (determinism boundary, ADR 0001).

### Scope

- New subcommand `living-docs check --scorecard` (or `living-docs scorecard`) emitting a per-attribute grade over the docs tree.
- Attributes and their deterministic signals:
  - Trusted: doc-gate conformance rate; projection freshness state (issue 0034 when available).
  - Contextual: glossary present; index rows complete; records per semantic-index home.
  - Traceable: supersede chains intact; records linking their motivating ADR/PRD.
  - Governed: owner coverage on ADR/BDR (issue 0036); stale review-queue findings (issue 0035).
- Output: human-readable table plus `--json`.
- KEPT: plain `check` output and exit-code semantics unchanged.

### Acceptance

- The scorecard runs on any conformant tree and exits zero; grades are informational, never gate.
- Signals that depend on issues 0034/0035/0036 degrade to "not measured" when those features are absent, instead of failing.
- The same tree always produces the same scorecard (fitness function: idempotent, deterministic output).

### Plan

1. Land after 0034–0036; ADR fixes the attribute/signal table.
2. Implement signal collectors + table/JSON rendering with fixture-tree tests.
