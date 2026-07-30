---
type: Research
title: <Short question or investigation title>
description: <One sentence — the question this record answers.>
status: Draft
timestamp: <ISO 8601 datetime>
---

# NNNN. <Short question or investigation title>

<!-- Status lives in frontmatter (`status`: Draft | Accepted | Superseded), not a body
     line. A research record is evidence, not a decision: it is superseded when
     re-investigation invalidates it, never "implemented". `living-docs supersede` adds
     `superseded_by` when a later record replaces this one. -->

## Question

<The single question this record answers, stated so that an answer could be wrong. If a
pending decision motivated the investigation, link it bundle-relative:
[ADR](/adr/NNNN-<slug>.md), [PRD](/prd/NNNN-<slug>.md).>

## Method

<How the question was investigated, in enough detail for someone else to repeat it: what
was read or measured, against which version or commit, with which tool, over what sample.
State the date the evidence was gathered — the findings below are true as of that date and
no later.>

## Findings

<What was observed, kept separate from what it means. One row per finding, each traceable
to the method above and to a reference below. Record contradicting evidence too — a
research record that only confirms its own premise has not investigated anything.>

| Finding | Evidence | Confidence |
|---|---|---|
| <what was observed> | <reference [1] / measurement / file:line> | high / medium / low |

## Implications

<What follows for this project, and what does not. Name the decisions this evidence
supports or rules out; do not make those decisions here — that is an ADR's job. Evidence
and decision stay separate records.>

## Open Questions

<What this investigation did not settle, so the next reader knows where the edge is.>

# References

<!-- OKF §8. Each entry per `rules/citation-conventions.md`: ABNT NBR 6023 structure,
     always carrying the link. -->
[1] [<source>](<url>). Available at: <url>. Accessed on: <date>.
