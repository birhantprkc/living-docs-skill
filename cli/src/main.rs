use args::{Cli, Command, DbCmd, HooksCmd, SkillCmd};
use clap::Parser;
use config::{is_default_local_sqlite, Backend, Engine, SQLITE_READ_MODEL_PATH};
use living_docs_core::{check, paths};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use store::{build_backend_store, build_runtime, report_failure};

mod args;
mod commands;
mod config;
mod hooks;
mod skill;
mod skill_install;
mod store;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Next { doc_type } => commands::next::run_next(&cli.docs_dir, &doc_type),
        Command::New {
            doc_type,
            title,
            description,
        } => commands::new::run_new(
            cli.backend,
            cli.engine,
            &cli.docs_dir,
            &doc_type,
            &title,
            description.as_deref(),
        ),
        Command::Brief {
            doc_type,
            title,
            from_diff,
        } => commands::brief::run_brief(
            cli.backend,
            cli.engine,
            &cli.docs_dir,
            &doc_type,
            &title,
            from_diff,
        ),
        Command::Index {
            doc_type,
            visibility,
        } => {
            commands::index::run_index(cli.backend, cli.engine, &cli.docs_dir, doc_type, visibility)
        }
        Command::Supersede { old, new } => {
            commands::supersede::run_supersede(cli.backend, cli.engine, &cli.docs_dir, &old, &new)
        }
        Command::Status { number, new_status } => commands::status::run_status(
            cli.backend,
            cli.engine,
            &cli.docs_dir,
            &number,
            &new_status,
        ),
        Command::Describe {
            number,
            description,
        } => commands::describe::run_describe(
            cli.backend,
            cli.engine,
            &cli.docs_dir,
            &number,
            &description,
        ),
        Command::Check {
            paths,
            mermaid_only,
        } if mermaid_only => check::run_mermaid_only(&paths),
        Command::Check { paths, .. } => run_check(cli.backend, cli.engine, &cli.docs_dir, paths),
        Command::Fmt { paths } => run_fmt(&cli.docs_dir, paths),
        Command::Export {
            out_dir,
            visibility,
        } => run_export(cli.backend, cli.engine, &cli.docs_dir, &out_dir, visibility),
        Command::LeakGate {
            bundle,
            check_tier3,
        } => run_leak_gate(&bundle, check_tier3),
        Command::Db {
            cmd: DbCmd::Sync { project },
        } => run_db_sync(&cli.docs_dir, cli.engine, project),
        Command::Search { query, project } => run_search(&query, cli.engine, project),
        Command::Skill {
            action:
                Some(SkillCmd::Install {
                    harness,
                    project,
                    dir,
                    dry_run,
                }),
            ..
        } => skill_install::install(harness, project, dir, dry_run),
        Command::Skill {
            name,
            topic,
            list,
            json,
            plain,
            ..
        } => run_skill(name, topic, list, json, plain),
        Command::Hooks {
            cmd: HooksCmd::Install { dir, dry_run },
        } => run_hooks_install(dir, dry_run, &cli.docs_dir),
        Command::Hooks {
            cmd: HooksCmd::Uninstall { dir, dry_run },
        } => run_hooks_uninstall(dir, dry_run),
    }
}

/// Defaults `--dir` to the current directory when omitted, matching every
/// other subcommand's cwd-relative default. `docs_dir` is the CLI's global
/// `--docs-dir` flag, resolved at install time and pinned into the
/// generated `LIVING_DOCS_BUNDLE=` commands (ADR 0020 scope, resolved once
/// here rather than at hook run time).
fn run_hooks_install(dir: Option<PathBuf>, dry_run: bool, docs_dir: &Path) -> ExitCode {
    let project_root = dir.unwrap_or_else(|| PathBuf::from("."));
    hooks::install(&project_root, docs_dir, dry_run)
}

/// Defaults `--dir` to the current directory, mirroring [`run_hooks_install`].
fn run_hooks_uninstall(dir: Option<PathBuf>, dry_run: bool) -> ExitCode {
    let project_root = dir.unwrap_or_else(|| PathBuf::from("."));
    hooks::uninstall(&project_root, dry_run)
}

fn run_check(backend: Backend, engine: Engine, docs_dir: &Path, paths: Vec<PathBuf>) -> ExitCode {
    let bundle = check_bundle(backend, docs_dir, paths);
    match build_backend_store(backend, engine, &bundle) {
        Ok(store) => check::run(store.as_ref(), &bundle),
        Err(err) => report_failure(&err),
    }
}

/// The db backend has no notion of `check`'s `[BUNDLE_ROOT]` positional
/// argument — its `DocStore` is scoped to `--docs-dir` at construction — so
/// it always checks `docs_dir`, ignoring `paths`; the fs backend keeps its
/// existing `lint-docs.sh`-compatible behavior unchanged.
fn check_bundle(backend: Backend, docs_dir: &Path, paths: Vec<PathBuf>) -> PathBuf {
    match backend {
        Backend::Db => docs_dir.to_path_buf(),
        Backend::Fs => paths
            .into_iter()
            .next()
            .unwrap_or_else(|| PathBuf::from("docs")),
    }
}

/// `fmt` is fs-backend only (db-mode is canonical by construction on
/// export), so it needs no `build_backend_store`/`Engine` plumbing — it
/// reuses [`check_bundle`]'s `[BUNDLE_ROOT]` resolution against a fixed
/// [`fs_store::FsStore`], the same way [`run_leak_gate`] always inspects a
/// materialized filesystem bundle regardless of `--backend`.
fn run_fmt(docs_dir: &Path, paths: Vec<PathBuf>) -> ExitCode {
    let bundle = check_bundle(Backend::Fs, docs_dir, paths);
    living_docs_core::commands::fmt::run(&fs_store::FsStore::new(), &bundle)
}

