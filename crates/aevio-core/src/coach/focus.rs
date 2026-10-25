//! Recovery models refer to existing ActivityStore segments, never a second clock.
use serde::Serialize;
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Focus {
    pub id: String,
    pub task_id: String,
    pub segment_id: i64,
    pub title: String,
    pub goal: String,
    pub next_step: String,
    pub planned_minutes: u32,
    pub body_doubling: bool,
    pub state: String,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub elapsed_ms: u64,
}
pub struct Start<'a> {
    pub goal: &'a str,
    pub next_step: &'a str,
    pub minutes: u32,
    pub body: bool,
    pub segment_id: i64,
}
