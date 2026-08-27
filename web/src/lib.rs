//! Library surface shared between the `living-docs-web` binary and its
//! integration tests: the read-only axum router, its `GET /` search handler
//! (staleness-flagged, see [`staleness_for`]), its `GET /record/{*path}`
//! record handler, its `GET /style.css` stylesheet route, the Cmd+K
//! palette's `GET /palette.js`/`GET /palette` endpoints, and — mounted only
//! when the caller supplies an [`AuthoringConfig`] — the `/new`/`/edit`/
//! `/supersede`/`/delete` authoring routes, which commit through
//! `db_store::DbDocStore`'s checked write methods via
//! [`tokio::task::spawn_blocking`].

mod create;
pub mod views;

use std::path::PathBuf;

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::Router;
use db_store::{NavEntry, ProjectView, RecordMeta, RecordView, SearchHit};
use maud::{html, Markup};
use sea_orm::DatabaseConnection;
use serde::Deserialize;

const STYLESHEET: &str = include_str!("style.css");
const PALETTE_SCRIPT: &str = include_str!("palette.js");

/// Query-string parameters accepted by the search page. `project`, when
/// present and non-empty, narrows the search to one project's slug (ADR
/// 0005, issue 0005 slice 0005-C2); omitted or blank spans every project.
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    q: Option<String>,
    project: Option<String>,
}

/// The web front's authoring configuration: the SQLite/FTS5 read-model's
/// connection URL and the docs bundle root a `db_store::DbDocStore` opens
/// against inside the authoring handlers, and [`staleness_for`] recomputes
/// a fresh fingerprint from. Constructed by the binary only for
/// `--backend db`; its absence keeps `/new` unregistered in file-mode.
#[derive(Clone)]
pub struct AuthoringConfig {
    pub db_url: String,
    pub docs_root: PathBuf,
}

/// The axum router's shared state: the read-model connection every route
/// uses, plus the optional [`AuthoringConfig`] that gates whether `/new` is
/// registered and whether [`staleness_for`] has a docs root to check
/// against. A single `Clone` struct because axum's `State` extractor
/// requires one state type per router.
#[derive(Clone)]
struct AppState {
    conn: DatabaseConnection,
    authoring: Option<AuthoringConfig>,
}

/// Builds the router backed by `conn`. Always exposes the read-only
/// `GET /`, `GET /record/{*path}`, `GET /style.css`, `GET /palette.js`, and
/// `GET /palette` routes; additionally mounts the `/new`/`/edit`/
/// `/supersede`/`/delete` authoring routes only when `authoring` is `Some`
/// — in file-mode a request to any of them 404s like any unknown path,
/// since the routes are never registered rather than refused at runtime.
pub fn build_router(conn: DatabaseConnection, authoring: Option<AuthoringConfig>) -> Router {
    let mut router = Router::new()
        .route("/", get(search_handler))
        .route("/record/{*path}", get(record_handler))
        .route("/style.css", get(style_handler))
        .route("/palette.js", get(palette_script_handler))
        .route("/palette", get(palette_handler));

    if authoring.is_some() {
        router = router
            .route("/new", get(new_form_handler).post(create::create_handler))
            .route("/edit/{*path}", get(edit_form_handler).post(edit_handler))
            .route("/supersede/{*path}", axum::routing::post(supersede_handler))
            .route("/delete/{*path}", axum::routing::post(delete_handler));
    }

    router.with_state(AppState { conn, authoring })
}

async fn style_handler() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css")], STYLESHEET)
}

async fn palette_script_handler() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript")],
        PALETTE_SCRIPT,
    )
}

