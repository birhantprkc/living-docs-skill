use super::*;

fn read_line(ts: &str, tokens: u64, status: &str) -> String {
    format!("{{\"ts\":\"{ts}\",\"path\":\"docs/adr/0001-x.md\",\"tokens\":{tokens},\"status\":\"{status}\"}}")
}

#[test]
fn summarize_computes_token_sum_median_p90_and_stale_reads() {
    let jsonl = [
        read_line("2026-09-01T00:00:00Z", 100, "Accepted"),
        read_line("2026-09-01T00:00:00Z", 200, "Superseded"),
        read_line("2026-09-01T00:00:00Z", 300, "Deprecated"),
        read_line("2026-09-01T00:00:00Z", 400, "Accepted"),
    ]
    .join("\n");
    let c = summarize(&jsonl, None, None);
    assert_eq!(c.reads, 4);
    assert_eq!(c.tokens_sum, 1000);
    assert_eq!(c.stale_reads, 2, "Superseded + Deprecated");
    assert_eq!(c.tokens_median, 300);
    assert_eq!(c.tokens_p90, 400);
}

#[test]
fn since_filters_to_the_window_ending_at_the_newest_read() {
    let jsonl = [
        read_line("2026-01-01T00:00:00Z", 100, "Accepted"),
        read_line("2026-09-20T00:00:00Z", 200, "Accepted"),
        read_line("2026-09-25T00:00:00Z", 300, "Accepted"),
    ]
    .join("\n");
    let c = summarize(&jsonl, None, Some(30));
    assert_eq!(c.reads, 2, "only the two September reads fall within 30 days of the newest");
    assert_eq!(c.tokens_sum, 500);
}

#[test]
fn missing_capture_reads_as_zero_not_an_error() {
    let c = summarize("", None, None);
    assert_eq!(c.reads, 0);
    assert_eq!(c.tokens_median, 0);
    assert!(c.findings.is_none());
}

#[test]
fn a_malformed_line_is_skipped_rather_than_failing() {
    let jsonl = format!("not json\n{}", read_line("2026-09-01T00:00:00Z", 42, "Accepted"));
    let c = summarize(&jsonl, None, None);
    assert_eq!(c.reads, 1);
    assert_eq!(c.tokens_sum, 42);
}

#[test]
fn findings_share_counts_doc_trail_and_unclassified() {
    let findings = [
        "{\"text\":\"this comment cites an ADR\"}",
        "{\"text\":\"non-canonical frontmatter\"}",
        "{\"text\":\"off-by-one in the loop bound\"}",
    ]
    .join("\n");
    let c = summarize("", Some(&findings), None);
    let share = c.findings.expect("findings supplied");
    assert_eq!(share.total, 3);
    assert_eq!(share.doc_trail, 2);
    assert_eq!(share.unclassified, 1);
}

#[test]
fn render_block_names_the_three_metrics() {
    let jsonl = read_line("2026-09-01T00:00:00Z", 100, "Superseded");
    let block = render_block(&summarize(&jsonl, None, None));
    assert!(block.contains("consumption —"));
    assert!(block.contains("1 doc reads"));
    assert!(block.contains("1 stale reads"));
    assert!(block.contains("findings: not measured"));
}
