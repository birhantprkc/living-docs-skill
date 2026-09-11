//! End-to-end wiring for the `why` verb (ADR 0051): arg parsing, the query
//! path, and the compiled result reaching stdout; plus the empty-list exit 0.

use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, stdout_of, write};

fn why(docs_dir: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["--docs-dir", docs_dir.to_str().unwrap(), "why"];
    args.extend_from_slice(extra);
    living_docs()
        .args(&args)
        .output()
        .expect("failed to run living-docs why")
}

fn write_impact_adr(bundle: &Path) {
    write(
        bundle,
        "adr/0001-store.md",
        "---\ntype: ADR\ntitle: Store\ndescription: d\nstatus: Accepted\n---\n\n# 0001. Store\n\n## Verification\n\n**Implementation impact:** `src/**`.\n\n**Verification criteria:**\n- the port stays stable\n",
    );
}

#[test]
fn why_reports_the_governing_record_and_its_criteria() {
    let bundle = common::temp_bundle("why", "match");
    write_impact_adr(&bundle);

    let stdout = stdout_of(&why(&bundle, &["--include-stale", "src/store.rs"]));

    assert!(stdout.contains("[ADR 0001]"), "governing record present:\n{stdout}");
    assert!(stdout.contains("- the port stays stable"), "criteria present:\n{stdout}");
}

#[test]
fn why_exits_zero_with_no_output_when_nothing_matches() {
    let bundle = common::temp_bundle("why", "nomatch");
    write_impact_adr(&bundle);

    let output = why(&bundle, &["--include-stale", "unrelated/elsewhere.rs"]);

    assert_eq!(output.status.code(), Some(0), "empty list still exits 0");
    assert!(stdout_of(&output).is_empty(), "no record should match");
}
