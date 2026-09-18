---
type: ADR
title: A single DocType registry replaces nine hand-synced enumerations, and research and constitution enter as rows
description: One compile-time DocTypeSpec table becomes the sole enumeration of the doc-type taxonomy, so the nine sites that hand-synced it derive instead, and `research` and `constitution` are added as rows.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-07-29T23:49:51Z
---

# 0026. A single DocType registry replaces nine hand-synced enumerations, and research and constitution enter as rows

## Context

The doc-type taxonomy is four tokens — `adr`, `bdr`, `prd`, `issue` — and it is written out by hand in **nine** places:

| Site | Shape |
|---|---|
| `paths::dir_for` | token → directory |
| `paths::doc_type_for_dir` | directory → token |
| `paths::frontmatter_type_for` | token → frontmatter `type:` value |
| `templates::template_for` | token → `include_str!` template |
| `record::NUMBERED_DOC_TYPES` | which tokens carry number identity |
| `index::SUPPORTED_TYPES` | which tokens `index` regenerates |
| `commands::new::unsupported_type_message` | the list quoted at the user |
| `commands::index::unsupported_type_message` | a byte-identical second copy |
| `web::views::CREATABLE_DOC_TYPES` | which tokens the web create form offers |

Nothing makes those nine agree. `new::plan_at` *asserts* the agreement at runtime with `.expect()` calls chained after `paths::dir_for`, `paths::frontmatter_type_for` and `templates::template_for`, so adding a token to `dir_for` and forgetting `template_for` does not fail to compile and does not return an error — it **panics**, in a release binary, on a user's `new` invocation.

Everywhere else the failure is quieter still. Unknown types degrade to defaults by design: `index::render_body` falls back to a flat listing, `heading_title_for` to the generic `"Index"`, `brief::slots_for` to an empty slot array. Each fallback is individually reasonable. Together they mean a **half-added type looks like a working type** — it creates records, it indexes, it just quietly lacks a partition axis, a heading and judgment slots.

This is not hypothetical. It already happened:

- `skills/living-docs/templates/constitution.md` exists, is fully authored, and states its own semantics in a body comment: *"This file is singular — no NNNN prefix, and it is NOT listed as a concept in any index.md."* It is wired into nothing.
- `templates::template_for` carries a test that **asserts** the gap: `assert_eq!(template_for("constitution"), None)`.
- `paths.rs` has two tests asserting `dir_for("constitution")` and `dir_for("glossary")` are `None`.
- `skills/living-docs/SKILL.md`'s tags already advertise `constitution` and `research` as doc types the skill handles.

So the taxonomy is advertised in the skill manifest, specified in a template, pinned as absent by three tests, and implemented for four of six types. The duplication did not merely risk drift — it produced it, and then the tests froze the drift in place.

