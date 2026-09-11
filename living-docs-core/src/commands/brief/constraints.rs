//! Per-slot authoring constraints (ADR 0054): the instruction for a judgment
//! slot travels *in the slot*, as a `<!-- hint: … -->` line beside its
//! `<!-- judgment: … -->` marker, so guidance reaches the agent exactly where
//! it writes and disappears once the slot is filled — rather than as prose in
//! a rules file far from the moment of writing. Keyed by `(doc_type, marker)`;
//! a slot with no entry simply carries no hint.
pub(super) fn constraint_for(doc_type: &str, marker: &str) -> Option<&'static str> {
    let constraint = match (doc_type, marker) {
        ("adr", "context") => "the forces that made a decision necessary; <= 80 words; no solution here",
        ("adr", "decision") => "one choice in active voice, and the rejected alternatives named",
        ("adr", "consequences") => "what got easier and the trade-offs accepted; do not restate the code",
        ("bdr", "textual-description") => "what an external observer sees; no rationale (that belongs in an ADR)",
        ("bdr", "scenarios") => "each an observable Given/When/Then an assertion can check, with a `Proves:` requirement id",
        ("prd", "problem-motivation") => "who asked and why now; no solution",
        ("prd", "non-goals") => "what is explicitly out of scope",
        ("prd", "requirements") => "EARS-patterned FR-N/NFR-N; each an NFR is a quality-attribute scenario bound to an instrument",
        ("issue", "context") => "the diff and its acceptance; decide small choices here, never defer with \"Needs an ADR\"",
        ("research", "question") => "one answerable question; the claim must cite an external source",
        _ => return None,
    };
    Some(constraint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_adr_slots_carry_a_constraint() {
        assert!(constraint_for("adr", "context").is_some());
        assert!(constraint_for("adr", "decision").is_some());
        assert!(constraint_for("adr", "consequences").is_some());
    }

    #[test]
    fn an_unmapped_slot_carries_no_hint() {
        assert_eq!(constraint_for("adr", "references"), None);
        assert_eq!(constraint_for("view", "anything"), None);
    }

    #[test]
    fn the_bdr_scenarios_hint_names_the_proves_line() {
        assert!(constraint_for("bdr", "scenarios").unwrap().contains("Proves:"));
    }
}
