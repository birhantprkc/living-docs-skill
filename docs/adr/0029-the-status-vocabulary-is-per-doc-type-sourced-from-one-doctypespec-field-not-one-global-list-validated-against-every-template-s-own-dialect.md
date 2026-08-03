---
type: ADR
title: The status vocabulary is per doc type, sourced from one DocTypeSpec field -- not one global list validated against every template's own dialect
description: Each doc type's status vocabulary is one DocTypeSpec field that the validator, the template comment, and new's initial seed all read from — resolving the three-way mismatch issue.md and issue 0017 both hit.
status: Accepted
timestamp: 2026-08-03T12:12:00Z
---

# 0029. The status vocabulary is per doc type, sourced from one DocTypeSpec field

<!-- Status lives in frontmatter (`status`), not a body line. When superseding a
     prior ADR, set `supersedes` here and `superseded_by` on the old record.
     # Proposed | Accepted | Superseded | Deprecated -->

## Context

Four independent places each carry their own idea of the status vocabulary, and they
disagree.

`living-docs status <NNNN> <Status>` (`living-docs-core/src/commands/status.rs`) validates
every record — regardless of type — against one hardcoded list: `Proposed | Accepted |
Deprecated` (`Superseded` is rejected with a hint toward `supersede`). But every generated
record's body carries an HTML comment stating what that type's vocabulary *actually* is, and
none of the five numbered types (`status` addresses a record by `NNNN`, so the singleton
`Constitution` — no `NNNN` — is out of scope for this decision regardless) agree with the
validator or with each other:

- ADR: `Proposed | Accepted | Superseded | Deprecated` — close to the validator, but still
  wrong (includes `Superseded`, which the validator rejects).
- BDR / PRD: `Draft | Accepted | Implemented | Superseded` — a different lifecycle entirely.
- Issue: `open | in-progress | closed | superseded` — tracker language, not decision-record
  language.
- Research: `Draft | Accepted | Superseded` — its own comment explains why it has no
  `Implemented`: "a research record is evidence, not a decision."

