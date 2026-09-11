//! Tier rendering and the hard token budget for the effective view (ADR
//! 0050). Budgeting degrades the tier first (full → outline → index), then
//! drops the lowest-ranked records, so the output is a hard cap on tokens
//! (estimated at ~4 chars each), never a suggestion.

use super::{Tier, View};

pub(super) fn render(views: &[View], tier: Tier, budget: Option<usize>) -> String {
    match budget {
        None => render_at(views, tier),
        Some(budget) => render_within(views, tier, budget),
    }
}

/// Renders at the requested tier when it fits; otherwise steps down the tier
/// ladder, and once even the index tier of every record overflows, drops the
/// lowest-ranked records until the index fits.
fn render_within(views: &[View], tier: Tier, budget: usize) -> String {
    for candidate in tier_ladder(tier) {
        let out = render_at(views, candidate);
        if tokens(&out) <= budget {
            return out;
        }
    }
    let mut kept = views.len();
    loop {
        let out = render_at(&views[..kept], Tier::Index);
        if kept == 0 || tokens(&out) <= budget {
            return out;
        }
        kept -= 1;
    }
}

fn tier_ladder(tier: Tier) -> Vec<Tier> {
    match tier {
        Tier::Full => vec![Tier::Full, Tier::Outline, Tier::Index],
        Tier::Outline => vec![Tier::Outline, Tier::Index],
        Tier::Index => vec![Tier::Index],
    }
}

fn render_at(views: &[View], tier: Tier) -> String {
    let blocks: Vec<String> = views.iter().map(|view| render_one(view, tier)).collect();
    match tier {
        Tier::Index => join_lines(&blocks),
        Tier::Outline | Tier::Full => join_blocks(&blocks),
    }
}

fn render_one(view: &View, tier: Tier) -> String {
    match tier {
        Tier::Index => index_line(view),
        Tier::Outline => outline_block(view),
        Tier::Full => full_block(view),
    }
}

fn index_line(view: &View) -> String {
    let mut line = format!("- [{}] {}", label(view), view.title);
    if !view.description.is_empty() {
        line.push_str(&format!(" — {}", view.description));
    }
    if let Some(lineage) = &view.lineage {
        line.push_str(&format!(" ({lineage})"));
    }
    line
}

fn outline_block(view: &View) -> String {
    let headings: Vec<String> = view
        .body
        .lines()
        .filter(|line| line.starts_with("## ") || line.starts_with("### "))
        .map(|line| format!("  {}", line.trim()))
        .collect();
    let header = heading_line(view);
    if headings.is_empty() {
        header
    } else {
        format!("{header}\n{}", headings.join("\n"))
    }
}

fn full_block(view: &View) -> String {
    format!("{}\n\n{}", heading_line(view), view.body.trim_end())
}

fn heading_line(view: &View) -> String {
    let mut header = format!("## [{}] {}", label(view), view.title);
    if let Some(lineage) = &view.lineage {
        header.push_str(&format!("\n_{lineage}_"));
    }
    header
}

fn label(view: &View) -> String {
    match view.number {
        Some(number) => format!("{} {number:04}", view.doc_type),
        None => view.doc_type.clone(),
    }
}

fn join_lines(blocks: &[String]) -> String {
    if blocks.is_empty() {
        return String::new();
    }
    format!("{}\n", blocks.join("\n"))
}

fn join_blocks(blocks: &[String]) -> String {
    if blocks.is_empty() {
        return String::new();
    }
    format!("{}\n", blocks.join("\n\n"))
}

/// Estimates tokens for the budget at ~4 characters per token — deterministic
/// and backend-free, matching the budget's role as a coarse window guard.
fn tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}
