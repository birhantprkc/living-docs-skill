//! End-to-end wiring for the `effective` verb (ADR 0050): arg parsing, the
//! `--docs-dir` bundle, and the compiled output reaching stdout.

use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, stdout_of, write};

fn effective(docs_dir: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["--docs-dir", docs_dir.to_str().unwrap(), "effective"];
    args.extend_from_slice(extra);
    living_docs()
        .args(&args)
        .output()
        .expect("failed to run living-docs effective")
}

fn write_chain_bundle(bundle: &Path) {
    write(
        bundle,
        "adr/0001-old.md",
        "---\ntype: ADR\ntitle: Old\ndescription: d\nstatus: Superseded\nsuperseded_by: \"0002\"\n---\n\n# 0001. Old\n\nabout widgets\n",
    );
    write(
        bundle,
        "adr/0002-new.md",
        "---\ntype: ADR\ntitle: New\ndescription: the widget rule in force\nstatus: Accepted\nsupersedes: \"0001\"\n---\n\n# 0002. New\n\nabout widgets\n",
    );
}

#[test]
fn effective_topic_returns_the_head_only_with_a_lineage_line() {
    let bundle = common::temp_bundle("effective", "chain");
    write_chain_bundle(&bundle);

    let stdout = stdout_of(&effective(&bundle, &["--topic", "widget"]));

    assert!(stdout.contains("[ADR 0002]"), "head present:\n{stdout}");
    assert!(stdout.contains("supersedes 0001"), "lineage present:\n{stdout}");
    assert!(!stdout.contains("[ADR 0001]"), "superseded record absent:\n{stdout}");
}
