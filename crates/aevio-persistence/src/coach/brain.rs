use crate::database::DatabaseConnection;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrainEntry {
    pub id: String,
    pub content: String,
    pub category: String,
    pub converted_task_id: Option<String>,
    pub updated_at: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoutineTask {
    pub title: String,
    pub estimate_minutes: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Routine {
    pub id: String,
    pub title: String,
    pub tasks: Vec<RoutineTask>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Command {
    SaveEntry { entry: BrainEntry },
    RemoveEntry { id: String },
    Convert { id: String },
    SaveRoutine { routine: Routine },
    RemoveRoutine { id: String },
    ApplyRoutine { id: String },
}
pub fn valid_category(value: &str) -> bool {
    matches!(
        value,
        "task" | "idea" | "reminder" | "note" | "question" | "later"
    )
}
pub fn classify(text: &str) -> &'static str {
    let text = text.to_lowercase();
    if text.contains('?') {
        "question"
    } else if ["später", "someday", "irgendwann"]
        .iter()
        .any(|word| text.contains(word))
    {
        "later"
    } else if ["erinner", "remind"].iter().any(|word| text.contains(word)) {
        "reminder"
    } else if ["idee", "idea"].iter().any(|word| text.contains(word)) {
        "idea"
    } else {
        "note"
    }
}

pub fn apply_with_database(
    database: &DatabaseConnection,
    day: &str,
    zone: &str,
    command: Command,
    now: u64,
) -> Result<(), String> {
    aevio_core::coach::dates::bounds(day, zone)?;
    let now = now as i64;
    database
        .transaction(|tx| {
            crate::database::repositories::coach_brain_commands::CoachBrainCommandRepository::apply(
                tx, day, zone, command, now,
            )
            .map_err(sea_orm::DbErr::Custom)?;
            crate::database::repositories::coach_commands::CoachCommandRepository::audit(
                tx,
                day,
                "brain.changed",
                now,
            )?;
            Ok::<(), sea_orm::DbErr>(())
        })
        .map_err(|error| error.to_string())
}
