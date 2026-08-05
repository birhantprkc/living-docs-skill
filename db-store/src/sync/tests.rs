use super::frontmatter::{encode_tail_value, TAIL_SEQUENCE_MARKER};
use super::test_support::seeded_corpus;
use super::*;
use crate::record::TailValue;
use crate::{connect_in_memory, migrate};
use sea_orm::{FromQueryResult, QueryOrder};

#[derive(Debug, FromQueryResult, PartialEq, Eq)]
struct RecordRow {
    path: String,
    title: String,
}

async fn all_records(conn: &DatabaseConnection) -> Vec<RecordRow> {
    Records::find()
        .order_by_asc(crate::entity::Column::Path)
        .into_model::<RecordRow>()
        .all(conn)
        .await
        .expect("query records")
}

async fn row_count(conn: &DatabaseConnection, table: &str) -> i64 {
    conn.query_one(Statement::from_string(
        conn.get_database_backend(),
        format!("SELECT COUNT(*) AS n FROM {table}"),
    ))
    .await
    .expect("run count query")
    .expect("count query returns one row")
    .try_get::<i64>("", "n")
    .expect("n column")
}

#[tokio::test]
async fn sync_skips_reserved_files_and_inserts_the_rest() {
    let conn = connect_in_memory().await.expect("connect");
    migrate(&conn).await.expect("migrate");
    let (store, bundle) = seeded_corpus();

    let count = sync(&conn, &store, &bundle).await.expect("sync");

    assert_eq!(count, 2);
    let rows = all_records(&conn).await;
    assert_eq!(
        rows,
        vec![
            RecordRow {
                path: "adr/0001-quokka-caching.md".to_owned(),
                title: "Quokka Caching Strategy".to_owned(),
            },
            RecordRow {
                path: "adr/0002-unrelated.md".to_owned(),
                title: "Unrelated Decision".to_owned(),
            },
        ]
    );
}

#[tokio::test]
async fn sync_is_idempotent_across_repeated_runs() {
    let conn = connect_in_memory().await.expect("connect");
    migrate(&conn).await.expect("migrate");
    let (store, bundle) = seeded_corpus();

    sync(&conn, &store, &bundle).await.expect("first sync");
    let first_rows = all_records(&conn).await;

    let second_count = sync(&conn, &store, &bundle).await.expect("second sync");
    let second_rows = all_records(&conn).await;

    assert_eq!(second_count, 2);
    assert_eq!(first_rows, second_rows);
    assert_eq!(row_count(&conn, "records").await, 2);
}

#[tokio::test]
async fn sync_populates_the_fts_index() {
    let conn = connect_in_memory().await.expect("connect");
    migrate(&conn).await.expect("migrate");
    let (store, bundle) = seeded_corpus();

    sync(&conn, &store, &bundle).await.expect("sync");

    assert_eq!(row_count(&conn, "records_fts").await, 2);
}

#[tokio::test]
async fn rebuild_search_index_is_a_no_op_on_postgres() {
    let mut options = sea_orm::ConnectOptions::new("postgres://user:pass@localhost/db");
    options.connect_lazy(true);
    let conn = sea_orm::Database::connect(options)
        .await
        .expect("lazy postgres connect never touches the network");

    rebuild_search_index(&conn)
        .await
        .expect("postgres rebuild is a no-op that never issues SQL");
}

#[tokio::test]
async fn sync_project_upserts_a_named_project_and_scopes_records_to_it() {
    let conn = connect_in_memory().await.expect("connect");
    migrate(&conn).await.expect("migrate");
    let (store, bundle) = seeded_corpus();

    let count = sync_project(&conn, &store, &bundle, "team-a")
        .await
        .expect("sync_project");

    assert_eq!(count, 2);
    let project = projects::Entity::find()
        .filter(projects::Column::Slug.eq("team-a"))
        .one(&conn)
        .await
        .expect("query project")
        .expect("sync_project upserts the named project");
    let records = all_records(&conn).await;
    assert_eq!(records.len(), 2);
    let stored = Records::find()
        .filter(Column::ProjectId.eq(project.id))
        .all(&conn)
        .await
        .expect("query records for project");
    assert_eq!(stored.len(), 2);
}

#[tokio::test]
async fn sync_persists_status_from_frontmatter_and_none_when_absent() {
    let conn = connect_in_memory().await.expect("connect");
    migrate(&conn).await.expect("migrate");
    let (store, bundle) = super::test_support::corpus_with_and_without_status();

    sync(&conn, &store, &bundle).await.expect("sync");

    let with_status = Records::find()
        .filter(Column::Path.eq("adr/0001-with-status.md"))
        .one(&conn)
        .await
        .expect("query with-status record")
        .expect("with-status record exists");
    assert_eq!(with_status.status, Some("Accepted".to_owned()));

    let without_status = Records::find()
        .filter(Column::Path.eq("adr/0002-without-status.md"))
        .one(&conn)
        .await
        .expect("query without-status record")
        .expect("without-status record exists");
    assert_eq!(without_status.status, None);
}

/// Asserts the sync/load round trip preserves a list-valued tail key's
/// elements and order, and that an empty list survives as an empty
/// sequence rather than vanishing from the tail entirely (ADR 0019
/// slice S3b, closing ADR 0007's lossless-export gap for `labels:`/
/// `blocked_by:`-shaped keys).
#[tokio::test]
async fn sync_and_load_round_trips_a_list_valued_frontmatter_tail_key() {
    let conn = connect_in_memory().await.expect("connect");
    migrate(&conn).await.expect("migrate");
    let (store, bundle) = super::test_support::list_valued_tail_corpus();

    sync(&conn, &store, &bundle).await.expect("sync");

    let record = Records::find()
        .filter(Column::Path.eq("issues/0001-list-tail.md"))
        .one(&conn)
        .await
        .expect("query record")
        .expect("record was synced");

    let tail = load_frontmatter_tail(&conn, record.id)
        .await
        .expect("load frontmatter tail");

    assert_eq!(
        tail,
        vec![
            (
                "labels".to_owned(),
                TailValue::Sequence(vec!["slice".to_owned(), "skeleton".to_owned()])
            ),
            ("blocked_by".to_owned(), TailValue::Sequence(Vec::new())),
        ]
    );
}

#[test]
fn encode_tail_value_marks_every_sequence_element_and_leaves_a_scalar_unmarked() {
    assert_eq!(
        encode_tail_value(&TailValue::Scalar("important".to_owned())),
        vec!["important".to_owned()]
    );
    assert_eq!(
        encode_tail_value(&TailValue::Sequence(vec!["a".to_owned(), "b".to_owned()])),
        vec![
            format!("{TAIL_SEQUENCE_MARKER}a"),
            format!("{TAIL_SEQUENCE_MARKER}b"),
        ]
    );
    assert_eq!(
        encode_tail_value(&TailValue::Sequence(Vec::new())),
        vec![TAIL_SEQUENCE_MARKER.to_string()]
    );
}
