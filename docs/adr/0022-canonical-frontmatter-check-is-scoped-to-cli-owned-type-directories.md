---
type: ADR
title: Canonical frontmatter check is scoped to CLI-owned type directories
description: Apply the ADR 0020 category fix to the ADR 0019 canonical round-trip check — it flags non-canonical frontmatter only for records directly inside the four CLI-owned type directories, leaving hand-authored docs (research, bundle-root notes) free-form.
status: Accepted
tags: [check, cli, enforcement, frontmatter]
timestamp: 2026-07-29T01:57:27Z
---

# 0022. Canonical frontmatter check is scoped to CLI-owned type directories

## Context

The ADR 0019 canonical round-trip check (`check_canonical_frontmatter`) flags any record
whose frontmatter block deviates from its canonical re-serialization, naming
`living-docs fmt` as remediation. As landed, it runs over **every** non-reserved record
in the bundle. Dogfooding surfaced the mis-fire the moment it met the repo's own
hostile fixtures: `04-frontmatter-quoted-commented` and `06-block-scalar-ok` exist to
prove that *valid but non-canonical* YAML (quoted values, inline comments, block
scalars) is read correctly — and the bundle-wide canonical check now fails them,
breaking `make check` on a clean tree.

This is the same category error ADR 0020 corrected for the write-time hook: "inside
the bundle" is not "CLI-owned". Canonical form is a property of CLI *output*; the CLI
scaffolds only the four type directories (`paths::doc_type_for_dir` is the source of
truth). Hand-authored docs — `research` records, bundle-root notes — never came from a
CLI verb, so demanding they byte-match one is unactionable in the ADR 0020 sense, even
if `fmt` could technically rewrite them.

## Decision

We will scope `check_canonical_frontmatter` to records **directly inside a CLI-owned
type directory** — a record whose parent directory name resolves through
`paths::doc_type_for_dir` (`adr`, `bdr`, `prd`, `issues`). Every other record with a
frontmatter block is skipped by this check; the ordinary frontmatter invariants
(non-empty `type`, supersede symmetry, indexing) continue to cover it. This mirrors,
in the check layer, exactly the scope ADR 0020 gave the write-time hook, and amends
ADR 0020's statement that the round-trip check was "unchanged".

## Consequences

**Easier / gained:**
- `make check` is green again on a clean tree: fixtures 04/06 pass because their docs
  sit at the bundle root, outside any CLI-owned directory.
- The check, the write-time hook, and `paths` now agree on one definition of
  CLI-owned, keyed off the same mapping.

**Harder / accepted trade-offs:**
- A hand-authored record outside the four directories can carry non-canonical
  frontmatter indefinitely. Accepted: that is the definition of hand-authored; there
  is no CLI verb whose output it should match.

**Follow-ups:**
- None.

## Verification

**Implementation impact:** `living-docs-core/src/check/canonical.rs` (scope guard +
unit tests).

**Verification criteria:**
- A non-canonical record under `adr/`, `bdr/`, `prd/`, or `issues/` is flagged; the
  same content under `research/` or the bundle root is not.
- Fitness function: `make test-fixtures` passes 04/06 again, and the canonical.rs unit
  tests pin both sides of the scope.
