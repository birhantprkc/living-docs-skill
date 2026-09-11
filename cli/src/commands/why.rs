//! `why` verb wrapper: resolves the query paths (a positional path, or the
//! files a `--from-diff` range touched — resolved here so core stays git-free,
//! as `brief` does), builds the store, and delegates to
//! `living_docs_core::commands::why::run`.

use crate::args::WhyArgs;
use crate::config::{Backend, Engine};
use crate::store::{build_backend_store, report_failure};
use living_docs_core::commands::why::{self, Query};
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_why(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    args: WhyArgs,
) -> ExitCode {
    let paths = match resolve_paths(&args) {
        Ok(paths) => paths,
        Err(err) => return report_failure(&err),
    };
    let query = Query {
        paths,
        include_stale: args.include_stale,
    };
    match build_backend_store(backend, engine, docs_dir) {
        Ok(store) => why::run(store.as_ref(), docs_dir, &query),
        Err(err) => report_failure(&err),
    }
}

/// The query paths: a `--from-diff` range's touched files, or the positional
/// path. Exactly one source must be given.
fn resolve_paths(args: &WhyArgs) -> Result<Vec<String>, String> {
    match (&args.from_diff, &args.path) {
        (Some(range), _) => diff_files(range),
        (None, Some(path)) => Ok(vec![path.clone()]),
        (None, None) => Err("why: give a <path> or --from-diff <range>".to_string()),
    }
}

fn diff_files(range: &str) -> Result<Vec<String>, String> {
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
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}
