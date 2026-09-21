---
type: Issue
title: set accepts title so a Proposed record can be retitled without hand-editing frontmatter
description: "A record that is still Proposed has no CLI path to correct its title: set covers only status, description and owner, and the rule forbids hand-editing frontmatter, so the only options are supersede or delete and recreate."
owner: Evaldo Klock
status: open
timestamp: 2026-09-21T16:55:12Z
---

## 0047. set accepts title so a Proposed record can be retitled without hand-editing frontmatter

While reviewing a fresh bundle, a `Proposed` ADR was born with a wrong count in its title (three hooks; the decision settled on two). `living-docs set` accepts `status`, `description` and `owner`, and the CLI-first rule forbids hand-editing frontmatter, so the record had to be deleted and recreated through `new`, which also rewrote its filename and timestamp. The procedure topic already names the remedy: a deterministic frontmatter mutation done by hand becomes a `set` key.

### Scope

- `set <ref> title <value>` on any record type: rewrites the frontmatter `title`, the numbered heading line, and renames the file to the new slug while keeping the number.
- Allowed only while `status` is `Proposed` or `Draft`; on an accepted record it fails with the pointer to `supersede`, because an accepted title is history.
- `index` regenerates the row on the next run; `check` reports a heading that no longer matches the title.

### Decision

Renaming the file is part of the verb, chosen over keeping the old slug because the slug is derived from the title and a mismatch is what `check` would otherwise have to tolerate.

### Acceptance

- `set adr/0011 title "..."` on a Proposed record rewrites frontmatter, heading and filename, and `check` passes after `index`.
- The same call on an Accepted record exits 1 and names `supersede`.
- A title that produces an existing slug is refused.
