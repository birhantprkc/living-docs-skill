//! Callout invariant (invariant 4's companion): a Superseded or Deprecated
//! record's body must open with its exact status callout, and an active
//! record must carry none. Reuses `crate::callout`'s `expected`/`leading`/
//! `successor_filename` so the expected wording and successor resolution
//! never drift from what `living-docs fmt` writes.

use super::records::{has_frontmatter, is_reserved};
use super::{file_name_str, Reporter};
use crate::callout;
use crate::record::extract_record;
use crate::store::DocStore;
use std::path::{Path, PathBuf};

pub(crate) fn check_callouts(store: &dyn DocStore, all_md: &[PathBuf], reporter: &mut Reporter) {
    for f in all_md {
        if is_reserved(&file_name_str(f)) {
            continue;
        }
        let Ok(contents) = store.read(f) else {
            continue;
        };
        if !has_frontmatter(&contents) {
            continue;
        }
        check_record_callout(store, f, &contents, reporter);
    }
}

fn check_record_callout(store: &dyn DocStore, f: &Path, contents: &str, reporter: &mut Reporter) {
    let record = extract_record(f, contents);
    let successor = record
        .superseded_by
        .as_deref()
        .map(|superseded_by| callout::successor_filename(store, f, superseded_by));
    let expected = callout::expected(record.status.as_deref(), successor.as_deref());
    let leading = callout::leading(&record.body);

    match expected {
        Some(line) if leading != Some(line.as_str()) => {
            let status = record.status.unwrap_or_default();
            reporter.report(
                f,
                format!(
                    "status: {status} but the body does not open with its callout — run living-docs fmt (invariant 4)"
                ),
            );
        }
        None if leading.is_some() => {
            reporter.report(
                f,
                "active record opens with a retired callout — run living-docs fmt (invariant 4)",
            );
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests;
