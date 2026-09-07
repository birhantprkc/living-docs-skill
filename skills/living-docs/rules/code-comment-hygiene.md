# Code comments must not reference doc artifacts

A docblock or code comment must never name or number a documentation artifact. This covers an ADR, PRD, BDR, issue, constitution article, research note, or delivery slice. Remove references such as `see ADR 0046`, `per BDR 0012`, `constitution rule 4`, `issue 0028`, or `slice R3b`.

## Why

Doc numbers change. A record is superseded, an issue closes, a slice is renamed between plan rounds. A comment that points at one of these rots the moment the artifact moves, and the code then lies about where its reason lives. The spine of this skill is one home per fact: a decision's home is its doc, never a comment beside the code.

## What to write instead

Keep the reason when it is load-bearing, but state the invariant itself. Never point at the document that recorded it.

- Bad: `// batch size capped at 500 - see ADR 0046`
- Good: `// batch size capped at 500: larger payloads exceed the 6 MB request limit`

The traceability chain (constitution -> PRD -> ADR/BDR -> issue -> code) lives in the docs and their indexes, which the doc-gate keeps in sync. Code stays self-explanatory; the docs carry the numbering.

## Scope

This bans references to the project's own living-docs artifacts and delivery slices. It does not ban an external specification identifier such as an RFC, a W3C standard, or a CVE. Those are stable identifiers the code legitimately implements against.
