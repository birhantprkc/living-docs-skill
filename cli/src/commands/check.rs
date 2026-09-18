//! `check` verb wrapper: resolves the bundle path, then delegates to `living_docs_core::check::run_require_owner`.

use crate::store::build_store;
use living_docs_core::check;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub(crate) fn run_check(docs_dir: &Path, paths: Vec<PathBuf>, require_owner: bool) -> ExitCode {
    let bundle = check_bundle(docs_dir, paths);
    check::run_require_owner(build_store().as_ref(), &bundle, require_owner)
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
