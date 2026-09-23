use super::*;
use crate::test_support::MapStore;
use std::collections::BTreeMap;
use std::process::ExitCode;

fn exit_code_is_success(code: ExitCode) -> bool {
    format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
}

#[test]
fn check_callouts_reports_a_retired_record_without_its_callout() {
    let (store, all_md) = MapStore::seeded(&[
        (
            "/bundle/adr/0001-old.md",
            "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0002\n---\n# Old\n",
        ),
        (
            "/bundle/adr/0002-new.md",
            "---\ntype: ADR\nstatus: Accepted\n---\n# New\n",
        ),
    ]);
    let mut reporter = Reporter::new();

    check_callouts(&store, &all_md, &mut reporter);

    let code = reporter.finish();
    assert!(!exit_code_is_success(code));
}

#[test]
fn check_callouts_passes_a_retired_record_whose_callout_matches_expected() {
    let old_path = PathBuf::from("/bundle/adr/0001-old.md");
    let new_path = PathBuf::from("/bundle/adr/0002-new.md");
    let mut files = BTreeMap::new();
    files.insert(old_path.clone(), String::new());
    files.insert(new_path.clone(), "---\ntype: ADR\n---\n# New\n".to_string());
    let probe = MapStore {
        files: files.clone(),
    };
    let successor = callout::successor_filename(&probe, &old_path, "0002");
    let line = callout::expected(Some("Superseded"), Some(successor.as_str())).unwrap();

    files.insert(
        old_path.clone(),
        format!(
            "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0002\n---\n\n{line}\n\n# Old\n"
        ),
    );
    let store = MapStore { files };
    let all_md = vec![old_path, new_path];
    let mut reporter = Reporter::new();

    check_callouts(&store, &all_md, &mut reporter);

    assert!(exit_code_is_success(reporter.finish()));
}

#[test]
fn check_callouts_reports_an_active_record_opening_with_a_retired_callout() {
    let line = callout::expected(Some("Deprecated"), None).unwrap();
    let (store, all_md) = MapStore::seeded(&[(
        "/bundle/adr/0003-active.md",
        &format!("---\ntype: ADR\nstatus: Accepted\n---\n\n{line}\n\n# Active\n"),
    )]);
    let mut reporter = Reporter::new();

    check_callouts(&store, &all_md, &mut reporter);

    assert!(!exit_code_is_success(reporter.finish()));
}

#[test]
fn check_callouts_passes_an_active_record_without_a_callout() {
    let (store, all_md) = MapStore::seeded(&[(
        "/bundle/adr/0004-active.md",
        "---\ntype: ADR\nstatus: Accepted\n---\n# Active\n",
    )]);
    let mut reporter = Reporter::new();

    check_callouts(&store, &all_md, &mut reporter);

    assert!(exit_code_is_success(reporter.finish()));
}
