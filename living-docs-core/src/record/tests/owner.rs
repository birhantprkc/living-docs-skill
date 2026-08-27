use super::super::*;
use super::{assert_round_trips, numbered_record};

#[test]
fn extract_record_reads_owner_when_present() {
    let contents = "---\ntype: ADR\ntitle: Owned\nowner: alice\n---\nBody.\n";
    let extracted = extract_record(Path::new("/bundle/adr/0001-owned.md"), contents);

    assert_eq!(extracted.owner, Some("alice".to_owned()));
}

#[test]
fn extract_record_defaults_missing_owner_to_none() {
    let contents = "---\ntype: ADR\ntitle: Unowned\n---\nBody.\n";
    let extracted = extract_record(Path::new("/bundle/adr/0002-unowned.md"), contents);

    assert_eq!(extracted.owner, None);
}

#[test]
fn to_canonical_markdown_emits_owner_immediately_after_description() {
    let record = numbered_record();

    let markdown = to_canonical_markdown(&record);

    let description_index = markdown.find("description:").unwrap();
    let owner_index = markdown.find("owner: carol").unwrap();
    let status_index = markdown.find("status:").unwrap();
    assert!(description_index < owner_index && owner_index < status_index);
}

#[test]
fn to_canonical_markdown_omits_owner_when_absent() {
    let mut record = numbered_record();
    record.owner = None;

    let markdown = to_canonical_markdown(&record);

    assert!(!markdown.contains("owner:"));
}

#[test]
fn to_canonical_markdown_round_trips_a_record_with_owner_byte_identically() {
    let record = numbered_record();

    let markdown = to_canonical_markdown(&record);
    let reparsed = extract_record(Path::new("adr/0001-tailed.md"), &markdown);
    let re_markdown = to_canonical_markdown(&reparsed);

    assert_round_trips(&reparsed, &record);
    assert_eq!(
        frontmatter_block(&markdown),
        frontmatter_block(&re_markdown),
        "the frontmatter block, including owner, is a fixed point"
    );
}

#[test]
fn to_canonical_markdown_round_trips_a_record_without_owner_gaining_no_field() {
    let mut record = numbered_record();
    record.owner = None;

    let markdown = to_canonical_markdown(&record);
    let reparsed = extract_record(Path::new("adr/0001-tailed.md"), &markdown);

    assert_eq!(reparsed.owner, None);
    assert!(!markdown.contains("owner:"));
}
