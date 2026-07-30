---
type: ADR
title: A single DocType registry replaces nine hand-synced enumerations, and research and constitution enter as rows
description: One compile-time DocTypeSpec table becomes the sole enumeration of the doc-type taxonomy, so the nine sites that hand-synced it derive instead, and `research` and `constitution` are added as rows.
status: Accepted
timestamp: 2026-07-29T23:49:51Z
---

# 0026. A single DocType registry replaces nine hand-synced enumerations, and research and constitution enter as rows

## Context

The doc-type taxonomy is four tokens — `adr`, `bdr`, `prd`, `issue` — and it is
written out by hand in **nine** places:

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

Nothing makes those nine agree. `new::plan_at` *asserts* the agreement at
runtime:

```rust
let dir_name = paths::dir_for(doc_type).ok_or_else(|| unsupported_type_message(doc_type))?;
let frontmatter_type = paths::frontmatter_type_for(doc_type)
    .expect("dir_for and frontmatter_type_for cover the same doc types");
let template = templates::template_for(doc_type)
    .expect("dir_for and template_for cover the same doc types");
```

Adding a token to `dir_for` and forgetting `template_for` does not fail to
compile and does not return an error. It **panics**, in a release binary, on a
user's `new` invocation.

Everywhere else the failure is quieter still. Unknown types degrade to defaults
by design: `index::render_body` falls back to a flat listing, `heading_title_for`
to the generic `"Index"`, `brief::slots_for` to an empty slot array. Each
fallback is individually reasonable. Together they mean a **half-added type
looks like a working type** — it creates records, it indexes, it just quietly
lacks a partition axis, a heading and judgment slots.

This is not hypothetical. It already happened:

- `skills/living-docs/templates/constitution.md` exists, is fully authored, and
  states its own semantics in a body comment: *"This file is singular — no NNNN
  prefix, and it is NOT listed as a concept in any index.md."* It is wired into
  nothing.
- `templates::template_for` carries a test that **asserts** the gap:
  `assert_eq!(template_for("constitution"), None)`.
- `paths.rs` has two tests asserting `dir_for("constitution")` and
  `dir_for("glossary")` are `None`.
- `skills/living-docs/SKILL.md`'s tags already advertise `constitution` and
  `research` as doc types the skill handles.

So the taxonomy is advertised in the skill manifest, specified in a template,
pinned as absent by three tests, and implemented for four of six types. The
duplication did not merely risk drift — it produced it, and then the tests
froze the drift in place.

Two new types are wanted: `research` (this repo's own ADR template already links
to `[research](/research/NNNN-<slug>.md)`, and consuming bundles carry
`docs/research/NNNN-slug.md` records the CLI cannot create or index) and
`constitution`. Adding them the current way costs eighteen edits across nine
sites, with a silent-fallback failure mode on any one omission.

## Decision

We will make one compile-time table the sole enumeration of the taxonomy, have
every existing site derive from it, and then add the two types as rows.

**1. A registry module, `living-docs-core/src/doc_type.rs`.**

```rust
pub enum Identity {
    /// `<dir>/NNNN-<slug>.md`; the number is allocated by `next`.
    Numbered { dir: &'static str },
    /// A single `<file>` relative to the bundle root; a second one is refused.
    Singleton { file: &'static str },
}

pub struct DocTypeSpec {
    pub token: &'static str,
    pub identity: Identity,
    pub frontmatter: &'static str,
    pub template: &'static str,
    pub index_heading: &'static str,
    pub index_partition: IndexPartition,
    pub web_creatable: bool,
}

pub const DOC_TYPES: &[DocTypeSpec] = &[ADR, BDR, PRD, ISSUE, RESEARCH, CONSTITUTION];

pub fn spec_for(token: &str) -> Option<&'static DocTypeSpec>;
pub fn spec_for_dir(dir: &str) -> Option<&'static DocTypeSpec>;
```

**Identity carries the path shape, so an inconsistent type is unrepresentable.**
A directory is a field of the `Numbered` variant, not of the struct, so a
singleton cannot have a stale directory and `dir_for` cannot return one for it.
This is the load-bearing choice: the invariant `plan_at` asserts at runtime
becomes a property of the type system.

**2. Every one of the nine sites becomes a lookup.** `paths::dir_for`,
`doc_type_for_dir` and `frontmatter_type_for` delegate to the registry;
`templates::template_for` returns `spec.template`; `record::NUMBERED_DOC_TYPES`
and `index::SUPPORTED_TYPES` are derived from `DOC_TYPES`;
`web::CREATABLE_DOC_TYPES` filters on `web_creatable`. The public signatures of
`paths::*` and `templates::template_for` do not change, so no caller outside
these modules is touched.

