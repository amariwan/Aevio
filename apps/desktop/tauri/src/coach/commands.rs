use super::provider::CoachAi;
use crate::{timers::now_epoch_ms, AppState};
use aevio_core::coach::models::{AssistRequest, Command, Day, Proposal};
use aevio_persistence::coach::service;
use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
pub fn coach_day(app: AppHandle, day: String, time_zone: String) -> Result<Day, String> {
    let state = app.state::<AppState>();
    let _store = state.activity.lock().map_err(|e| e.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    service::day_with_database(&database, &day, &time_zone)
}
#[tauri::command]
pub fn coach_apply(
    app: AppHandle,
    day: String,
    time_zone: String,
    command: Command,
) -> Result<Day, String> {
    let snapshot = {
        let state = app.state::<AppState>();
        let _store = state.activity.lock().map_err(|e| e.to_string())?;
        let database = aevio_persistence::database::connect(&state.database_path)
            .map_err(|error| error.to_string())?;
        service::apply_with_database(&database, &day, &time_zone, command, now_epoch_ms())?
    };
    if let Err(error) = super::widget::set_visible(&app, snapshot.preferences.widget_visible) {
        eprintln!("Coach widget visibility: {error}");
    }
    for event in [super::EVENT_COACH_DAY_CHANGED, crate::EVENT_ENTRIES_CHANGED] {
        if let Err(error) = app.emit(event, ()) {
            eprintln!("Coach invalidation failed for {event}: {error}");
        }
    }
    Ok(snapshot)
}
#[tauri::command]
pub async fn coach_propose(app: AppHandle, request: AssistRequest) -> Result<Proposal, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (preferences, memory) = {
            let state = app.state::<AppState>();
            let _writer = state.activity.lock().map_err(|e| e.to_string())?;
            let database = aevio_persistence::database::connect(&state.database_path)
                .map_err(|error| error.to_string())?;
            let metadata =
                aevio_persistence::database::repositories::coach::CoachMetadataRepository::new(
                    &database,
                );
            let preferences = metadata.preferences()?;
            let memory = if preferences.memory_enabled {
                metadata
                    .memory()?
                    .into_iter()
                    .take(8)
                    .map(|note| note.content)
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            (preferences, memory)
        };
        app.state::<CoachAi>()
            .propose(&request, &preferences, &memory)
    })
    .await
    .map_err(|error| error.to_string())?
}
