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

/// Whether a doc type's body is measured against the advisory 100/120-line
/// target in `check::size`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BodySize {
    Targeted,
    Exempt,
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
/// index rendering, web-creatability, and whether its body carries the
/// advisory size target.
#[derive(PartialEq, Debug)]
pub struct DocTypeSpec {
    pub token: &'static str,
    pub identity: Identity,
    pub frontmatter: &'static str,
    pub template: &'static str,
    pub index_heading: &'static str,
    pub index_partition: IndexPartition,
    pub web_creatable: bool,
    pub body_size: BodySize,
    /// The values `living-docs status` is willing to set on this type's
    /// records, in seed order — `status_vocabulary[0]` is what `new`/`brief`
    /// seed a fresh record with (ADR 0029). `Superseded` is deliberately
    /// never a member of any row: it is reachable only through
    /// `living-docs supersede`, which also wires the
    /// `supersedes`/`superseded_by` links.
    pub status_vocabulary: &'static [&'static str],
}

const ADR: DocTypeSpec = DocTypeSpec {
    token: "adr",
    identity: Identity::Numbered { dir: "adr" },
    frontmatter: "ADR",
    template: include_str!("../../skills/living-docs/templates/adr.md"),
    index_heading: "ADRs",
    index_partition: IndexPartition::ActiveSuperseded,
    web_creatable: true,
    body_size: BodySize::Targeted,
    status_vocabulary: &["Proposed", "Accepted", "Deprecated"],
};

const BDR: DocTypeSpec = DocTypeSpec {
    token: "bdr",
    identity: Identity::Numbered { dir: "bdr" },
    frontmatter: "BDR",
    template: include_str!("../../skills/living-docs/templates/bdr.md"),
    index_heading: "BDRs",
    index_partition: IndexPartition::ActiveSuperseded,
    web_creatable: true,
    body_size: BodySize::Targeted,
    status_vocabulary: &["Draft", "Accepted", "Implemented"],
};

const PRD: DocTypeSpec = DocTypeSpec {
    token: "prd",
    identity: Identity::Numbered { dir: "prd" },
    frontmatter: "PRD",
    template: include_str!("../../skills/living-docs/templates/prd.md"),
    index_heading: "PRDs",
    index_partition: IndexPartition::ActiveSuperseded,
    web_creatable: true,
    body_size: BodySize::Targeted,
    status_vocabulary: &["Draft", "Accepted", "Implemented"],
};

const ISSUE: DocTypeSpec = DocTypeSpec {
    token: "issue",
    identity: Identity::Numbered { dir: "issues" },
    frontmatter: "Issue",
    template: include_str!("../../skills/living-docs/templates/issue.md"),
    index_heading: "Issues",
    index_partition: IndexPartition::OpenClosed,
    web_creatable: true,
    body_size: BodySize::Targeted,
    status_vocabulary: &["open", "in-progress", "closed"],
};

const RESEARCH: DocTypeSpec = DocTypeSpec {
    token: "research",
    identity: Identity::Numbered { dir: "research" },
    frontmatter: "Research",
    template: include_str!("../../skills/living-docs/templates/research.md"),
    index_heading: "Research",
    index_partition: IndexPartition::Flat,
    web_creatable: true,
    body_size: BodySize::Exempt,
    status_vocabulary: &["Draft", "Accepted"],
};

/// `index_heading`/`index_partition` are inert for a singleton — it has no
/// directory index to render — and are set to placeholder values rather than
/// wrapped in an `Option`, since no directory-index code path ever reads them
/// for this row (ADR 0026 decision point 6). `status_vocabulary` is empty for
/// the same reason: a singleton carries no `NNNN`, so `living-docs status
/// <NNNN>` can never resolve one — Constitution's own `Draft | Ratified |
/// Amended` vocabulary is out of this row's scope (ADR 0029).
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
    body_size: BodySize::Exempt,
    status_vocabulary: &[],
};

/// The sole enumeration of the doc-type taxonomy. Every consumer derives
/// from this table instead of hand-syncing its own copy.
pub const DOC_TYPES: &[DocTypeSpec] = &[ADR, BDR, PRD, ISSUE, RESEARCH, CONSTITUTION];

/// Looks up a doc type by its CLI token.
pub fn spec_for(token: &str) -> Option<&'static DocTypeSpec> {
    DOC_TYPES.iter().find(|spec| spec.token == token)
}

