# Code comments must not reference doc artifacts

A docblock or code comment must never name or number a documentation artifact. This covers an ADR, PRD, issue, constitution article, research note, or delivery slice. Remove references such as `see ADR 0046`, `constitution rule 4`, `issue 0028`, or `slice R3b`.

## Why

Doc numbers change. A record is superseded, an issue closes, a slice is renamed between plan rounds. A comment that points at one of these rots the moment the artifact moves, and the code then lies about where its reason lives. The spine of this skill is one home per fact: a decision's home is its doc, never a comment beside the code.

## What to write instead

Keep the reason when it is load-bearing, but state the invariant itself. Never point at the document that recorded it.

- Bad: `// batch size capped at 500 - see ADR 0046`
- Good: `// batch size capped at 500: larger payloads exceed the 6 MB request limit`

The traceability chain (constitution -> PRD -> ADR -> issue -> code) lives in the docs and their indexes, which the doc-gate keeps in sync. Code stays self-explanatory; the docs carry the numbering.

## The ban is enforced by a gate, not a prompt line

Instructions never block; only gates block. Keep the ban honest with a pre-commit or CI step that rejects any diff introducing a doc-artifact citation in code, and let the author state the invariant instead:

```bash
# pre-commit / CI: reject a doc-artifact citation in staged code
git diff --cached -U0 -- '*.rs' '*.ts' '*.py' | grep -nE '^\+.*(ADR|PRD|issue)[ -]?[0-9]{4}' && {
  echo "state the invariant itself; the docs carry the numbering, not the code" >&2
  exit 1
}
```

## Scope

This bans references to the project's own living-docs artifacts and delivery slices. It does not ban an external specification identifier such as an RFC, a W3C standard, or a CVE. Those are stable identifiers the code legitimately implements against.
