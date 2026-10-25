use crate::{timers::now_epoch_ms, AppState};
use aevio_core::coach::focus::{Focus, Start};
use aevio_persistence::coach::service;
use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
pub fn coach_focus(app: AppHandle) -> Result<Option<Focus>, String> {
    let state = app.state::<AppState>();
    let _store = state.activity.lock().map_err(|e| e.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    aevio_persistence::database::repositories::coach_focus::CoachFocusRepository::reconcile(
        &database,
        now_epoch_ms() as i64,
    )?;
    aevio_persistence::database::repositories::coach_focus::CoachFocusRepository::attribute_entries(
        &database,
    )?;
    aevio_persistence::database::repositories::coach::CoachTimeRepository::new(&database)
        .current_focus(now_epoch_ms() as i64)
}
#[tauri::command]
pub fn coach_start_focus(
    app: AppHandle,
    day: String,
    time_zone: String,
    id: String,
    minutes: u32,
    next_step: String,
    body_doubling: bool,
) -> Result<Option<Focus>, String> {
    if day != chrono::Local::now().format("%Y-%m-%d").to_string() {
        return Err("Focus can only start from today's plan".into());
    }
    if !(2..=180).contains(&minutes) {
        return Err("Choose between 2 and 180 focus minutes".into());
    }
    aevio_core::coach::policies::text(&next_step, 1000)?;
    let now = now_epoch_ms();
    let result = {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|e| e.to_string())?;
        let database = aevio_persistence::database::connect(&state.database_path)
            .map_err(|error| error.to_string())?;
        let snapshot = service::day_with_database(&database, &day, &time_zone)?;
        let task = snapshot
            .tasks
            .iter()
            .find(|task| task.id == id && !task.done)
            .cloned()
            .ok_or("Choose an unfinished task from today's plan")?;
        store.start_session("focus", Some(&task.title), now)?;
        let segment = store
            .live_state(now)
            .segment_info()
            .ok_or("Focus session did not start")?;
        if let Err(error) =
            aevio_persistence::database::repositories::coach_focus::CoachFocusRepository::begin(
                &database,
                &task.id,
                Start {
                    goal: snapshot
                        .plan
                        .as_ref()
                        .map(|p| p.goal.as_str())
                        .unwrap_or(""),
                    next_step: &next_step,
                    minutes,
                    body: body_doubling,
                    segment_id: segment.id,
                },
                now as i64,
            )
        {
            store.stop_session(now)?;
            return Err(error);
        }
        store.rebuild_range(
            now.saturating_sub(crate::activity::REBUILD_WINDOW_MS),
            now,
            now,
        )?;
        aevio_persistence::database::repositories::coach::CoachTimeRepository::new(&database)
            .current_focus(now as i64)?
    };
    crate::activity::emit_full(&app);
    if let Err(error) = app.emit(super::EVENT_COACH_DAY_CHANGED, ()) {
        eprintln!("Coach focus invalidation: {error}");
    }
    Ok(result)
}
#[tauri::command]
pub fn coach_end_focus(app: AppHandle, finish: bool) -> Result<Option<Focus>, String> {
    let now = now_epoch_ms();
    {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|e| e.to_string())?;
        let database = aevio_persistence::database::connect(&state.database_path)
            .map_err(|error| error.to_string())?;
        if let Some(current) =
            aevio_persistence::database::repositories::coach::CoachTimeRepository::new(&database)
                .current_focus(now as i64)?
        {
            if store
                .live_state(now)
                .segment_info()
                .is_some_and(|s| s.id == current.segment_id)
            {
                store.stop_session(now)?;
                store.rebuild_range(
                    now.saturating_sub(crate::activity::REBUILD_WINDOW_MS),
                    now,
                    now,
                )?;
            }
            aevio_persistence::database::repositories::coach_focus::CoachFocusRepository::reconcile(
                &database,
                now as i64,
            )?;
            aevio_persistence::database::repositories::coach_focus::CoachFocusRepository::attribute_entries(
                &database,
            )?;
            if finish {
                aevio_persistence::database::repositories::coach_focus::CoachFocusRepository::finish(
                    &database,
                    &current.id,
                    now as i64,
                )?;
            }
        }
    }
    crate::activity::emit_full(&app);
    if let Err(error) = app.emit(super::EVENT_COACH_DAY_CHANGED, ()) {
        eprintln!("Coach focus invalidation: {error}");
    }
    coach_focus(app)
}
