//! `check` verb wrapper: resolves the bundle path, compiles the report
//! through `living_docs_core::check::compile`, and renders it as colored
//! text or JSON per the resolved output mode (ADR 0060).

use crate::output::{self, ColorMode, OutputMode, Style};
use crate::store::build_store;
use living_docs_core::check::{self, Report};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub(crate) fn run_check(
    docs_dir: &Path,
    paths: Vec<PathBuf>,
    require_owner: bool,
    mode: OutputMode,
    color: ColorMode,
) -> ExitCode {
    let bundle = check_bundle(docs_dir, paths);
    let Some(report) = check::compile(build_store().as_ref(), &bundle, require_owner) else {
        eprintln!(
            "living-docs check: bundle root not found: {}",
            bundle.display()
        );
        eprintln!(
            "       run from the repo root, or pass the docs directory: living-docs check path/to/docs"
        );
        return ExitCode::from(2);
    };
    if mode.is_json() {
        println!("{}", output::to_json(&report));
    } else {
        render_text(&report, color);
    }
    if report.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn render_text(report: &Report, color: ColorMode) {
    println!("Living Docs lint — bundle: {}", report.bundle);
    println!();
    for advisory in &report.advisories {
        let line = format!("  {:<44} {}", advisory.file, advisory.message);
        println!("{}", output::paint(color, Style::Yellow, &line));
    }
    if !report.advisories.is_empty() {
        println!();
    }
    for violation in &report.violations {
        let line = format!("  {:<44} {}", violation.file, violation.message);
        println!("{}", output::paint(color, Style::Red, &line));
    }
    println!();
    render_verdict(report, color);
}

fn render_verdict(report: &Report, color: ColorMode) {
    if report.ok {
        let line = format!("OK — {} docs, no invariant violations.", report.docs);
        println!("{}", output::paint(color, Style::Green, &line));
    } else {
        let line = format!(
            "FAIL — {} violation(s) across {} docs.",
            report.violations.len(),
            report.docs
        );
        println!("{}", output::paint(color, Style::Red, &line));
    }
}

/// A positional `[BUNDLE_ROOT]` wins when given; otherwise the global
/// `--docs-dir`, so `--docs-dir X fmt`/`check` operates on `X`, never a
/// hardcoded `docs`.
pub(crate) fn check_bundle(docs_dir: &Path, paths: Vec<PathBuf>) -> PathBuf {
    paths
        .into_iter()
        .next()
        .unwrap_or_else(|| docs_dir.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_bundle_uses_the_first_path_argument() {
        let bundle = check_bundle(Path::new("/repo/docs"), vec![PathBuf::from("/bundle")]);
        assert_eq!(bundle, PathBuf::from("/bundle"));
    }

    #[test]
    fn check_bundle_falls_back_to_docs_dir_when_no_paths_are_given() {
        let bundle = check_bundle(Path::new("/repo/custom"), Vec::new());
        assert_eq!(bundle, PathBuf::from("/repo/custom"));
    }
}
