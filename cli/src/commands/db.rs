//! `db sync` verb wrapper: rebuilds the read-model from the docs bundle.

use crate::config::Engine;
use crate::store::{build_runtime, report_failure};
use living_docs_core::paths;
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_db_sync(docs_dir: &Path, engine: Engine, project: Option<String>) -> ExitCode {
    let url = match engine.resolve_url() {
        Ok(url) => url,
        Err(err) => return report_failure(&err),
    };
    let project_slug = project.unwrap_or_else(|| derive_project_slug(docs_dir));
    let runtime = match build_runtime() {
        Ok(runtime) => runtime,
        Err(err) => return report_failure(&err.to_string()),
    };
    match runtime.block_on(sync_read_model(docs_dir, &url, &project_slug)) {
        Ok(count) => {
            println!("Indexed {count} records. (project: {project_slug})");
            ExitCode::SUCCESS
        }
        Err(err) => report_failure(&err.to_string()),
    }
}

async fn sync_read_model(
    docs_dir: &Path,
    url: &str,
    project_slug: &str,
) -> db_store::Result<usize> {
    let conn = db_store::connect(url).await?;
    db_store::migrate(&conn).await?;
    db_store::sync_project(&conn, &fs_store::FsStore::new(), docs_dir, project_slug).await
}

const DEFAULT_PROJECT_SLUG: &str = "default";

/// Derives a stable project slug from `docs_dir` when `--project` is
/// omitted, so repeated syncs of the same bundle land in the same project.
/// The final path component names the project, UNLESS it is literally
/// `docs` and a parent directory exists — then the parent directory's name
/// is used instead, so every repo's `<repo>/docs` bundle gets a slug unique
/// to that repo rather than every repo colliding on the literal word
/// `docs` (issue 0026). Falls back to `"default"` when no usable component
/// is derivable (e.g. `.` or `/`).
pub(crate) fn derive_project_slug(docs_dir: &Path) -> String {
    let canonical = docs_dir
        .canonicalize()
        .unwrap_or_else(|_| docs_dir.to_path_buf());
    project_slug_component(&canonical)
        .map(paths::slugify)
        .filter(|slug| !slug.is_empty())
        .unwrap_or_else(|| DEFAULT_PROJECT_SLUG.to_owned())
}

/// The path component `derive_project_slug` should slugify: the parent
/// directory's name when `path`'s final component is literally `docs` and
/// a parent name exists, otherwise `path`'s own final component.
fn project_slug_component(path: &Path) -> Option<&str> {
    let final_name = path.file_name()?.to_str()?;
    if final_name == "docs" {
        if let Some(parent_name) = path
            .parent()
            .and_then(Path::file_name)
            .and_then(|n| n.to_str())
        {
            return Some(parent_name);
        }
    }
    Some(final_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_project_slug_uses_the_parent_directory_name_when_the_final_component_is_docs() {
        assert_eq!(
            derive_project_slug(Path::new("/repo-name/docs")),
            "repo-name"
        );
        assert_eq!(derive_project_slug(Path::new("/repo/docs")), "repo");
    }

    #[test]
    fn derive_project_slug_keeps_the_final_component_when_it_is_not_literally_docs() {
        assert_eq!(
            derive_project_slug(Path::new("/repo/client-docs")),
            "client-docs"
        );
    }

    #[test]
    fn derive_project_slug_keeps_docs_when_it_has_no_parent_directory() {
        assert_eq!(derive_project_slug(Path::new("/docs")), "docs");
    }

    #[test]
    fn derive_project_slug_is_stable_across_repeated_calls_on_the_same_dir() {
        let docs_dir = Path::new("/repo/docs");
        assert_eq!(derive_project_slug(docs_dir), derive_project_slug(docs_dir));
    }

    #[test]
    fn derive_project_slug_falls_back_to_default_when_docs_dir_has_no_final_component() {
        assert_eq!(derive_project_slug(Path::new("/")), DEFAULT_PROJECT_SLUG);
        assert_eq!(derive_project_slug(Path::new("")), DEFAULT_PROJECT_SLUG);
    }
}
