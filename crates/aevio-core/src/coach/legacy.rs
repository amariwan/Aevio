//! Compatibility contracts for existing coach IPC clients.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoachTask {
    pub id: String,
    pub title: String,
    pub notes: Option<String>,
    pub priority: i64,
    pub status: String,
    pub due_at: Option<u64>,
    pub reminder_at: Option<u64>,
    pub estimate_ms: Option<u64>,
    pub created_at: u64,
    pub updated_at: u64,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewCoachTask {
    pub title: String,
    pub notes: Option<String>,
    pub priority: Option<i64>,
    pub due_at: Option<u64>,
    pub reminder_at: Option<u64>,
    pub estimate_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCoachTask {
    pub title: Option<String>,
    pub notes: Option<Option<String>>,
    pub priority: Option<i64>,
    pub status: Option<String>,
    pub due_at: Option<Option<u64>>,
    pub reminder_at: Option<Option<u64>>,
    pub estimate_ms: Option<Option<u64>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoachRecommendation {
    pub task_id: String,
    pub title: String,
    pub reason: String,
    pub score: i64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoachSnapshot {
    pub tasks: Vec<CoachTask>,
    pub recommendations: Vec<CoachRecommendation>,
    pub tracked_ms: u64,
    pub daily_target_ms: u64,
    pub ai_engine: String,
    pub ai_available: bool,
    pub next_reminder_at: Option<u64>,
}
