use std::time::Duration;

use tauri::{Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::AppState;

/// Emits the publisher-specific event retained for compatibility and the
/// global event type consumed by authorized cross-plugin subscribers.
pub fn emit_plugin_event(
    app: &tauri::AppHandle,
    event_name: &str,
    payload: serde_json::Value,
) -> Result<(), String> {
    app.emit(event_name, payload.clone())
        .map_err(|error| error.to_string())?;
    let Some(rest) = event_name.strip_prefix("plugin-event:") else {
        return Ok(());
    };
    let Some((_, event_type)) = rest.split_once(':') else {
        return Ok(());
    };
    app.emit(&format!("plugin-event:{event_type}"), payload)
        .map_err(|error| error.to_string())
}

pub fn spawn_automation_scheduler(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(60));
        let state = app.state::<AppState>();
        let result = (|| {
            let _writer = state.activity.lock().map_err(|error| error.to_string())?;
            let database = aevio_persistence::database::connect(&state.database_path)
                .map_err(|error| error.to_string())?;
            aevio_services::automation::run_daily_workflow(
                &database,
                aevio_core::timers::now_epoch_ms(),
            )
        })();
        match result {
            Ok(run) if run.queued_entries > 0 => {
                let _ = app.emit(crate::EVENT_ENTRIES_CHANGED, ());
            }
            Ok(_) => {}
            Err(error) => eprintln!("automation scheduler failed: {error}"),
        }
        if let Err(error) = run_plugin_jobs(&app) {
            eprintln!("plugin scheduler failed: {error}");
        }
        if let Err(error) = run_plugin_event_deliveries(&app) {
            eprintln!("plugin event delivery failed: {error}");
        }
    });
}

fn run_plugin_jobs(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let _writer = state.activity.lock().map_err(|error| error.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    let now = aevio_core::timers::now_epoch_ms() as i64;
    let mut failures = Vec::new();
    for job in
        aevio_services::plugins_jobs::due(&database, now).map_err(|error| error.to_string())?
    {
        let Some(job) = aevio_services::plugins_jobs::claim(&database, &job.id, now)
            .map_err(|error| error.to_string())?
        else {
            continue;
        };
        let event = aevio_services::plugins_jobs::event_name(&job.plugin_id, &job.job_id);
        let delivery = app.emit(
            &event,
            serde_json::json!({
                "pluginId": job.plugin_id,
                "jobId": job.job_id,
                "scheduledAt": now,
                "attempt": job.attempt_count,
                "payload": job.payload,
            }),
        );
        match delivery {
            Ok(()) => {
                if let Err(error) = aevio_services::plugins_jobs::mark_run(&database, &job.id, now)
                {
                    failures.push(error.to_string());
                }
            }
            Err(error) => {
                let message = error.to_string();
                let _ =
                    aevio_services::plugins_jobs::mark_failure(&database, &job.id, now, &message);
                failures.push(message);
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

fn run_plugin_event_deliveries(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let _writer = state.activity.lock().map_err(|error| error.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    let dispatch = aevio_services::plugins_events::dispatch_due(
        &database,
        aevio_core::timers::now_epoch_ms() as i64,
        &state.device_ref,
    )
    .map_err(|error| error.to_string())?;
    for effect in dispatch.effects {
        match effect {
            aevio_services::plugins_host::PluginRuntimeEffect::Event {
                event_name,
                payload,
            } => emit_plugin_event(app, &event_name, payload)?,
            aevio_services::plugins_host::PluginRuntimeEffect::Notification { title, body } => app
                .notification()
                .builder()
                .title(title)
                .body(body)
                .show()
                .map_err(|error| error.to_string())?,
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn install_bundled_cli() {
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let bundled = std::env::current_exe()?.with_file_name("aevio-cli");
        if !bundled.is_file() {
            return Ok(());
        }
        let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
        let directory = std::path::PathBuf::from(home).join(".local/bin");
        std::fs::create_dir_all(&directory)?;
        let destination = directory.join("aevio");
        let temporary = directory.join(format!(".aevio-{}", std::process::id()));
        let _ = std::fs::remove_file(&temporary);
        std::os::unix::fs::symlink(&bundled, &temporary)?;
        std::fs::rename(&temporary, &destination)?;
        Ok(())
    })();
    if let Err(error) = result {
        eprintln!("could not update ~/.local/bin/aevio: {error}");
    }
}

#[cfg(not(target_os = "macos"))]
pub fn install_bundled_cli() {}