async fn palette_handler(
    State(AppState { conn, .. }): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Markup {
    let query = params.q.filter(|value| !value.trim().is_empty());
    let hits = match &query {
        Some(term) => search_or_log(&conn, term, None).await,
        None => Vec::new(),
    };
    views::palette_fragment(&hits)
}

async fn search_handler(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Markup {
    let query = params.q.filter(|value| !value.trim().is_empty());
    let project = params.project.filter(|value| !value.trim().is_empty());
    let hits = match &query {
        Some(term) => search_or_log(&state.conn, term, project.as_deref()).await,
        None => Vec::new(),
    };
    let projects = list_projects_or_log(&state.conn).await;
    let nav = nav_entries_or_log(&state.conn).await;
    let stale = staleness_for(&state, project.as_deref()).await;
    let main = views::search_page(query.as_deref(), project.as_deref(), &projects, &hits);
    let main = with_staleness_banner(stale, main);
    views::shell("living-docs search", &nav, None, main, None)
}

/// `false` whenever [`AppState::authoring`] is configured: db-mode authoring
/// writes commit straight to the projection, so it is canonical there and
/// carries no staleness to report. Otherwise `true` when
/// `project`'s (`"default"` when unset) `sync_meta` row, or its recorded
/// `root_path`, is missing, or no longer matches a fresh
/// [`living_docs_core::fingerprint`] of that root.
async fn staleness_for(state: &AppState, project: Option<&str>) -> bool {
    if state.authoring.is_some() {
        return false;
    }
    let project_slug = project.unwrap_or("default");
    let Ok(Some(meta)) = db_store::sync_meta::sync_meta(&state.conn, project_slug).await else {
        return true;
    };
    let Ok(Some(root_path)) = db_store::project_root_path(&state.conn, project_slug).await else {
        return true;
    };
    match living_docs_core::fingerprint::tree_fingerprint(&root_path) {
        Ok(fingerprint) => fingerprint != meta.tree_fingerprint,
        Err(_) => true,
    }
}

/// Prepends a visible staleness marker above `main` when `stale`; returns
/// `main` unchanged otherwise.
fn with_staleness_banner(stale: bool, main: Markup) -> Markup {
    if !stale {
        return main;
    }
    html! {
        div class="staleness-banner" {
            "Search results may be stale — run `living-docs db sync`."
        }
        (main)
    }
}

async fn search_or_log(
    conn: &DatabaseConnection,
    query: &str,
    project: Option<&str>,
) -> Vec<SearchHit> {
    let result = match project {
        Some(slug) => db_store::search_in_project(conn, query, slug).await,
        None => db_store::search(conn, query).await,
    };
    match result {
        Ok(hits) => hits,
        Err(err) => {
            eprintln!("search query {query:?} (project: {project:?}) failed: {err}");
            Vec::new()
        }
    }
}

async fn list_projects_or_log(conn: &DatabaseConnection) -> Vec<ProjectView> {
    match db_store::list_projects(conn).await {
        Ok(projects) => projects,
        Err(err) => {
            eprintln!("listing projects failed: {err}");
            Vec::new()
        }
    }
}

async fn nav_entries_or_log(conn: &DatabaseConnection) -> Vec<NavEntry> {
    match db_store::records_by_type(conn).await {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("listing nav entries failed: {err}");
            Vec::new()
        }
    }
}

async fn record_handler(State(state): State<AppState>, Path(path): Path<String>) -> Response {
    record_page_response(
        &state,
        &path,
        SupersedeSubmission::default(),
        DeleteSubmission::default(),
    )
    .await
}

/// A supersede confirm form submission's render state: the number last
/// typed into the `new` field and, when just rejected, the error to show.
#[derive(Default)]
struct SupersedeSubmission<'a> {
    value: &'a str,
    error: Option<&'a str>,
}

/// A delete confirm form submission's render state, mirroring
/// [`SupersedeSubmission`] minus its `value` field.
#[derive(Default)]
struct DeleteSubmission<'a> {
    error: Option<&'a str>,
}

/// Renders `path`'s full record page — body, metadata panel, and (in
/// db-mode authoring) the Edit link, the supersede confirm form, and the
/// delete confirm form — the shared render every rejected-submission
/// re-render also uses, so no call site duplicates this lookup-and-render
/// sequence. 404s like [`not_found_response`] when no record exists at
/// `path`. The delete form is never shown for an already soft-deleted
/// record, or when no record/meta was found at all.
async fn record_page_response(
    state: &AppState,
    path: &str,
    supersede_submission: SupersedeSubmission<'_>,
    delete_submission: DeleteSubmission<'_>,
) -> Response {
    let nav = nav_entries_or_log(&state.conn).await;
    let Some(record) = record_or_log(&state.conn, path).await else {
        return not_found_response(&state.conn).await;
    };
    let meta = record_meta_or_log(&state.conn, path).await;
    let body_html = render_markdown(&record.body);
    let title = format!("{} — living-docs", record.title);
    let edit_href = state.authoring.is_some().then(|| format!("/edit/{path}"));
    let supersede_href = state
        .authoring
        .is_some()
        .then(|| format!("/supersede/{path}"));
    let supersede = supersede_href
        .as_deref()
        .map(|href| views::SupersedeFormState {
            href,
            value: supersede_submission.value,
            error: supersede_submission.error,
        });
    let delete_href = delete_href_for(state, path, meta.as_ref());
    let delete = delete_href.as_deref().map(|href| views::DeleteFormState {
        href,
        error: delete_submission.error,
    });
    let main = views::record_page(&body_html, edit_href.as_deref(), supersede, delete);
    let aside = meta.map(|meta| views::metadata_panel(&meta));
    let shell = views::shell(&title, &nav, Some(path), main, aside);
    (StatusCode::OK, shell).into_response()
}

