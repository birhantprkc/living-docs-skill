---
type: Issue
title: <Issue title>
description: <One sentence — the change and its motivation.>
status: open
timestamp: <ISO 8601 datetime>
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## <Issue title>

{{SUMMARY}}

If it implements a PRD or ADR, link it bundle-relative: "Implements [ADR NNNN](/adr/NNNN-<slug>.md)" / "Part of [PRD NNNN](/prd/NNNN-<slug>.md)".

### Scope

<!-- For removals/refactors, state what is explicitly KEPT. -->

{{SCOPE}}

### Acceptance

<!-- An observable, testable condition. -->

- {{ACCEPTANCE_CRITERION}}

### Plan

<!-- For a large task, list the slices. -->

{{PLAN}}
