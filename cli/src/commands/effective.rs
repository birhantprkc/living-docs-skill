//! `effective` verb wrapper: resolves the FTS5 relevance order for `--topic`
//! when a fresh search projection is available (ADR 0056), builds the
//! active-backend store, then delegates to
//! `living_docs_core::commands::effective::run` to compile and print the view.

use crate::args::{EffectiveArgs, TierArg};
use crate::commands::db::derive_project_slug;
use crate::config::{is_default_local_sqlite, Backend, Engine, SQLITE_READ_MODEL_PATH};
use crate::store::{build_backend_store, build_runtime, report_failure};
use living_docs_core::commands::effective::{self, Options, Tier};
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_effective(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    args: EffectiveArgs,
) -> ExitCode {
    let ranked_topic = args
        .topic
        .as_deref()
        .and_then(|topic| resolve_ranked_topic(engine, docs_dir, topic));
    let options = Options {
        topic: args.topic,
        tier: tier_of(args.tier),
        budget: args.budget,
        include_stale: args.include_stale,
        ranked_topic,
    };
    match build_backend_store(backend, engine, docs_dir) {
        Ok(store) => effective::run(store.as_ref(), docs_dir, &options),
        Err(err) => report_failure(&err),
    }
}

/// The FTS5 relevance order for `topic` (record paths, `docs_dir`-prefixed to
/// match `DocStore::list`), or `None` — which makes `effective` fall back to
/// its deterministic relevance rank. Never fails: a missing URL, an absent or
/// stale projection, or any DB error all degrade to `None` rather than
/// erroring this read-only verb.
fn resolve_ranked_topic(engine: Engine, docs_dir: &Path, topic: &str) -> Option<Vec<String>> {
    let url = engine.resolve_url().ok()?;
    if is_default_local_sqlite(engine, &url) && !Path::new(SQLITE_READ_MODEL_PATH).exists() {
        return None;
    }
    let runtime = build_runtime().ok()?;
    let slug = derive_project_slug(docs_dir);
    runtime.block_on(ranked_paths(&url, topic, &slug, docs_dir))
}

async fn ranked_paths(url: &str, topic: &str, slug: &str, docs_dir: &Path) -> Option<Vec<String>> {
    if !projection_is_fresh(url, slug, docs_dir).await {
        eprintln!(
            "warning: the search index is behind the records tree; --topic is using a deterministic rank. run: living-docs db sync"
        );
        return None;
    }
    let conn = db_store::connect(url).await.ok()?;
    let hits = db_store::search_in_project(&conn, topic, slug).await.ok()?;
    Some(
        hits.iter()
            .map(|hit| docs_dir.join(&hit.path).display().to_string())
            .collect(),
    )
}

/// True when the projection's `sync_meta` fingerprint matches `docs_dir`'s
/// current records tree — a stale or missing projection reads as not fresh
/// (mirroring `search`'s own staleness check), so `--topic` falls back to the
/// deterministic rank rather than trusting an index the tree has moved past.
async fn projection_is_fresh(url: &str, slug: &str, docs_dir: &Path) -> bool {
    let Ok(conn) = db_store::connect(url).await else {
        return false;
    };
    let Ok(Some(meta)) = db_store::sync_meta::sync_meta(&conn, slug).await else {
        return false;
    };
    living_docs_core::fingerprint::tree_fingerprint(docs_dir)
        .map(|fingerprint| fingerprint == meta.tree_fingerprint)
        .unwrap_or(false)
}

fn tier_of(tier: TierArg) -> Tier {
    match tier {
        TierArg::Index => Tier::Index,
        TierArg::Outline => Tier::Outline,
        TierArg::Full => Tier::Full,
    }
}