/// Looks up a doc type by its `type:` frontmatter value. Returns the first
/// match, which is well-defined only because `frontmatter` values are unique
/// across `DOC_TYPES` — an invariant guarded by
/// `frontmatter_values_are_unique_so_spec_for_frontmatter_is_well_defined`.
pub fn spec_for_frontmatter(frontmatter: &str) -> Option<&'static DocTypeSpec> {
    DOC_TYPES
        .iter()
        .find(|spec| spec.frontmatter == frontmatter)
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
    use regex::Regex;

    fn body_after_frontmatter(template: &str) -> String {
        let mut dashes_seen = 0;
        let mut past_frontmatter = false;
        let mut lines = Vec::new();

        for line in template.lines() {
            if past_frontmatter {
                lines.push(line);
                continue;
            }
            if line.trim_end() == "---" {
                dashes_seen += 1;
                past_frontmatter = dashes_seen == 2;
            }
        }

        lines.join("\n")
    }

    fn skip_first_title_heading(body: &str) -> String {
        let heading = Regex::new(r"^#{1,2} ").expect("valid heading regex");
        let mut heading_skipped = false;
        let mut lines = Vec::new();

        for line in body.lines() {
            if !heading_skipped && heading.is_match(line) {
                heading_skipped = true;
                continue;
            }
            lines.push(line);
        }

        lines.join("\n")
    }

    fn strip_html_comments(body: &str) -> String {
        Regex::new(r"(?s)<!--.*?-->")
            .expect("valid html comment regex")
            .replace_all(body, "")
            .into_owned()
    }

    fn strip_fenced_code_blocks(body: &str) -> String {
        Regex::new(r"(?s)```.*?```")
            .expect("valid fenced code block regex")
            .replace_all(body, "")
            .into_owned()
    }

    fn strip_inline_code_spans(body: &str) -> String {
        Regex::new(r"`[^`]*`")
            .expect("valid inline code regex")
            .replace_all(body, "")
            .into_owned()
    }

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

    /// ADR 0029: every numbered type carries its own settable status values,
    /// in seed order; Constitution carries none — it is a singleton with no
    /// `NNNN`, so `living-docs status <NNNN>` can never reach it.
    #[test]
    fn status_vocabulary_matches_adr_0029_per_type() {
        assert_eq!(
            spec_for("adr").unwrap().status_vocabulary,
            &["Proposed", "Accepted", "Deprecated"]
        );
        assert_eq!(
            spec_for("bdr").unwrap().status_vocabulary,
            &["Draft", "Accepted", "Implemented"]
        );
        assert_eq!(
            spec_for("prd").unwrap().status_vocabulary,
            &["Draft", "Accepted", "Implemented"]
        );
        assert_eq!(
            spec_for("issue").unwrap().status_vocabulary,
            &["open", "in-progress", "closed"]
        );
        assert_eq!(
            spec_for("research").unwrap().status_vocabulary,
            &["Draft", "Accepted"]
        );
        assert!(spec_for("constitution")
            .unwrap()
            .status_vocabulary
            .is_empty());
    }

    #[test]
    fn status_vocabulary_never_carries_superseded_for_any_type() {
        for spec in DOC_TYPES {
            assert!(
                !spec
                    .status_vocabulary
                    .iter()
                    .any(|value| value.eq_ignore_ascii_case("superseded")),
                "{} must never list Superseded in status_vocabulary",
                spec.token
            );
        }
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

    /// ADR 0029 fitness function: a numbered type's template comment must
    /// name every one of that type's own `status_vocabulary` values and
    /// mention Superseded, so the registry and the template comment cannot
    /// silently drift apart. Constitution is skipped -- its vocabulary is
    /// empty and out of scope (ADR 0029).
    #[test]
    fn template_comments_agree_with_their_own_status_vocabulary() {
        for spec in DOC_TYPES {
            if matches!(spec.identity, Identity::Singleton { .. }) {
                continue;
            }

            for value in spec.status_vocabulary {
                assert!(
                    spec.template.contains(*value),
                    "{} template is missing status_vocabulary value {value:?}",
                    spec.token
                );
            }

            assert!(
                spec.template.to_lowercase().contains("uperseded"),
                "{} template must mention Superseded via `living-docs supersede`",
                spec.token
            );
        }
    }

    #[test]
    fn spec_for_dir_returns_none_for_an_unknown_directory() {
        assert!(spec_for_dir("constitution").is_none());
        assert!(spec_for_dir("issue").is_none());
        assert!(spec_for_dir("").is_none());
    }

    /// ADR 0027: `spec_for_frontmatter` resolves the first row whose
    /// `frontmatter` matches, so a duplicate would make that resolution
    /// non-deterministic. This guards the invariant, not a literal list.
    #[test]
    fn frontmatter_values_are_unique_so_spec_for_frontmatter_is_well_defined() {
        let unique: std::collections::HashSet<&str> =
            DOC_TYPES.iter().map(|spec| spec.frontmatter).collect();
        assert_eq!(
            unique.len(),
            DOC_TYPES.len(),
            "DOC_TYPES has duplicate frontmatter values"
        );
    }

    /// ADR 0030 fitness function: a template's body may no longer carry the legacy
    /// angle-bracket-with-embedded-space placeholder (e.g. `<the choice, in active
    /// voice -- specific and testable>`) that made programmatic edits fragile
    /// (issue 0022). The scan works on what remains after, in order: (1) the
    /// frontmatter block is stripped through the second `---` line inclusive; (2)
    /// the first H1/H2 title-heading line is skipped -- that placeholder is out of
    /// scope per ADR 0030 rule 5; (3) every HTML comment span (`<!-- ... -->`,
    /// non-greedy, may span multiple lines) is stripped, since guidance comments
    /// are not placeholders; (4) every fenced code block (triple backtick to the
    /// next triple backtick, non-greedy, may span multiple lines) is stripped, so
    /// a future Mermaid node label like `A[<foo bar>]` inside a ```mermaid fence
    /// can never false-trip the guard; (5) every inline code span (backtick to
    /// the next backtick) is stripped, so a worked-example Rust generic like ``
    /// `Result<R, E>` `` in a table cell is never mistaken for a placeholder.
    /// What is left is searched for any angle-bracket span containing at least
    /// one whitespace character; any match fails the assertion, naming the
    /// spec's token and the exact matched text, so a future template edit cannot
    /// reintroduce the fragile shape.
    #[test]
    fn fitness_function_no_legacy_angle_bracket_placeholder_survives_in_any_template_body() {
        let placeholder_span = Regex::new(r"<[^<>]*\s[^<>]*>").expect("valid placeholder regex");

        for spec in DOC_TYPES {
            let body = body_after_frontmatter(spec.template);
            let body = skip_first_title_heading(&body);
            let body = strip_html_comments(&body);
            let body = strip_fenced_code_blocks(&body);
            let body = strip_inline_code_spans(&body);

            let legacy_placeholder = placeholder_span.find(&body).map(|m| m.as_str());
            assert!(
                legacy_placeholder.is_none(),
                "{} template still contains a legacy angle-bracket placeholder: {:?}",
                spec.token,
                legacy_placeholder
            );
        }
    }

    /// Direct unit test for [`strip_fenced_code_blocks`]: proves the helper
    /// actually removes a fenced block's contents (not merely that the
    /// caller's scan passes), so it stays non-vacuous if the helper is ever
    /// reduced to a no-op.
    #[test]
    fn strip_fenced_code_blocks_removes_a_mermaid_fence_with_an_angle_bracket_span() {
        let body = "Before.\n\n```mermaid\ngraph TD\n  A[<foo bar>] --> B\n```\n\nAfter.";

        let stripped = strip_fenced_code_blocks(body);

        assert!(!stripped.contains("foo bar"));
        assert!(stripped.contains("Before."));
        assert!(stripped.contains("After."));
    }

    /// Issue 0021 gap: `commands::new::fill_frontmatter_description` only
    /// replaces an existing `description:` scalar -- it has no insert path,
    /// unlike `describe`'s insert-capable `set_frontmatter_fields`. This
    /// fitness function keeps that asymmetry safe by asserting every
    /// registered template's frontmatter block already carries a
    /// `description:` line for `new` to replace, so a template that ever
    /// dropped the line would fail loudly here instead of silently leaving
    /// `--description` a no-op.
    #[test]
    fn every_registered_template_frontmatter_carries_a_description_line() {
        for spec in DOC_TYPES {
            let lines: Vec<&str> = spec.template.lines().collect();
            let close = lines
                .iter()
                .skip(1)
                .position(|&line| line == "---")
                .map(|i| i + 1)
                .unwrap_or_else(|| {
                    panic!("{} template has no closing frontmatter '---'", spec.token)
                });

            let has_description_line = lines[..close]
                .iter()
                .any(|line| line.starts_with("description:"));

            assert!(
                has_description_line,
                "{} template frontmatter is missing a description: line",
                spec.token
            );
        }
    }
}
