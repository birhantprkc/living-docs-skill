---
type: Issue
title: fmt rewrites record bodies (reference lists collapse to one line) and supersede emits non-canonical frontmatter that sends users to fmt
description: living-docs fmt 0.14.0 collapses multi-line reference lists into one line across the whole bundle (179 records, -12951 lines on ai-configs); supersede writes superseded_by after timestamp so check routes users to fmt; fmt accepts bundle roots only.
status: closed
timestamp: 2026-09-07T16:33:10Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## fmt rewrites record bodies (reference lists collapse to one line) and supersede emits non-canonical frontmatter that sends users to fmt

`living-docs fmt` (0.14.0) is documented as the remediation verb for `check`'s canonical-frontmatter invariant, yet it rewrites record BODIES: every multi-line `# References` list is collapsed into one paragraph line. On the ai-configs bundle (314 docs) one bare `living-docs fmt` rewrote 179 records with `179 files changed, 3512 insertions(+), 12951 deletions(-)`. Sample (`docs/adr/0001-distillation-loop-over-agent-memory-mcp.md`): five reference lines `[1] ...` through `[5] ...` became a single line `[1] ... [2] ... [3] ... [4] ... [5] ...`. The ADR 0064 body dropped from 362 to 173 lines. A second, related defect makes `fmt` unavoidable: `living-docs supersede <old> <new>` writes `superseded_by:` AFTER `timestamp:` in the old record, and `check` then reports that record as `non-canonical (hand-written?) frontmatter -- run living-docs fmt`. So a user who follows the CLI's own advice after a supersede triggers the bundle-wide body rewrite. `fmt [PATHS]` accepts bundle roots only (`bundle root not found: docs/adr/0134-...md`), so there is no way to canonicalize one record. Recovery used in ai-configs: `git checkout -- docs`, then copy the two fmt-canonical records back from a scratch dir.

### Scope

- `fmt` touches frontmatter only; the body below the closing `---` is byte-identical before and after.
- `supersede` (and `status`, `new`) emit canonical key order directly, so `check` never sends a user to `fmt` after a CLI verb.
- `fmt [PATHS]` accepts record files as well as bundle roots.
- Out of scope: the SIZE advisory, the owner ratchet.

### Acceptance

- On a bundle where every record is already canonical, `living-docs fmt` rewrites 0 records and `git status` stays clean.
- A record whose body carries a five-line `# References` list keeps five lines after `fmt` (regression fixture).
- After `living-docs supersede 0134 0146`, `living-docs check` reports no `non-canonical frontmatter` finding for either record without running `fmt`.
- `living-docs fmt docs/adr/0134-x.md` canonicalizes that one file and exits 0.

### Plan

1. Add a body round-trip test: parse record, re-emit, assert body bytes unchanged; find the reference-list collapse (likely the markdown re-serializer joining list items).
2. Route `supersede`/`status`/`new` through the same canonical frontmatter serializer `fmt` uses.
3. Accept file paths in `fmt [PATHS]`; resolve the bundle root from the file's ancestors.
4. Add a `fmt --check` (or dry-run) that lists what would change, so a bundle-wide rewrite is visible before it happens.

