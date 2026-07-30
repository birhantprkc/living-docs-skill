# Issues

Vertical, demoable slices for the living-docs system (greenfield backlog). Each issue
delivers one user-observable behavior end-to-end and links up the trail
([Constitution](../constitution.md) → [ADRs](../adr/)). Consume them smallest/lowest first,
one slice per fresh context, starting from the skeleton.

## Open

* [0010 — Atlas create — db-mode authoring walking skeleton (mode guard, revision, transactional write+check)](0010-atlas-create-db-mode-authoring-walking-skeleton.md) - open
* [0011 — Atlas edit — optimistic concurrency via a revision precondition](0011-atlas-edit-optimistic-concurrency-via-revision-precondition.md) - open
* [0012 — Atlas supersede — browser parity with the CLI supersede verb](0012-atlas-supersede-browser-parity-with-the-cli-supersede-verb.md) - open
* [0013 — Atlas delete — a new verb with no CLI precedent](0013-atlas-delete-a-new-verb-with-no-cli-precedent.md) - open
* [0014 — --docs-dir is silently accepted but ignored by fmt and check, which operate on the cwd bundle](0014-docs-dir-is-silently-accepted-but-ignored-by-fmt-and-check-which-operate-on-the-cwd-bundle.md) - Proposed
* [0015 — status with a bare record number resolves across type directories, ADRs first](0015-status-with-a-bare-record-number-resolves-across-type-directories-adrs-first.md) - Proposed
* [0016 — check needs a ratchet or changed-files mode so brownfield repos can arm the pre-commit channel](0016-check-needs-a-ratchet-or-changed-files-mode-so-brownfield-repos-can-arm-the-pre-commit-channel.md) - Proposed
* [0017 — bundle vocabulary gaps: no research doc type in index and no terminal closed status for issues](0017-bundle-vocabulary-gaps-no-research-doc-type-in-index-and-no-terminal-closed-status-for-issues.md) - Proposed
* [0018 — Identity cannot express an author-named record in a directory, so glossary and the Context family stay hand-authored](0018-identity-cannot-express-an-author-named-record-in-a-directory-so-glossary-and-the-context-family-stay-hand-authored.md) - Proposed
* [0019 — The harness matrix is six targets where four are the same copy with a different destination](0019-the-harness-matrix-is-six-targets-where-four-are-the-same-copy-with-a-different-destination.md) - Proposed

## Closed

* [0001 — Walking skeleton — Cargo workspace + living-docs-core + fs-store + thin cli](0001-workspace-core-skeleton.md) - done
* [0002 — Findability — db sync builds a SQLite/FTS5 read-model and living-docs search queries it](0002-findability-search.md) - done
* [0003 — Read-only web view — axum search + record page over the read-model](0003-web-read-only.md) - done
* [0004 — ParadeDB (Postgres + BM25) as a selectable db engine alongside SQLite](0004-paradedb-engine.md) - done
* [0005 — projects root + multi-project ingestion and cross-project search](0005-projects-multi-project.md) - done
* [0006 — db-mode authoritative authoring — new/index/supersede/check on db-store, lossless .md export](0006-db-mode-authoring.md) - done
* [0007 — Dev environment — docker-compose (ParadeDB) + Makefile targets over the workspace](0007-docker-compose-dev-env.md) - done
* [0008 — living-docs brief — deterministic pre-filled scaffold that leaves only the judgment slots for the authoring model](0008-brief-scaffold.md) - done
* [0008 — Three-pane web shell with metadata panel and Cmd+K palette](0008-three-pane-web-shell-with-metadata-panel-and-cmd-k-palette.md) - done
* [0009 — Per-type doc size targets — authoring convention in the skill corpus + advisory warning in check, plus the same-change economic rationale](0009-doc-size-targets.md) - done