Two new types are wanted: `research` (this repo's own ADR template already links to `[research](/research/NNNN-<slug>.md)`, and consuming bundles carry `docs/research/NNNN-slug.md` records the CLI cannot create or index) and `constitution`. Adding them the current way costs eighteen edits across nine sites, with a silent-fallback failure mode on any one omission.

## Decision

We will make one compile-time table the sole enumeration of the taxonomy, have every existing site derive from it, and then add the two types as rows.

**1. A registry module, `living-docs-core/src/doc_type.rs`.** It defines an `Identity` enum (`Numbered { dir }` for `<dir>/NNNN-<slug>.md` records, `Singleton { file }` for a single root-level file), a `DocTypeSpec` struct (`token`, `identity`, `frontmatter`, `template`, `index_heading`, `index_partition`, `web_creatable`), the `DOC_TYPES` table (`ADR, BDR, PRD, ISSUE, RESEARCH, CONSTITUTION`), and lookup functions `spec_for` / `spec_for_dir`. Identity carries the path shape, so an inconsistent type is unrepresentable: a directory is a field of the `Numbered` variant, not of the struct, so a singleton cannot have a stale directory and `dir_for` cannot return one for it. This is the load-bearing choice: the invariant `plan_at` asserts at runtime becomes a property of the type system.

**2. Every one of the nine sites becomes a lookup.** `paths::dir_for`, `doc_type_for_dir` and `frontmatter_type_for` delegate to the registry; `templates::template_for` returns `spec.template`; `record::NUMBERED_DOC_TYPES` and `index::SUPPORTED_TYPES` are derived from `DOC_TYPES`; `web::CREATABLE_DOC_TYPES` filters on `web_creatable`. The public signatures of `paths::*` and `templates::template_for` do not change, so no caller outside these modules is touched.

**3. All four `.expect()` calls are deleted.** The fragile three-line resolution appears **twice**, byte-identically: in `commands::new::plan_at` and in `commands::brief::scaffold_brief`. Each carries its own copy of both panics, so there are four, not two. One `spec_for(token)?` in each site yields the directory, the frontmatter value and the template together. Partial agreement is no longer expressible, so there is nothing left to assert.

`brief.rs`'s per-type `slots_for` and `trail_comment_for` match arms are a different thing and stay: they enumerate *judgment-slot content*, not the taxonomy's identity.

**4. The unsupported-type message is generated from the registry** and the second copy is removed. A stale list at the user is no longer possible.

**5. `research` enters as a `Numbered` row:** directory `research`, frontmatter `Research`, index heading `Research`, flat partition (a research record is a point-in-time audit; it has no Active/Superseded or Open/Closed lifecycle), `web_creatable: true`, and a new `skills/living-docs/templates/research.md`.

**6. `constitution` enters as a `Singleton` row:** file `constitution.md` at the bundle root, frontmatter `Constitution`, `web_creatable: true`. Its existing template is wired with **one correction**: `status: Draft # Draft | Ratified | Amended` carried the status domain as a trailing YAML comment, which no other template does and which a canonical frontmatter round-trip would flag; the domain moves into the HTML comment below the frontmatter, where `adr.md` and `bdr.md` already keep theirs. `living-docs new constitution "<title>"` writes `docs/constitution.md`; a second invocation is refused by the clobber guard `plan_at` already applies (`store.read(&target_path).is_ok()`), so the refusal needs no new code.

**`index` sweeps `Numbered` rows only.** A singleton has no directory and therefore no directory index, so the bare `index` sweep iterates the `Identity::Numbered` tokens rather than every token — otherwise `compute` would resolve no directory for `constitution` and the sweep would exit non-zero. An explicit `index constitution` is an error with its own message: the type is supported, it simply has no index to regenerate, and reusing the unsupported-type message would lie. This narrows fitness function B accordingly: the token set `index` regenerates equals the registry's **Numbered** token set.

**7. `check` honors the singleton contract.** `docs/constitution.md` is CLI-owned for the canonical-frontmatter invariant, and is **exempt from index membership** (invariant 3) — exactly as its template states. Both types are optional: a bundle with no `research/` directory and no `constitution.md` stays conformant, and `index` skips a type whose directory does not exist rather than creating one — a bare `index` run must never break a bundle that previously passed `check`.

**8. The one enumeration the registry cannot reach stays hardcoded, and is pinned by a cross-language fitness test.** `skills/living-docs/hooks/block-docs-handwrite.sh` is bash and matches CLI-owned directories with a literal alternation (`(adr|bdr|prd|issues)`, ADR 0020's scope). It cannot read a Rust `const`, and a PreToolUse gate must stay fast and fail-open rather than shell out to the binary on every write. So the bash copy remains a copy, and a Rust test reads the script, extracts the alternation group, and asserts it equals the registry's `Identity::Numbered` directories. The duplication survives; the *drift* does not.

**Rejected alternatives:**
- `is_bundle_singleton` rejected a `canonicalize` call: it would make a pure predicate touch the filesystem and would break every in-memory-store unit test — a real regression traded for an imaginary one.

## Consequences

**Easier / gained:**
- Adding a doc type is one registry row plus one template file.
- Two runtime panics on taxonomy inconsistency are removed, replaced by a compile-time impossibility.
- Three tests that pinned `constitution` and `glossary` as unsupported are replaced by tests that hold the registry to its contract.
- The error message the user reads can never disagree with what the tool accepts.

**Harder / accepted trade-offs:**
- The registry is compile-time, so a doc type cannot be added by configuration. This is deliberate: templates are `include_str!`-embedded to keep the binary self-contained (ADR 0001), so a configured type could name a template that does not exist — the config would be able to express a broken state that the table cannot.
- `Identity` becoming an enum makes `dir_for` fallible for a reason other than "unknown type": a singleton has no directory. Callers that assumed a directory for every known type must handle that, which is the point.
- Corrected invariant: a registry row the CLI will `brief` must carry slot definitions, enforced by fitness function `brief_output_passes_check_for_every_supported_doc_type` (ADR 0057 later removed `brief`, so the fitness function now applies to `new` only).
- The `scaffold_brief` identity branch this ADR added closed the contradiction that `brief constitution` would have reported the type as unsupported while `new constitution` succeeded.
- Adding the `research` row silently made `docs/research/` CLI-owned; that belongs in the release notes.
- Whether the size advisory becomes a registry field is a separate decision.

## Verification

**Implementation impact:** `living-docs-core/src/doc_type.rs` (new), `living-docs-core/src/paths.rs`, `living-docs-core/src/templates.rs`, `living-docs-core/src/record.rs`, `living-docs-core/src/commands/new.rs`, `living-docs-core/src/commands/brief.rs` (its `scaffold_brief` resolution, plus a `slots_for`/`trail_comment_for`/`context_marker_for` arm per new row), `living-docs-core/src/commands/index.rs`, `living-docs-core/src/check/canonical.rs`, `living-docs-core/src/check/mod.rs`, `living-docs-core/src/check/graph.rs`, `web/src/views.rs`, `living-docs-core/tests/hook_registry_parity.rs` (new), `skills/living-docs/hooks/block-docs-handwrite.sh`, `skills/living-docs/tests/hooks/run.sh`, `skills/living-docs/templates/research.md` (new), `skills/living-docs/templates/constitution.md`.

`skills/living-docs/SKILL.md` was expected to need an edit and did not: it and every other prose surface spell the verb `living-docs new <type> "<title>"` with a placeholder rather than an enumeration, so a new row is documented the moment it exists. That is the shape the rest of this ADR is trying to reach.

**Verification criteria:**
- `living-docs new research "..."` creates `docs/research/0001-....md` with `type: Research`, and `living-docs index research` renders a `Research` heading listing it.
- `living-docs new constitution "..."` creates `docs/constitution.md` with no number; a second invocation exits non-zero saying it already exists, and creates nothing.
- `living-docs check` passes on a bundle containing both, and also on a bundle containing neither — both types are optional.
- `living-docs check` reports `docs/constitution.md` for non-canonical frontmatter (it is CLI-owned) but never as an index orphan (it is exempt).
- **Fitness function A:** a test iterating `DOC_TYPES` asserts every spec resolves a non-empty template whose first line matches its `frontmatter` value, and that `spec_for(spec.token)` round-trips. A row added with a mismatched template fails it.
- **Fitness function B:** a test asserts the number of `web_creatable` specs equals the number of options the web create form renders, and that the set of tokens `index` regenerates equals the `DOC_TYPES` **`Identity::Numbered`** tokens (see decision 6: a singleton has no directory index). The three surfaces cannot drift apart.
- **Fitness function C:** a test asserts the unsupported-type error message contains every token in `DOC_TYPES`, so the message cannot go stale.
- **Fitness function D:** `grep` finds no literal `"bdr"` string list outside `doc_type.rs` and `#[cfg(test)]` modules, *except* `brief::slots_for` and `brief::trail_comment_for` — the nine taxonomy-identity enumerations are gone, not merely supplemented. The two exempt sites match on type token to select **judgment-slot content**, which is per-type prose, not identity; they stayed hand-synced until ADR 0027 moved every type-keyed rule into a registry field. This is a prose check, not a test: a test asserting the absence of a string in sibling source files would couple the registry's tests to file layout, which is a worse invariant than the one it guards.
- The invariant that the paths a `DocStore` enumerates under a bundle are rooted at the bundle it was given, pinned by a test.

## Alternatives rejected

**Add the two types to all nine sites and move on.** Rejected: that is the exact mechanism that produced the half-wired `constitution`, and it would leave the next type facing the same eighteen edits and the same two `.expect()` panics. Cheaper as a diff, more expensive as a repository.

**A trait with one impl per doc type.** Rejected: the variation between types is *data* (a directory, a string, a partition axis), not behavior. Six near-identical impls is more code than six table rows, and iterating them still requires an enumeration — so the duplication would survive in the very place the trait was meant to remove it.

**Runtime-configurable types from a TOML file.** Rejected: templates are compile-time embedded per ADR 0001. A configured type whose template is not in the binary cannot be created, so configuration would be able to express states the tool cannot honor. The table cannot.

**Make `constitution` a numbered series** so it reuses the existing path entirely. Rejected: its own template already specifies singular semantics, and "constitution 0003" misrepresents the artifact — a project has one constitution, amended (there is an Amendment Log section for exactly that), not a series.
