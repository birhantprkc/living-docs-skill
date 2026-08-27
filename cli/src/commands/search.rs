//! `search` verb wrapper: queries the read-model and prints ranked hits,
//! warning (or, with `--strict`, refusing) when the projection is behind
//! the records tree.

use crate::commands::db::derive_project_slug;
use crate::config::{is_default_local_sqlite, Engine, SQLITE_READ_MODEL_PATH};
use crate::store::{build_runtime, report_failure};
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_search(
    query: &str,
    engine: Engine,
    project: Option<String>,
    docs_dir: &Path,
    strict: bool,
) -> ExitCode {
    let url = match engine.resolve_url() {
        Ok(url) => url,
        Err(err) => return report_failure(&err),
    };
    if is_default_local_sqlite(engine, &url) && !Path::new(SQLITE_READ_MODEL_PATH).exists() {
        eprintln!("no index found at {SQLITE_READ_MODEL_PATH}; run: living-docs db sync");
        return ExitCode::FAILURE;
    }

    let runtime = match build_runtime() {
        Ok(runtime) => runtime,
        Err(err) => return report_failure(&err.to_string()),
    };
    let project_slug = project
        .clone()
        .unwrap_or_else(|| derive_project_slug(docs_dir));
    if warn_if_stale(&runtime, &url, &project_slug, docs_dir, strict) {
        return ExitCode::FAILURE;
    }

    match runtime.block_on(search_read_model(query, &url, project.as_deref())) {
        Ok(hits) => {
            print_hits(&hits);
            ExitCode::SUCCESS
        }
        Err(err) => report_failure(&err.to_string()),
    }
}

/// Prints one stderr warning naming `db sync` as the fix when the
/// projection is behind `docs_dir`'s records tree, and returns whether the
/// caller must refuse outright (`strict` and stale) — the single
/// staleness-vs-refusal decision [`run_search`] delegates to.
fn warn_if_stale(
    runtime: &tokio::runtime::Runtime,
    url: &str,
    project_slug: &str,
    docs_dir: &Path,
    strict: bool,
) -> bool {
    if !runtime.block_on(is_stale(url, project_slug, docs_dir)) {
        return false;
    }
    eprintln!("warning: the search index may be behind the records tree; run: living-docs db sync");
    strict
}

/// True when the projection's `sync_meta` row for `project_slug` is
/// missing, or its stored fingerprint no longer matches `docs_dir`'s
/// current records tree. A database or filesystem failure here counts as
/// stale rather than panicking this read-only verb.
async fn is_stale(url: &str, project_slug: &str, docs_dir: &Path) -> bool {
    let Ok(conn) = db_store::connect(url).await else {
        return true;
    };
    let Ok(Some(meta)) = db_store::sync_meta::sync_meta(&conn, project_slug).await else {
        return true;
    };
    match living_docs_core::fingerprint::tree_fingerprint(docs_dir) {
        Ok(fingerprint) => fingerprint != meta.tree_fingerprint,
        Err(_) => true,
    }
}

async fn search_read_model(
    query: &str,
    url: &str,
    project: Option<&str>,
) -> db_store::Result<Vec<db_store::SearchHit>> {
    let conn = db_store::connect(url).await?;
    match project {
        Some(slug) => db_store::search_in_project(&conn, query, slug).await,
        None => db_store::search(&conn, query).await,
    }
}

fn print_hits(hits: &[db_store::SearchHit]) {
    for hit in hits {
        println!("[{}] {} — {}", hit.project, hit.path, hit.title);
        println!("{}", hit.snippet);
    }
}