/// `/delete/{path}`, when authoring is configured AND `meta` shows a record
/// that is not already soft-deleted — `None` when authoring is
/// unconfigured, no meta was found, or `deleted_at` is already set.
fn delete_href_for(state: &AppState, path: &str, meta: Option<&RecordMeta>) -> Option<String> {
    state.authoring.as_ref()?;
    let is_deleted = meta.is_none_or(|meta| meta.deleted_at.is_some());
    (!is_deleted).then(|| format!("/delete/{path}"))
}

/// The center-pane content shown when a path resolves to no record —
/// [`record_handler`]'s own 404, and reused by the `/edit` handlers below so
/// every "no record at that path" response renders identically.
async fn not_found_response(conn: &DatabaseConnection) -> Response {
    let nav = nav_entries_or_log(conn).await;
    let shell = views::shell(
        "Not found — living-docs",
        &nav,
        None,
        views::not_found(),
        None,
    );
    (StatusCode::NOT_FOUND, shell).into_response()
}

/// `GET /new`'s handler, mounted only when [`AuthoringConfig`] is `Some`
/// (see [`build_router`]): renders an empty [`views::create_form`] inside
/// the same three-pane shell every other page uses.
async fn new_form_handler(State(AppState { conn, .. }): State<AppState>) -> Markup {
    let nav = nav_entries_or_log(&conn).await;
    let main = views::create_form(None, None, None);
    views::shell("New record — living-docs", &nav, None, main, None)
}

/// `POST /new`'s submitted fields: the doc-type token
/// (`adr`/`bdr`/`prd`/`issue`) and the record's title.
#[derive(Debug, Deserialize)]
pub struct CreateForm {
    doc_type: String,
    title: String,
}

pub(crate) fn replace_title_line(line: &str, title: &str) -> Option<String> {
    let rest = line.strip_prefix("title:")?;
    let quoted = yaml_double_quoted(title);
    match rest.find('#') {
        Some(hash) => Some(format!("title: {quoted} {}", &rest[hash..])),
        None => Some(format!("title: {quoted}")),
    }
}

/// A YAML double-quoted scalar for `value`: backslashes and double quotes
/// are escaped and newlines collapse to a space, so a free-text title
/// (unlike `plan`'s own type/status/timestamp fills, which never carry
/// user-controlled content) can never produce malformed frontmatter.
fn yaml_double_quoted(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push(' '),
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
}

/// `target_path` (docs-root-joined) rendered relative to `docs_root`, so a
/// freshly created record's redirect target matches what
/// `GET /record/{*path}` reads it back at.
pub(crate) fn relative_record_path(
    docs_root: &std::path::Path,
    target_path: &std::path::Path,
) -> String {
    target_path
        .strip_prefix(docs_root)
        .unwrap_or(target_path)
        .to_string_lossy()
        .into_owned()
}

pub(crate) async fn create_form_response(
    conn: &DatabaseConnection,
    doc_type: &str,
    title: &str,
    error: &str,
) -> Response {
    let nav = nav_entries_or_log(conn).await;
    let main = views::create_form(Some(doc_type), Some(title), Some(error));
    let shell = views::shell("New record — living-docs", &nav, None, main, None);
    (StatusCode::OK, shell).into_response()
}

/// `GET /edit/{*path}`'s handler, mounted only when [`AuthoringConfig`] is
/// `Some`: loads the record's current content and `revision`, pre-fills
/// [`views::edit_form`] with them, and 404s like [`record_handler`] when no
/// record exists at `path`.
async fn edit_form_handler(State(state): State<AppState>, Path(path): Path<String>) -> Response {
    let authoring = state
        .authoring
        .clone()
        .expect("edit_form_handler is only mounted when authoring is configured");

    match spawn_read_with_revision(&authoring, &path).await {
        Ok(Some((content, revision))) => {
            edit_form_response(&state.conn, &path, &content, revision, None).await
        }
        Ok(None) => not_found_response(&state.conn).await,
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal error loading record",
        )
            .into_response(),
    }
}

