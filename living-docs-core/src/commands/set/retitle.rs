//! The `title` key of `set` (ADR 0061): one transaction over the frontmatter
//! `title`, the record's heading line, its filename, and every in-bundle
//! reference to the old filename. A record whose status is terminal for its
//! type, or `Superseded`, is refused — its title is history.

use super::resolve_spec;
use crate::commands::supersede::set_frontmatter_fields;
use crate::doc_type::{DocTypeSpec, Identity};
use crate::paths::slugify;
use crate::record::{extract_record, format_scalar};
use crate::store::DocStore;
use std::path::{Path, PathBuf};

/// What a retitle did: where the record now lives, and which other bundle
/// files had a reference to the old filename rewritten.
#[derive(Debug)]
pub(crate) struct Retitled {
    pub(crate) path: PathBuf,
    pub(crate) rewritten: Vec<PathBuf>,
}

pub(crate) fn apply(
    store: &dyn DocStore,
    docs_dir: &Path,
    path: &Path,
    title: &str,
) -> Result<Retitled, String> {
    let contents = store.read(path).map_err(|e| e.to_string())?;
    let spec = resolve_spec(path, &contents)?;
    guard_status(path, &contents, spec)?;
    let target = target_path(spec, path, title)?;
    if let Some(target) = target.as_ref() {
        refuse_collision(store, docs_dir, target)?;
    }

    set_frontmatter_fields(store, path, &[("title", format_scalar(title))])?;
    rewrite_heading(store, path, title)?;

    let Some(target) = target else {
        return Ok(Retitled {
            path: path.to_path_buf(),
            rewritten: Vec::new(),
        });
    };
    store.rename(path, &target).map_err(|e| e.to_string())?;
    let rewritten = rewrite_references(store, docs_dir, path, &target)?;
    Ok(Retitled {
        path: target,
        rewritten,
    })
}

/// A record closed for good keeps the title it was closed under: `supersede`
/// is the verb that revises a decision, and a `Superseded` record is already
/// cited by its successor's callout.
fn guard_status(path: &Path, contents: &str, spec: &DocTypeSpec) -> Result<(), String> {
    let Some(status) = extract_record(path, contents).status else {
        return Ok(());
    };
    let terminal = spec
        .terminal_statuses
        .iter()
        .any(|known| status.eq_ignore_ascii_case(known));
    if !terminal && !status.eq_ignore_ascii_case("superseded") {
        return Ok(());
    }
    Err(format!(
        "{}: status '{status}' is terminal; a closed record keeps its title — revise it with `living-docs supersede <old> <new>`",
        path.display()
    ))
}

/// Where the retitled record belongs: a numbered record keeps its number and
/// takes the new slug, a named record is its slug, and a singleton has a
/// fixed filename that no title can move.
fn target_path(spec: &DocTypeSpec, path: &Path, title: &str) -> Result<Option<PathBuf>, String> {
    let slug = slugify(title);
    if slug.is_empty() {
        return Err(format!("'{title}' has no slug; a title needs alphanumerics"));
    }
    let dir = path.parent().unwrap_or(Path::new(""));
    let target = match spec.identity {
        Identity::Singleton { .. } => return Ok(None),
        Identity::Named { .. } => dir.join(format!("{slug}.md")),
        Identity::Numbered { .. } => dir.join(format!("{}-{slug}.md", number_prefix(path)?)),
    };
    Ok((target.as_path() != path).then_some(target))
}

fn number_prefix(path: &Path) -> Result<String, String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    name.split('-')
        .next()
        .filter(|prefix| prefix.len() == 4 && prefix.chars().all(|c| c.is_ascii_digit()))
        .map(str::to_owned)
        .ok_or_else(|| format!("{}: filename carries no NNNN- number", path.display()))
}

fn refuse_collision(store: &dyn DocStore, docs_dir: &Path, target: &Path) -> Result<(), String> {
    let taken = store
        .list(docs_dir)
        .map_err(|e| e.to_string())?
        .iter()
        .any(|path| path == target);
    if taken {
        return Err(format!(
            "{}: a record already holds that slug; choose another title",
            target.display()
        ));
    }
    Ok(())
}

/// Rewrites the record's own heading line to the new title, keeping the
/// heading level and the `NNNN.` number prefix the template writes.
fn rewrite_heading(store: &dyn DocStore, path: &Path, title: &str) -> Result<(), String> {
    let contents = store.read(path).map_err(|e| e.to_string())?;
    let Some(updated) = replace_heading(&contents, title) else {
        return Ok(());
    };
    store.write(path, &updated).map_err(|e| e.to_string())
}

/// The record's heading line rewritten to `title`, or `None` when the body
/// carries no heading. Pure: every caller reads and writes around it.
pub(crate) fn replace_heading(contents: &str, title: &str) -> Option<String> {
    let mut lines: Vec<String> = contents.lines().map(str::to_owned).collect();
    let index = heading_index(&lines)?;
    let heading = &lines[index];
    let hashes = heading.chars().take_while(|c| *c == '#').collect::<String>();
    lines[index] = match heading_number(heading) {
        Some(number) => format!("{hashes} {number}. {title}"),
        None => format!("{hashes} {title}"),
    };
    Some(format!("{}\n", lines.join("\n")))
}

fn heading_index(lines: &[String]) -> Option<usize> {
    let body_start = match lines.first().map(String::as_str) {
        Some("---") => lines.iter().skip(1).position(|line| line == "---")? + 2,
        _ => 0,
    };
    lines
        .iter()
        .skip(body_start)
        .position(|line| line.starts_with('#'))
        .map(|offset| offset + body_start)
}

/// The `NNNN` a numbered record's heading opens with, as written — `# 0061.
/// Title` yields `0061`. `None` for a heading with no number prefix.
fn heading_number(heading: &str) -> Option<&str> {
    let rest = heading.trim_start_matches('#').trim_start();
    let number = rest.split('.').next()?;
    (number.len() == 4 && number.chars().all(|c| c.is_ascii_digit())).then_some(number)
}

/// Rewrites every other bundle record that names the old filename, so no
/// link is broken by the rename. The old basename is unique within the
/// bundle, which makes the substitution deterministic and reaches prose that
/// names the file as well as markdown link destinations.
fn rewrite_references(
    store: &dyn DocStore,
    docs_dir: &Path,
    old_path: &Path,
    new_path: &Path,
) -> Result<Vec<PathBuf>, String> {
    let (Some(old_name), Some(new_name)) = (file_name(old_path), file_name(new_path)) else {
        return Ok(Vec::new());
    };
    let mut rewritten = Vec::new();
    for path in store.list(docs_dir).map_err(|e| e.to_string())? {
        if path == new_path {
            continue;
        }
        let contents = store.read(&path).map_err(|e| e.to_string())?;
        if !contents.contains(&old_name) {
            continue;
        }
        let updated = contents.replace(&old_name, &new_name);
        store.write(&path, &updated).map_err(|e| e.to_string())?;
        rewritten.push(path);
    }
    Ok(rewritten)
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests;