fn run_export(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    out_dir: &Path,
    visibility: Option<Vec<String>>,
) -> ExitCode {
    match build_backend_store(backend, engine, docs_dir) {
        Ok(store) => living_docs_core::commands::export::export(
            store.as_ref(),
            docs_dir,
            out_dir,
            visibility,
        ),
        Err(err) => report_failure(&err),
    }
}

/// Always inspects a materialized filesystem bundle — a bundle is a directory
/// tree `export` already wrote, not a `--backend`-selectable projection.
/// `check_tier3` threads `--check-tier3` down to the opt-in Tier-3 PII scan.
fn run_leak_gate(bundle: &Path, check_tier3: bool) -> ExitCode {
    living_docs_core::commands::leak_gate::run(&fs_store::FsStore::new(), bundle, check_tier3)
}

fn run_db_sync(docs_dir: &Path, engine: Engine, project: Option<String>) -> ExitCode {
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

fn run_search(query: &str, engine: Engine, project: Option<String>) -> ExitCode {
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

/// The resolved output shape for `skill`'s success path (ADR 0014, "output
/// format is context-aware"). Errors are unaffected — they always print
/// plain text to stderr regardless of mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputMode {
    Plain,
    Json,
}

/// Resolves `skill`'s effective [`OutputMode`]: `json`/`plain` are explicit
/// overrides and always win over autodetection, `json` taking precedence if
/// both were somehow set (clap's `conflicts_with` already rejects that
/// combination before this runs). With neither flag given, `is_tty` decides
/// — a TTY (interactive human) defaults to plain text, anything else
/// (piped, an agent consuming the output) defaults to JSON. `is_tty` is a
/// plain parameter, not a live syscall, so this stays unit-testable without
/// a real terminal.
fn resolve_skill_output(json: bool, plain: bool, is_tty: bool) -> OutputMode {
    if json {
        return OutputMode::Json;
    }
    if plain {
        return OutputMode::Plain;
    }
    if is_tty {
        OutputMode::Plain
    } else {
        OutputMode::Json
    }
}

/// `--list` takes priority over `name`/`topic`; otherwise `name` is
/// required and `topic`, when given, narrows the body to one topic's
/// detail (ADR 0014). The resolved [`OutputMode`] swaps every branch's
/// plain-text renderer for its minified-JSON counterpart without changing
/// the selection logic or error handling (errors always stay plain text on
/// stderr).
fn run_skill(
    name: Option<String>,
    topic: Option<String>,
    list: bool,
    json: bool,
    plain: bool,
) -> ExitCode {
    let mode = resolve_skill_output(json, plain, std::io::stdout().is_terminal());
    let as_json = mode == OutputMode::Json;
    if list {
        return print_skill_result(if as_json {
            skill::list_json()
        } else {
            skill::list()
        });
    }
    let Some(name) = name else {
        return report_failure("skill: NAME is required unless --list is given");
    };
    match topic {
        Some(topic) => print_skill_result(if as_json {
            skill::topic_json(&name, &topic)
        } else {
            skill::topic(&name, &topic)
        }),
        None => print_skill_result(if as_json {
            skill::body_json(&name)
        } else {
            skill::body(&name)
        }),
    }
}

fn print_skill_result(result: Result<String, String>) -> ExitCode {
    match result {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(err) => report_failure(&err),
    }
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

    #[test]
    fn check_bundle_uses_docs_dir_for_the_db_backend_ignoring_paths() {
        let bundle = check_bundle(
            Backend::Db,
            Path::new("/repo/docs"),
            vec![PathBuf::from("/ignored")],
        );
        assert_eq!(bundle, PathBuf::from("/repo/docs"));
    }

    #[test]
    fn check_bundle_uses_the_first_path_argument_for_the_fs_backend() {
        let bundle = check_bundle(
            Backend::Fs,
            Path::new("/repo/docs"),
            vec![PathBuf::from("/bundle")],
        );
        assert_eq!(bundle, PathBuf::from("/bundle"));
    }

    #[test]
    fn check_bundle_defaults_to_docs_for_the_fs_backend_when_no_paths_are_given() {
        let bundle = check_bundle(Backend::Fs, Path::new("/repo/docs"), Vec::new());
        assert_eq!(bundle, PathBuf::from("docs"));
    }

    #[test]
    fn resolve_skill_output_json_flag_wins_regardless_of_tty() {
        assert_eq!(resolve_skill_output(true, false, true), OutputMode::Json);
        assert_eq!(resolve_skill_output(true, false, false), OutputMode::Json);
    }

    #[test]
    fn resolve_skill_output_plain_flag_wins_regardless_of_tty() {
        assert_eq!(resolve_skill_output(false, true, true), OutputMode::Plain);
        assert_eq!(resolve_skill_output(false, true, false), OutputMode::Plain);
    }

    #[test]
    fn resolve_skill_output_defaults_to_json_when_stdout_is_not_a_tty() {
        assert_eq!(resolve_skill_output(false, false, false), OutputMode::Json);
    }

    #[test]
    fn resolve_skill_output_defaults_to_plain_when_stdout_is_a_tty() {
        assert_eq!(resolve_skill_output(false, false, true), OutputMode::Plain);
    }
}