/// `POST /edit/{*path}`'s submitted fields: the edited markdown source and
/// the `revision` the editor last read, carried as a hidden field so
/// [`db_store::DbDocStore::update_checked`] can enforce ADR 0016's
/// optimistic-concurrency precondition.
#[derive(Debug, Deserialize)]
pub struct EditForm {
    content: String,
    base_revision: i64,
}

/// Every way `POST /edit/{*path}`'s handler can fail to commit an edit:
/// opening the store itself failed, or
/// [`db_store::DbDocStore::update_checked`]'s own
/// [`db_store::WriteCheckedError`].
#[derive(Debug)]
enum EditError {
    Open(String),
    Write(db_store::WriteCheckedError),
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditError::Open(message) => write!(f, "{message}"),
            EditError::Write(err) => write!(f, "{err}"),
        }
    }
}

/// Normalizes `value`'s line endings to bare `\n`: browsers submit
/// `<textarea>` content with CRLF, but living-docs-core's frontmatter
/// parser splits on bare `\n`, so a stray `\r` breaks the match.
fn normalize_line_endings(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

/// `POST /edit/{*path}`'s handler, mounted only when [`AuthoringConfig`] is
/// `Some`. On success, redirects to the record's page; on a stale
/// `base_revision`, re-renders [`views::edit_form`] with the CURRENT server
/// content, never the user's rejected submission; on any other
/// [`EditError`], re-renders with the user's own submitted content
/// preserved so they can fix and resubmit.
async fn edit_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
    axum::Form(input): axum::Form<EditForm>,
) -> Response {
    let authoring = state
        .authoring
        .clone()
        .expect("edit_handler is only mounted when authoring is configured");
    let EditForm {
        content,
        base_revision,
    } = input;
    let content = normalize_line_endings(&content);

    let outcome = spawn_update_checked(&authoring, &path, &content, base_revision).await;

    match outcome {
        Ok(Ok(_revision)) => Redirect::to(&views::record_href(&path)).into_response(),
        Ok(Err(err)) => {
            edit_error_response(&state, &authoring, &path, &content, base_revision, err).await
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal error editing record",
        )
            .into_response(),
    }
}

/// Runs [`db_store::DbDocStore::read_with_revision`] inside
/// [`tokio::task::spawn_blocking`], folding "no record at `path`" into
/// `Ok(None)` rather than an [`EditError`] — the read side of an edit has
/// no error to preserve a failed submission against, unlike the write side.
async fn spawn_read_with_revision(
    authoring: &AuthoringConfig,
    path: &str,
) -> std::result::Result<Option<(String, i64)>, tokio::task::JoinError> {
    let db_url = authoring.db_url.clone();
    let docs_root = authoring.docs_root.clone();
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || read_record_with_revision(&db_url, &docs_root, &path)).await
}

fn read_record_with_revision(
    db_url: &str,
    docs_root: &std::path::Path,
    path: &str,
) -> Option<(String, i64)> {
    let store = db_store::DbDocStore::new(db_url, docs_root.to_path_buf()).ok()?;
    store.read_with_revision(std::path::Path::new(path)).ok()
}

/// Runs [`update_record`] inside [`tokio::task::spawn_blocking`], mirroring
/// [`create_handler`]'s own bridge from this async handler onto
/// `DbDocStore`'s synchronous SeaORM runtime.
async fn spawn_update_checked(
    authoring: &AuthoringConfig,
    path: &str,
    content: &str,
    base_revision: i64,
) -> std::result::Result<std::result::Result<i64, EditError>, tokio::task::JoinError> {
    let db_url = authoring.db_url.clone();
    let docs_root = authoring.docs_root.clone();
    let path = path.to_owned();
    let content = content.to_owned();
    tokio::task::spawn_blocking(move || {
        update_record(&db_url, &docs_root, &path, &content, base_revision)
    })
    .await
}

fn update_record(
    db_url: &str,
    docs_root: &std::path::Path,
    path: &str,
    content: &str,
    base_revision: i64,
) -> std::result::Result<i64, EditError> {
    let store = db_store::DbDocStore::new(db_url, docs_root.to_path_buf())
        .map_err(|err| EditError::Open(err.to_string()))?;
    store
        .update_checked(std::path::Path::new(path), content, Some(base_revision))
        .map_err(EditError::Write)
}

