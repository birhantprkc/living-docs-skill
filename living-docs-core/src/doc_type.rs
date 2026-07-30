//! The single compile-time enumeration of the doc-type taxonomy (ADR 0026).
//! Every site that once hand-wrote the doc-type tokens looks them up here
//! instead, so a token's directory, frontmatter value and template can never
//! disagree — the invariant `commands::new::plan_at` used to assert at
//! runtime becomes unrepresentable.

/// Where a doc type's records live, carried as an enum variant field rather
/// than a struct field so a singleton type cannot have a stale directory
/// (ADR 0026 decision point 1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Identity {
    /// `<dir>/NNNN-<slug>.md`; the number is allocated by `next`.
    Numbered { dir: &'static str },
    /// A single `<file>` relative to the bundle root; a second one is
    /// refused.
    Singleton { file: &'static str },
}

/// The axis `index` partitions a type's records along.
#[derive(PartialEq, Eq, Debug)]
pub enum IndexPartition {
    OpenClosed,
    ActiveSuperseded,
    Flat,
}

/// Everything a doc type needs to be created, indexed and offered by every
/// consumer: its token, path shape, frontmatter value, embedded template,
/// index rendering and web-creatability.
#[derive(PartialEq, Debug)]
pub struct DocTypeSpec {
    pub token: &'static str,
    pub identity: Identity,
    pub frontmatter: &'static str,
    pub template: &'static str,
    pub index_heading: &'static str,
    pub index_partition: IndexPartition,
    pub web_creatable: bool,
}

const ADR: DocTypeSpec = DocTypeSpec {
    token: "adr",
    identity: Identity::Numbered { dir: "adr" },
    frontmatter: "ADR",
    template: include_str!("../../skills/living-docs/templates/adr.md"),
    index_heading: "ADRs",
    index_partition: IndexPartition::ActiveSuperseded,
    web_creatable: true,
};

const BDR: DocTypeSpec = DocTypeSpec {
    token: "bdr",
    identity: Identity::Numbered { dir: "bdr" },
    frontmatter: "BDR",
    template: include_str!("../../skills/living-docs/templates/bdr.md"),
    index_heading: "BDRs",
    index_partition: IndexPartition::ActiveSuperseded,
    web_creatable: true,
};

const PRD: DocTypeSpec = DocTypeSpec {
    token: "prd",
    identity: Identity::Numbered { dir: "prd" },
    frontmatter: "PRD",
    template: include_str!("../../skills/living-docs/templates/prd.md"),
    index_heading: "PRDs",
    index_partition: IndexPartition::ActiveSuperseded,
    web_creatable: true,
};

const ISSUE: DocTypeSpec = DocTypeSpec {
    token: "issue",
    identity: Identity::Numbered { dir: "issues" },
    frontmatter: "Issue",
    template: include_str!("../../skills/living-docs/templates/issue.md"),
    index_heading: "Issues",
    index_partition: IndexPartition::OpenClosed,
    web_creatable: true,
};

const RESEARCH: DocTypeSpec = DocTypeSpec {
    token: "research",
    identity: Identity::Numbered { dir: "research" },
    frontmatter: "Research",
    template: include_str!("../../skills/living-docs/templates/research.md"),
    index_heading: "Research",
    index_partition: IndexPartition::Flat,
    web_creatable: true,
};

/// `index_heading`/`index_partition` are inert for a singleton — it has no
/// directory index to render — and are set to placeholder values rather than
/// wrapped in an `Option`, since no directory-index code path ever reads them
/// for this row (ADR 0026 decision point 6).
const CONSTITUTION: DocTypeSpec = DocTypeSpec {
    token: "constitution",
    identity: Identity::Singleton {
        file: "constitution.md",
    },
    frontmatter: "Constitution",
    template: include_str!("../../skills/living-docs/templates/constitution.md"),
    index_heading: "Constitution",
    index_partition: IndexPartition::Flat,
    web_creatable: true,
};

/// The sole enumeration of the doc-type taxonomy. Every consumer derives
/// from this table instead of hand-syncing its own copy.
pub const DOC_TYPES: &[DocTypeSpec] = &[ADR, BDR, PRD, ISSUE, RESEARCH, CONSTITUTION];

/// Looks up a doc type by its CLI token.
pub fn spec_for(token: &str) -> Option<&'static DocTypeSpec> {
    DOC_TYPES.iter().find(|spec| spec.token == token)
}

/// Looks up a doc type by its numbered-series directory name — the reverse
/// of a [`Identity::Numbered`] spec's `dir`. A singleton type has no
/// directory, so it never matches.
pub fn spec_for_dir(dir: &str) -> Option<&'static DocTypeSpec> {
    DOC_TYPES.iter().find(|spec| matches_dir(spec, dir))
}

fn matches_dir(spec: &DocTypeSpec, dir: &str) -> bool {
    match spec.identity {
        Identity::Numbered { dir: spec_dir } => spec_dir == dir,
        Identity::Singleton { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ADR 0026 fitness function A: every row's template must actually carry
    /// that row's frontmatter type, and `spec_for` must resolve each token
    /// back to the exact same static row — so a row added with a mismatched
    /// template, or one that fails to round-trip, fails to compile-time
    /// agreement here rather than surfacing as a runtime panic.
    #[test]
    fn fitness_function_a_every_spec_matches_its_template_and_round_trips() {
        for spec in DOC_TYPES {
            assert!(
                !spec.template.is_empty(),
                "{} has an empty template",
                spec.token
            );

            let type_line = spec
                .template
                .lines()
                .find(|line| line.starts_with("type:"))
                .unwrap_or_else(|| {
                    panic!("{} template has no 'type:' frontmatter line", spec.token)
                });
            assert_eq!(
                type_line,
                format!("type: {}", spec.frontmatter),
                "{} template's frontmatter type disagrees with its spec",
                spec.token
            );

            let resolved = spec_for(spec.token)
                .unwrap_or_else(|| panic!("{} did not round-trip through spec_for", spec.token));
            assert_eq!(
                resolved, spec,
                "{} did not round-trip to an identical spec",
                spec.token
            );
        }
    }

    #[test]
    fn spec_for_returns_none_for_an_unknown_token() {
        assert!(spec_for("glossary").is_none());
        assert!(spec_for("").is_none());
    }

    /// The row this slice adds: `constitution` now resolves, and it resolves
    /// as a [`Identity::Singleton`] naming exactly `constitution.md` — the
    /// row `commands::new`/`commands::brief` branch on to write the bundle's
    /// single unnumbered record.
    #[test]
    fn spec_for_resolves_constitution_as_a_singleton_named_constitution_md() {
        let spec = spec_for("constitution").expect("constitution must be a registered token");
        assert_eq!(
            spec.identity,
            Identity::Singleton {
                file: "constitution.md"
            }
        );
    }

    #[test]
    fn spec_for_dir_matches_the_plural_issues_directory() {
        assert_eq!(spec_for_dir("issues").map(|spec| spec.token), Some("issue"));
        assert_eq!(spec_for_dir("adr").map(|spec| spec.token), Some("adr"));
    }

    #[test]
    fn spec_for_dir_returns_none_for_an_unknown_directory() {
        assert!(spec_for_dir("constitution").is_none());
        assert!(spec_for_dir("issue").is_none());
        assert!(spec_for_dir("").is_none());
    }
}
