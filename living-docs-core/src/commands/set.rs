use crate::callout;
use crate::commands::supersede::{find_record, set_frontmatter_fields};
use crate::doc_type::{self, DocTypeSpec};
use crate::record::{extract_record, format_scalar};
use crate::store::DocStore;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn run(
    store: &dyn DocStore,
    docs_dir: &Path,
    reference: &str,
    key: &str,
    value: &str,
) -> ExitCode {
    match apply(store, docs_dir, reference, key, value) {
        Ok(_) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("living-docs set: {message}");
            ExitCode::from(2)
        }
    }
}

/// Sets one CLI-owned frontmatter field on a record: `status`, `description`,
/// or `owner`. Record resolution ([`find_record`], accepting a bare `NNNN` or
/// a type-qualified `TYPE/NNNN` reference) and the frontmatter write
/// ([`set_frontmatter_fields`]) are shared with `supersede`. Only `status`
/// carries a vocabulary constraint (validated against the record's own type,
/// `Superseded` reserved for `supersede`); `description`/`owner` accept any
/// string, quoted via [`format_scalar`]. Nothing is written when the
/// reference is unresolvable or the value fails validation. Returns the
/// written record's path — the CLI front renders this as colored text or
/// JSON (ADR 0060); [`run`] is the plain-text-always convenience wrapper.
pub fn apply(
    store: &dyn DocStore,
    docs_dir: &Path,
    reference: &str,
    key: &str,
    value: &str,
) -> Result<PathBuf, String> {
    let path = find_record(store, docs_dir, reference)?;
    let field = resolve_field(store, &path, key, value)?;
    set_frontmatter_fields(store, &path, &[field])?;
    callout::reconcile(store, &path)?;
    Ok(path)
}

fn resolve_field(
    store: &dyn DocStore,
    path: &Path,
    key: &str,
    value: &str,
) -> Result<(&'static str, String), String> {
    match key {
        "status" => {
            let contents = store.read(path).map_err(|e| e.to_string())?;
            let spec = resolve_spec(path, &contents)?;
            validate_status(value, spec)?;
            Ok(("status", value.to_string()))
        }
        "description" => Ok(("description", format_scalar(value))),
        "owner" => Ok(("owner", format_scalar(value))),
        other => Err(format!(
            "'{other}' is not a settable field; expected one of status, description, owner"
        )),
    }
}

/// Resolves the [`DocTypeSpec`] the record at `path` belongs to, from its own
/// `type:` frontmatter, so a status is validated against its own type's
/// vocabulary (ADR 0029).
fn resolve_spec(path: &Path, contents: &str) -> Result<&'static DocTypeSpec, String> {
    let doc_type = extract_record(path, contents).doc_type;
    doc_type::spec_for_frontmatter(&doc_type).ok_or_else(|| {
        format!(
            "{}: unrecognized 'type: {doc_type}' frontmatter",
            path.display()
        )
    })
}

fn validate_status(new_status: &str, spec: &DocTypeSpec) -> Result<(), String> {
    if spec.status_vocabulary.contains(&new_status) {
        return Ok(());
    }
    if new_status.eq_ignore_ascii_case("superseded") {
        return Err(
            "'Superseded' must be set via `living-docs supersede <old> <new>`, which also wires the supersedes/superseded_by links".to_string(),
        );
    }
    Err(format!(
        "'{new_status}' is not a valid status for {}; expected one of {}",
        spec.frontmatter,
        spec.status_vocabulary.join(", ")
    ))
}

#[cfg(test)]
mod tests;
