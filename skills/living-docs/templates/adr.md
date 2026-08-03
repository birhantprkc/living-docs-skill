---
type: ADR
title: <Short decision title>
description: <One sentence — the decision and its scope.>
status: Proposed
timestamp: <ISO 8601 datetime, e.g. 2026-06-13T00:00:00Z>
---

# NNNN. <Short decision title>

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly Proposed | Accepted | Deprecated. When superseding a prior ADR, set
     `supersedes` here; `living-docs supersede` sets Superseded on the old record
     -- never set it by hand. -->

## Context

<!-- The forces at play. What problem forced a decision? What constraints bound it?
     Written so a newcomer understands the pressure without prior knowledge. Link any
     research artifact or PRD/issue that motivates it, bundle-relative:
     [research](/research/NNNN-<slug>.md), [PRD](/prd/NNNN-<slug>.md). -->

{{CONTEXT}}

## Decision

<!-- State the choice in active voice -- specific and testable. -->

We will {{DECISION}}.

## Consequences

**Easier / gained:**
- {{GAINED}}

**Harder / accepted trade-offs:**
- {{COST}}

**Follow-ups:**
- {{FOLLOW_UP}}

## Verification

<!-- OPTIONAL — include when this decision must be honored in code, so the doc closes the
     doc → implement → verify loop an agent (and any review step) can consume. Omit
     for a purely advisory record. Keep criteria checkable, not aspirational.
     Implementation impact: files / modules this decision touches, e.g. `src/store.py`.
     Fitness function: the test / lint / arch-unit assertion that fails if the second
     verification criterion is violated (see `rules/adr-conventions.md` rule 6). -->

**Implementation impact:** {{IMPLEMENTATION_IMPACT}}

**Verification criteria:**
- {{VERIFICATION_CRITERION}}
- {{FITNESS_FUNCTION}}

# References

<!-- Optional (OKF §8). External sources backing claims in Context. -->
[1] [{{SOURCE}}]({{URL}})
