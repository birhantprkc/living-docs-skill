//! `living-docs scorecard`: aggregates the existing `check` passes into the
//! fixed Trusted/Contextual/Traceable/Governed attribute-signal table,
//! grades each attribute, and derives the overall grade as the minimum
//! across the measured ones — never an average. Read-only and informational:
//! it never mutates the tree store and always exits zero; the grades never
//! gate.

use crate::check::{self, Findings};
use crate::doc_type::{self, Identity};
use crate::store::DocStore;
use std::path::Path;

/// Where a record stands in the readiness spectrum for one attribute, or an
/// attribute whose signal source is entirely absent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Grade {
    HumanEra,
    InTransition,
    AgentReady,
    NotMeasured,
}

impl Grade {
    pub fn as_str(self) -> &'static str {
        match self {
            Grade::HumanEra => "human-era",
            Grade::InTransition => "in-transition",
            Grade::AgentReady => "agent-ready",
            Grade::NotMeasured => "not measured",
        }
    }

    /// Lower is worse; [`Grade::NotMeasured`] ranks above every measured
    /// grade so it never wins a minimum unless every input is unmeasured.
    fn rank(self) -> u8 {
        match self {
            Grade::HumanEra => 0,
            Grade::InTransition => 1,
            Grade::AgentReady => 2,
            Grade::NotMeasured => u8::MAX,
        }
    }
}

/// The freshness of the derived projection a CLI front reads via
/// `db-store`'s `sync_meta`, compared against the records tree's own
/// fingerprint. Absent (`None`) when no projection is configured.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Freshness {
    Fresh,
    Stale,
}

impl Freshness {
    fn as_str(self) -> &'static str {
        match self {
            Freshness::Fresh => "fresh",
            Freshness::Stale => "stale",
        }
    }
}

/// The fixed four-attribute readiness table plus the overall grade.
pub struct Scorecard {
    pub trusted: Grade,
    pub contextual: Grade,
    pub traceable: Grade,
    pub governed: Grade,
    pub freshness: Option<Freshness>,
    pub overall: Grade,
}

/// Computes the scorecard over `bundle` through `store`. `freshness` is
/// supplied by the CLI front when a db projection is configured, `None`
/// otherwise — it never blocks the other, always-measurable attributes.
pub fn compute(store: &dyn DocStore, bundle: &Path, freshness: Option<Freshness>) -> Scorecard {
    let findings = check::findings(store, bundle);
    let trusted = trusted_grade(&findings, freshness);
    let contextual = contextual_grade(store, bundle, &findings);
    let traceable = traceable_grade(&findings);
    let governed = governed_grade(check::owner_coverage(store, bundle));
    let overall = overall_grade(&[trusted, contextual, traceable, governed]);
    Scorecard {
        trusted,
        contextual,
        traceable,
        governed,
        freshness,
        overall,
    }
}

fn overall_grade(grades: &[Grade]) -> Grade {
    grades
        .iter()
        .copied()
        .filter(|grade| *grade != Grade::NotMeasured)
        .min_by_key(|grade| grade.rank())
        .unwrap_or(Grade::NotMeasured)
}

const INVARIANT_3_MARK: &str = "(invariant 3)";
const INVARIANT_4_MARK: &str = "(invariant 4)";
const MOVED_SOURCE_PREFIX: &str = "MOVED-SOURCE";
const NO_OWNER_MARK: &str = "record has no owner";

fn is_index_finding(message: &str) -> bool {
    message.contains(INVARIANT_3_MARK)
}

fn is_supersede_finding(message: &str) -> bool {
    message.contains(INVARIANT_4_MARK)
}

fn is_moved_source_finding(message: &str) -> bool {
    message.starts_with(MOVED_SOURCE_PREFIX)
}

fn is_owner_finding(message: &str) -> bool {
    message.contains(NO_OWNER_MARK)
}

/// A finding the other three attributes have already claimed for
/// themselves — Trusted grades everything the classification leaves over.
fn is_claimed_elsewhere(message: &str) -> bool {
    is_index_finding(message)
        || is_supersede_finding(message)
        || is_moved_source_finding(message)
        || is_owner_finding(message)
}

fn trusted_grade(findings: &Findings, freshness: Option<Freshness>) -> Grade {
    let violations = count_unclaimed(&findings.violations);
    let advisories = count_unclaimed(&findings.advisories);
    if violations > 0 {
        return Grade::HumanEra;
    }
    if advisories > 0 {
        return Grade::InTransition;
    }
    match freshness {
        Some(Freshness::Stale) => Grade::InTransition,
        Some(Freshness::Fresh) | None => Grade::AgentReady,
    }
}

