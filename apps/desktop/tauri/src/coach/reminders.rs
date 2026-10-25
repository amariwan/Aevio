//! Native delivery stays in the desktop adapter.
use crate::{timers::now_epoch_ms, AppState};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;
pub fn spawn_scheduler(app: AppHandle) {
    std::thread::Builder::new()
        .name("coach-reminders".to_string())
        .spawn(move || loop {
            if let Err(error) = send_due_reminders(&app) {
                eprintln!("coach reminder scheduler: {error}");
            }
            std::thread::sleep(std::time::Duration::from_secs(15));
        })
        .expect("spawn coach reminder scheduler");
}

fn send_due_reminders(app: &AppHandle) -> Result<(), String> {
    use chrono::Timelike;
    let state = app.state::<AppState>();
    let language = crate::i18n::from_settings(&state.settings_snapshot());
    let now = now_epoch_ms();
    if crate::breaks::current_state(app)?
        .paused_until
        .is_some_and(|until| until > now)
    {
        return Ok(());
    }
    let local = chrono::Local::now();
    let day = local.format("%Y-%m-%d").to_string();
    let minute = (local.hour() * 60 + local.minute()) as u16;
    // Serialize preference changes with delivery, so disabling reminders wins
    // before the next native notification is submitted.
    let _activity = state.activity.lock().map_err(|e| e.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    aevio_persistence::database::repositories::coach_reminders::CoachReminderRepository::reconcile_and_attribute(
        &database,
        now as i64,
    )?;
    let metadata =
        aevio_persistence::database::repositories::coach::CoachMetadataRepository::new(&database);
    let preferences = metadata.preferences()?;
    if !aevio_persistence::database::repositories::coach_focus::CoachFocusRepository::allowed(
        &database,
        &preferences,
        minute,
        now as i64,
    )? {
        return Ok(());
    }
    let focus_due =
        aevio_persistence::database::repositories::coach_focus::CoachFocusRepository::due(
            &database, now as i64,
        )?;
    if let Some(id) = focus_due {
        app.notification()
            .builder()
            .title(crate::i18n::coach_title(language))
            .body(crate::i18n::focus_due(language))
            .show()
            .map_err(|e| e.to_string())?;
        aevio_persistence::database::repositories::coach_focus::CoachFocusRepository::mark_reminded(
            &database,
            &id,
            now as i64,
        )?;
        aevio_persistence::database::repositories::coach_reminders::CoachReminderRepository::mark_sent(
            &database,
            &day,
            "focus.check_in",
            now as i64,
        )?;
        return Ok(());
    }
    let due =
        aevio_persistence::database::repositories::coach_focus::CoachFocusRepository::due_task(
            &database, now as i64,
        )?;
    let review = due.is_none()
        && aevio_persistence::database::repositories::coach_reminders::CoachReminderRepository::review_due(
            &database,
            &day,
            minute,
            &preferences,
        )?;
    if due.is_none() && !review {
        if preferences.automatic_hints
            && aevio_persistence::database::repositories::coach_reminders::CoachReminderRepository::guardian_due(
                &database,
                now as i64,
            )?
        {
            app.notification()
                .builder()
                .title(crate::i18n::coach_title(language))
                .body(crate::i18n::guardian(language))
                .show()
                .map_err(|e| e.to_string())?;
            aevio_persistence::database::repositories::coach_reminders::CoachReminderRepository::mark_sent(
                &database,
                &day,
                "guardian.offered",
                now as i64,
            )?;
        }
        return Ok(());
    }
    let body = due
        .as_ref()
        .map(|(_, title)| crate::i18n::task_reminder(language, title))
        .unwrap_or_else(|| crate::i18n::review(language).to_string());
    app.notification()
        .builder()
        .title(crate::i18n::coach_title(language))
        .body(body)
        .show()
        .map_err(|e| e.to_string())?;
    aevio_persistence::database::repositories::coach_reminders::CoachReminderRepository::mark_task_and_sent(
        &database,
        due,
        &day,
        if review { "review.offered" } else { "task.reminded" },
        now as i64,
    )?;
    Ok(())
}
