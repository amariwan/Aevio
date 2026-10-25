//! Optional personalization uses explicit notes and transparent aggregate evidence.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Memory {
    pub id: String,
    pub content: String,
    pub kind: String,
    pub updated_at: u64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekDay {
    pub day: String,
    pub completed: u32,
    pub planned_minutes: u32,
    pub actual_ms: u64,
    pub outcome: String,
    pub focus_sessions: u32,
    pub postponed: u32,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Weekly {
    pub starts_on: String,
    pub days: Vec<WeekDay>,
    pub previous: Vec<WeekDay>,
    pub recommendations: Vec<String>,
    pub source: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Estimate {
    pub source: String,
    pub task_id: String,
    pub minutes: u32,
    pub samples: u32,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanningProposal {
    pub source: String,
    pub selected: Vec<String>,
    pub deferred: Vec<String>,
    pub minutes: u32,
    pub capacity: u32,
    pub buffer_minutes: u32,
    pub effective_capacity: u32,
    pub reason: String,
}

pub fn planning(day: &super::models::Day) -> PlanningProposal {
    let mut tasks = day
        .tasks
        .iter()
        .filter(|task| !task.done)
        .collect::<Vec<_>>();
    tasks.sort_by_key(|task| (!task.important, -task.priority, task.position));
    let capacity = day
        .plan
        .as_ref()
        .map(|plan| plan.available_minutes)
        .unwrap_or(0);
    let buffer = day.preferences.planning_buffer_minutes as u32;
    let effective = capacity.saturating_sub(buffer);
    let total_estimated: u32 = tasks.iter().map(|task| task.estimate_minutes).sum();
    let mut remaining = effective;
    let mut selected = Vec::new();
    let mut deferred = Vec::new();
    for task in tasks {
        if task.estimate_minutes > 0 && task.estimate_minutes <= remaining {
            selected.push(task.id.clone());
            remaining = remaining.saturating_sub(task.estimate_minutes);
        } else {
            deferred.push(task.id.clone());
        }
    }
    let reason = if total_estimated > effective {
        format!(
            "Regelbasiert: wichtige Aufgaben und Priorität zuerst. Die offenen Aufgaben benötigen etwa {} Minuten, aber nur {} von {} verfügbaren Minuten sind nach {} Minuten Puffer wirksam; die verbleibenden Aufgaben bleiben zur manuellen Entscheidung offen. Keine Änderung ohne Bestätigung.",
            total_estimated, effective, capacity, buffer
        )
    } else {
        format!(
            "Regelbasiert: wichtige Aufgaben und Priorität zuerst; {} von {} wirksamen Minuten werden empfohlen. Aufgaben ohne Schätzung bleiben zur manuellen Prüfung offen. Keine Änderung ohne Bestätigung.",
            effective.saturating_sub(remaining),
            effective
        )
    };
    PlanningProposal {
        source: "rules".into(),
        selected,
        deferred,
        minutes: effective.saturating_sub(remaining),
        capacity,
        buffer_minutes: buffer,
        effective_capacity: effective,
        reason,
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanSelection {
    pub selected: Vec<String>,
    pub reason: String,
}
pub fn validate_selection(
    day: &super::models::Day,
    selection: PlanSelection,
) -> Result<PlanningProposal, String> {
    if selection.selected.len() > 3 {
        return Err("A planning suggestion may recommend at most three priorities".into());
    }
    let mut unique = std::collections::HashSet::new();
    let mut minutes = 0;
    for id in &selection.selected {
        if !unique.insert(id) {
            return Err("Planning suggestion repeated a task".into());
        }
        let task = day
            .tasks
            .iter()
            .find(|task| task.id == *id && !task.done)
            .ok_or("Planning suggestion referenced an unknown task")?;
        minutes += task.estimate_minutes;
    }
    let capacity = day
        .plan
        .as_ref()
        .map(|plan| plan.available_minutes)
        .unwrap_or(0);
    let buffer = day.preferences.planning_buffer_minutes as u32;
    let effective = capacity.saturating_sub(buffer);
    if minutes > effective {
        return Err("Planning suggestion exceeds the available time".into());
    }
    super::policies::text(&selection.reason, 1000)?;
    Ok(PlanningProposal {
        deferred: day
            .tasks
            .iter()
            .filter(|task| !task.done && !selection.selected.contains(&task.id))
            .map(|task| task.id.clone())
            .collect(),
        selected: selection.selected,
        minutes,
        capacity,
        buffer_minutes: buffer,
        effective_capacity: effective,
        reason: selection.reason,
        source: "on_device".into(),
    })
}