Meanwhile `living-docs-core/src/commands/index.rs`'s partition predicates were written
*against the templates' vocabularies, not the validator's*: `is_active_status` already
treats `Draft`/`Implemented`/`Proposed` as active (matching BDR/PRD's template), and
`is_open_status` already treats `open`/`in-progress` as open and `closed`/`done` as closed
—case-insensitively, with a comment noting "the repo's real tracker uses `done` as its
closed value" (this repo's own `docs/issues/0001`–`0009` carry `status: done` today). `new`'s
`fill_frontmatter` seeds every new record with `status: Proposed`, including issues, which
doesn't match the issue template's own `open`-first vocabulary either.

So the validator is the outlier. It was written once, shaped like ADR's lifecycle, and
applied globally — while the template comments, the index predicates, and the real record
corpus all independently converged on each type having its *own* lifecycle. A user who
copies a template's own comment (exactly what the comment invites them to do) gets rejected
with exit 2, and is nudged toward hand-editing frontmatter — precisely what the CLI-ownership
contract forbids (issue.md, filed as issue 0020). Separately, issues have no terminal
`Closed`/`Resolved` state distinct from `Deprecated` (issue 0017's open half) — `is_open_status`
already supports one (`closed`/`done`), but the validator refuses to let `status` set it.

This is the same shape ADR 0026 already generalized once: a taxonomy that was N hand-synced
enumerations became one `DocTypeSpec` registry row per type. Status vocabulary is the Nth
hand-synced enumeration that ADR 0026 didn't yet cover.

## Decision

We will add `status_vocabulary: &'static [&'static str]` to `DocTypeSpec` — the ordered list
of values that type's `status` verb may set directly, index `0` being the value `new` seeds
a fresh record with. `Superseded` is never a member of any type's list; it stays a
cross-cutting special case rejected by `status` with today's hint toward `supersede`, for
every type. `Constitution` (a `Identity::Singleton`, unreachable by `status <NNNN>`) carries
no `status_vocabulary` obligation — its own `Draft | Ratified | Amended` comment is out of
scope here and stays whatever it already is.

Per-type values, extracted from what each type's own template comment and `index.rs`'s
predicates already independently expect — this decision canonicizes an intent that already
existed in five different, disagreeing places, rather than inventing a new one (changes zero
read-side behavior, only what `status` is willing to write):

- ADR: `["Proposed", "Accepted", "Deprecated"]` (unchanged).
- BDR: `["Draft", "Accepted", "Implemented"]` (newly enforced; unchanged from the template).
- PRD: `["Draft", "Accepted", "Implemented"]` (newly enforced; unchanged from the template).
- Research: `["Draft", "Accepted"]` (newly enforced; unchanged from the template — no
  `Implemented`, per the template's own rationale that a research record is evidence).
- Issue: `["open", "in-progress", "closed"]` (lowercase, tracker-cased, matching the
  template and `is_open_status`'s existing test cases). `closed` is the terminal state issue
  0017 asked for. `done` remains a recognized *read-side* synonym for `closed` in
  `is_open_status` — unchanged — so the nine existing `status: done` issue records keep
  rendering as closed without a migration.

`validate_status` resolves the record's type (via `find_record` reading its `type:`
frontmatter, `doc_type::spec_for_frontmatter`) and checks membership in that type's
`status_vocabulary` instead of one global constant. `new`'s `fill_frontmatter` seeds
`status_vocabulary[0]` instead of the literal `"Proposed"`, so a new issue starts `open`
instead of an unsettable-once-this-lands `Proposed`. Each type's template body comment is
hand-aligned to read its own `status_vocabulary` joined by `" | "` plus `superseded`
appended, and a fitness function (mirroring ADR 0026's fitness function A) asserts the
comment text and the registry field agree, so they cannot drift apart silently again.

## Consequences

**Easier / gained:**
- One place decides a type's status vocabulary; the validator, the template comment, and
  `new`'s seed all read from it, so they cannot disagree again — the same guarantee ADR 0026
  gave `token`/`directory`/`frontmatter`/`template`.
- Issues gain a real `in-progress` state and a real terminal `closed` state, both already
  supported read-side by `index.rs` and now finally settable, closing issue 0017 and issue.md
  point 2 in the same change.
- A record created by `new` and immediately validated with its own template's vocabulary
  never produces the exit-2 rejection issue.md was filed over.

**Harder / accepted trade-offs:**
- `status.rs` moves from one hardcoded constant to a per-type lookup with a fallible type
  resolution step; a record whose `type:` frontmatter cannot be resolved needs an explicit,
  tested error path rather than falling through to the old global list.
- BDR, PRD, and Research records gain enforcement of a vocabulary the validator previously
  never checked in practice — any existing record with a status outside its type's set
  (none exist yet in this bundle for BDR/PRD; Research's own records would need checking)
  would need reconciling.
- Every type's template comment is now a claim a fitness function holds it to; adding a
  fifth doc type means adding its `status_vocabulary` row and comment together, not either
  alone.

**Follow-ups:**
- Issue 0020 (comment/validator mismatch, in-progress state) and issue 0017's open half
  (terminal state for issues) are resolved by this decision's implementation.
- Issue 0021 (`--description` flag) and issue 0022 (placeholder format) are independent and
  do not require this ADR.

## Verification

**Implementation impact:** `living-docs-core/src/doc_type.rs`, `living-docs-core/src/commands/status.rs`,
`living-docs-core/src/commands/new.rs`, `skills/living-docs/templates/{adr,bdr,prd,issue,research}.md`.

**Verification criteria:**
- `living-docs status <NNNN> <Status>` accepts exactly each type's own `status_vocabulary`
  and rejects every other type's values with a message naming the valid set for *that*
  record's type.
- `living-docs new issue "<title>"` seeds `status: open`; `new adr`/`bdr`/`prd` seed their
  own first vocabulary entry.
- Fitness function: for every `DocTypeSpec` row, the template's body comment text contains
  exactly that row's `status_vocabulary` (plus `superseded`) — the same shape as ADR 0026's
  fitness function A, extended to the new field.
- `is_open_status`/`is_active_status` in `index.rs` are unchanged and continue passing their
  existing tests — this decision only affects what `status` is willing to write, not how the
  index reads records.
