---
type: Issue
title: next reports 0001 for issue and bdr regardless of existing records
description: Fix living-docs next to scan existing issue and bdr records the same way it correctly does for adr, instead of always reporting 0001.
status: Proposed
timestamp: 2026-08-03T12:05:00Z
---

<!-- OKF frontmatter above carries the tracker metadata (`status`: open | in-progress |
     closed | superseded) that previously lived only in the directory index. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## next reports 0001 for issue and bdr regardless of existing records

Found while standardizing a batch of issue records in this same session: `living-docs next
issue` and `living-docs next bdr` both report `0001` in a bundle that already holds 19
issues and several bdrs, while `living-docs next adr` correctly reports the next free number
(`0029` in a bundle with ADRs through 0028). Reproduced against both the installed release
binary and a freshly rebuilt workspace binary, and against a scratch copy of `docs/` — not a
stale-build artifact.

Note this is a display-only bug so far: `living-docs new issue "<title>"` itself numbers the
new record correctly (it picked `0020` in a bundle already holding 0001–0019), so `next`'s
scan and `new`'s own numbering step disagree — `next` is the one that's wrong.

### Scope

Included: fixing `next`'s scan for the `issue` and `bdr` doc types to match `adr`'s
(evidently correct) behavior.

Explicitly out: any change to `new`'s numbering, which is already correct.

### Acceptance

- `living-docs next issue` and `living-docs next bdr` report one past the highest existing
  number for their type, matching `living-docs next adr`'s already-correct behavior.
- A regression test pins this for at least `issue` and `bdr` against a fixture bundle with
  pre-existing records of each type.

### Plan

Locate wherever `next`'s per-type scan diverges from `new`'s own numbering step (which is
correct) and align it — likely a doc-type-registry lookup gap the same shape as ADR 0026
was written to close.