**3. All four `.expect()` calls are deleted.** The fragile three-line resolution
appears **twice**, byte-identically: in `commands::new::plan_at` and in
`commands::brief::scaffold_brief`. Each carries its own copy of both panics, so
there are four, not two. One `spec_for(token)?` in each site yields the
directory, the frontmatter value and the template together. Partial agreement is
no longer expressible, so there is nothing left to assert.

`brief.rs`'s per-type `slots_for` and `trail_comment_for` match arms are a
different thing and stay: they enumerate *judgment-slot content*, not the
taxonomy's identity, and are recorded as a follow-up below.

**4. The unsupported-type message is generated from the registry** and the
second copy is removed. A stale list at the user is no longer possible.

**5. `research` enters as a `Numbered` row:** directory `research`, frontmatter
`Research`, index heading `Research`, flat partition (a research record is a
point-in-time audit; it has no Active/Superseded or Open/Closed lifecycle),
`web_creatable: true`, and a new `skills/living-docs/templates/research.md`.

**6. `constitution` enters as a `Singleton` row:** file `constitution.md` at the
bundle root, frontmatter `Constitution`, `web_creatable: true`. Its existing
template is wired with **one correction**: `status: Draft               # Draft |
Ratified | Amended` carries the status domain as a trailing YAML comment, which
no other template does and which a canonical frontmatter round-trip would flag.
The domain moves into the HTML comment below the frontmatter, where `adr.md` and
`bdr.md` already keep theirs. `living-docs new constitution "<title>"` writes
`docs/constitution.md`; a second invocation is refused by the clobber guard
`plan_at` already applies (`store.read(&target_path).is_ok()`), so the refusal
needs no new code.

**`index` sweeps Numbered rows only.** A singleton has no directory and therefore
no directory index, so the bare `index` sweep iterates the `Identity::Numbered`
tokens rather than every token — otherwise `compute` would resolve no directory
for `constitution` and the sweep would exit non-zero. An explicit `index
constitution` is an error with its own message: the type is supported, it simply
has no index to regenerate, and reusing the unsupported-type message would lie.
This narrows fitness function B accordingly: the token set `index` regenerates
equals the registry's **Numbered** token set.

**7. `check` honors the singleton contract.** `docs/constitution.md` is
CLI-owned for the canonical-frontmatter invariant, and is **exempt from index
membership** (invariant 3) — exactly as its template states. Both types are
optional: a bundle with no `research/` directory and no `constitution.md` stays
conformant, and `index` skips a type whose directory does not exist rather than
creating it.

**That last clause described an intention, not the code.** `commands::index::regenerate`
called `fs::create_dir_all` unconditionally, so a bare `living-docs index` created
an empty `index.md` for *every* registry token. With four rows this was invisible
pollution — a bundle that uses the CLI has all four directories anyway. The fifth
row turned it into a **check-breaking regression**: `examples/linkly/docs` passes
`check`, and after one bare `index` run it fails with
`research/index.md directory index not reachable from index.md (invariant 3)`,
because the new directory index is not linked from the bundle root. So this
decision now carries a code change it did not anticipate: `regenerate` is a no-op
for a type whose directory does not exist. Directories come into existence when
`new` writes the first record, never from `index`. The fitness function is the
property, not the symptom: **a bundle that passes `check` still passes `check`
after a bare `index` run.**

