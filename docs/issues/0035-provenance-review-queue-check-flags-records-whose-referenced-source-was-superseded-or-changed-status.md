---
type: Issue
title: "Provenance review queue: check flags records whose referenced source was superseded or changed status"
description: check flags records whose linked source was superseded or left Accepted status, producing a human review queue
status: closed
timestamp: 2026-08-27T19:04:19Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## Provenance review queue: check flags records whose referenced source was superseded or changed status

A record that links another record inherits meaning from it. When the source is superseded or leaves Accepted status, the dependent record may be silently wrong, and an agent consumes it at machine speed. "Retrieved text informs, it never gates": detecting the change is deterministic and belongs to the tool; judging whether it invalidates the dependent record is human. `check` produces the review queue; a person clears it.

This respects the determinism boundary (ADR 0001): the tool never decides that a record IS invalid, only that its source moved.

### Scope

- `check` resolves record-to-record links (the bundle-relative `/adr/NNNN-...` form) and reports every record whose linked target is Superseded, Rejected, or Deprecated.
- The report names the dependent record, the moved source, and the source's new status. It is a distinct finding class, separate from broken-link errors.
- A dependent record acknowledges a moved source with an explicit updated link (to the successor) or a body note referencing the superseding record; either clears the finding.
- KEPT: `supersede` mechanics unchanged. Broken-link checking unchanged.

### Acceptance

- Given A links B and `supersede B C` runs, `check` reports A with a review-queue finding until A links C or references C in its body.
- A link to an Accepted or open record produces no finding.
- The finding class is warn-level first (does not fail the doc-gate); a follow-up decision promotes it to error once the existing corpus is clean.

### Plan

1. ADR first: finding class, clearing rule, warn-vs-error policy.
2. Implement link-target status resolution in `check` with tests over a fixture corpus.
3. Sweep the real `docs/` corpus and clear the initial queue.
