use crate::doc_type;

/// Compile-time embedded doc templates, keyed by the CLI's doc-type token
/// and looked up through the doc-type registry (ADR 0026). Embedding
/// (rather than reading from disk at runtime) keeps the binary
/// self-contained per ADR 0001.
pub fn template_for(token: &str) -> Option<&'static str> {
    Some(doc_type::spec_for(token)?.template)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_for_returns_the_matching_embedded_template() {
        assert!(template_for("adr").unwrap().starts_with("---\ntype: ADR"));
        assert!(template_for("bdr").unwrap().starts_with("---\ntype: BDR"));
        assert!(template_for("prd").unwrap().starts_with("---\ntype: PRD"));
        assert!(template_for("issue")
            .unwrap()
            .starts_with("---\ntype: Issue"));
    }

    #[test]
    fn template_for_rejects_unknown_types() {
        let unsupported = "glossary";
        assert!(
            doc_type::spec_for(unsupported).is_none(),
            "fixture premise broken: `{unsupported}` is now a registry token — pick another",
        );

        assert_eq!(template_for("bogus"), None);
        assert_eq!(template_for(unsupported), None);
    }
}