**8. The one enumeration the registry cannot reach stays hardcoded, and is
pinned by a cross-language fitness test.** `skills/living-docs/hooks/block-docs-handwrite.sh`
is bash and matches CLI-owned directories with a literal alternation
(`(adr|bdr|prd|issues)`, ADR 0020's scope). It cannot read a Rust `const`, and we
will not make a PreToolUse gate shell out to the binary on every write — the hook
must stay fast and fail-open, and the binary may be absent. So the bash copy
remains a copy, and a Rust test reads the script, extracts the alternation group,
and asserts it equals the registry's `Identity::Numbered` directories. The
duplication survives; the *drift* does not.

## Consequences

**Easier / gained:**
- Adding a doc type is one registry row plus one template file.
- Two runtime panics on taxonomy inconsistency are removed, replaced by a
  compile-time impossibility.
- Three tests that pinned `constitution` and `glossary` as unsupported are
  replaced by tests that hold the registry to its contract.
- The error message the user reads can never disagree with what the tool
  accepts.

**Harder / accepted trade-offs:**
- The registry is compile-time, so a doc type cannot be added by configuration.
  This is deliberate: templates are `include_str!`-embedded to keep the binary
  self-contained (ADR 0001), so a configured type could name a template that
  does not exist — the config would be able to express a broken state that the
  table cannot.
- `Identity` becoming an enum makes `dir_for` fallible for a reason other than
  "unknown type": a singleton has no directory. Callers that assumed a
  directory for every known type must handle that, which is the point.

**Follow-ups:**
- ~~Whether `research` and `constitution` get `brief::slots_for` definitions is a
  separate decision.~~ **Corrected during implementation: it is not separate, and
  it is not optional.** `slots_for` returning `&[]` does not merely produce a
  slot-less scaffold — it produces a **broken** one. Every judgment section a
  slot collapses takes its placeholder links with it, which is precisely why
  `trail_comment_for` wraps its stubs in an HTML comment ("so an unfilled
  scaffold carries no dangling markdown links — `check` stays green on the raw
  `brief` output"). A type with no slots keeps its template's
  `[ADR](/adr/NNNN-<slug>.md)` placeholders as live links, and `check` reports
  them. So the real invariant is: **a registry row the CLI will `brief` must
  carry slot definitions**, and the registry-driven
  `brief_output_passes_check_for_every_supported_doc_type` test is the fitness
  function that enforces it — it caught this the first time a row was added
  without them. `slots_for` and `trail_comment_for` stay outside the registry
  (they are judgment-slot *content*, not identity), but they are no longer
  optional per row.
- `scaffold_brief` builds a numbered path unconditionally, so the identity branch
  decision 6 adds to `new` must land in `brief` too. Until it does, `brief
  constitution` would report the type as unsupported while `new constitution`
  succeeds — a contradiction the singleton slice must close, not defer.
- `check::size::has_size_target` matches on frontmatter values
  (`"ADR" | "BDR" | "PRD" | "Issue"`) to decide which types carry the ~100-line
  advisory. It is not one of the nine and does not drift when a row is added —
  both new types correctly fall through to exempt, and a test already pins that.
  Whether the advisory becomes a registry field is a separate decision; it
  encodes an *epistemic category* (decision/execution record vs. long-form
  evidence), not identity.
- `check::canonical::in_cli_owned_dir` resolves ownership through
  `paths::doc_type_for_dir`, so adding the `research` row silently makes
  `docs/research/` CLI-owned for the canonical-frontmatter invariant in every
  consuming bundle. That is the intended semantics — a type the CLI can create
  is a type the CLI owns — but it means a bundle carrying hand-written research
  records will start reporting them. This is a `check` behavior change delivered
  by a registry row, and it belongs in the release notes.
- `glossary` is the remaining authored-but-unwired template. It is now a
  one-row addition whenever it is wanted.
- **`is_bundle_singleton` is form-sensitive, and that is safe for a reason worth
  writing down.** It decides ownership with `path == bundle.join(file)`, where
  its sibling `in_cli_owned_dir` compares only a parent directory's `file_name`
  and is therefore form-*insensitive*. Review raised the obvious worry: on macOS
  `/tmp` symlinks to `/private/tmp`, so a bundle expressed one way and walked the
  other would not compare equal, silently un-exempting the singleton and
  reporting it as an orphan — a false positive in the gate. **It does not
  reproduce**, and the reason is structural rather than lucky: `collect_md_files`
  builds every path by joining onto the bundle root it was handed, and
  `read_dir` does not resolve symlinks, so both sides of the comparison derive
  from the same root by construction. A bundle reached through a symlink checks
  identically to the same bundle reached directly.
  The correctness therefore rests on an invariant that was unstated: **the paths
  a `DocStore` enumerates under a bundle are rooted at the bundle it was given.**
  That is the thing to pin with a test, not to paper over with a
  `canonicalize` call — which would make a pure predicate touch the filesystem
  and would break every in-memory-store unit test, whose fixture paths do not
  exist on disk. A real regression traded for an imaginary one.
- **The singleton's canonical contract now gates the db-mode write path.**
  `check_violations` shares `run_all_checks`, so `db_store::write_checked`
  rejects a `constitution.md` whose frontmatter is not canonical. That is the
  intended direction — ADR 0016's write gate is "the same invariants `check`
  enforces" — but it is a behavior change the web surface inherits without a
  code change of its own.
- **`docs/research/index.md` in this repository predates its own registry row.**
  It was hand-written in a format the renderer no longer produces, so
  `living-docs index research` will diff it. Regenerating it is mechanical and
  deliberately deferred so it does not bury this ADR's implementation diff.

## Verification

**Implementation impact:** `living-docs-core/src/doc_type.rs` (new),
`living-docs-core/src/paths.rs`, `living-docs-core/src/templates.rs`,
`living-docs-core/src/record.rs`, `living-docs-core/src/commands/new.rs`,
`living-docs-core/src/commands/brief.rs` (its `scaffold_brief` resolution, plus a
`slots_for`/`trail_comment_for`/`context_marker_for` arm per new row — see the
corrected follow-up), `living-docs-core/src/commands/index.rs`,
`living-docs-core/src/check/canonical.rs`, `living-docs-core/src/check/mod.rs`,
`living-docs-core/src/check/graph.rs`, `web/src/views.rs`,
`living-docs-core/tests/hook_registry_parity.rs` (new),
`skills/living-docs/hooks/block-docs-handwrite.sh`,
`skills/living-docs/tests/hooks/run.sh`,
`skills/living-docs/templates/research.md` (new),
`skills/living-docs/templates/constitution.md`.

`skills/living-docs/SKILL.md` was expected to need an edit and did not: it and
every other prose surface spell the verb `living-docs new <type> "<title>"` with
a placeholder rather than an enumeration, so a new row is documented the moment
it exists. That is the shape the rest of this ADR is trying to reach.

**Verification criteria:**
- `living-docs new research "..."` creates `docs/research/0001-....md` with
  `type: Research`, and `living-docs index research` renders a `Research`
  heading listing it.
- `living-docs new constitution "..."` creates `docs/constitution.md` with no
  number; a second invocation exits non-zero saying it already exists, and
  creates nothing.
- `living-docs check` passes on a bundle containing both, and also on a bundle
  containing neither — both types are optional.
- `living-docs check` reports `docs/constitution.md` for non-canonical
  frontmatter (it is CLI-owned) but never as an index orphan (it is exempt).
- **Fitness function A:** a test iterating `DOC_TYPES` asserts every spec
  resolves a non-empty template whose first line matches its `frontmatter`
  value, and that `spec_for(spec.token)` round-trips. A row added with a
  mismatched template fails it.
- **Fitness function B:** a test asserts the number of `web_creatable` specs
  equals the number of options the web create form renders, and that the
  set of tokens `index` regenerates equals the `DOC_TYPES` **`Identity::Numbered`**
  tokens (see decision 6: a singleton has no directory index). The three
  surfaces cannot drift apart.
- **Fitness function C:** a test asserts the unsupported-type error message
  contains every token in `DOC_TYPES`, so the message cannot go stale.
- **Fitness function D:** `grep` finds no literal `"bdr"` string list outside
  `doc_type.rs` and `#[cfg(test)]` modules, *except* `brief::slots_for` and
  `brief::trail_comment_for` — the nine taxonomy-identity enumerations are gone,
  not merely supplemented. The two exempt sites match on type token to select
  **judgment-slot content**, which is per-type prose, not identity; they are a
  named follow-up below, not a leak. This is a prose check, not a test: a test
  asserting the absence of a string in sibling source files would couple the
  registry's tests to file layout, which is a worse invariant than the one it
  guards.

## Alternatives rejected

**Add the two types to all nine sites and move on.** Rejected: that is the exact
mechanism that produced the half-wired `constitution`, and it would leave the
next type facing the same eighteen edits and the same two `.expect()` panics.
Cheaper as a diff, more expensive as a repository.

**A trait with one impl per doc type.** Rejected: the variation between types is
*data* (a directory, a string, a partition axis), not behavior. Six near-identical
impls is more code than six table rows, and iterating them still requires an
enumeration — so the duplication would survive in the very place the trait was
meant to remove it.

**Runtime-configurable types from a TOML file.** Rejected: templates are
compile-time embedded per ADR 0001. A configured type whose template is not in
the binary cannot be created, so configuration would be able to express states
the tool cannot honor. The table cannot.

**Make `constitution` a numbered series** so it reuses the existing path
entirely. Rejected: its own template already specifies singular semantics, and
"constitution 0003" misrepresents the artifact — a project has one constitution,
amended (there is an Amendment Log section for exactly that), not a series.
