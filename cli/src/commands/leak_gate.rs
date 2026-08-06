//! `leak-gate` verb wrapper: always inspects a materialized filesystem bundle.

use std::path::Path;
use std::process::ExitCode;

/// Always inspects a materialized filesystem bundle — a bundle is a directory
/// tree `export` already wrote, not a `--backend`-selectable projection.
/// `check_tier3` threads `--check-tier3` down to the opt-in Tier-3 PII scan.
pub(crate) fn run_leak_gate(bundle: &Path, check_tier3: bool) -> ExitCode {
    living_docs_core::commands::leak_gate::run(&fs_store::FsStore::new(), bundle, check_tier3)
}
