use super::*;
use crate::test_support::MapStore;
use std::process::ExitCode;

#[test]
fn is_reserved_matches_index_and_log_only() {
    assert!(is_reserved("index.md"));
    assert!(is_reserved("log.md"));
    assert!(!is_reserved("foo.md"));
}

#[test]
fn sibling_record_exists_matches_dash_prefixed_and_bare_forms() {
    let dir = Path::new("docs/adr");
    let all_md = vec![dir.join("0007-old.md"), dir.join("0042.md")];

    assert!(sibling_record_exists(dir, "0007", &all_md));
    assert!(!sibling_record_exists(dir, "9999", &all_md));
    assert!(sibling_record_exists(dir, "0042", &all_md));
}

#[test]
fn sibling_record_exists_ignores_matches_outside_the_directory() {
    let dir = Path::new("docs/adr");
    let all_md = vec![Path::new("docs/bdr").join("0007-old.md")];

    assert!(!sibling_record_exists(dir, "0007", &all_md));
}

fn exit_code_is_success(code: ExitCode) -> bool {
    format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
}

const OLD_ADR: &str = "/bundle/adr/0001-old.md";
const OLD_ADR_SUPERSEDED: &str =
    "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0002\n---\n# Old\n";
const NEW_ADR: &str = "/bundle/adr/0002-new.md";
const NEW_ADR_ACCEPTED: &str = "---\ntype: ADR\nstatus: Accepted\n---\n# New\n";

#[test]
fn check_frontmatter_and_format_accepts_content_the_store_serves_with_no_disk_backing() {
    let (store, all_md) = MapStore::seeded(&[(
        "/bundle/adr/0001-title.md",
        "---\ntype: ADR\n---\n# Title\n",
    )]);
    let root_index = PathBuf::from("/bundle/index.md");
    let mut reporter = Reporter::new();

    check_frontmatter_and_format(&store, &all_md, &root_index, &mut reporter);

    assert!(exit_code_is_success(reporter.finish()));
}

#[test]
fn check_frontmatter_and_format_reports_content_the_store_serves_as_missing_frontmatter() {
    let (store, all_md) = MapStore::seeded(&[("/bundle/adr/0001-title.md", "# No frontmatter\n")]);
    let root_index = PathBuf::from("/bundle/index.md");
    let mut reporter = Reporter::new();

    check_frontmatter_and_format(&store, &all_md, &root_index, &mut reporter);

    assert!(!exit_code_is_success(reporter.finish()));
}

#[test]
fn check_supersede_chain_reports_a_target_absent_from_all_md() {
    let (store, all_md) = MapStore::seeded(&[(OLD_ADR, OLD_ADR_SUPERSEDED)]);
    let mut reporter = Reporter::new();

    check_supersede_chain(&store, &all_md, &mut reporter);

    assert!(!exit_code_is_success(reporter.finish()));
}

#[test]
fn check_supersede_chain_passes_when_the_target_is_present_in_all_md() {
    let (store, all_md) =
        MapStore::seeded(&[(OLD_ADR, OLD_ADR_SUPERSEDED), (NEW_ADR, NEW_ADR_ACCEPTED)]);
    let mut reporter = Reporter::new();

    check_supersede_chain(&store, &all_md, &mut reporter);

    assert!(exit_code_is_success(reporter.finish()));
}

#[test]
fn check_owner_requirement_advises_on_an_adr_without_owner_and_stays_exit_zero() {
    let (store, all_md) = MapStore::seeded(&[(
        "/bundle/adr/0001-title.md",
        "---\ntype: ADR\ntitle: Title\n---\n# Title\n",
    )]);
    let mut reporter = Reporter::new();

    check_owner_requirement(&store, &all_md, false, &mut reporter);

    assert!(exit_code_is_success(reporter.finish()));
}

#[test]
fn check_owner_requirement_reports_a_violation_on_an_adr_without_owner_when_required() {
    let (store, all_md) = MapStore::seeded(&[(
        "/bundle/adr/0001-title.md",
        "---\ntype: ADR\ntitle: Title\n---\n# Title\n",
    )]);
    let mut reporter = Reporter::new();

    check_owner_requirement(&store, &all_md, true, &mut reporter);

    assert!(!exit_code_is_success(reporter.finish()));
}

#[test]
fn check_owner_requirement_passes_when_an_adr_carries_an_owner() {
    let (store, all_md) = MapStore::seeded(&[(
        "/bundle/adr/0001-title.md",
        "---\ntype: ADR\ntitle: Title\nowner: alice\n---\n# Title\n",
    )]);
    let mut reporter = Reporter::new();

    check_owner_requirement(&store, &all_md, true, &mut reporter);

    assert!(exit_code_is_success(reporter.finish()));
}

fn heading_advisories(reporter: &Reporter) -> Vec<&str> {
    reporter
        .advisories
        .iter()
        .filter(|advisory| advisory.kind == "heading")
        .map(|advisory| advisory.message.as_str())
        .collect()
}

fn heading_drift_adr(status: &str) -> (MapStore, Vec<PathBuf>) {
    MapStore::seeded(&[(
        "/bundle/adr/0001-a-decision.md",
        &format!(
            "---\ntype: ADR\ntitle: A Decision\nstatus: {status}\n---\n\n# 0001. Not The Title\n"
        ),
    )])
}

fn assert_heading_drift(status: &str, expected: &[&str]) {
    let (store, all_md) = heading_drift_adr(status);
    let mut reporter = Reporter::new();

    check_heading_matches_title(&store, &all_md, &mut reporter);

    assert_eq!(heading_advisories(&reporter), expected);
}

#[test]
fn check_heading_matches_title_advises_on_a_live_heading_drift() {
    let (store, all_md) = heading_drift_adr("Accepted");
    let mut reporter = Reporter::new();

    check_heading_matches_title(&store, &all_md, &mut reporter);

    let messages = heading_advisories(&reporter);
    assert_eq!(messages.len(), 1);
    assert!(messages[0].contains("HEADING"), "{messages:?}");
}

#[test]
fn check_heading_matches_title_skips_a_deprecated_records_drift() {
    assert_heading_drift("Deprecated", &[]);
}

#[test]
fn check_heading_matches_title_skips_a_superseded_records_drift() {
    assert_heading_drift("Superseded", &[]);
}

#[test]
fn check_heading_matches_title_skips_a_case_insensitive_deprecated_status() {
    assert_heading_drift("deprecated", &[]);
}

#[test]
fn check_supersede_chain_still_reports_a_retired_records_invariant_violation() {
    let (store, all_md) = MapStore::seeded(&[(
        OLD_ADR,
        "---\ntype: ADR\nstatus: Superseded\n---\n# 0001. Old\n",
    )]);
    let mut reporter = Reporter::new();

    check_supersede_chain(&store, &all_md, &mut reporter);

    assert!(!exit_code_is_success(reporter.finish()));
}

#[test]
fn check_owner_requirement_never_flags_a_type_whose_registry_row_does_not_require_it() {
    let (store, all_md) = MapStore::seeded(&[(
        "/bundle/issues/0001-title.md",
        "---\ntype: Issue\ntitle: Title\n---\n# Title\n",
    )]);
    let mut reporter = Reporter::new();

    check_owner_requirement(&store, &all_md, true, &mut reporter);

    assert!(exit_code_is_success(reporter.finish()));
}
