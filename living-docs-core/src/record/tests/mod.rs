use super::*;

mod canonical;
mod extract;
mod owner;

#[allow(clippy::too_many_lines)]
fn numbered_record() -> ExtractedRecord {
    ExtractedRecord {
        doc_type: "ADR".to_owned(),
        number: Some(1),
        concept_id: None,
        identity_kind: NUMBER_IDENTITY_KIND.to_owned(),
        title: "Tailed Decision".to_owned(),
        description: "d.".to_owned(),
        body: "# 0001. Tailed Decision\n\nBody.\n".to_owned(),
        supersedes: Some("0001".to_owned()),
        superseded_by: None,
        tags: vec!["caching".to_owned(), "performance".to_owned()],
        status: Some("Accepted".to_owned()),
        owner: Some("carol".to_owned()),
        frontmatter_tail: vec![
            (
                "labels".to_owned(),
                TailValue::Scalar("important".to_owned()),
            ),
            (
                "blocked_by".to_owned(),
                TailValue::Scalar("0002".to_owned()),
            ),
            (
                "tracker".to_owned(),
                TailValue::Scalar("JIRA-42".to_owned()),
            ),
            (
                "timestamp".to_owned(),
                TailValue::Scalar("2026-07-17T00:00:00Z".to_owned()),
            ),
        ],
    }
}

/// Asserts the canonical round-trip fixed point: `doc_type`, `title`,
/// `description`, typed identity, `supersedes`/`superseded_by`, `tags`,
/// `status`, `owner`, and `frontmatter_tail`. `body` is excluded because
/// the canonical serializer inserts a blank line before it, so a source
/// body that already started with one round-trips with an extra line.
fn assert_round_trips(reparsed: &ExtractedRecord, original: &ExtractedRecord) {
    assert_eq!(reparsed.doc_type, original.doc_type);
    assert_eq!(reparsed.title, original.title);
    assert_eq!(reparsed.description, original.description);
    assert_eq!(reparsed.number, original.number);
    assert_eq!(reparsed.concept_id, original.concept_id);
    assert_eq!(reparsed.identity_kind, original.identity_kind);
    assert_eq!(reparsed.supersedes, original.supersedes);
    assert_eq!(reparsed.superseded_by, original.superseded_by);
    assert_eq!(reparsed.tags, original.tags);
    assert_eq!(reparsed.status, original.status);
    assert_eq!(reparsed.owner, original.owner);
    assert_eq!(reparsed.frontmatter_tail, original.frontmatter_tail);
}

/// A record whose frontmatter tail carries list-valued `labels:` and
/// `blocked_by:` keys — the EAV tail supports both sequence-valued and
/// scalar keys.
fn issue_record_with_list_valued_tail() -> ExtractedRecord {
    ExtractedRecord {
        doc_type: "Issue".to_owned(),
        number: Some(6),
        concept_id: None,
        identity_kind: NUMBER_IDENTITY_KIND.to_owned(),
        title: "Findability Search".to_owned(),
        description: "d.".to_owned(),
        body: "# 0006. Findability Search\n\nBody.\n".to_owned(),
        supersedes: None,
        superseded_by: None,
        tags: vec!["slice".to_owned()],
        status: Some("done".to_owned()),
        owner: None,
        frontmatter_tail: vec![
            (
                "labels".to_owned(),
                TailValue::Sequence(vec![
                    "slice".to_owned(),
                    "skeleton".to_owned(),
                    "refactor".to_owned(),
                ]),
            ),
            ("blocked_by".to_owned(), TailValue::Sequence(Vec::new())),
            (
                "timestamp".to_owned(),
                TailValue::Scalar("2026-07-16T00:00:00Z".to_owned()),
            ),
        ],
    }
}
