//! Frontmatter-tail phase: the EAV `frontmatter_fields` encode/decode path.

use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    QueryOrder,
};

use crate::entity::frontmatter_fields;
use crate::record::TailValue;
use crate::Result;

/// Marks a `frontmatter_fields.value` row as one element of a
/// `TailValue::Sequence`, distinguishing it from a `TailValue::Scalar` row
/// with no change to `frontmatter_fields`'s `(record_id, key, value,
/// ordinal)` shape (ADR 0007 decision 1 unchanged, ADR 0019 slice S3b): a
/// user-authored scalar is free-form text but never opens with this control
/// character, so its presence unambiguously flags a sequence element. Never
/// seen outside this module's own [`encode_tail_value`]/[`decode_tail_run`]
/// pair — it never reaches a `.md` file.
pub(super) const TAIL_SEQUENCE_MARKER: char = '\u{1}';

/// Inserts one `frontmatter_fields` row per tail entry, `ordinal` set to
/// its position in the flattened row sequence so the tail reconstructs by
/// ascending `ordinal` in the same order it was encountered in the source
/// frontmatter. A [`TailValue::Sequence`] flattens to one marked row per
/// element (see [`encode_tail_value`]), so a list-valued key spans more than
/// one row sharing that key.
pub(super) async fn insert_frontmatter_tail<C: ConnectionTrait>(
    conn: &C,
    record_id: i32,
    tail: &[(String, TailValue)],
) -> Result<()> {
    let mut ordinal = 0i32;
    for (key, value) in tail {
        for row_value in encode_tail_value(value) {
            frontmatter_fields::ActiveModel {
                record_id: ActiveValue::Set(record_id),
                key: ActiveValue::Set(key.clone()),
                value: ActiveValue::Set(row_value),
                ordinal: ActiveValue::Set(ordinal),
                ..Default::default()
            }
            .insert(conn)
            .await?;
            ordinal += 1;
        }
    }
    Ok(())
}

/// Flattens one tail entry's value into the `frontmatter_fields.value`
/// strings it becomes: a [`TailValue::Scalar`] is exactly one unmarked
/// value; a [`TailValue::Sequence`] is one [`TAIL_SEQUENCE_MARKER`]-prefixed
/// value per element, or — for an empty sequence — a single marker-only
/// value, so the key still survives the round trip with no elements. See
/// [`decode_tail_run`] for the inverse.
pub(super) fn encode_tail_value(value: &TailValue) -> Vec<String> {
    match value {
        TailValue::Scalar(scalar) => vec![scalar.clone()],
        TailValue::Sequence(items) if items.is_empty() => {
            vec![TAIL_SEQUENCE_MARKER.to_string()]
        }
        TailValue::Sequence(items) => items
            .iter()
            .map(|item| format!("{TAIL_SEQUENCE_MARKER}{item}"))
            .collect(),
    }
}

/// Reassembles `record_id`'s ordered [`TailValue`] tail from its
/// `frontmatter_fields` rows — the read-side counterpart to
/// [`insert_frontmatter_tail`], reused by `db_store::load_record`.
pub(crate) async fn load_frontmatter_tail<C: ConnectionTrait>(
    conn: &C,
    record_id: i32,
) -> Result<Vec<(String, TailValue)>> {
    let rows = frontmatter_fields::Entity::find()
        .filter(frontmatter_fields::Column::RecordId.eq(record_id))
        .order_by_asc(frontmatter_fields::Column::Ordinal)
        .all(conn)
        .await?;
    Ok(group_tail_rows(&rows))
}

/// Groups ordinal-ordered `frontmatter_fields` rows back into tail entries:
/// each maximal run of consecutive rows sharing the same `key` becomes one
/// `(key, TailValue)` pair, decoded by [`decode_tail_run`]. A key never
/// repeats non-contiguously — a YAML mapping key is unique — so grouping by
/// adjacency alone is exact.
fn group_tail_rows(rows: &[frontmatter_fields::Model]) -> Vec<(String, TailValue)> {
    let mut tail = Vec::new();
    let mut start = 0;
    while start < rows.len() {
        let key = &rows[start].key;
        let run_len = rows[start..]
            .iter()
            .take_while(|row| &row.key == key)
            .count();
        tail.push((key.clone(), decode_tail_run(&rows[start..start + run_len])));
        start += run_len;
    }
    tail
}

/// Decodes one same-key run of rows into its [`TailValue`]: a single
/// unmarked row is a [`TailValue::Scalar`]; a single marker-only row is an
/// empty [`TailValue::Sequence`]; any other run (a single marked row, or
/// more than one row) is a [`TailValue::Sequence`] of each row's
/// marker-stripped value, in row order.
fn decode_tail_run(run: &[frontmatter_fields::Model]) -> TailValue {
    if let [only] = run {
        match strip_sequence_marker(&only.value) {
            None => return TailValue::Scalar(only.value.clone()),
            Some("") => return TailValue::Sequence(Vec::new()),
            Some(_) => {}
        }
    }
    let items = run
        .iter()
        .filter_map(|row| strip_sequence_marker(&row.value))
        .map(str::to_owned)
        .collect();
    TailValue::Sequence(items)
}

fn strip_sequence_marker(value: &str) -> Option<&str> {
    value.strip_prefix(TAIL_SEQUENCE_MARKER)
}

pub(super) async fn replace_frontmatter_tail<C: ConnectionTrait>(
    conn: &C,
    record_id: i32,
    tail: &[(String, TailValue)],
) -> Result<()> {
    frontmatter_fields::Entity::delete_many()
        .filter(frontmatter_fields::Column::RecordId.eq(record_id))
        .exec(conn)
        .await?;
    insert_frontmatter_tail(conn, record_id, tail).await
}
