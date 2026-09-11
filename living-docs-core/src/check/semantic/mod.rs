//! Instruments for the semantic refusal triggers (ADR 0055): the `DIAGRAM`
//! (stale diagram) and `DUPLICATE` (duplicate home) advisories that replace
//! two of the three prose-only hard stops. Both are advisory — the exit code
//! never moves, same posture as `SIZE`/`LIVENESS`/`LEAK` — and both
//! under-report to stay trustworthy (`DIAGRAM` needs a declared scope,
//! `DUPLICATE` a high similarity threshold).

mod diagram;
mod duplicate;

use super::Reporter;
use crate::store::DocStore;
use std::path::{Path, PathBuf};

pub(crate) fn check_semantic(
    store: &dyn DocStore,
    bundle: &Path,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    diagram::check(store, bundle, all_md, reporter);
    duplicate::check(store, all_md, reporter);
}