fn count_unclaimed(entries: &[check::Finding]) -> usize {
    entries
        .iter()
        .filter(|(_, message)| !is_claimed_elsewhere(message))
        .count()
}

fn traceable_grade(findings: &Findings) -> Grade {
    let violations = findings
        .violations
        .iter()
        .filter(|(_, message)| is_supersede_finding(message))
        .count();
    let advisories = findings
        .advisories
        .iter()
        .filter(|(_, message)| is_moved_source_finding(message))
        .count();
    if violations > 0 {
        Grade::HumanEra
    } else if advisories > 0 {
        Grade::InTransition
    } else {
        Grade::AgentReady
    }
}

fn governed_grade(coverage: Option<(usize, usize)>) -> Grade {
    match coverage {
        None => Grade::NotMeasured,
        Some((owned, required)) if owned == required => Grade::AgentReady,
        Some((0, _)) => Grade::HumanEra,
        Some(_) => Grade::InTransition,
    }
}

fn contextual_grade(store: &dyn DocStore, bundle: &Path, findings: &Findings) -> Grade {
    let index_violations = findings
        .violations
        .iter()
        .filter(|(_, message)| is_index_finding(message))
        .count();
    let complete = index_violations == 0
        && glossary_present(store, bundle)
        && constitution_present(store, bundle);
    if complete {
        Grade::AgentReady
    } else {
        Grade::HumanEra
    }
}

fn glossary_present(store: &dyn DocStore, bundle: &Path) -> bool {
    store
        .read(&bundle.join("context").join("glossary.md"))
        .is_ok()
}

fn constitution_present(store: &dyn DocStore, bundle: &Path) -> bool {
    let Some(spec) = doc_type::spec_for("constitution") else {
        return false;
    };
    match spec.identity {
        Identity::Singleton { file } => store.read(&bundle.join(file)).is_ok(),
        Identity::Numbered { .. } | Identity::Named { .. } => false,
    }
}

pub fn render_table(scorecard: &Scorecard) -> String {
    [
        row_line(
            "Trusted",
            scorecard.trusted,
            freshness_note(scorecard.freshness),
        ),
        row_line("Contextual", scorecard.contextual, None),
        row_line("Traceable", scorecard.traceable, None),
        row_line("Governed", scorecard.governed, None),
        row_line("Overall", scorecard.overall, None),
    ]
    .join("\n")
}

fn row_line(label: &str, grade: Grade, note: Option<String>) -> String {
    match note {
        Some(note) => format!("{label:<12}{}  ({note})", grade.as_str()),
        None => format!("{label:<12}{}", grade.as_str()),
    }
}

fn freshness_note(freshness: Option<Freshness>) -> Option<String> {
    Some(match freshness {
        Some(freshness) => format!("freshness: {}", freshness.as_str()),
        None => format!("freshness: {}", Grade::NotMeasured.as_str()),
    })
}

#[derive(serde::Serialize)]
struct AttributeJson {
    grade: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    freshness: Option<String>,
}

#[derive(serde::Serialize)]
struct ScorecardJson {
    trusted: AttributeJson,
    contextual: AttributeJson,
    traceable: AttributeJson,
    governed: AttributeJson,
    overall: String,
}

/// Deterministic JSON: field declaration order fixes the key order, so two
/// computations over the same tree serialize byte-for-byte identically.
pub fn render_json(scorecard: &Scorecard) -> String {
    let freshness = Some(match scorecard.freshness {
        Some(freshness) => freshness.as_str().to_string(),
        None => Grade::NotMeasured.as_str().to_string(),
    });
    let payload = ScorecardJson {
        trusted: AttributeJson {
            grade: scorecard.trusted.as_str().to_string(),
            freshness,
        },
        contextual: AttributeJson {
            grade: scorecard.contextual.as_str().to_string(),
            freshness: None,
        },
        traceable: AttributeJson {
            grade: scorecard.traceable.as_str().to_string(),
            freshness: None,
        },
        governed: AttributeJson {
            grade: scorecard.governed.as_str().to_string(),
            freshness: None,
        },
        overall: scorecard.overall.as_str().to_string(),
    };
    serde_json::to_string(&payload).unwrap_or_default()
}

#[cfg(test)]
mod tests;
