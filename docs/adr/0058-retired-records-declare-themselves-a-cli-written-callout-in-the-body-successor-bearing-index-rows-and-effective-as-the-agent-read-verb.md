---
type: ADR
title: "Retired records declare themselves: a CLI-written callout in the body, successor-bearing index rows, and effective as the agent read verb"
description: A Superseded or Deprecated record opens with a CLI-written imperative callout naming its successor or the absence of one, index rows carry the successor, effective reports what it withheld, and check enforces the callout so an agent that opens a raw record cannot miss that it is history.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-09-17T14:49:14Z
---

# 0058. Retired records declare themselves: a CLI-written callout in the body, successor-bearing index rows, and effective as the agent read verb

## Context

The corpus is append-only. A retired decision keeps its file, its number, and its body; only the frontmatter changes (`status: Superseded` + `superseded_by`, or `status: Deprecated`). This repo already has 17 retired ADRs next to 41 active ones, and a consumer corpus grows the same way.

The retirement signal lives where a machine reads it and where an agent does not. `living-docs effective` withholds retired records and collapses supersede chains, and the skill rule says "read `effective`, never `index.md`". But an agent that opens a record file directly — from a grep hit, a link in another record, or a `Read` on a path — skims the YAML block and lands on the title and the Decision section. Nothing in the visible body says "this is history". The agent then plans on a dead decision. The word in the frontmatter is not the problem; its position and its neutral tone are.

Three surfaces compound the gap: the `index.md` retired row reads `- Superseded` without naming the successor, so a reader who finds the row still has to open the file to learn where to go; `effective` omits retired records silently, so a reader cannot tell that something was withheld; and the always-loaded rule block teaches the verbs but never says what to do on contact with a retired record.

One more ambiguity: ADR 0057 marked ADR 0050 (`effective`) as `Deprecated` while keeping the simplified verb in the binary and in the skill rules. The verb is the answer to this problem, so its standing must be explicit.

Renaming `Deprecated` to `Archived` was considered and rejected: the word would stay in the same invisible place, `Superseded` carries the successor link that `Archived` loses, and ADR 0057 defers status-vocabulary changes. Moving retired records to a `superseded/` directory was rejected: numbers and paths are permanent, links would break, and the callout gives the same effect without moving files.

## Decision

We will make a retired record declare itself in its own body, in the first line an agent reads, and make every read surface carry the same fact.

1. **A CLI-written callout is the first non-blank line of a retired record's body**, placed above the H1. Its text is fixed and imperative. For `Superseded`: `> **SUPERSEDED — do not act on this record.** Replaced by [NNNN](<successor filename>). Run living-docs effective for what is in force.` For `Deprecated`: `> **DEPRECATED — do not act on this record.** It has no successor. Run living-docs effective for what is in force.` The successor link is a sibling-relative link to the record `superseded_by` names, so the existing link check validates it.
2. **The callout is CLI-owned, never hand-written.** `living-docs supersede` writes it on the old record. `living-docs set <ref> status <value>` reconciles it: `Deprecated` adds it, any active value removes it. `living-docs fmt` reconciles it on every record — the remediation path for an existing corpus and the only body mutation `fmt` performs beside frontmatter canonicalization. The reconcile logic has one home in `living-docs-core`; the db-mode write path reaches it through the same core `supersede` call the CLI uses.
3. **`living-docs check` enforces the callout as an invariant.** A `Superseded` or `Deprecated` record whose body does not open with its exact expected callout is a violation; an active record whose body opens with a retired callout is a violation. `fmt` is the named remediation.
4. **`index.md` retired rows name the successor.** A superseded row renders `- Superseded by [NNNN](<filename>)`; a deprecated row renders `- Deprecated (no successor)`. The retired section opens with one fixed note: ``_History only. Do not act on these records — run `living-docs effective` for what is in force._`` The `## Active` / `## Superseded` heading names do not change.
5. **`effective` reports what it withheld.** When at least one retired record exists, the output opens with one line: `_Withheld N retired record(s) (superseded or deprecated): history only, never act on them._` With none, the output is unchanged.
6. **The skill corpus teaches the stop at the point of contact.** The always-loaded rule block (`templates/claude-hard-rules.md` §10), `SKILL.md`, `adr-conventions`, `semantic-index`, `procedure`, and the `check` checklist all state one rule: a body that opens with a `SUPERSEDED` or `DEPRECATED` callout is history — follow the successor link or discard the record; never plan on it; read `living-docs effective` for what is in force.
7. **`effective` is the agent read verb, in force.** This ADR re-anchors the simplified verb ADR 0057 kept; ADR 0050 stays `Deprecated` as the record of the tiered/budgeted design that was cut.

## Consequences

**Easier / gained:**
- An agent cannot open a retired record without reading, in the first visible line, that it is history and where the live decision is. The signal no longer depends on the agent knowing a convention.
- The same fact renders identically in the body, the index row, and the `effective` header, from one status field: no second source of truth.
- The invariant is a gate, not prose: a hand-retired record without the callout fails `check` in the session that produced it.

**Harder / accepted trade-offs:**
- Every retired record in every existing corpus changes once when `fmt` runs. This repo's 17 retired ADRs and the `examples/linkly` fixture take that one-time diff. Consumer corpora see `check` fail until they run `fmt`; that is the intended forcing function.
- `fmt` now mutates one body line. The "body is byte-for-byte untouched" promise narrows to "body below the callout is untouched".
- The callout text is a contract: changing its wording later means another `fmt` sweep across every corpus.

**Follow-ups:**
- `supersede` still expresses only 1:1 replacement (ADR 0057 follow-up). A one-replaces-many callout would need that verb to accept a set first.
- The web record page should render the callout as-is; whether it also wants a distinct visual treatment is a separate, cheap decision for the issue that touches it.

## Verification

**Implementation impact:** `living-docs-core/src/callout.rs` (new: expected text, detection, reconcile), `living-docs-core/src/commands/{supersede,set,fmt,effective,index}.rs`, `living-docs-core/src/check/callout.rs` (new pass), `cli/tests/index_supersede.rs`, `db-store/src/lib.rs` (supersede fixture), `skills/living-docs/{SKILL.md,rules/*,templates/*}`, `docs/adr/*` retired records, `docs/*/index.md`, `examples/linkly/docs`.

**Verification criteria:**
- After `living-docs supersede 0001 0002`, record 0001's body opens with the Superseded callout linking 0002's filename, and `set 0001 status Deprecated` on a fresh record yields the Deprecated callout; `set` back to an active status removes it; `fmt` reconciles a corpus in one run and is a no-op on the second.
- Fitness function: `living-docs check` fails on a retired record without its callout and on an active record with one, and `check docs` over this repo exits 0 after `fmt` + `index`; `cargo test` in the workspace covers each verb's callout behavior and the index/effective rendering.
