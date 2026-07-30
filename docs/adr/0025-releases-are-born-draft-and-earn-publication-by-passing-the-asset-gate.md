---
type: ADR
title: Releases are born draft and earn publication by passing the asset gate
description: The release workflow creates every release as a draft and the asset gate promotes it only after verifying all ten assets, so an incomplete release is never published even for a moment.
status: Accepted
supersedes: 0024
timestamp: 2026-07-29T18:49:15Z
---

# 0025. Releases are born draft and earn publication by passing the asset gate

## Context

ADR 0024 made a release atomic by adding a `verify-release-assets` gate that asserts all ten
expected assets are present and checksum-valid, demoting the release to a draft when they are
not. It shipped and it works: `v0.8.0` is the first tag in this project's history to carry
binaries, and running the gate by hand against `v0.7.0` still reports all ten as missing.

It leaves one window open. Under 0024 a release is created **published** and is only demoted
*after* the gate notices something wrong. Between `softprops/action-gh-release` creating the
release and the gate finishing, the tag is publicly installable. That window is not
theoretical — the `release-binaries` matrix builds and uploads four targets, so the release is
published and visibly empty for the entire duration of those builds. Anyone running
`install.sh` in that window gets the exact silent source-build fallback 0024 set out to
eliminate.

Demotion is also the weaker half of the mechanism. It is a compensating action: it depends on
the gate running, succeeding at reaching the API, and winning a race against the consumer. A
precondition is stronger than a compensation — if the release is never published until it is
proven complete, there is no window to lose the race in.

0024 recorded this inversion as a follow-up rather than deciding it, because the viability
hinged on an unverified assumption: that `gh` can read and download assets from a *draft*
release. If it cannot, the gate cannot verify what it is meant to gate. That was probed
directly against this repository before writing this record, using a throwaway draft release:

- `gh release view <tag> --json isDraft,assets` — works, reports `isDraft: true`
- `gh release upload <tag> <file>` — works
- `gh release download <tag> --pattern <name>` — works
- `gh release edit <tag> --draft=false` — promotes, and the release becomes reachable at its
  tag URL

All four succeed against a draft, so the inversion is implementable.

## Decision

We will invert the default: **a release is born draft and is published only by the gate.**

**1. Both `softprops/action-gh-release` steps set `draft: true`.** The `release` job creates
the release as a draft, and the `release-binaries` upload step must also declare `draft: true`.
This second one is not redundant: the action creates-or-updates, and its `draft` input defaults
to `false`, so an upload step that omits it would publish the release mid-matrix — reopening
exactly the window this ADR closes.

**2. `verify-release-assets` promotes on success.** After confirming all ten assets are present
and checksum-valid, the gate runs `gh release edit "$tag" --draft=false`. Publication becomes
an *earned* state rather than the starting state.

**3. On failure the gate leaves the release draft and still exits non-zero.** It issues
`gh release edit "$tag" --draft` as an idempotent safety net in case something published the
release out of band, then fails. The failure mode is now "the release stays invisible", which
is safe by construction — the opposite of 0024, where the failure mode was "the release stays
published until demotion succeeds".

**4. `--no-demote` becomes `--verify-only`.** The flag's meaning under 0024 was "do not demote";
under this ADR the script can both promote and demote, so the honest name is "verify and mutate
nothing". This preserves the auditing use case — running the gate against a historical tag to
see what it actually shipped — without touching that release.

## Consequences

**Easier / gained:**
- The window in which an incomplete release is publicly installable is closed entirely, rather
  than narrowed.
- The failure mode becomes fail-safe: a broken build yields an invisible release, requiring no
  compensating action to succeed.
- A failed release is recovered by fixing the build and re-running the workflow, which promotes
  the same draft — no tag deletion, no version burn.

**Harder / accepted trade-offs:**
- A release is invisible until the whole matrix plus the gate completes, so `gh release list`
  shows nothing for several minutes after the tag is pushed. Accepted: absence is a truthful
  signal, whereas a published-but-empty release is a lie.
- Publication now depends on the gate *succeeding*. A gate bug or a GitHub API outage leaves a
  perfectly good release unpublished. Accepted, and strictly preferable to the inverse failure:
  an unnoticed broken release ships to users, while an unpublished good one is loud and
  trivially fixed by re-running the job.
- Two `draft: true` declarations must stay in sync across two jobs. Mitigated by the fitness
  function below, which fails the build if either is missing.

**Follow-ups:**
- None. This closes the release-integrity thread opened by ADR 0024.

## Verification

**Implementation impact:** `.github/workflows/release.yml` (both `action-gh-release` steps plus
the gate job), `scripts/verify-release-assets.sh` (promote path, flag rename),
`scripts/tests/verify-release-assets/run.sh` (new cases).

**Verification criteria:**
- With all ten assets present and valid the script exits 0 and issues
  `gh release edit <tag> --draft=false`; the resulting release reports `isDraft: false`.
- With any asset missing or mismatched the script exits 1, issues `gh release edit <tag> --draft`,
  and never issues `--draft=false`.
- `--verify-only` exits with the same code in both cases while issuing no `gh release edit` at all.
- Fitness function: a fixture case asserts that both `action-gh-release` steps in
  `.github/workflows/release.yml` declare `draft: true`, so removing either one fails
  `make check` rather than silently reopening the publication window.
