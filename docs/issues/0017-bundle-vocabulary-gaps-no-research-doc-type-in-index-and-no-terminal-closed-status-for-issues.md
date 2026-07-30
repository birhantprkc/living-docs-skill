---
type: Issue
title: "bundle vocabulary gaps: no research doc type in index and no terminal closed status for issues"
description: Close the two bundle vocabulary gaps — a research type the index cannot generate, and issues with no terminal closed status.
status: Proposed
timestamp: 2026-07-30T18:59:13Z
---

## Problem

Two vocabulary gaps surfaced by a downstream bundle (ai-configs) that this repo's own bundle shares:

- `index` supports only `adr|bdr|prd|issue`. Bundles carrying a `research/` directory (including THIS repo's `docs/research/`) cannot generate `research/index.md` via the CLI, so every research note is a permanent invariant-3 orphan that `check` flags and nothing can fix.
- `status` accepts only `Proposed|Accepted|Deprecated` (plus `supersede`). Issues have no terminal `closed`/`resolved` state: a downstream repo had to close a FIXED bug report as `Deprecated`, which misstates the outcome.

## Status of each half

The first half is **done**: ADR 0026 made `research` a row in the `DocTypeSpec`
registry, so `living-docs new research` and `living-docs index research` both
work and a research note is no longer a permanent invariant-3 orphan. It was not
added to a type set — the type set was replaced by the registry, which is why the
fix generalizes to the next type instead of repeating this issue.

The second half is **open**: `status` still has no terminal state for an issue.

## Proposal

- ~~Add `research` to the `index` type set (same list format; title/status columns as for issues), or make the type set bundle-configurable.~~ Delivered by ADR 0026.
- Add issue-appropriate terminal statuses (`Resolved`, `Closed`, or a `resolution:` field) to the `status` verb's vocabulary, keeping the ADR lifecycle untouched.
