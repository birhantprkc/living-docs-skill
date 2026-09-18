use super::*;
use crate::test_support::MapStore;
use std::collections::BTreeMap;
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

#[test]
fn check_frontmatter_and_format_accepts_content_the_store_serves_with_no_disk_backing() {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("/bundle/adr/0001-title.md"),
        "---\ntype: ADR\n---\n# Title\n".to_string(),
    );
    let store = MapStore { files };
    let all_md = vec![PathBuf::from("/bundle/adr/0001-title.md")];
    let root_index = PathBuf::from("/bundle/index.md");
    let mut reporter = Reporter::new();

    check_frontmatter_and_format(&store, &all_md, &root_index, &mut reporter);

    assert!(exit_code_is_success(reporter.finish(1)));
}

#[test]
fn check_frontmatter_and_format_reports_content_the_store_serves_as_missing_frontmatter() {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("/bundle/adr/0001-title.md"),
        "# No frontmatter\n".to_string(),
    );
    let store = MapStore { files };
    let all_md = vec![PathBuf::from("/bundle/adr/0001-title.md")];
    let root_index = PathBuf::from("/bundle/index.md");
    let mut reporter = Reporter::new();

    check_frontmatter_and_format(&store, &all_md, &root_index, &mut reporter);

    assert!(!exit_code_is_success(reporter.finish(1)));
}

#[test]
fn check_supersede_chain_reports_a_target_absent_from_all_md() {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("/bundle/adr/0001-old.md"),
        "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0002\n---\n# Old\n".to_string(),
    );
    let store = MapStore { files };
    let all_md = vec![PathBuf::from("/bundle/adr/0001-old.md")];
    let mut reporter = Reporter::new();

    check_supersede_chain(&store, &all_md, &mut reporter);

    assert!(!exit_code_is_success(reporter.finish(1)));
}

#[test]
fn check_supersede_chain_passes_when_the_target_is_present_in_all_md() {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("/bundle/adr/0001-old.md"),
        "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0002\n---\n# Old\n".to_string(),
    );
    files.insert(
        PathBuf::from("/bundle/adr/0002-new.md"),
        "---\ntype: ADR\nstatus: Accepted\n---\n# New\n".to_string(),
    );
    let store = MapStore { files };
    let all_md = vec![
        PathBuf::from("/bundle/adr/0001-old.md"),
        PathBuf::from("/bundle/adr/0002-new.md"),
    ];
    let mut reporter = Reporter::new();

    check_supersede_chain(&store, &all_md, &mut reporter);

    assert!(exit_code_is_success(reporter.finish(2)));
}

#[test]
fn check_owner_requirement_advises_on_an_adr_without_owner_and_stays_exit_zero() {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("/bundle/adr/0001-title.md"),
        "---\ntype: ADR\ntitle: Title\n---\n# Title\n".to_string(),
    );
    let store = MapStore { files };
    let all_md = vec![PathBuf::from("/bundle/adr/0001-title.md")];
    let mut reporter = Reporter::new();

    check_owner_requirement(&store, &all_md, false, &mut reporter);

    assert!(exit_code_is_success(reporter.finish(1)));
}

#[test]
fn check_owner_requirement_reports_a_violation_on_an_adr_without_owner_when_required() {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("/bundle/adr/0001-title.md"),
        "---\ntype: ADR\ntitle: Title\n---\n# Title\n".to_string(),
    );
    let store = MapStore { files };
    let all_md = vec![PathBuf::from("/bundle/adr/0001-title.md")];
    let mut reporter = Reporter::new();

    check_owner_requirement(&store, &all_md, true, &mut reporter);

    assert!(!exit_code_is_success(reporter.finish(1)));
}

#[test]
fn check_owner_requirement_passes_when_an_adr_carries_an_owner() {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("/bundle/adr/0001-title.md"),
        "---\ntype: ADR\ntitle: Title\nowner: alice\n---\n# Title\n".to_string(),
    );
    let store = MapStore { files };
    let all_md = vec![PathBuf::from("/bundle/adr/0001-title.md")];
    let mut reporter = Reporter::new();

    check_owner_requirement(&store, &all_md, true, &mut reporter);

    assert!(exit_code_is_success(reporter.finish(1)));
}

#[test]
fn check_owner_requirement_never_flags_a_type_whose_registry_row_does_not_require_it() {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("/bundle/issues/0001-title.md"),
        "---\ntype: Issue\ntitle: Title\n---\n# Title\n".to_string(),
    );
    let store = MapStore { files };
    let all_md = vec![PathBuf::from("/bundle/issues/0001-title.md")];
    let mut reporter = Reporter::new();

    check_owner_requirement(&store, &all_md, true, &mut reporter);

    assert!(exit_code_is_success(reporter.finish(1)));
}