/// Routes an [`EditError`] to the right re-render: a stale `base_revision`
/// reloads the CURRENT server content (never the rejected submission, ADR
/// 0016); any other error preserves the user's own submitted `content`/
/// `base_revision` so they can fix and resubmit.
async fn edit_error_response(
    state: &AppState,
    authoring: &AuthoringConfig,
    path: &str,
    content: &str,
    base_revision: i64,
    err: EditError,
) -> Response {
    let message = err.to_string();
    if matches!(
        err,
        EditError::Write(db_store::WriteCheckedError::StaleRevision { .. })
    ) {
        return stale_edit_response(state, authoring, path, &message).await;
    }
    edit_form_response(&state.conn, path, content, base_revision, Some(&message)).await
}

/// Re-fetches `path`'s CURRENT server content and revision and re-renders
/// [`views::edit_form`] with them plus `message`. Falls back to
/// [`not_found_response`] if the record was deleted since the rejected edit.
async fn stale_edit_response(
    state: &AppState,
    authoring: &AuthoringConfig,
    path: &str,
    message: &str,
) -> Response {
    match spawn_read_with_revision(authoring, path)
        .await
        .ok()
        .flatten()
    {
        Some((content, revision)) => {
            edit_form_response(&state.conn, path, &content, revision, Some(message)).await
        }
        None => not_found_response(&state.conn).await,
    }
}

async fn edit_form_response(
    conn: &DatabaseConnection,
    path: &str,
    content: &str,
    base_revision: i64,
    error: Option<&str>,
) -> Response {
    let nav = nav_entries_or_log(conn).await;
    let main = views::edit_form(path, content, base_revision, error);
    let shell = views::shell("Edit record — living-docs", &nav, Some(path), main, None);
    (StatusCode::OK, shell).into_response()
}

/// `POST /supersede/{*path}`'s submitted field: the superseding record's
/// bare number (issue 0012) — matching the CLI's own `living-docs
/// supersede` argument shape, never a path.
#[derive(Debug, Deserialize)]
pub struct SupersedeForm {
    new: String,
}

/// Every way `POST /supersede/{*path}`'s handler can fail to commit: reading
/// the current record or deriving its number failed, or
/// [`db_store::DbDocStore::supersede_checked`]'s own
/// [`db_store::SupersedeCheckedError`].
#[derive(Debug)]
enum SupersedeError {
    Open(String),
    Write(db_store::SupersedeCheckedError),
}

impl std::fmt::Display for SupersedeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupersedeError::Open(message) => write!(f, "{message}"),
            SupersedeError::Write(err) => write!(f, "{err}"),
        }
    }
}

/// `POST /supersede/{*path}`'s handler, mounted only when
/// [`AuthoringConfig`] is `Some`. On success, redirects to `path`'s own
/// record page; on a [`SupersedeError`], re-renders `path`'s record page
/// with the error visible and the submitted `new` value preserved.
async fn supersede_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
    axum::Form(input): axum::Form<SupersedeForm>,
) -> Response {
    let authoring = state
        .authoring
        .clone()
        .expect("supersede_handler is only mounted when authoring is configured");
    let new = input.new.trim().to_owned();

    let outcome = spawn_supersede_checked(&authoring, &path, &new).await;

    match outcome {
        Ok(Ok(())) => Redirect::to(&views::record_href(&path)).into_response(),
        Ok(Err(err)) => {
            let message = err.to_string();
            record_page_response(
                &state,
                &path,
                SupersedeSubmission {
                    value: &new,
                    error: Some(&message),
                },
                DeleteSubmission::default(),
            )
            .await
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal error superseding record",
        )
            .into_response(),
    }
}

/// Runs [`supersede_record`] inside [`tokio::task::spawn_blocking`],
/// mirroring [`spawn_update_checked`]'s own bridge from this async handler
/// onto `DbDocStore`'s synchronous SeaORM runtime.
async fn spawn_supersede_checked(
    authoring: &AuthoringConfig,
    path: &str,
    new: &str,
) -> std::result::Result<std::result::Result<(), SupersedeError>, tokio::task::JoinError> {
    let db_url = authoring.db_url.clone();
    let docs_root = authoring.docs_root.clone();
    let path = path.to_owned();
    let new = new.to_owned();
    tokio::task::spawn_blocking(move || supersede_record(&db_url, &docs_root, &path, &new)).await
}

