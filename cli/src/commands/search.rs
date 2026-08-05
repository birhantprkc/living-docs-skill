//! `search` verb wrapper: queries the read-model and prints ranked hits.

use crate::config::{is_default_local_sqlite, Engine, SQLITE_READ_MODEL_PATH};
use crate::store::{build_runtime, report_failure};
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_search(query: &str, engine: Engine, project: Option<String>) -> ExitCode {
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
    match runtime.block_on(search_read_model(query, &url, project.as_deref())) {
        Ok(hits) => {
            print_hits(&hits);
            ExitCode::SUCCESS
        }
        Err(err) => report_failure(&err.to_string()),
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
