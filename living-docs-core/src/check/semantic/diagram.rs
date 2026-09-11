//! `DIAGRAM` advisory (ADR 0055): the instrument behind the "stale diagram"
//! refusal trigger. Mermaid node labels in `docs/architecture/*.md` are
//! compared with a declared module list (`architecture/diagram-scope.txt`,
//! one name per line). A node with no module, or a module with no node, is an
//! advisory `DIAGRAM` line. No scope file is a no-op — there is nothing to
//! compare against, so nothing is claimed.

use crate::check::{file_name_str, Reporter};
use crate::store::DocStore;
use regex::Regex;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(super) fn check(
    store: &dyn DocStore,
    bundle: &Path,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    let scope = read_scope(store, bundle);
    if scope.is_empty() {
        return;
    }
    let arch_dir = bundle.join("architecture");
    let mut used: BTreeSet<String> = BTreeSet::new();
    for path in all_md.iter().filter(|p| is_architecture_doc(p, &arch_dir)) {
        let Ok(contents) = store.read(path) else {
            continue;
        };
        for label in node_labels(&contents) {
            match matching_module(&label, &scope) {
                Some(module) => {
                    used.insert(module);
                }
                None => reporter.advise(path, node_without_module(&label)),
            }
        }
    }
    for module in scope.difference(&used) {
        reporter.advise(&arch_dir, module_without_node(module));
    }
}

fn read_scope(store: &dyn DocStore, bundle: &Path) -> BTreeSet<String> {
    let path = bundle.join("architecture").join("diagram-scope.txt");
    store
        .read(&path)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

fn is_architecture_doc(path: &Path, arch_dir: &Path) -> bool {
    path.parent() == Some(arch_dir)
        && file_name_str(path) != "index.md"
        && path.extension().and_then(|e| e.to_str()) == Some("md")
}

/// A scope module matches a node label when either contains the other,
/// case-insensitively — so a node `living-docs-core (fs)` still resolves to
/// the `living-docs-core` module.
fn matching_module(label: &str, scope: &BTreeSet<String>) -> Option<String> {
    let label = label.to_lowercase();
    scope.iter().find_map(|module| {
        let m = module.to_lowercase();
        (label.contains(&m) || m.contains(&label)).then(|| module.clone())
    })
}

/// Node labels inside ```mermaid fences: bracketed nodes (`ID[Label]`,
/// `ID(Label)`, `ID{Label}`) and `participant`/`actor` declarations.
fn node_labels(contents: &str) -> Vec<String> {
    let bracket = Regex::new(r#"[A-Za-z0-9_]+[\[\(\{]+"?([^"\]\)\}|]+?)"?[\]\)\}]+"#).unwrap();
    let participant = Regex::new(r"(?m)^\s*(?:participant|actor)\s+(.+?)\s*$").unwrap();
    let mut labels = Vec::new();
    for fence in mermaid_fences(contents) {
        for cap in bracket.captures_iter(&fence) {
            labels.push(cap[1].trim().to_string());
        }
        for cap in participant.captures_iter(&fence) {
            labels.push(participant_name(&cap[1]));
        }
    }
    labels
}

fn participant_name(decl: &str) -> String {
    decl.split(" as ").last().unwrap_or(decl).trim().to_string()
}

fn mermaid_fences(contents: &str) -> Vec<String> {
    let mut fences = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    for line in contents.lines() {
        let trimmed = line.trim_start();
        if let Some(lines) = current.as_mut() {
            if trimmed.starts_with("```") {
                fences.push(lines.join("\n"));
                current = None;
            } else {
                lines.push(line);
            }
        } else if trimmed.starts_with("```mermaid") {
            current = Some(Vec::new());
        }
    }
    fences
}

fn node_without_module(label: &str) -> String {
    format!("DIAGRAM node '{label}' has no module in diagram-scope.txt — the diagram may be stale")
}

fn module_without_node(module: &str) -> String {
    format!("DIAGRAM module '{module}' has no node in any architecture diagram — the diagram may be stale")
}

#[cfg(test)]
mod tests;
