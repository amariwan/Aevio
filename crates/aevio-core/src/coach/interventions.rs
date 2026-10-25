//! Deterministic, privacy-first proactive coaching eligibility.
use super::{models::Day, policies};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterventionAction {
    pub id: String,
    pub label: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Intervention {
    pub id: String,
    pub kind: String,
    pub priority: u8,
    pub title: String,
    pub message: String,
    pub actions: Vec<InterventionAction>,
}

pub fn suggest(
    day: &Day,
    now_minute: u16,
    now_ms: u64,
    last_intervention_ms: Option<u64>,
) -> Option<Intervention> {
    if !day.preferences.enabled {
        return None;
    }
    if policies::quiet(&day.preferences, now_minute) {
        return None;
    }
    if let Some(last) = last_intervention_ms {
        let cooldown = day.preferences.cooldown_minutes as u64 * 60_000;
        if now_ms.saturating_sub(last) < cooldown {
            return None;
        }
    }
    suggest_content(day)
}

pub fn suggest_content(day: &Day) -> Option<Intervention> {
    if day.overbooked_minutes > 0 {
        return Some(Intervention {
            id: format!("overcommitment:{}", day.day),
            kind: "overcommitment".into(),
            priority: 2,
            title: "Realistischer planen".into(),
            message: format!(
                "Du hast {} Minuten eingeplant, aber effektiv {} Minuten verfügbar. Soll ich dir eine realistischere Auswahl vorschlagen?",
                day.planned_minutes,
                day.plan
                    .as_ref()
                    .map(|p| p.available_minutes)
                    .unwrap_or(0)
                    .saturating_sub(day.preferences.planning_buffer_minutes as u32)
            ),
            actions: vec![
                InterventionAction {
                    id: "view".into(),
                    label: "Vorschlag ansehen".into(),
                    kind: "navigate".into(),
                },
                InterventionAction {
                    id: "dismiss".into(),
                    label: "Ignorieren".into(),
                    kind: "dismiss".into(),
                },
            ],
        });
    }
    if day.plan.is_none() {
        return Some(Intervention {
            id: format!("morning_planning:{}", day.day),
            kind: "morning_planning".into(),
            priority: 4,
            title: "Tag planen".into(),
            message: "Möchtest du deine wichtigsten Aufgaben für heute festlegen?".into(),
            actions: vec![
                InterventionAction {
                    id: "plan".into(),
                    label: "Tag planen".into(),
                    kind: "navigate".into(),
                },
                InterventionAction {
                    id: "dismiss".into(),
                    label: "Später".into(),
                    kind: "dismiss".into(),
                },
            ],
        });
    }
    None
}
