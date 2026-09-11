//! Table-driven classifier for doc-trail review findings (issue #58): no LLM,
//! just case-insensitive substring rules loaded from a committed phrasings
//! file. A finding text lands in the first category whose phrase it contains,
//! or `Unclassified` — the classifier reports "don't know" rather than
//! guessing, so a miss is visible instead of silently miscounted.

const PHRASINGS: &str = include_str!("../doc-trail-phrasings.txt");

/// The doc-trail finding categories the ai-configs audit named, plus the
/// explicit "no rule matched" bucket.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    Citation,
    IndexRow,
    Frontmatter,
    EnvelopeProtocol,
    Unclassified,
}

impl Category {
    pub fn is_doc_trail(self) -> bool {
        !matches!(self, Category::Unclassified)
    }

    fn from_token(token: &str) -> Option<Category> {
        match token {
            "citation" => Some(Category::Citation),
            "index-row" => Some(Category::IndexRow),
            "frontmatter" => Some(Category::Frontmatter),
            "envelope-protocol" => Some(Category::EnvelopeProtocol),
            _ => None,
        }
    }
}

/// Classifies one finding's text against the committed phrasing table.
pub fn classify(text: &str) -> Category {
    let haystack = text.to_lowercase();
    rules()
        .into_iter()
        .find(|(_, phrase)| haystack.contains(phrase))
        .map(|(category, _)| category)
        .unwrap_or(Category::Unclassified)
}

fn rules() -> Vec<(Category, String)> {
    PHRASINGS.lines().filter_map(parse_rule).collect()
}

fn parse_rule(line: &str) -> Option<(Category, String)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (token, phrase) = line.split_once('|')?;
    Category::from_token(token.trim()).map(|category| (category, phrase.trim().to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_citation_phrasing_classifies_as_citation() {
        assert_eq!(
            classify("This comment cites an ADR, remove it"),
            Category::Citation
        );
    }

    #[test]
    fn an_index_phrasing_classifies_as_index_row() {
        assert_eq!(
            classify("record is an orphan record, add its index row"),
            Category::IndexRow
        );
    }

    #[test]
    fn a_frontmatter_phrasing_classifies_as_frontmatter() {
        assert_eq!(
            classify("non-canonical frontmatter, run fmt"),
            Category::Frontmatter
        );
    }

    #[test]
    fn an_envelope_phrasing_classifies_as_envelope_protocol() {
        assert_eq!(
            classify("the response envelope is malformed"),
            Category::EnvelopeProtocol
        );
    }

    #[test]
    fn an_unmatched_finding_is_unclassified_not_guessed() {
        let category = classify("the algorithm has an off-by-one error in the loop bound");
        assert_eq!(category, Category::Unclassified);
        assert!(!category.is_doc_trail());
    }

    #[test]
    fn every_doc_trail_category_is_reachable_from_the_table() {
        for expected in [
            Category::Citation,
            Category::IndexRow,
            Category::Frontmatter,
            Category::EnvelopeProtocol,
        ] {
            assert!(
                rules().iter().any(|(c, _)| *c == expected),
                "{expected:?} has no phrasing rule",
            );
        }
    }
}
