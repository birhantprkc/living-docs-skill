//! `scorecard` verb wrapper: builds the active-backend store, resolves the
//! optional projection-freshness signal exactly as `search` does, computes
//! the doc-readiness scorecard, prints it, and appends the consumption block
//! (issue #58) from the capture logs when they exist. Always read-only and
//! always exits zero — grading never gates.

use crate::args::ScorecardArgs;
use crate::commands::db::derive_project_slug;
use crate::config::{is_default_local_sqlite, Backend, Engine, SQLITE_READ_MODEL_PATH};
use crate::store::{build_backend_store, build_runtime, report_failure};
use living_docs_core::commands::scorecard::{
    compute, consumption, render_json, render_table, Freshness,
};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

const CONSUMPTION_LOG_ENV: &str = "LIVING_DOCS_CONSUMPTION_LOG";
const FINDINGS_LOG_ENV: &str = "LIVING_DOCS_FINDINGS_LOG";
const DEFAULT_CONSUMPTION_LOG: &str = ".living-docs/consumption.jsonl";

pub(crate) fn run_scorecard(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    args: ScorecardArgs,
) -> ExitCode {
    let store = match build_backend_store(backend, engine, docs_dir) {
        Ok(store) => store,
        Err(err) => return report_failure(&err),
    };
    let freshness = resolve_freshness(engine, docs_dir);
    let scorecard = compute(store.as_ref(), docs_dir, freshness);
    if args.json {
        println!("{}", render_json(&scorecard));
    } else {
        println!("{}", render_table(&scorecard));
        println!("\n{}", consumption_block(&args));
    }
    ExitCode::SUCCESS
}

/// The consumption block from the capture logs, or a "not measured" line when
/// no doc-read capture is configured — the hook is off by default, so an
/// absent log is the normal case, never an error.
fn consumption_block(args: &ScorecardArgs) -> String {
    let Some(reads) = read_capture(CONSUMPTION_LOG_ENV, Some(DEFAULT_CONSUMPTION_LOG)) else {
        return "consumption — not measured (enable the observe-docs-read hook)".to_string();
    };
    let findings = match &args.findings {
        Some(path) => std::fs::read_to_string(path).ok(),
        None => read_capture(FINDINGS_LOG_ENV, None),
    };
    let since = args.since.as_deref().and_then(parse_since_days);
    consumption::render_block(&consumption::summarize(&reads, findings.as_deref(), since))
}

/// Reads a capture log from `$<env>`, falling back to `default` relative to
/// the working directory. `None` when neither is set/readable.
fn read_capture(env: &str, default: Option<&str>) -> Option<String> {
    let path = std::env::var(env)
        .ok()
        .or_else(|| default.map(str::to_string))?;
    fs::read_to_string(path).ok()
}

/// Parses a window like `7d`, `2w`, or a bare day count into days.
fn parse_since_days(spec: &str) -> Option<i64> {
    let spec = spec.trim();
    if let Some(weeks) = spec.strip_suffix('w') {
        return weeks.parse::<i64>().ok().map(|w| w * 7);
    }
    spec.strip_suffix('d').unwrap_or(spec).parse().ok()
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
