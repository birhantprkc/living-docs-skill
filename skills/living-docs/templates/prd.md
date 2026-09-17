---
type: PRD
title: <Feature / capability name>
description: <One sentence — what capability this specifies.>
status: Draft
timestamp: <ISO 8601 datetime>
---

# NNNN. <Feature / capability name>

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly Draft | Accepted | Implemented. `superseded_by` is absent by default;
     `living-docs supersede` sets Superseded on this record -- never by hand --
     when a later PRD replaces it. A Superseded or Deprecated record also carries
     a callout above this heading naming its successor -- written by `supersede`,
     `set`, or `fmt`, never by hand. -->

## Problem / Motivation

<!-- Lead with the problem, not the solution. If you can only state it as a solution,
     grill it first to find the underlying need. -->

{{PROBLEM}}

## Goals

<!-- What success looks like, not a task. -->

- {{GOAL}}

## Non-goals

<!-- Name the tempting-but-excluded things. -->

- {{NON_GOAL}}

## Requirements

<!-- Testable statements of what the system must do. Each must be falsifiable — a condition
     you could write a test for — not "should be fast". A quality requirement (performance,
     availability, scale, security) states its measure and how it is verified (a load test, a
     CI floor, a security check); one without a way to verify it is a vibe. -->

- {{REQUIREMENT}}

## Acceptance criteria

<!-- An observable condition proving a requirement is met. -->

- {{ACCEPTANCE_CRITERION}}

## Success metrics

<!-- A quantified outcome that confirms the problem is solved after delivery -- not task
     completion, e.g. "Checkout abandonment rate drops by ≥10% within 30 days of
     launch." -->

- {{SUCCESS_METRIC}}

## Open questions

<!-- Each headed toward an ADR when its resolution is a decision expensive to reverse;
     a cheap resolution goes in the issue that carries the work. -->

- {{OPEN_QUESTION}}

## Related

- Constitution: [/constitution.md](/constitution.md)
- Issues: [/issues/NNNN-<slug>.md](/issues/NNNN-<slug>.md)
- Research: [/research/NNNN-<slug>.md](/research/NNNN-<slug>.md)
