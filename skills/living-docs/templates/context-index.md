# Context — Domain & Module Vocabulary

The shared language for this project: the names used consistently across code, docs,
and reviews. This index is the entry point; each group file owns one coherent slice of
the vocabulary. A concept lives in exactly one file — cross-reference, never duplicate.

## Groups

| File | Covers |
|---|---|
| [domain-concepts.md](domain-concepts.md)         | Core domain entities and rules |
| [modules-<group-a>.md](modules-group-a.md)        | <e.g. write-path modules> |
| [modules-<group-b>.md](modules-group-b.md)        | <e.g. read-path modules> |
| [<shapes>.md](shapes.md)                          | <e.g. read/response shapes> |
| [<storage>.md](storage.md)                        | <e.g. storage internals> |

Each group file is a standalone OKF concept: `type: Context` frontmatter, one `#` title, then `##` sections. Add a row here whenever a new group file is created.
