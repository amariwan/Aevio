use crate::{timers::now_epoch_ms, AppState};
use aevio_core::coach::policies;
use aevio_persistence::coach::brain::{self, BrainEntry, Command, Routine};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Brain {
    entries: Vec<BrainEntry>,
    routines: Vec<Routine>,
}
#[tauri::command]
pub fn coach_brain(app: AppHandle) -> Result<Brain, String> {
    let state = app.state::<AppState>();
    let _store = state.activity.lock().map_err(|e| e.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    let repository =
        aevio_persistence::database::repositories::coach::CoachBrainRepository::new(&database);
    Ok(Brain {
        entries: repository.entries()?,
        routines: repository.routines()?,
    })
}
#[tauri::command]
pub fn coach_brain_apply(
    app: AppHandle,
    day: String,
    time_zone: String,
    command: Command,
) -> Result<Brain, String> {
    {
        let state = app.state::<AppState>();
        let _store = state.activity.lock().map_err(|e| e.to_string())?;
        let database = aevio_persistence::database::connect(&state.database_path)
            .map_err(|error| error.to_string())?;
        brain::apply_with_database(&database, &day, &time_zone, command, now_epoch_ms())?;
    }
    if let Err(error) = app.emit(super::EVENT_COACH_DAY_CHANGED, ()) {
        eprintln!("Coach brain invalidation: {error}");
    }
    coach_brain(app)
}
#[tauri::command]
pub fn coach_classify_thought(content: String) -> Result<String, String> {
    policies::text(&content, 10000)?;
    Ok(brain::classify(&content).into())
}
