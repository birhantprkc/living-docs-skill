---
type: Issue
title: the okf skill version tracks the vendored spec version, not the repo release
description: exempt skills/okf-knowledge-format from the repo VERSION sync; its version field mirrors the vendored reference/SPEC.md version and check-version.sh gates that pairing instead
owner: Evaldo Klock
status: closed
timestamp: 2026-08-27T23:39:50Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## The OKF skill version tracks the vendored spec version, not the repo release

The `skills/okf-knowledge-format/SKILL.md` frontmatter declares `version: 0.13.0` because `check-version.sh` forces every SKILL.md to match the repo `VERSION`. That coupling is wrong for OKF: the skill wraps a vendored upstream spec (`reference/SPEC.md`, from Google Cloud Platform), and that spec carries its own version ("Version 0.1 — Draft"). Every repo release therefore mislabels the OKF spec version. The skill's `version` must mirror the vendored spec version, and the version gate must verify that pairing instead of the repo release pairing.

### Scope

- `skills/okf-knowledge-format/SKILL.md` `version:` becomes the vendored spec version (`0.1`).
- `scripts/check-version.sh` exempts the OKF skill from the repo-VERSION sync and instead gates its `version:` against the version declared in `reference/SPEC.md`.
- The release workflow tag-verification step honors the same exemption.
- KEPT: every other SKILL.md, `cli/Cargo.toml`, `plugin.json`, and instruction files stay repo-VERSION synced; `scripts/update-spec.sh` remains the refresh path.

### Acceptance

- `./scripts/check-version.sh` passes with OKF at `version: 0.1` and every other declaration site at the repo `VERSION`.
- Editing the OKF `version:` to a value that disagrees with `reference/SPEC.md` makes `./scripts/check-version.sh` fail, naming the spec pairing.
- The `scripts/tests/check-version/` fixtures cover both the pass and the mismatch cases.

### Plan

Single slice: SKILL.md version flip + check-version.sh exemption + fixture coverage + release-workflow parity.