/// Reads `path`'s current content to derive its own record number (the
/// `old` argument [`db_store::DbDocStore::supersede_checked`] needs — the
/// URL carries the record's path, not its bare number), then delegates to
/// `supersede_checked` with `new` exactly as submitted.
fn supersede_record(
    db_url: &str,
    docs_root: &std::path::Path,
    path: &str,
    new: &str,
) -> std::result::Result<(), SupersedeError> {
    let store = db_store::DbDocStore::new(db_url, docs_root.to_path_buf())
        .map_err(|err| SupersedeError::Open(err.to_string()))?;
    let (content, _revision) = store
        .read_with_revision(std::path::Path::new(path))
        .map_err(|err| SupersedeError::Open(err.to_string()))?;
    let old_number = record_number(path, &content).ok_or_else(|| {
        SupersedeError::Open(format!("{path}: record carries no number identity"))
    })?;
    store
        .supersede_checked(&old_number, new)
        .map_err(SupersedeError::Write)
}

/// `path`'s record's own zero-padded `NNNN` number, derived from its
/// filename exactly as [`db_store::record::extract_record`] derives every
/// record's identity — `None` for a doc type with no numbered identity
/// (issue 0012 scopes supersede to numbered doc types only, matching the
/// CLI's own precondition).
fn record_number(path: &str, content: &str) -> Option<String> {
    db_store::record::extract_record(std::path::Path::new(path), content)
        .number
        .map(|number| format!("{number:04}"))
}

/// Every way `POST /delete/{*path}`'s handler can fail to commit: opening
/// the store itself failed, or
/// [`db_store::DbDocStore::delete_checked`]'s own
/// [`db_store::DeleteCheckedError`].
#[derive(Debug)]
enum DeleteError {
    Open(String),
    Write(db_store::DeleteCheckedError),
}

impl std::fmt::Display for DeleteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteError::Open(message) => write!(f, "{message}"),
            DeleteError::Write(err) => write!(f, "{err}"),
        }
    }
}

/// `POST /delete/{*path}`'s handler, mounted only when [`AuthoringConfig`]
/// is `Some`. The delete form submits no fields, so unlike the other
/// authoring handlers this one takes no `axum::Form` extractor. On success,
/// redirects to `path`'s own record page (a soft-deleted record stays
/// viewable); on a [`DeleteError`], re-renders it with the error visible.
async fn delete_handler(State(state): State<AppState>, Path(path): Path<String>) -> Response {
    let authoring = state
        .authoring
        .clone()
        .expect("delete_handler is only mounted when authoring is configured");

    let outcome = spawn_delete_checked(&authoring, &path).await;

    match outcome {
        Ok(Ok(())) => Redirect::to(&views::record_href(&path)).into_response(),
        Ok(Err(err)) => {
            let message = err.to_string();
            record_page_response(
                &state,
                &path,
                SupersedeSubmission::default(),
                DeleteSubmission {
                    error: Some(&message),
                },
            )
            .await
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal error deleting record",
        )
            .into_response(),
    }
}

/// Runs [`delete_record`] inside [`tokio::task::spawn_blocking`], mirroring
/// [`spawn_supersede_checked`]'s own bridge from this async handler onto
/// `DbDocStore`'s synchronous SeaORM runtime.
async fn spawn_delete_checked(
    authoring: &AuthoringConfig,
    path: &str,
) -> std::result::Result<std::result::Result<(), DeleteError>, tokio::task::JoinError> {
    let db_url = authoring.db_url.clone();
    let docs_root = authoring.docs_root.clone();
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || delete_record(&db_url, &docs_root, &path)).await
}

fn delete_record(
    db_url: &str,
    docs_root: &std::path::Path,
    path: &str,
) -> std::result::Result<(), DeleteError> {
    let store = db_store::DbDocStore::new(db_url, docs_root.to_path_buf())
        .map_err(|err| DeleteError::Open(err.to_string()))?;
    store
        .delete_checked(std::path::Path::new(path))
        .map_err(DeleteError::Write)
}

async fn record_or_log(conn: &DatabaseConnection, path: &str) -> Option<RecordView> {
    match db_store::record_by_path(conn, path).await {
        Ok(record) => record,
        Err(err) => {
            eprintln!("record lookup {path:?} failed: {err}");
            None
        }
    }
}

async fn record_meta_or_log(conn: &DatabaseConnection, path: &str) -> Option<RecordMeta> {
    match db_store::record_meta(conn, path).await {
        Ok(meta) => meta,
        Err(err) => {
            eprintln!("record_meta lookup {path:?} failed: {err}");
            None
        }
    }
}

fn render_markdown(body: &str) -> String {
    let parser = pulldown_cmark::Parser::new(body);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}

#[cfg(test)]
mod tests;
