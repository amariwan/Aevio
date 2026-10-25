//! Explainable, opt-in personal intelligence observations and recommendations.
use super::models::Day;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    pub id: String,
    pub kind: String,
    pub explanation: String,
    pub samples: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub message: String,
    pub evidence: String,
}

pub fn summarize(day: &Day) -> (Vec<Observation>, Vec<Recommendation>) {
    let mut observations = Vec::new();
    let mut recommendations = Vec::new();
    if !day.preferences.personal_intelligence_enabled {
        return (observations, recommendations);
    }
    if day.tracked_ms > 0 {
        observations.push(Observation {
            id: "obs:tracked".into(),
            kind: "time_tracking".into(),
            explanation: format!("Heute wurden {} Minuten erfasst.", day.tracked_ms / 60_000),
            samples: 1,
        });
    }
    if day.overbooked_minutes > 0 {
        recommendations.push(Recommendation {
            id: "rec:buffer".into(),
            kind: "planning_buffer".into(),
            title: "Planungspuffer prüfen".into(),
            message:
                "Dein Plan liegt über der wirksamen Kapazität. Ein größerer Puffer könnte helfen."
                    .into(),
            evidence: format!("{} Minuten überbucht.", day.overbooked_minutes),
        });
    }
    (observations, recommendations)
}
