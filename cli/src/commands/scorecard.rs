//! `scorecard` verb wrapper: builds the active-backend store, resolves the
//! optional projection-freshness signal exactly as `search` does, computes
//! the doc-readiness scorecard, and prints it. Always read-only and always
//! exits zero — grading never gates.

use crate::commands::db::derive_project_slug;
use crate::config::{is_default_local_sqlite, Backend, Engine, SQLITE_READ_MODEL_PATH};
use crate::store::{build_backend_store, build_runtime, report_failure};
use living_docs_core::commands::scorecard::{compute, render_json, render_table, Freshness};
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_scorecard(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    json: bool,
) -> ExitCode {
    let store = match build_backend_store(backend, engine, docs_dir) {
        Ok(store) => store,
        Err(err) => return report_failure(&err),
    };
    let freshness = resolve_freshness(engine, docs_dir);
    let scorecard = compute(store.as_ref(), docs_dir, freshness);
    println!(
        "{}",
        if json {
            render_json(&scorecard)
        } else {
            render_table(&scorecard)
        }
    );
    ExitCode::SUCCESS
}

/// The projection-freshness signal `scorecard` grades Trusted with, or
/// `None` when no db projection is configured — mirroring `search`'s own
/// `sync_meta`/fingerprint comparison, but never refusing: an unresolvable
/// engine URL, a missing local index, or a connection failure all read as
/// "not measured" rather than an error, since `scorecard` never fails.
fn resolve_freshness(engine: Engine, docs_dir: &Path) -> Option<Freshness> {
    let url = engine.resolve_url().ok()?;
    if is_default_local_sqlite(engine, &url) && !Path::new(SQLITE_READ_MODEL_PATH).exists() {
        return None;
    }
    let runtime = build_runtime().ok()?;
    runtime.block_on(freshness_from_projection(&url, docs_dir))
}

async fn freshness_from_projection(url: &str, docs_dir: &Path) -> Option<Freshness> {
    let conn = db_store::connect(url).await.ok()?;
    let project_slug = derive_project_slug(docs_dir);
    let meta = db_store::sync_meta::sync_meta(&conn, &project_slug)
        .await
        .ok()??;
    let fingerprint = living_docs_core::fingerprint::tree_fingerprint(docs_dir).ok()?;
    Some(if fingerprint == meta.tree_fingerprint {
        Freshness::Fresh
    } else {
        Freshness::Stale
    })
}
