---
type: Issue
title: Owner field in record frontmatter, required for ADR and BDR by check
description: add a CLI-owned owner frontmatter field, required on ADR and BDR by check, because unowned decision records drift
status: closed
timestamp: 2026-08-27T19:04:22Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## Owner field in record frontmatter, required for ADR and BDR by check

An artifact with no owner drifts. A decision record with no accountable person forks back into conflicting versions, and an agent propagates the drift at machine speed. Data-as-product discipline: every decision record names who owns it.

### Scope

- Add an optional `owner` frontmatter field to every record type. The value is a free-form identifier (name or email); the tool never validates it against a directory.
- `owner` is CLI-owned like all frontmatter: set at `new` time via a flag (`--owner`) or a config default; a `living-docs owner <NNNN> <value>` verb updates it, mirroring `status`.
- `check` requires `owner` on ADR and BDR records. Other types stay optional.
- Grandfather rule: existing ADR/BDR records without `owner` produce a warn, not an error, until backfilled; the ratchet then flips to error.
- KEPT: frontmatter remains CLI-owned; the hand-write hook keeps blocking manual frontmatter edits.

### Acceptance

- `living-docs new adr "x" --owner alice` emits a record with `owner: alice`.
- `living-docs owner 0007 bob` rewrites only the `owner` field and passes `check`.
- `check` warns on an ADR/BDR without `owner`; after the backfill flag/config flips, it errors.
- `new` without `--owner` and without a config default still succeeds for non-ADR/BDR types.

### Plan

1. Schema + verb in core and CLI (`--owner`, `owner` verb), with tests.
2. `check` rule with the warn-then-error ratchet.
3. Backfill the existing ADR/BDR corpus, flip the ratchet.
