---
type: Issue
title: Test suite duplication exceeds the jscpd five percent ratchet; extract shared helpers across db-store and command tests
description: Repo-wide test-suite duplication is about 5.14 percent, above the jscpd 5 percent ratchet; extract shared test scaffolding into common helpers to drop below the threshold without changing behavior.
status: open
timestamp: 2026-09-07T18:13:59Z
---

## Test suite duplication exceeds the jscpd five percent ratchet

The `jscpd` quality gate counts duplicated lines across the whole repository. The working tree reports about 5.14 percent duplicated lines, above the 5.0 percent threshold. The duplication is spread across the test suite, not one file. It predates this record and shipped in v0.15.0. It surfaced during the delivery of [issue 0041](/issues/0041-fmt-rewrites-record-bodies-reference-lists-collapse-to-one-line-and-supersede-emits-non-canonical-frontmatter-that-sends-users-to-fmt.md), whose own change reduces duplication yet cannot pull the repository under the threshold.

Top contributors, by duplicated line count:

- `db-store/tests/*.rs`: `parity.rs`, `schema.rs`, `sync_meta.rs`, `authoring.rs`, `check_parity.rs`, `parity_owner.rs`.
- `living-docs-core` check and command test modules.
- Several `cli/tests/*.rs` integration files.

Each module repeats the same scaffolding: a temp bundle setup, a seed of one or two records, and an assert-success wrapper.

### Scope

Extract the repeated test scaffolding into a shared helper module per test crate, then replace each copy with a call. This is a mechanical de-duplication.

KEEP every test's observable assertions and coverage. Do not weaken, skip, or delete any test.

### Acceptance

- `se-core qg jscpd` reports duplicated lines below the 5.0 percent threshold on the working tree.
- `cargo test --workspace` stays green with no loss of coverage.

### Plan

- Inventory the exact clone pairs from a `jscpd` JSON report.
- Add or extend a shared `common` helper module in each affected test crate.
- Replace each duplicated block with a helper call, one crate at a time.
