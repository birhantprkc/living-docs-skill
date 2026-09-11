//! Deterministic JSON rendering for `scorecard` (moved out of the parent
//! module for the file-size ratchet). Field declaration order fixes the key
//! order, so two computations over the same tree serialize byte-for-byte
//! identically.

use super::inflation::Inflation;
use super::{Grade, Scorecard};

#[derive(serde::Serialize)]
struct AttributeJson {
    grade: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    freshness: Option<String>,
}

#[derive(serde::Serialize)]
struct InflationJson {
    adrs: usize,
    adrs_recent: usize,
    adr_bdr_pairing_ratio: Option<f64>,
    superseded: usize,
    proposed_stale: usize,
}

#[derive(serde::Serialize)]
struct ScorecardJson {
    trusted: AttributeJson,
    contextual: AttributeJson,
    traceable: AttributeJson,
    governed: AttributeJson,
    inflation: InflationJson,
    overall: String,
}

pub fn render(scorecard: &Scorecard) -> String {
    let freshness = Some(match scorecard.freshness {
        Some(freshness) => freshness.as_str().to_string(),
        None => Grade::NotMeasured.as_str().to_string(),
    });
    let payload = ScorecardJson {
        trusted: AttributeJson {
            grade: scorecard.trusted.as_str().to_string(),
            freshness,
        },
        contextual: attribute(scorecard.contextual),
        traceable: attribute(scorecard.traceable),
        governed: attribute(scorecard.governed),
        inflation: inflation_json(&scorecard.inflation),
        overall: scorecard.overall.as_str().to_string(),
    };
    serde_json::to_string(&payload).unwrap_or_default()
}

fn attribute(grade: Grade) -> AttributeJson {
    AttributeJson {
        grade: grade.as_str().to_string(),
        freshness: None,
    }
}

fn inflation_json(inflation: &Inflation) -> InflationJson {
    InflationJson {
        adrs: inflation.adrs,
        adrs_recent: inflation.adrs_recent,
        adr_bdr_pairing_ratio: inflation.pairing_ratio(),
        superseded: inflation.superseded,
        proposed_stale: inflation.proposed_stale,
    }
}
