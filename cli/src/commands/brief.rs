//! `brief` verb wrapper, including its `--from-diff` git-I/O helper.

use crate::config::{Backend, Engine};
use crate::store::{build_backend_store, report_failure};
use living_docs_core::commands;
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_brief(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    from_diff: Option<String>,
) -> ExitCode {
    let diff = match from_diff.map(|range| resolve_diff(&range)).transpose() {
        Ok(diff) => diff,
        Err(err) => return report_failure(&err),
    };
    match build_backend_store(backend, engine, docs_dir) {
        Ok(store) => commands::brief::run(store.as_ref(), docs_dir, doc_type, title, diff.as_ref()),
        Err(err) => report_failure(&err),
    }
}

/// Resolves `--from-diff` in the front so `living-docs-core` stays I/O-free:
/// the touched-file list is exactly `git diff --name-only <range>` against
/// the current working directory's repository.
fn resolve_diff(range: &str) -> Result<commands::brief::DiffContext, String> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", range])
        .output()
        .map_err(|e| format!("failed to run git diff --name-only {range}: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git diff --name-only {range} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    Ok(commands::brief::DiffContext {
        range: range.to_string(),
        files,
    })
}
