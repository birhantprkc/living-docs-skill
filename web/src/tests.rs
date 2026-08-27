use super::*;

const TEMPLATE: &str = "---\ntype: ADR\ntitle: <Short decision title>\ndescription: <One sentence.>\nstatus: Proposed            # Proposed | Accepted\ntimestamp: <ISO 8601 datetime>\n---\n\n# NNNN. <Short decision title>\n\nBody.\n";

#[test]
fn fill_title_replaces_only_the_frontmatter_title_line() {
    let filled = create::fill_title(TEMPLATE, "New Feature");

    assert!(filled.contains("title: \"New Feature\"\n"));
    assert!(filled.contains("# NNNN. <Short decision title>"));
    assert!(filled.contains("description: <One sentence.>"));
}

#[test]
fn fill_title_preserves_a_trailing_guidance_comment() {
    let template = "---\ntitle: <placeholder>          # Fill this in\n---\nBody.\n";

    let filled = create::fill_title(template, "My Title");

    assert!(filled.contains("title: \"My Title\""));
    assert!(filled.contains("# Fill this in"));
}

#[test]
fn fill_title_leaves_content_unchanged_without_a_closing_frontmatter_fence() {
    let no_fence = "title: <placeholder>\nBody with no frontmatter fence.\n";

    assert_eq!(create::fill_title(no_fence, "My Title"), no_fence);
}

#[test]
fn fill_title_escapes_a_colon_and_a_double_quote_so_the_frontmatter_stays_valid_yaml() {
    let filled = create::fill_title(TEMPLATE, "Weird: \"Title\"");

    assert!(filled.contains("title: \"Weird: \\\"Title\\\"\"\n"));
}

#[test]
fn relative_record_path_strips_the_docs_root_prefix() {
    let docs_root = std::path::Path::new("/bundle/docs");
    let target = docs_root.join("adr").join("0002-new-feature.md");

    assert_eq!(
        relative_record_path(docs_root, &target),
        "adr/0002-new-feature.md"
    );
}

#[test]
fn normalize_line_endings_collapses_crlf_to_lf() {
    let crlf = "---\r\ntype: ADR\r\ntitle: \"X\"\r\n---\r\n\r\nBody.\r\n";

    let normalized = normalize_line_endings(crlf);

    assert_eq!(normalized, "---\ntype: ADR\ntitle: \"X\"\n---\n\nBody.\n");
    assert!(!normalized.contains('\r'));
}

#[test]
fn normalize_line_endings_collapses_lone_cr_and_leaves_lf_only_content_unchanged() {
    assert_eq!(normalize_line_endings("a\rb\nc"), "a\nb\nc");
    assert_eq!(
        normalize_line_endings("already\nlf\nonly\n"),
        "already\nlf\nonly\n"
    );
}

#[test]
fn create_error_display_surfaces_the_plan_message_and_the_write_error() {
    let plan_error = create::CreateError::Plan("adr/0002-taken.md already exists".to_owned());
    assert_eq!(plan_error.to_string(), "adr/0002-taken.md already exists");

    let write_error = create::CreateError::Write(db_store::WriteCheckedError::AlreadyExists(
        "adr/0002-taken.md".to_owned(),
    ));
    assert_eq!(write_error.to_string(), "adr/0002-taken.md already exists");
}

#[test]
fn with_staleness_banner_prepends_the_marker_only_when_stale() {
    let main = html! { p { "content" } };
    let fresh = with_staleness_banner(false, main.clone());
    let stale = with_staleness_banner(true, main);
    assert!(!fresh.into_string().contains("staleness-banner"));
    assert!(stale.into_string().contains("staleness-banner"));
}

fn write_staleness_record(type_dir: &std::path::Path, filename: &str, title: &str) {
    let contents = format!(
        "---\ntype: ADR\ntitle: {title}\ndescription: d.\nstatus: Accepted\n---\n\n# {title}\n\nBody.\n"
    );
    std::fs::write(type_dir.join(filename), contents).expect("write record");
}

async fn synced_state_over(root: &std::path::Path) -> AppState {
    let conn = db_store::connect_in_memory().await.expect("connect");
    db_store::migrate(&conn).await.expect("migrate");
    db_store::sync_project(&conn, &fs_store::FsStore, root, "default")
        .await
        .expect("sync project");
    AppState {
        conn,
        authoring: None,
    }
}

#[tokio::test]
async fn staleness_for_is_true_after_an_unsynced_edit_and_false_right_after_a_sync() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("living-docs-web-staleness-{nanos}"));
    let type_dir = root.join("adr");
    std::fs::create_dir_all(&type_dir).expect("create adr dir");
    write_staleness_record(&type_dir, "0001-quokka.md", "Quokka");

    let state = synced_state_over(&root).await;
    assert!(!staleness_for(&state, None).await);

    write_staleness_record(&type_dir, "0002-second.md", "Second");
    assert!(staleness_for(&state, None).await);

    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn staleness_for_is_always_false_when_db_mode_authoring_is_configured() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("living-docs-web-staleness-authoring-{nanos}"));
    let type_dir = root.join("adr");
    std::fs::create_dir_all(&type_dir).expect("create adr dir");
    write_staleness_record(&type_dir, "0001-quokka.md", "Quokka");

    let mut state = synced_state_over(&root).await;
    write_staleness_record(&type_dir, "0002-second.md", "Second");
    state.authoring = Some(AuthoringConfig {
        db_url: String::new(),
        docs_root: root.clone(),
    });

    assert!(!staleness_for(&state, None).await);

    let _ = std::fs::remove_dir_all(&root);
}
