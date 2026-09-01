use super::*;

#[test]
fn wrapped_paragraph_joins_into_one_line() {
    let body = "This is a\nwrapped paragraph.\n";

    assert_eq!(reflow_body(body), "This is a wrapped paragraph.\n");
}

#[test]
fn two_paragraphs_separated_by_a_blank_line_stay_two_lines() {
    let body = "First paragraph\nline two.\n\nSecond paragraph\nline two.\n";

    assert_eq!(
        reflow_body(body),
        "First paragraph line two.\n\nSecond paragraph line two.\n"
    );
}

#[test]
fn list_item_and_its_continuations_join_and_the_next_item_stays_separate() {
    let body = "- Item one\n  continues here\n  and here.\n- Item two\n";

    assert_eq!(
        reflow_body(body),
        "- Item one continues here and here.\n- Item two\n"
    );
}

#[test]
fn numbered_list_items_each_stay_their_own_line() {
    let body = "1. First\n2) Second\n";

    assert_eq!(reflow_body(body), "1. First\n2) Second\n");
}

#[test]
fn fenced_backtick_block_stays_untouched_including_prose_looking_interior() {
    let body = "```\nThis looks like\nprose but isn't.\n```\n";

    assert_eq!(reflow_body(body), body);
}

#[test]
fn fenced_tilde_block_stays_untouched() {
    let body = "~~~\nAlso looks like\nprose but isn't.\n~~~\n";

    assert_eq!(reflow_body(body), body);
}

#[test]
fn table_rows_stay_untouched() {
    let body = "| A | B |\n| - | - |\n| 1 | 2 |\n";

    assert_eq!(reflow_body(body), body);
}

#[test]
fn a_heading_directly_after_a_wrapped_paragraph_stays_separate() {
    let body = "Wrapped over\ntwo lines.\n## Heading\n";

    assert_eq!(reflow_body(body), "Wrapped over two lines.\n## Heading\n");
}

#[test]
fn blockquote_lines_stay_untouched() {
    let body = "> Quoted line one\n> quoted line two.\n";

    assert_eq!(reflow_body(body), body);
}

#[test]
fn thematic_break_and_setext_underline_stay_untouched() {
    let body = "Some text\n---\nMore text\n===\n";

    assert_eq!(reflow_body(body), body);
}

#[test]
fn link_reference_definitions_stay_untouched() {
    let body = "[1]: https://example.com/one\n[2]: https://example.com/two\n";

    assert_eq!(reflow_body(body), body);
}

#[test]
fn multiline_html_comment_stays_untouched_and_prose_after_it_reflows() {
    let body = "<!-- comment\nspanning lines -->\nWrapped after\ncomment.\n";

    assert_eq!(
        reflow_body(body),
        "<!-- comment\nspanning lines -->\nWrapped after comment.\n"
    );
}

#[test]
fn a_two_space_hard_break_stays_exact_and_the_next_line_starts_a_new_block() {
    let body = "First line.  \nSecond starts\nhere.\n";

    assert_eq!(reflow_body(body), "First line.  \nSecond starts here.\n");
}

#[test]
fn a_backslash_hard_break_stays_exact() {
    let body = "Line one\\\nLine two continues.\n";

    assert_eq!(reflow_body(body), "Line one\\\nLine two continues.\n");
}

#[test]
fn indented_code_after_a_blank_line_stays_untouched() {
    let body = "Paragraph text.\n\n    code line one\n    code line two\n";

    assert_eq!(reflow_body(body), body);
}

#[test]
fn an_indented_line_right_after_prose_is_a_lazy_continuation_that_joins() {
    let body = "Wrapped line\n    with an indented continuation.\n";

    assert_eq!(
        reflow_body(body),
        "Wrapped line with an indented continuation.\n"
    );
}

#[test]
fn trailing_newline_is_preserved_when_the_input_has_one() {
    let body = "One.\nTwo.\n";

    assert_eq!(reflow_body(body), "One. Two.\n");
}

#[test]
fn trailing_newline_is_absent_when_the_input_has_none() {
    let body = "One.\nTwo.";

    assert_eq!(reflow_body(body), "One. Two.");
}

#[test]
fn an_empty_body_reflows_to_an_empty_body() {
    assert_eq!(reflow_body(""), "");
}

fn composite_fixture() -> &'static str {
    "Wrapped over\ntwo lines.\n\n- Item one\n  continues here.\n- Item two\n\n```\nfenced code\nstays put\n```\n\n| A | B |\n| - | - |\n\n## Heading\n\n> Quoted line.\n\n---\n\n[1]: https://example.com/one\n\n<!-- comment\nspanning lines -->\n\nEnds with a break.  \nNext block.\n\nParagraph before code.\n\n    indented code\n    stays put\n"
}

#[test]
fn reflow_is_idempotent_over_a_composite_fixture() {
    let fixture = composite_fixture();

    let once = reflow_body(fixture);
    let twice = reflow_body(&once);

    assert_eq!(once, twice);
}
