use super::*;

fn adr(body: &str) -> Vec<String> {
    leaks("ADR", body)
}

#[test]
fn a_given_when_then_scenario_in_an_adr_leaks_to_bdr() {
    let body = "## Decision\n\nGiven a record\nWhen it changes\nThen check reports it\n";
    assert!(
        adr(body).iter().any(|m| m.contains("Given/When/Then")),
        "got: {:?}",
        adr(body)
    );
}

#[test]
fn a_given_when_then_scenario_in_a_prd_leaks_to_bdr() {
    let body = "Given input\nWhen processed\nThen output\n";
    assert!(leaks("PRD", body)
        .iter()
        .any(|m| m.contains("Given/When/Then")));
}

#[test]
fn one_stray_given_in_prose_is_not_a_scenario() {
    let body = "## Context\n\nGiven the constraints, we chose the simplest path.\n";
    assert!(adr(body).is_empty(), "got: {:?}", adr(body));
}

#[test]
fn a_given_in_the_verification_block_is_out_of_scope() {
    let body = "## Decision\n\nWe will X.\n\n## Verification\n\n**Verification criteria:**\n- Given A, When B runs, Then C\n";
    assert!(
        adr(body).is_empty(),
        "verification criteria may use Given/When/Then prose: {:?}",
        adr(body)
    );
}

#[test]
fn a_json_fence_in_an_adr_outside_verification_leaks_to_an_issue() {
    let body = "## Consequences\n\n```json\n{\"tests\": 42}\n```\n";
    assert!(
        adr(body).iter().any(|m| m.contains("data/JSON block")),
        "got: {:?}",
        adr(body)
    );
}

#[test]
fn a_json_fence_inside_verification_is_not_a_leak() {
    let body = "## Verification\n\n```json\n{\"tests\": 42}\n```\n";
    assert!(adr(body).is_empty(), "got: {:?}", adr(body));
}

#[test]
fn an_unfilled_placeholder_in_any_record_leaks() {
    assert!(leaks("Research", "## Intro\n\n{{CONTEXT}}\n")
        .iter()
        .any(|m| m.contains("PLACEHOLDER")));
}

#[test]
fn a_placeholder_shown_inside_inline_code_is_not_a_leak() {
    let body = "The marker `{{DECISION}}` is filled by the tool.\n";
    assert!(
        leaks("ADR", body).is_empty(),
        "got: {:?}",
        leaks("ADR", body)
    );
}

#[test]
fn a_bdr_scenario_without_a_proves_line_leaks() {
    let body = "## Scenarios\n\nGiven x\nWhen y\nThen z\n";
    assert!(leaks("BDR", body).iter().any(|m| m.contains("Proves:")));
}

#[test]
fn a_bdr_scenario_with_a_proves_line_is_clean() {
    let body = "## Scenarios\n\nGiven x\nWhen y\nThen z\n\nProves: FR-1\n";
    assert!(!leaks("BDR", body).iter().any(|m| m.contains("Proves:")));
}

#[test]
fn needs_an_adr_in_an_issue_leaks() {
    let body = "## Task\n\nThis needs an ADR (how) and a BDR (behavior).\n";
    let out = leaks("Issue", body);
    assert!(
        out.iter().any(|m| m.contains("deferred decision")),
        "got: {out:?}"
    );
}

#[test]
fn a_clean_issue_has_no_leak() {
    assert!(leaks("Issue", "## Task\n\nThe diff adds a --liveness flag.\n").is_empty());
}
