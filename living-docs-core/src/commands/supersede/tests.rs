use super::*;

#[test]
fn apply_frontmatter_field_fills_an_existing_empty_key_line() {
    let contents = "---\ntype: ADR\nsupersedes:\nsuperseded_by:\n---\n\n# Body\n";
    let updated = apply_frontmatter_field(contents, "superseded_by", "0002").unwrap();
    assert!(updated.contains("superseded_by: 0002"));
    assert!(updated.contains("supersedes:\n"));
}

#[test]
fn apply_frontmatter_field_preserves_a_trailing_guidance_comment() {
    let contents =
        "---\nsupersedes:                 # NNNN of the ADR this replaces, if any\n---\n\n# Body\n";
    let updated = apply_frontmatter_field(contents, "supersedes", "0001").unwrap();
    assert!(updated.contains("supersedes: 0001 # NNNN of the ADR this replaces, if any"));
}

#[test]
fn apply_frontmatter_field_inserts_an_absent_key_before_the_closing_fence() {
    let contents = "---\ntype: BDR\nsuperseded_by:\n---\n\n# Body\n";
    let updated = apply_frontmatter_field(contents, "supersedes", "0001").unwrap();
    assert!(updated.contains("supersedes: 0001"));
    assert!(updated.contains("---\ntype: BDR\nsuperseded_by:\nsupersedes: 0001\n---"));
}

#[test]
fn apply_frontmatter_field_leaves_the_body_untouched() {
    let contents = "---\ntype: ADR\nsupersedes:\n---\n\n## Context\n\nSome body text.\n";
    let updated = apply_frontmatter_field(contents, "supersedes", "0001").unwrap();
    assert!(updated.contains("## Context\n\nSome body text.\n"));
}

#[test]
fn apply_frontmatter_field_without_a_frontmatter_block_returns_none() {
    assert_eq!(
        apply_frontmatter_field("no frontmatter here\n", "supersedes", "0001"),
        None
    );
}

#[test]
fn parse_record_number_rejects_non_numeric_input() {
    assert!(parse_record_number("abcd").is_err());
}

#[test]
fn parse_record_reference_accepts_a_bare_number_with_no_qualifier() {
    let (dir, number) = parse_record_reference("0028").expect("bare number must parse");

    assert_eq!(dir, None);
    assert_eq!(number, 28);
}

#[test]
fn parse_record_reference_resolves_a_type_qualifier_through_the_registry() {
    let (dir, number) = parse_record_reference("issue/0028").expect("qualifier must resolve");

    assert_eq!(dir, Some("issues"));
    assert_eq!(number, 28);
}

#[test]
fn parse_record_reference_rejects_an_unknown_type_qualifier() {
    let err = parse_record_reference("glossary/0028")
        .expect_err("an unregistered token must be rejected");

    assert!(err.contains("glossary"), "got: {err}");
    assert!(err.contains("issue"), "got: {err}");
}
