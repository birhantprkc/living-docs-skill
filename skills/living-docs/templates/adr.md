---
type: ADR
title: <Short decision title>
description: <One sentence — the decision and its scope.>
status: Proposed
timestamp: <ISO 8601 datetime, e.g. 2026-06-13T00:00:00Z>
---

# NNNN. <Short decision title>

## Context

{{CONTEXT: the forces at play and what forced a decision, written for a newcomer; link the research or issue that motivates it; no solution here}}

## Decision

We will {{DECISION}}.

{{REJECTED_ALTERNATIVES: each alternative considered and the reason it lost; an ADR with no rejected alternative belongs in the issue}}

## Consequences

**Easier / gained:**
- {{GAINED}}

**Harder / accepted trade-offs:**
- {{COST}}

**Follow-ups:**
- {{FOLLOW_UP: a follow-up is an issue to open, never a deferred decision}}

## Verification

**Implementation impact:** {{IMPLEMENTATION_IMPACT: the files or modules this decision touches}}

**Verification criteria:**
- {{VERIFICATION_CRITERION: a checkable condition, not an aspiration}}
- {{FITNESS_FUNCTION: the test or lint that fails when the decision is violated; drop the whole section for a purely advisory record}}

# References

[1] [{{SOURCE}}]({{URL}})
