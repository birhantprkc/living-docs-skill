use super::*;
use crate::test_support::MapStore;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn store_of(files: &[(&str, &str)]) -> MapStore {
    let mut map = BTreeMap::new();
    for (path, contents) in files {
        map.insert(PathBuf::from(path), (*contents).to_string());
    }
    MapStore { files: map }
}

fn adr_with_impact(number: &str, status: &str, impact: &str, criteria: &str) -> (String, String) {
    let body = format!(
        "# {number}\n\n## Verification\n\n**Implementation impact:** {impact}\n\n**Verification criteria:**\n- {criteria}\n"
    );
    (
        format!("docs/adr/{number}-x.md"),
        format!(
            "---\ntype: ADR\ntitle: ADR {number}\ndescription: d\nstatus: {status}\n---\n\n{body}"
        ),
    )
}

fn query(paths: &[&str], include_stale: bool) -> Query {
    Query {
        paths: paths.iter().map(|p| p.to_string()).collect(),
        include_stale,
    }
}

fn compile_of(files: &[(&str, &str)], q: &Query) -> String {
    let store = store_of(files);
    compile(&store, Path::new("docs"), q)
}

#[test]
fn exact_match_ranks_before_a_glob_match_for_the_same_path() {
    let exact = adr_with_impact("0001", "Accepted", "`src/store.rs`", "keeps the port");
    let glob = adr_with_impact("0002", "Accepted", "`src/**`", "covers the tree");
    let out = compile_of(
        &[(&exact.0, &exact.1), (&glob.0, &glob.1)],
        &query(&["src/store.rs"], true),
    );

    assert!(
        out.contains("[ADR 0001]") && out.contains("[ADR 0002]"),
        "both match:\n{out}"
    );
    assert!(
        out.find("[ADR 0001]").unwrap() < out.find("[ADR 0002]").unwrap(),
        "exact match must come first:\n{out}"
    );
}

#[test]
fn a_directory_prefix_entry_matches_a_file_under_it() {
    let dir = adr_with_impact("0001", "Accepted", "`src/`", "dir rule");
    let out = compile_of(&[(&dir.0, &dir.1)], &query(&["src/deep/mod.rs"], false));
    assert!(out.contains("[ADR 0001]"), "prefix should match:\n{out}");
}

#[test]
fn verification_criteria_are_listed_under_the_record() {
    let a = adr_with_impact(
        "0001",
        "Accepted",
        "`src/store.rs`",
        "the port stays stable",
    );
    let out = compile_of(&[(&a.0, &a.1)], &query(&["src/store.rs"], true));
    assert!(
        out.contains("- the port stays stable"),
        "criteria listed:\n{out}"
    );
}

#[test]
fn no_match_produces_empty_output() {
    let a = adr_with_impact("0001", "Accepted", "`src/store.rs`", "x");
    let out = compile_of(&[(&a.0, &a.1)], &query(&["other/file.rs"], false));
    assert!(out.is_empty(), "no match should be empty:\n{out:?}");
}

#[test]
fn a_superseded_record_never_appears() {
    let a = adr_with_impact("0001", "Superseded", "`src/store.rs`", "x");
    let out = compile_of(&[(&a.0, &a.1)], &query(&["src/store.rs"], false));
    assert!(out.is_empty(), "superseded excluded:\n{out:?}");
}

#[test]
fn a_stale_record_appears_only_under_include_stale() {
    let proposed = (
        "docs/adr/0001-x.md",
        "---\ntype: ADR\ntitle: ADR 0001\ndescription: d\nstatus: Proposed\n---\n\nSee [issue](/issues/0009-t.md).\n\n**Implementation impact:** `src/store.rs`.\n",
    );
    let issue = (
        "docs/issues/0009-t.md",
        "---\ntype: Issue\ntitle: t\nstatus: closed\n---\n\nb",
    );
    let files = [proposed, issue];

    let default = compile_of(&files, &query(&["src/store.rs"], false));
    assert!(
        default.is_empty(),
        "stale excluded by default:\n{default:?}"
    );

    let with_stale = compile_of(&files, &query(&["src/store.rs"], true));
    assert!(
        with_stale.contains("[ADR 0001]"),
        "included under --include-stale:\n{with_stale}"
    );
}

#[test]
fn from_diff_style_multiple_paths_union_the_matching_records() {
    let a = adr_with_impact("0001", "Accepted", "`src/a.rs`", "x");
    let b = adr_with_impact("0002", "Accepted", "`src/b.rs`", "y");
    let files = [(a.0.as_str(), a.1.as_str()), (b.0.as_str(), b.1.as_str())];
    let out = compile_of(&files, &query(&["src/a.rs", "src/b.rs"], true));
    assert!(
        out.contains("[ADR 0001]") && out.contains("[ADR 0002]"),
        "both touched:\n{out}"
    );
}

#[test]
fn glob_star_stays_within_a_segment_but_double_star_crosses() {
    assert!(glob_matches("src/*.rs", "src/a.rs"));
    assert!(!glob_matches("src/*.rs", "src/deep/a.rs"));
    assert!(glob_matches("src/**", "src/deep/a.rs"));
}
