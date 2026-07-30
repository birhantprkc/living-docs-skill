---
type: Issue
title: check needs a ratchet or changed-files mode so brownfield repos can arm the pre-commit channel
description: Give check a baseline or changed-files mode so a brownfield bundle can arm the pre-commit channel against new debt while legacy debt is paid down incrementally.
status: Proposed
timestamp: 2026-07-30T18:58:58Z
---

## Problem

The `hooks install` pre-commit channel (ADR 0023) execs a FULL-bundle `living-docs check`. In a brownfield bundle with pre-existing debt, one legacy violation fails every future commit regardless of what the commit touches — so the channel cannot be armed at all until the entire bundle is clean.

## Evidence

Downstream adoption attempt (ai-configs, 2026-07-30, v0.8.0): `living-docs check docs` → FAIL, 180 violations across 177 docs (hand-written frontmatter, orphans, broken links, size advisories). `fmt` did not reduce the residue (145 records rewritten, violations 180 -> 185). The repo adopted the channel DISARMED — artifacts committed, `core.hooksPath` left unset — with arming blocked on this gap (its ADR 0072 / issue 0017).

## Proposal

Any of: (a) `check --baseline <ref>` — fail only on violations not present at the ref (diff-aware ratchet, the quality-gate pattern); (b) `check --changed-files <list>` — scope invariants to the files in the commit; (c) an accepted-debt manifest (`.living-docs/check-baseline`) that `check` subtracts and `fmt`/authoring can shrink but never grow. The pre-commit hook then gates NEW debt while legacy debt is paid down incrementally.
