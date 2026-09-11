# Doc trail & document map

## Doc trail

Every change follows this chain, from foundational source of truth down to code:

```mermaid
flowchart LR
  C[constitution] --> P[PRD]
  P --> A[ADR]
  P --> B[BDR]
  A --> I[issues]
  B --> I
  I --> K[code]
```

| Artifact | Role |
|---|---|
| **constitution** | Foundational source of truth: what the product is, core data model, non-negotiables. All other docs sit under it. |
| **PRD** | What the system must do and why — feature/product requirement spec. |
| **ADR** | How the system is structured — architectural/implementation decision and rationale. |
| **BDR** | What the system must observably do — inputs, outputs, side effects, Given/When/Then scenarios — **and how each is tested** (the Test Design matrix; single home for "how to test", an execution issue links it). |
| **issues** | Execution slices — discrete units of work that trace back to ADRs/BDRs. |
| **code** | Implementation — every behavior, structure, and interface specified above, realized. |

---

## The decision test — which record, by one question (ADR 0054)

Definitions don't stop content from leaking across types; a single **decision test** per type does. Answer the question before writing; if the answer is empty, the content belongs elsewhere.

| Type | The question | If the answer is empty |
|---|---|---|
| **ADR** | Which alternative was rejected, and what would a future engineer do differently without this record? | Not an ADR — the decision goes in the issue's `## Decision`. |
| **BDR** | Which test fails if this behavior breaks? | Not a BDR — it is prose. |
| **PRD** | Who asked, and what is explicitly out of scope? | Not a PRD — it is a large issue. |
| **Issue** | What is the diff? | Not an issue — it is research. |
| **Research** | Which external source backs the claim? | Not research — it is opinion. |

## The leak table — content in the wrong record (counterexamples)

Agents learn boundaries from counterexamples better than from definitions. Each row is content that leaked, the type it landed in, and where it belongs. `check` emits a `LEAK` advisory for the **detectable** cases (marked ✓); the rest are judgement and never a hard stop (same posture as the semantic triggers).

| Leaked content | Landed in | Belongs in | `check` |
|---|---|---|---|
| Target behavior — "the system shall…", Given/When/Then | ADR / PRD | BDR | ✓ |
| Test results, benchmark numbers, JSON output | ADR | issue (or research if externally sourced) | ✓ (JSON/data block) |
| Implementation checkpoints / a delivery plan | ADR | issue | — |
| "Needs an ADR" / "Needs a BDR" (a deferred decision) | issue | decide in the issue now, or open the record now | ✓ |
| A scenario with no `Proves:` line | BDR | add the `Proves:` requirement id | ✓ |
| Rationale for a choice ("we chose X because…") | BDR | ADR | — |
| A source-less claim about the industry | ADR Context | research, or delete | — |
| An unfilled `{{PLACEHOLDER}}` | any record | fill it, or remove the slot | ✓ |

---

## Document map

| Type | Lives in | Purpose | Mutability |
|---|---|---|---|
| Project guide | `CLAUDE.md` / `README.md` (root) | Entry point: scope, stack, docs index, mandatory workflows | Live — edit freely |
| Constitution | `docs/constitution.md` | Foundational source of truth: product scope, data model, non-negotiables | Append-only once ratified (amendment log) |
| Context index | `docs/context/index.md` + group files | Domain & module vocabulary, semantically grouped | Live — edit freely |
| Glossary | `docs/context/glossary.md` | Terms & acronyms defined once, in the doc language (acronym headwords as-is) | Live — edit freely |
| Architecture | `docs/architecture.md` or `docs/architecture/` + index | Living Mermaid diagrams: structure, data model, flows, tool-calling | Live — must match code |
| ADR | `docs/adr/NNNN-slug.md` | One architectural/implementation decision | Append-only (supersede) |
| BDR | `docs/bdr/NNNN-slug.md` | One observable-behavior decision | Append-only (supersede or amend) |
| PRD | `docs/prd/NNNN-slug.md` | One feature/product requirement spec | Append-only once accepted |
| Issue | `docs/issues/NNNN-slug.md` | Tracker mirror (body), one per ticket | Body editable; published copy follows |
| Research | `docs/research/NNNN-<slug>.md` (single file, no subfolder; sequential number leads, date in frontmatter `timestamp`; ends in `# References`) | External evidence with sourced claims | Append-only (evidence is dated) |

Each directory carries its own `index.md` listing (OKF §6, no frontmatter). The project guide's "Docs index" links to the bundle-root `docs/index.md`. See `rules/semantic-index.md` for the indexing contract.
