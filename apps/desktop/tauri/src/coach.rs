//! Desktop coach facade; domains and SQL live in aevio-core.
pub mod adaptive_commands;
pub mod brain_commands;
pub mod commands;
pub mod focus_commands;
pub mod provider;
mod reminders;
pub mod widget;
use crate::{ai::AiRuntime, timers::now_epoch_ms, AppState};
pub use aevio_core::coach::legacy::{
    CoachRecommendation, CoachSnapshot, CoachTask, NewCoachTask, UpdateCoachTask,
};
use aevio_persistence::database::repositories::coach::CoachTaskRepository;
pub use reminders::spawn_scheduler;
use tauri::{AppHandle, Emitter, Manager};
pub const EVENT_COACH_CHANGED: &str = "coach-changed";
pub const EVENT_COACH_DAY_CHANGED: &str = "coach-day-changed";
pub fn list(state: &AppState) -> Result<Vec<CoachTask>, String> {
    let _writer = state.activity.lock().map_err(|e| e.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    CoachTaskRepository::new(&database)
        .list()
        .map_err(|error| error.to_string())
}
pub fn create(state: &AppState, input: NewCoachTask) -> Result<CoachTask, String> {
    let _writer = state.activity.lock().map_err(|e| e.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    CoachTaskRepository::new(&database).create(input, now_epoch_ms() as i64)
}
pub fn update(state: &AppState, id: &str, patch: UpdateCoachTask) -> Result<CoachTask, String> {
    let _writer = state.activity.lock().map_err(|e| e.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    CoachTaskRepository::new(&database).update(id, patch, now_epoch_ms() as i64)
}
pub fn delete(state: &AppState, id: &str) -> Result<(), String> {
    let _writer = state.activity.lock().map_err(|e| e.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    CoachTaskRepository::new(&database).delete(id)
}
pub fn publish(app: &AppHandle) {
    if let Ok(snapshot) = snapshot(app) {
        if let Err(error) = app.emit(EVENT_COACH_CHANGED, &snapshot) {
            eprintln!("Coach snapshot event: {error}");
        }
    }
    if let Err(error) = app.emit(EVENT_COACH_DAY_CHANGED, ()) {
        eprintln!("Coach invalidation: {error}");
    }
}
pub fn snapshot(app: &AppHandle) -> Result<CoachSnapshot, String> {
    let state = app.state::<AppState>();
    let tasks = list(&state)?;
    let now = now_epoch_ms();
    use chrono::TimeZone;
    let local = chrono::Local::now();
    let date = local.date_naive();
    let day_start = chrono::Local
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).ok_or("invalid day")?)
        .earliest()
        .ok_or("Invalid local midnight")?
        .timestamp_millis();
    let next = date.succ_opt().ok_or("Date out of range")?;
    let day_end = chrono::Local
        .from_local_datetime(&next.and_hms_opt(0, 0, 0).ok_or("invalid day")?)
        .earliest()
        .ok_or("Invalid local midnight")?
        .timestamp_millis();
    let tracked_ms = {
        let database = aevio_persistence::database::connect(&state.database_path)
            .map_err(|error| error.to_string())?;
        aevio_persistence::database::repositories::coach::CoachTimeRepository::new(&database)
            .entries(day_start, day_end, &[])?
            .iter()
            .map(|entry| entry.duration_ms)
            .sum()
    };
    let settings = state.settings_snapshot();
    let daily_target_ms = (settings.weekly_target_hours as u64 * 3_600_000) / 5;
    let ai = app.state::<AiRuntime>().status();
    let mut recommendations = tasks
        .iter()
        .filter(|task| task.status == "open")
        .map(|task| {
            let overdue = task.due_at.is_some_and(|due| due < now);
            let score = task.priority * 20
                + if overdue { 40 } else { 0 }
                + if task.due_at.is_some() { 10 } else { 0 };
            let reason = if overdue {
                "Überfällig – zuerst abschließen".to_string()
            } else if task.due_at.is_some() {
                "Nächster Termin und hohe Wirkung".to_string()
            } else {
                "Passt als nächster fokussierter Schritt".to_string()
            };
            CoachRecommendation {
                task_id: task.id.clone(),
                title: task.title.clone(),
                reason,
                score,
                source: "Regeln".to_string(),
            }
        })
        .collect::<Vec<_>>();
    recommendations.sort_by_key(|item| -item.score);
    recommendations.truncate(3);
    let ai_available = ai.llm == "available";
    if ai_available {
        for item in &mut recommendations {
            item.source = "Regeln · KI bereit".to_string();
        }
    }
    let next_reminder_at = tasks
        .iter()
        .filter(|task| task.status == "open")
        .filter_map(|task| task.reminder_at)
        .filter(|at| *at >= now)
        .min();
    Ok(CoachSnapshot {
        tasks,
        recommendations,
        tracked_ms,
        daily_target_ms,
        ai_engine: ai.engine,
        ai_available,
        next_reminder_at,
    })
}
