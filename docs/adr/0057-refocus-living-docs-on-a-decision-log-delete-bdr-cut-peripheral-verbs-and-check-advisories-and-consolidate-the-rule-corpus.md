---
type: ADR
title: "Refocus living-docs on a decision log: delete BDR, cut peripheral verbs and check advisories, and consolidate the rule corpus"
description: "Cut living-docs back to a decision log with a gate: fold status/describe/owner into set and remove brief/next/why/scorecard/seal, delete the BDR type, drop the LEAK/DIAGRAM/DUPLICATE/word-budget/traceability/seal check passes while keeping stale-proposed and promoting unfilled placeholders to violations, and consolidate the 22-file rule corpus to ~6."
owner: Evaldo Klock
status: Accepted
timestamp: 2026-09-11T15:57:55Z
---

# 0057. Refocus living-docs on a decision log

## Context

The tool's job is to register and trace engineering decisions without confusing agents or humans. Its own corpus shows it is doing the opposite: 56 ADRs, **zero BDRs**, one PRD, a `Draft` constitution untouched since July. Consumer corpora show the failure inverted — one that ships an `ADR (how) + BDR (behavior)` pair per change, because that is exactly what the rules instruct.

Three defects compound:

- **The always-loaded rules teach one-record-per-layer.** `templates/claude-hard-rules.md` — the block a consumer pastes into `CLAUDE.md`, so it is in every context — still reads `constitution → PRD → ADR (how) + BDR (behavior) → issues → code` and "a PR that changes behavior without a BDR is incomplete". Every change has a structural and a behavioral face, so a compliant agent writes both. ADR 0052 added the materiality criterion but never edited this file.
- **The taxonomy assigns the same word to several records.** ADR is "why" and "how" and "how and why"; PRD is "what and why"; BDR is "what". Two types claim "what", three claim "why/how". An agent cannot get a stable answer to "is this a what or a how?", so it writes both. "Contract" and "how do we know it's done" each carry three or four distinct meanings across the rules.
- **Heavy templates + a hard stop + no cheap "no".** Under a default strict mode with refusal triggers phrased as hard stops, the cheapest compliant move is to write the record. ADR 0055 diagnosed this ("a hard stop with no instrument over-produces") but answered by adding instruments, not removing the stop.

ADRs 0049–0056 (all landed today) layered a second vocabulary — liveness, contract/narrative, materiality signals, inflation, consumption grades, LEAK/DIAGRAM/DUPLICATE, FTS5-ranked `effective` — on top of the confused first layer. They are a well-intentioned response that added surface. The irreducible job is a **decision log with a gate**.

## Decision

We will cut living-docs back to a decision log with a gate, in four moves.

**Verb surface (21 → ~10 top-level).** Keep `new`, `set`, `supersede`, `index`, `check`, `fmt`, `migrate`, `search`, and the `skill`/`hooks` groups. Fold `status`, `describe`, and `owner` into one `set <ref> <key> <value>`. Remove `brief`, `next`, `why`, `scorecard`, and `seal` (its friction is already covered by the write-gate hook and the pre-commit gate). Leave the db/web/search layer otherwise untouched.

**Record types (7 → 4, +1 optional).** Delete the **BDR** type: the name "Behavior *Decision* Record" is itself the leak, its scenarios' home is the test suite, and no consumer corpus is served by making a behavior change require one. Keep ADR (a decision expensive to reverse, with its rejected alternatives), Issue (the diff; carries a cheap decision inline), Research (dated external evidence), Constitution (the singleton). PRD stays optional, without the FR-N/NFR-N traceability coupling (ADR 0035). The `View` type stays a type but leaves the doc-trail — a view is updated in place, never a record in the trail.

**`check` advisories (many → few).** Remove the LEAK, DIAGRAM, DUPLICATE, word-budget, requirement-traceability, and provenance-seal passes, and the contract/narrative and stale-impact liveness classes. Keep the `stale-proposed` liveness advisory (it has a real oracle and caught a true positive here). **Promote unfilled `{{PLACEHOLDER}}` from an advisory to a violation** — it is a mechanical defect, not a matter of taste. The materiality *criterion* survives as the single authoring rule; its *signals* (the inflation/word-budget instrumentation) do not.

**Rule corpus (22 files → ~6).** Consolidate into `spine`, `records`, `procedure`, `check`, `format`, `adoption`, and rewrite `claude-hard-rules.md` around materiality. The one rule an agent needs: *write an ADR when a future reader would pay to rediscover this choice; otherwise put the choice in the issue; write only the body; run `check`.*

This ADR deprecates 0008, 0035, 0045, and 0049–0056 (marked `Deprecated`, not `Superseded`: `supersede` is 1:1 and cannot express one-replaces-many honestly — see follow-ups).

## Consequences

**Easier / gained:**
- One stable answer to "what kind of record is this?", and a per-authoring-session rule load dropping from ~4–6k tokens to under 2k. The tool stops manufacturing the ADR:BDR pairing it was measuring as inflation.

**Harder / accepted trade-offs:**
- Downstream corpora with BDR records must migrate: once the type is deleted, `check` rejects the `BDR` frontmatter value like any unknown type. No grace window is offered here. This churn was accepted deliberately.
- Reverting nine same-day ADRs is itself an expensive-to-reverse decision — which is why it earns this ADR. The walk-back validates the core (ADR + the gate) while pruning the periphery.

**Follow-ups:**
- The tool cannot express one-ADR-supersedes-many (`supersedes` is a single scalar). Deprecation is the honest stand-in here; whether `supersede` should accept a set is left open.
- Status-vocabulary unification across types and the fate of the db/web/ParadeDB/export layer are deferred, not decided here.

## Verification

**Implementation impact:** `cli/src/args.rs`, `cli/src/args/sub.rs`, `cli/src/commands/` (verb wrappers), `living-docs-core/src/doc_type.rs` (BDR row removed), `living-docs-core/src/check/mod.rs` (pass list), `living-docs-core/src/commands/{set,effective}.rs`, and the deleted modules `check/{leak,semantic}`, most of `check/liveness`, `commands/{why,scorecard}`, `check/traceability`, `check/seal`; `skills/living-docs/rules/*` and `templates/*`.

**Verification criteria:**
- `living-docs --help` lists no `brief`/`next`/`why`/`scorecard`/`seal`/`status`/`describe`/`owner` verb; `set adr/NNNN status Accepted` sets the field; `new bdr` is rejected as an unknown type.
- An unfilled `{{PLACEHOLDER}}` in a record makes `check` exit non-zero; `check docs` over this repo stays green otherwise.
