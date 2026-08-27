//! The `MOVED-SOURCE` review queue: a warn-level finding when a record links
//! another record whose status has left the open/accepted state. Detection
//! reuses `check::links`' fence-aware extraction and resolution so the
//! markdown parsing lives in exactly one place; status/`superseded_by` reads
//! reuse `check::records::frontmatter_scalar`.
//!
//! The finding clears when the dependent's body links the successor
//! anywhere, or when the dependent itself is Superseded or closed — no new
//! annotation syntax, the acknowledgment IS the updated link.

use super::links::{link_destinations, resolve_destination};
use super::records::{frontmatter_scalar, is_reserved, record_id_matches};
use super::{file_name_str, Reporter};
use crate::store::DocStore;
use std::path::{Path, PathBuf};

pub(crate) fn check_moved_source(
    store: &dyn DocStore,
    bundle: &Path,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    let bundle_str = bundle.to_string_lossy();
    for f in all_md {
        check_file_moved_source(store, f, &bundle_str, reporter);
    }
}

fn check_file_moved_source(store: &dyn DocStore, f: &Path, bundle: &str, reporter: &mut Reporter) {
    if is_reserved(&file_name_str(f)) {
        return;
    }
    let Ok(contents) = store.read(f) else {
        return;
    };
    if is_closed_dependent(&contents) {
        return;
    }
    let file_str = f.to_string_lossy();
    for dest in link_destinations(&contents) {
        let Some(target) = resolve_destination(&file_str, &dest, bundle) else {
            continue;
        };
        check_target(
            store,
            f,
            &contents,
            bundle,
            &PathBuf::from(target),
            reporter,
        );
    }
}

fn check_target(
    store: &dyn DocStore,
    dependent: &Path,
    dependent_contents: &str,
    bundle: &str,
    target: &Path,
    reporter: &mut Reporter,
) {
    if is_reserved(&file_name_str(target)) {
        return;
    }
    let Ok(target_contents) = store.read(target) else {
        return;
    };
    let Some(status) = frontmatter_scalar(&target_contents, "status") else {
        return;
    };
    if !is_moved_status(&status) {
        return;
    }
    let successor = superseded_by(&status, &target_contents);
    if let Some(id) = &successor {
        if dependent_links_successor(dependent, dependent_contents, bundle, target, id) {
            return;
        }
    }
    reporter.advise(
        dependent,
        moved_source_finding(dependent, target, &status, successor.as_deref()),
    );
}

fn is_closed_dependent(contents: &str) -> bool {
    frontmatter_scalar(contents, "status")
        .map(|status| matches!(status.to_lowercase().as_str(), "superseded" | "closed"))
        .unwrap_or(false)
}

fn is_moved_status(status: &str) -> bool {
    matches!(
        status.to_lowercase().as_str(),
        "superseded" | "deprecated" | "rejected"
    )
}

fn superseded_by(status: &str, target_contents: &str) -> Option<String> {
    if !status.eq_ignore_ascii_case("superseded") {
        return None;
    }
    frontmatter_scalar(target_contents, "superseded_by")
}

fn dependent_links_successor(
    dependent: &Path,
    dependent_contents: &str,
    bundle: &str,
    source: &Path,
    successor_id: &str,
) -> bool {
    let Some(dir) = source.parent() else {
        return false;
    };
    let dependent_str = dependent.to_string_lossy();
    link_destinations(dependent_contents).iter().any(|dest| {
        resolve_destination(&dependent_str, dest, bundle)
            .map(|resolved| record_id_matches(dir, &PathBuf::from(resolved), successor_id))
            .unwrap_or(false)
    })
}

fn moved_source_finding(
    dependent: &Path,
    source: &Path,
    status: &str,
    successor: Option<&str>,
) -> String {
    let dependent_name = file_name_str(dependent);
    let source_name = file_name_str(source);
    match successor {
        Some(id) => format!(
            "MOVED-SOURCE: {dependent_name} links {source_name} whose status is {status}, superseded by {id}"
        ),
        None => format!("MOVED-SOURCE: {dependent_name} links {source_name} whose status is {status}"),
    }
}

#[cfg(test)]
mod tests;
