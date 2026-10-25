//! IPC surface.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::ipc::{InvokeBody, Request, Response};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_notification::NotificationExt;

use crate::activity::{self, ActivitySnapshot};
use crate::ai::metrics::AiMetrics;
use crate::ai::{self, AiRuntime, AiStatus};
use crate::breaks;
use crate::coach::{self, CoachSnapshot, CoachTask, NewCoachTask, UpdateCoachTask};
use crate::invoices::{self, profile, BillableEntry, DraftInput, Invoice, InvoiceSummary};
use crate::login_item::{self, LoginItemState};
use crate::models::{
    AppRecord, Category, Client, EntryDetail, Label, NewCategory, NewClient, NewLabel, NewProject,
    NewTask, NewTimeEntry, Project, RuleSuggestion, Task, TimeEntry, UpdateCategory, UpdateClient,
    UpdateLabel, UpdateProject, UpdateTask, UpdateTimeEntry,
};
use crate::projects::{HintPreview, ImportSummary, ProjectRule, ProjectStats, ProjectSuggestion};
use crate::settings::Settings;
use crate::tray;
use crate::updater::{self, UpdateStatus, UpdaterState};
use crate::usage::UsageSnapshot;
use crate::watch_bridge::PairingInfo;
use crate::AppState;
use aevio_contracts::plugin::{
    PluginInstallPlan, PluginInstallationScope, PluginInstallationSummary,
    PluginJobScheduleRequest, PluginJobSummary, PluginManifest, PluginRelease, PluginUpdatePolicy,
    MAX_ASSET_BYTES,
};
use aevio_core::integrations::IntegrationConnection;
use aevio_core::reports::{ExportFormat, ExportResult, GroupBy, RollupCell};
use aevio_core::timers::now_epoch_ms;
use aevio_core::EntryFilter;
use aevio_infrastructure::decision_settings::DecisionSettings;
use aevio_infrastructure::timers_store::Timer;
use aevio_persistence::reports;
use aevio_services::automation::{
    self, AutomationSnapshot, DailyWorkflowRun, DailyWorkflowSnapshot, Dashboard, ReportDefinition,
    Routine, Run,
};
use aevio_services::decision::{
    use_cases, DecisionPolicy, DecisionProvider, DecisionRequest, DecisionResult, DecisionRouter,
    RemoteProtocol, RemoteProvider, RuleProvider,
};
use aevio_services::finance::{
    self, CapacityPlan, CapacityPlanInput, Contract, ContractInput, DashboardShare, FinanceFilter,
    FinanceSnapshot, RateCard, RateCardInput,
};
use aevio_services::governance::{self, DefaultsInput, DepartmentInput, MemberInput, TeamInput};
use aevio_services::staffing::{
    self, Freelancer, FreelancerInput, StaffingMatch, StaffingSnapshot,
};
use aevio_services::timeline::{
    self, PublishInput, SyncPullResponse, SyncPushRequest, SyncPushResponse, TimelineDevice,
    TimelineEvent, TimelineSyncInput, TimelineSyncResult, TimelineSyncState,
};
use aevio_services::workspace::{
    self, NewAttachment, NewComment, NewCycle, NewDocument, NewInitiative, NewNote, UpdateDocument,
    UpdateNote,
};

type Timers = Result<Vec<Timer>, String>;

#[tauri::command]
pub fn watch_pairing(app: AppHandle) -> Result<PairingInfo, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    crate::watch_bridge::pairing_info(&data_dir)
}

#[tauri::command]
pub fn list_music_tracks(app: AppHandle) -> Result<Vec<aevio_core::music::MusicTrack>, String> {
    with_database(&app, |database| {
        aevio_persistence::database::repositories::music_tracks::MusicTrackRepository::new(database)
            .list()
            .map(|tracks| tracks.into_iter().map(to_music_track).collect())
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn import_music_track(app: AppHandle) -> Result<Option<aevio_core::music::MusicTrack>, String> {
    let Some(file) = app
        .dialog()
        .file()
        .add_filter("Audio files", &["mp3", "wav", "ogg", "m4a", "flac", "aac"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let source = file.into_path().map_err(|error| error.to_string())?;
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|value| {
            matches!(
                value.as_str(),
                "mp3" | "wav" | "ogg" | "m4a" | "flac" | "aac"
            )
        })
        .ok_or_else(|| "unsupported audio file type".to_owned())?;
    let metadata = std::fs::metadata(&source).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > 100 * 1024 * 1024 {
        return Err("audio file must be a regular file smaller than 100 MiB".into());
    }
    let state = app.state::<AppState>();
    let root = state
        .database_path
        .parent()
        .ok_or_else(|| "Aevio data directory is unavailable".to_string())?
        .join("music");
    std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let destination = root.join(format!("{}.{}", uuid::Uuid::now_v7(), extension));
    std::fs::copy(&source, &destination).map_err(|error| error.to_string())?;
    let title = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Imported track")
        .chars()
        .filter(|character| !character.is_control())
        .take(160)
        .collect::<String>();
    let result = with_database(&app, |database| {
        aevio_persistence::database::repositories::music_tracks::MusicTrackRepository::new(database)
            .insert_local(
                if title.is_empty() {
                    "Imported track".into()
                } else {
                    title
                },
                destination.to_string_lossy().into_owned(),
                now_epoch_ms() as i64,
            )
            .map(to_music_track)
            .map_err(|error| error.to_string())
    });
    if result.is_err() {
        let _ = std::fs::remove_file(&destination);
    }
    result.map(Some)
}

fn to_music_track(
    track: aevio_persistence::database::entities::music_tracks::Model,
) -> aevio_core::music::MusicTrack {
    let source_url = (track.kind == "local").then(|| format!("aevio-music://track/{}", track.id));
    aevio_core::music::MusicTrack {
        id: track.id,
        title: track.title,
        description: track.description,
        kind: track.kind,
        source_url,
    }
}

// --- Plugin platform ----------------------------------------------------

#[tauri::command]
pub fn plugin_catalog(
    app: AppHandle,
) -> Result<Vec<aevio_contracts::plugin::PluginCatalogEntry>, String> {
    with_database(&app, |database| {
        aevio_services::plugins::catalog(database).map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn plugin_installed(
    app: AppHandle,
) -> Result<Vec<aevio_contracts::plugin::PluginInstallationSummary>, String> {
    with_database(&app, |database| {
        aevio_services::plugins::installed(database).map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn plugin_contributions(
    app: AppHandle,
) -> Result<Vec<aevio_contracts::plugin::PluginContributionSnapshot>, String> {
    with_database(&app, |database| {
        aevio_services::plugins::active_contributions(database).map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn plugin_health(
    app: AppHandle,
) -> Result<Vec<aevio_contracts::plugin::PluginHealthSummary>, String> {
    with_database(&app, |database| {
        aevio_services::plugins_artifacts::health(database).map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn plugin_layout(
    app: AppHandle,
) -> Result<Vec<aevio_contracts::plugin::PluginContributionPreference>, String> {
    with_database_reader(&app, |database| {
        aevio_services::plugins_layout::list(database).map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn plugin_set_contribution_preference(
    app: AppHandle,
    preference: aevio_contracts::plugin::PluginContributionPreference,
) -> Result<aevio_contracts::plugin::PluginContributionPreference, String> {
    let result = with_database(&app, |database| {
        aevio_services::plugins_layout::set(database, preference, now_epoch_ms() as i64)
            .map_err(|error| error.to_string())
    });
    if result.is_ok() {
        let _ = app.emit(crate::EVENT_PLUGINS_CHANGED, ());
    }
    result
}

#[tauri::command]
pub fn plugin_restore_layout(app: AppHandle) -> Result<(), String> {
    let result = with_database(&app, |database| {
        aevio_services::plugins_layout::restore(database).map_err(|error| error.to_string())
    });
    if result.is_ok() {
        let _ = app.emit(crate::EVENT_PLUGINS_CHANGED, ());
    }
    result
}

#[tauri::command]
pub fn plugin_update_policy(
    app: AppHandle,
    plugin_id: String,
) -> Result<PluginUpdatePolicy, String> {
    with_database_reader(&app, |database| {
        aevio_services::plugins_update::get(database, &plugin_id).map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn plugin_set_update_policy(
    app: AppHandle,
    policy: PluginUpdatePolicy,
) -> Result<PluginUpdatePolicy, String> {
    let result = with_database(&app, |database| {
        aevio_services::plugins_update::set(database, policy, now_epoch_ms() as i64)
            .map_err(|error| error.to_string())
    });
    if result.is_ok() {
        let _ = app.emit(crate::EVENT_PLUGINS_CHANGED, ());
    }
    result
}

#[tauri::command]
pub fn plugin_pick_package(app: AppHandle) -> Result<Option<String>, String> {
    let Some(file) = app
        .dialog()
        .file()
        .add_filter("Aevio plugin package", &["zip", "aevio"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    file.into_path()
        .map(|path| Some(path.to_string_lossy().into_owned()))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn plugin_execute_command(
    app: AppHandle,
    request: aevio_contracts::plugin::PluginCommandRequest,
) -> Result<aevio_contracts::plugin::PluginCommandResult, String> {
    let plugin_id = request.plugin_id.clone();
    let device_ref = app.state::<AppState>().device_ref.clone();
    let execution = with_database(&app, |database| {
        aevio_services::plugins_runtime::execute_command_with_effects_for_device(
            database,
            request,
            &device_ref,
        )
        .map_err(|error| error.to_string())
    });
    let (result, effects) = match execution {
        Ok(value) => value,
        Err(error) => {
            let _ = with_database(&app, |database| {
                aevio_services::plugins_artifacts::record_runtime_failure(
                    database,
                    &plugin_id,
                    &error,
                    now_epoch_ms() as i64,
                )
                .map(|_| ())
                .map_err(|failure| failure.to_string())
            });
            return Err(error);
        }
    };
    for effect in effects {
        match effect {
            aevio_services::plugins_host::PluginRuntimeEffect::Event {
                event_name,
                payload,
            } => crate::runtime::emit_plugin_event(&app, &event_name, payload)?,
            aevio_services::plugins_host::PluginRuntimeEffect::Notification { title, body } => app
                .notification()
                .builder()
                .title(title)
                .body(body)
                .show()
                .map_err(|error| error.to_string())?,
        }
    }
    Ok(result)
}

#[tauri::command]
pub fn plugin_open_ui(app: AppHandle, plugin_id: String) -> Result<(), String> {
    let entrypoint = with_database_reader(&app, |database| {
        aevio_services::plugins_artifacts::ui_entrypoint(database, &plugin_id)
            .map_err(|error| error.to_string())
    })?;
    let label = format!("plugin-ui-{plugin_id}");
    if let Some(window) = app.get_webview_window(&label) {
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }
    let url = url::Url::parse(&format!("aevio-plugin://{plugin_id}/{entrypoint}"))
        .map_err(|error| format!("invalid plugin UI URL: {error}"))?;
    WebviewWindowBuilder::new(&app, label, WebviewUrl::CustomProtocol(url))
        .title(format!("Aevio · {plugin_id}"))
        .inner_size(960.0, 720.0)
        .build()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn plugin_search(
    app: AppHandle,
    plugin_id: String,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<aevio_contracts::plugin::PluginSearchResult>, String> {
    with_database_reader(&app, |database| {
        aevio_services::plugins_runtime::search(
            database,
            &plugin_id,
            &query,
            limit.unwrap_or(20) as usize,
        )
        .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn plugin_search_all(
    app: AppHandle,
    query: String,
    limit: Option<u32>,
) -> Result<aevio_contracts::plugin::PluginSearchAggregate, String> {
    with_database_reader(&app, |database| {
        aevio_services::plugins_runtime::search_all(database, &query, limit.unwrap_or(20) as usize)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn plugin_publish_event(
    app: AppHandle,
    plugin_id: String,
    event: String,
    payload: serde_json::Value,
) -> Result<(), String> {
    let event_name = with_database_reader(&app, |database| {
        aevio_services::plugins_runtime::publish_event(database, &plugin_id, &event, &payload)
            .map_err(|error| error.to_string())
    })?;
    crate::runtime::emit_plugin_event(&app, &event_name, payload)
}

#[tauri::command]
pub fn plugin_subscribe_event(
    app: AppHandle,
    plugin_id: String,
    event: String,
) -> Result<(), String> {
    with_database_reader(&app, |database| {
        aevio_services::plugins_runtime::authorize_subscription(database, &plugin_id, &event)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn plugin_show_notification(
    app: AppHandle,
    plugin_id: String,
    title: String,
    body: String,
) -> Result<(), String> {
    with_database(&app, |database| {
        aevio_services::plugins_notifications::show(
            database,
            &plugin_id,
            &title,
            &body,
            now_epoch_ms() as i64,
        )
        .map_err(|error| error.to_string())
    })?;
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn plugin_schedule_job(
    app: AppHandle,
    request: PluginJobScheduleRequest,
) -> Result<PluginJobSummary, String> {
    with_database(&app, |database| {
        aevio_services::plugins_jobs::schedule(database, request, now_epoch_ms() as i64)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn plugin_cancel_job(app: AppHandle, plugin_id: String, job_id: String) -> Result<(), String> {
    with_database(&app, |database| {
        aevio_services::plugins_jobs::cancel(database, &plugin_id, &job_id, now_epoch_ms() as i64)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn plugin_workflow_run(
    app: AppHandle,
    request: aevio_contracts::plugin::PluginWorkflowRequest,
) -> Result<aevio_contracts::plugin::PluginWorkflowRunSummary, String> {
    let device_ref = app.state::<AppState>().device_ref.clone();
    let (summary, effects) = with_database(&app, |database| {
        aevio_services::plugins_workflows::run(
            database,
            request,
            now_epoch_ms() as i64,
            &device_ref,
        )
        .map_err(|error| error.to_string())
    })?;
    for effect in effects {
        match effect {
            aevio_services::plugins_host::PluginRuntimeEffect::Event {
                event_name,
                payload,
            } => crate::runtime::emit_plugin_event(&app, &event_name, payload)?,
            aevio_services::plugins_host::PluginRuntimeEffect::Notification { title, body } => app
                .notification()
                .builder()
                .title(title)
                .body(body)
                .show()
                .map_err(|error| error.to_string())?,
        }
    }
    Ok(summary)
}

#[tauri::command]
pub fn plugin_workflow_cancel(
    app: AppHandle,
    plugin_id: String,
    workflow_id: String,
    idempotency_key: String,
) -> Result<(), String> {
    with_database(&app, |database| {
        aevio_services::plugins_workflows::cancel(
            database,
            &plugin_id,
            &workflow_id,
            &idempotency_key,
            now_epoch_ms() as i64,
        )
        .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn plugin_workflow_history(
    app: AppHandle,
    plugin_id: String,
) -> Result<Vec<aevio_contracts::plugin::PluginWorkflowRunSummary>, String> {
    with_database_reader(&app, |database| {
        aevio_services::plugins_workflows::history(database, &plugin_id)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn plugin_plan_install(
    app: AppHandle,
    plugin_id: String,
    scope: Option<PluginInstallationScope>,
) -> Result<PluginInstallPlan, String> {
    with_database(&app, |database| {
        aevio_services::plugins::plan_install(
            database,
            &plugin_id,
            scope.unwrap_or(PluginInstallationScope::Personal),
        )
        .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn plugin_install(
    app: AppHandle,
    plugin_id: String,
    scope: Option<PluginInstallationScope>,
    plan: Option<PluginInstallPlan>,
) -> Result<PluginInstallationSummary, String> {
    let result = with_database(&app, |database| {
        match plan {
            Some(plan) => aevio_services::plugins::install_builtin_from_plan(
                database,
                plan,
                now_epoch_ms() as i64,
            ),
            None => aevio_services::plugins::install_builtin(
                database,
                &plugin_id,
                scope.unwrap_or(PluginInstallationScope::Personal),
                now_epoch_ms() as i64,
            ),
        }
        .map_err(|error| error.to_string())
    });
    if result.is_ok() {
        let _ = app.emit(crate::EVENT_PLUGINS_CHANGED, ());
    }
    result
}

#[tauri::command]
pub fn plugin_grant_permissions(
    app: AppHandle,
    plugin_id: String,
    permissions: Vec<String>,
) -> Result<PluginInstallationSummary, String> {
    let result = with_database(&app, |database| {
        aevio_services::plugins::grant_permissions(
            database,
            &plugin_id,
            permissions,
            now_epoch_ms() as i64,
        )
        .map_err(|error| error.to_string())
    });
    if result.is_ok() {
        let _ = app.emit(crate::EVENT_PLUGINS_CHANGED, ());
    }
    result
}

#[tauri::command]
pub fn plugin_revoke_permissions(
    app: AppHandle,
    plugin_id: String,
    permissions: Vec<String>,
) -> Result<PluginInstallationSummary, String> {
    let result = with_database(&app, |database| {
        aevio_services::plugins::revoke_permissions(
            database,
            &plugin_id,
            permissions,
            now_epoch_ms() as i64,
        )
        .map_err(|error| error.to_string())
    });
    if result.is_ok() {
        let _ = app.emit(crate::EVENT_PLUGINS_CHANGED, ());
    }
    result
}

#[tauri::command]
pub fn plugin_set_enabled(
    app: AppHandle,
    plugin_id: String,
    enabled: bool,
) -> Result<PluginInstallationSummary, String> {
    let result = with_database(&app, |database| {
        aevio_services::plugins::set_enabled(database, &plugin_id, enabled, now_epoch_ms() as i64)
            .map_err(|error| error.to_string())
    });
    if result.is_ok() {
        let _ = app.emit(crate::EVENT_PLUGINS_CHANGED, ());
    }
    result
}

#[tauri::command]
pub fn plugin_uninstall(app: AppHandle, plugin_id: String) -> Result<(), String> {
    let artifact_root = app
        .state::<AppState>()
        .database_path
        .parent()
        .ok_or_else(|| "Aevio data directory is unavailable".to_string())?
        .join("plugins");
    let result = with_database(&app, |database| {
        aevio_services::plugins::uninstall(
            database,
            &plugin_id,
            now_epoch_ms() as i64,
            Some(&artifact_root),
        )
        .map_err(|error| error.to_string())
    });
    if result.is_ok() {
        let _ = app.emit(crate::EVENT_PLUGINS_CHANGED, ());
    }
    result
}

#[tauri::command]
pub fn plugin_delete_data(app: AppHandle, plugin_id: String) -> Result<(), String> {
    let result = with_database(&app, |database| {
        aevio_services::plugins::delete_data(database, &plugin_id, now_epoch_ms() as i64)
            .map_err(|error| error.to_string())
    });
    if result.is_ok() {
        let _ = app.emit(crate::EVENT_PLUGINS_CHANGED, ());
    }
    result
}

#[tauri::command]
pub fn plugin_storage_get(
    app: AppHandle,
    plugin_id: String,
    key: String,
) -> Result<Option<Value>, String> {
    let device_ref = app.state::<AppState>().device_ref.clone();
    with_database(&app, |database| {
        aevio_services::plugins_storage::get_for_device(database, &plugin_id, &key, &device_ref)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn plugin_storage_set(
    app: AppHandle,
    plugin_id: String,
    key: String,
    value: Value,
) -> Result<(), String> {
    let device_ref = app.state::<AppState>().device_ref.clone();
    with_database(&app, |database| {
        aevio_services::plugins_storage::set_for_device(
            database,
            &plugin_id,
            &key,
            value,
            now_epoch_ms() as i64,
            &device_ref,
        )
        .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn plugin_storage_delete(app: AppHandle, plugin_id: String, key: String) -> Result<(), String> {
    let device_ref = app.state::<AppState>().device_ref.clone();
    with_database(&app, |database| {
        aevio_services::plugins_storage::delete_for_device(
            database,
            &plugin_id,
            &key,
            now_epoch_ms() as i64,
            &device_ref,
        )
        .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn plugin_install_package(
    app: AppHandle,
    package_path: String,
    release: PluginRelease,
    scope: Option<PluginInstallationScope>,
    permissions: Vec<String>,
    enable: bool,
) -> Result<PluginInstallationSummary, String> {
    let package_path = std::path::PathBuf::from(package_path);
    let size = std::fs::metadata(&package_path)
        .map_err(|error| error.to_string())?
        .len();
    if size > MAX_ASSET_BYTES {
        return Err("plugin package exceeds the host size limit".into());
    }
    let bytes = std::fs::read(&package_path).map_err(|error| error.to_string())?;
    let verified = aevio_infrastructure::plugins_package::verify_package(&bytes, &release)
        .map_err(|error| error.to_string())?;
    let manifest: PluginManifest = verified.artifact.manifest.clone();
    let state = app.state::<AppState>();
    let root = state
        .database_path
        .parent()
        .ok_or_else(|| "Aevio data directory is unavailable".to_string())?
        .join("plugins");
    let staged = aevio_infrastructure::plugins_package::stage_package(&verified, &root)
        .map_err(|error| error.to_string())?;
    let mut persisted = false;
    let result = with_database(&app, |database| {
        let mut installation = aevio_services::plugins_artifacts::install_external(
            database,
            manifest,
            release,
            scope.unwrap_or(PluginInstallationScope::Personal),
            &staged.path,
            now_epoch_ms() as i64,
        )
        .map_err(|error| error.to_string())?;
        persisted = true;
        if !permissions.is_empty() {
            installation = aevio_services::plugins::grant_permissions(
                database,
                &installation.plugin_id,
                permissions,
                now_epoch_ms() as i64,
            )
            .map_err(|error| error.to_string())?;
        }
        if enable {
            installation = aevio_services::plugins::set_enabled(
                database,
                &installation.plugin_id,
                true,
                now_epoch_ms() as i64,
            )
            .map_err(|error| error.to_string())?;
        }
        Ok(installation)
    });
    if result.is_err() && !persisted {
        let _ = std::fs::remove_dir_all(&staged.path);
    } else {
        let _ = app.emit(crate::EVENT_PLUGINS_CHANGED, ());
    }
    result
}

#[tauri::command]
pub fn plugin_rollback(
    app: AppHandle,
    plugin_id: String,
) -> Result<PluginInstallationSummary, String> {
    let result = with_database(&app, |database| {
        aevio_services::plugins_artifacts::rollback(database, &plugin_id, now_epoch_ms() as i64)
            .map_err(|error| error.to_string())
    });
    if result.is_ok() {
        let _ = app.emit(crate::EVENT_PLUGINS_CHANGED, ());
    }
    result
}

#[tauri::command]
pub fn plugin_recover(
    app: AppHandle,
    plugin_id: String,
) -> Result<PluginInstallationSummary, String> {
    let result = with_database(&app, |database| {
        aevio_services::plugins_artifacts::recover(database, &plugin_id, now_epoch_ms() as i64)
            .map_err(|error| error.to_string())
    });
    if result.is_ok() {
        let _ = app.emit(crate::EVENT_PLUGINS_CHANGED, ());
    }
    result
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentChatReply {
    pub answer: String,
    pub mode: String,
    pub context: AgentChatContext,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentChatContext {
    pub pending_entries: u32,
    pub active_projects: u32,
    pub categories: u32,
    pub ai_engine: String,
    pub pending_preview: Vec<String>,
    pub project_names: Vec<String>,
    pub category_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewRoutine {
    pub name: String,
    pub prompt: String,
    pub cadence: Option<String>,
    pub send_at: Option<String>,
    pub delivery: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewReport {
    pub name: String,
    pub filter: Value,
    pub cadence: Option<String>,
    pub delivery: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewDashboard {
    pub name: String,
    pub filter: Value,
}

// --- Workspace content -------------------------------------------------

#[tauri::command]
pub fn list_documents(app: AppHandle) -> Result<Vec<workspace::Document>, String> {
    with_database(&app, workspace::list_documents)
}

#[tauri::command]
pub fn create_document(app: AppHandle, input: NewDocument) -> Result<workspace::Document, String> {
    with_database(&app, |db| {
        workspace::create_document(db, input, now_epoch_ms())
    })
}

#[tauri::command]
pub fn update_document(
    app: AppHandle,
    id: String,
    patch: UpdateDocument,
) -> Result<workspace::Document, String> {
    with_database(&app, |db| {
        workspace::update_document(db, &id, patch, now_epoch_ms())
    })
}

#[tauri::command]
pub fn delete_document(app: AppHandle, id: String) -> Result<(), String> {
    with_database(&app, |db| workspace::delete_document(db, &id))
}

#[tauri::command]
pub fn list_notes(app: AppHandle) -> Result<Vec<workspace::Note>, String> {
    with_database(&app, workspace::list_notes)
}

#[tauri::command]
pub fn create_note(app: AppHandle, input: NewNote) -> Result<workspace::Note, String> {
    with_database(&app, |db| workspace::create_note(db, input, now_epoch_ms()))
}

#[tauri::command]
pub fn update_note(
    app: AppHandle,
    id: String,
    patch: UpdateNote,
) -> Result<workspace::Note, String> {
    with_database(&app, |db| {
        workspace::update_note(db, &id, patch, now_epoch_ms())
    })
}

#[tauri::command]
pub fn delete_note(app: AppHandle, id: String) -> Result<(), String> {
    with_database(&app, |db| workspace::delete_note(db, &id))
}

#[tauri::command]
pub fn list_comments(
    app: AppHandle,
    target_type: String,
    target_id: String,
) -> Result<Vec<workspace::Comment>, String> {
    with_database(&app, |db| {
        workspace::list_comments(db, &target_type, &target_id)
    })
}

#[tauri::command]
pub fn create_comment(app: AppHandle, input: NewComment) -> Result<workspace::Comment, String> {
    with_database(&app, |db| {
        workspace::create_comment(db, input, now_epoch_ms())
    })
}

#[tauri::command]
pub fn list_attachments(
    app: AppHandle,
    target_type: String,
    target_id: String,
) -> Result<Vec<workspace::Attachment>, String> {
    with_database(&app, |db| {
        workspace::list_attachments(db, &target_type, &target_id)
    })
}

#[tauri::command]
pub fn create_attachment(
    app: AppHandle,
    input: NewAttachment,
) -> Result<workspace::Attachment, String> {
    with_database(&app, |db| {
        workspace::create_attachment(db, input, now_epoch_ms())
    })
}

#[tauri::command]
pub fn list_mentions(
    app: AppHandle,
    target_type: String,
    target_id: String,
) -> Result<Vec<workspace::Mention>, String> {
    with_database(&app, |db| {
        workspace::list_mentions(db, &target_type, &target_id)
    })
}

#[tauri::command]
pub fn list_cycles(app: AppHandle) -> Result<Vec<workspace::Cycle>, String> {
    with_database(&app, workspace::list_cycles)
}

#[tauri::command]
pub fn create_cycle(app: AppHandle, input: NewCycle) -> Result<workspace::Cycle, String> {
    with_database(&app, |db| {
        workspace::create_cycle(db, input, now_epoch_ms())
    })
}

#[tauri::command]
pub fn list_initiatives(app: AppHandle) -> Result<Vec<workspace::Initiative>, String> {
    with_database(&app, workspace::list_initiatives)
}

#[tauri::command]
pub fn create_initiative(
    app: AppHandle,
    input: NewInitiative,
) -> Result<workspace::Initiative, String> {
    with_database(&app, |db| {
        workspace::create_initiative(db, input, now_epoch_ms())
    })
}

// --- Workspace governance ---------------------------------------------

#[tauri::command]
pub fn governance_snapshot(
    app: AppHandle,
    actor_id: Option<String>,
) -> Result<governance::GovernanceSnapshot, String> {
    with_database(&app, |db| {
        governance::snapshot(db, actor_id.as_deref().unwrap_or("member-local"))
    })
}

#[tauri::command]
pub fn save_governance_team(
    app: AppHandle,
    actor_id: Option<String>,
    input: TeamInput,
) -> Result<governance::Team, String> {
    with_database(&app, |db| {
        governance::save_team(db, actor_id.as_deref().unwrap_or("member-local"), input)
    })
}

#[tauri::command]
pub fn save_governance_department(
    app: AppHandle,
    actor_id: Option<String>,
    input: DepartmentInput,
) -> Result<governance::Department, String> {
    with_database(&app, |db| {
        governance::save_department(db, actor_id.as_deref().unwrap_or("member-local"), input)
    })
}

#[tauri::command]
pub fn save_governance_member(
    app: AppHandle,
    actor_id: Option<String>,
    input: MemberInput,
) -> Result<(), String> {
    with_database(&app, |db| {
        governance::save_member(db, actor_id.as_deref().unwrap_or("member-local"), input)
    })
}

#[tauri::command]
pub fn save_governance_defaults(
    app: AppHandle,
    actor_id: Option<String>,
    input: DefaultsInput,
) -> Result<(), String> {
    with_database(&app, |db| {
        governance::save_defaults(db, actor_id.as_deref().unwrap_or("member-local"), input)
    })
}

#[tauri::command]
pub fn authorize_governance(
    app: AppHandle,
    actor_id: Option<String>,
    permission: String,
) -> Result<bool, String> {
    with_database(&app, |db| {
        governance::authorize(
            db,
            actor_id.as_deref().unwrap_or("member-local"),
            &permission,
        )
    })
}

// --- Productivity coach ------------------------------------------------

#[tauri::command]
pub fn coach_snapshot(app: AppHandle) -> Result<CoachSnapshot, String> {
    coach::snapshot(&app)
}

#[tauri::command]
pub fn list_coach_tasks(app: AppHandle) -> Result<Vec<CoachTask>, String> {
    coach::list(&app.state::<AppState>())
}

#[tauri::command]
pub fn create_coach_task(app: AppHandle, task: NewCoachTask) -> Result<CoachTask, String> {
    let created = coach::create(&app.state::<AppState>(), task)?;
    coach::publish(&app);
    Ok(created)
}

#[tauri::command]
pub fn update_coach_task(
    app: AppHandle,
    id: String,
    patch: UpdateCoachTask,
) -> Result<CoachTask, String> {
    let updated = coach::update(&app.state::<AppState>(), &id, patch)?;
    coach::publish(&app);
    Ok(updated)
}

#[tauri::command]
pub fn complete_coach_task(app: AppHandle, id: String) -> Result<CoachTask, String> {
    let updated = coach::update(
        &app.state::<AppState>(),
        &id,
        UpdateCoachTask {
            status: Some("done".to_string()),
            ..Default::default()
        },
    )?;
    coach::publish(&app);
    Ok(updated)
}

#[tauri::command]
pub fn delete_coach_task(app: AppHandle, id: String) -> Result<(), String> {
    coach::delete(&app.state::<AppState>(), &id)?;
    coach::publish(&app);
    Ok(())
}

pub(crate) fn refresh_tray(app: &AppHandle, timers: &[Timer]) {
    if let Err(error) = tray::refresh(app, timers) {
        eprintln!("tray refresh failed: {error}");
    }
}

// --- Timers -------------------------------------------------------------

#[tauri::command]
pub fn list_timers(app: AppHandle) -> Timers {
    let state = app.state::<AppState>();
    let store = state.store.lock().map_err(|error| error.to_string())?;
    store.snapshot()
}

#[tauri::command]
pub fn create_timer(app: AppHandle, label: String) -> Timers {
    let timers = {
        let state = app.state::<AppState>();
        let mut store = state.store.lock().map_err(|error| error.to_string())?;
        store.create(&label)?
    };
    refresh_tray(&app, &timers);
    Ok(timers)
}

#[tauri::command]
pub fn start_timer(app: AppHandle, id: String) -> Timers {
    let timers = {
        let state = app.state::<AppState>();
        let mut store = state.store.lock().map_err(|error| error.to_string())?;
        store.start(&id)?
    };
    refresh_tray(&app, &timers);
    Ok(timers)
}

#[tauri::command]
pub fn pause_timer(app: AppHandle, id: String) -> Timers {
    pause_timer_by_id(&app, &id)
}

pub fn pause_timer_by_id(app: &AppHandle, id: &str) -> Timers {
    let timers = {
        let state = app.state::<AppState>();
        let mut store = state.store.lock().map_err(|error| error.to_string())?;
        store.pause(id)?
    };
    refresh_tray(app, &timers);
    Ok(timers)
}

#[tauri::command]
pub fn reset_timer(app: AppHandle, id: String) -> Timers {
    let timers = {
        let state = app.state::<AppState>();
        let mut store = state.store.lock().map_err(|error| error.to_string())?;
        store.reset(&id)?
    };
    refresh_tray(&app, &timers);
    Ok(timers)
}

#[tauri::command]
pub fn rename_timer(app: AppHandle, id: String, label: String) -> Timers {
    let timers = {
        let state = app.state::<AppState>();
        let mut store = state.store.lock().map_err(|error| error.to_string())?;
        store.rename(&id, &label)?
    };
    refresh_tray(&app, &timers);
    Ok(timers)
}

#[tauri::command]
pub fn add_timer_segment(app: AppHandle, id: String, duration_ms: u64) -> Timers {
    let timers = {
        let state = app.state::<AppState>();
        let mut store = state.store.lock().map_err(|error| error.to_string())?;
        store.add_segment(&id, duration_ms)?
    };
    refresh_tray(&app, &timers);
    Ok(timers)
}

#[tauri::command]
pub fn update_timer_segment(
    app: AppHandle,
    id: String,
    segment_id: i64,
    duration_ms: u64,
) -> Timers {
    let timers = {
        let state = app.state::<AppState>();
        let mut store = state.store.lock().map_err(|error| error.to_string())?;
        store.update_segment(&id, segment_id, duration_ms)?
    };
    refresh_tray(&app, &timers);
    Ok(timers)
}

#[tauri::command]
pub fn delete_timer_segment(app: AppHandle, id: String, segment_id: i64) -> Timers {
    let timers = {
        let state = app.state::<AppState>();
        let mut store = state.store.lock().map_err(|error| error.to_string())?;
        store.delete_segment(&id, segment_id)?
    };
    refresh_tray(&app, &timers);
    Ok(timers)
}

#[tauri::command]
pub fn delete_timer(app: AppHandle, id: String) -> Timers {
    let timers = {
        let state = app.state::<AppState>();
        let mut store = state.store.lock().map_err(|error| error.to_string())?;
        store.delete(&id)?
    };
    refresh_tray(&app, &timers);
    Ok(timers)
}

// --- Activity capture ---------------------------------------------------

#[tauri::command]
pub fn activity_snapshot(app: AppHandle, since_ms: u64) -> Result<ActivitySnapshot, String> {
    activity::snapshot_for(&app, since_ms)
}

#[tauri::command]
pub fn usage_snapshot(app: AppHandle, since_ms: u64) -> Result<UsageSnapshot, String> {
    crate::usage::snapshot_for(&app, since_ms)
}

#[tauri::command]
pub fn set_capture_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|error| error.to_string())?;
        store.set_capture_enabled(enabled, crate::timers::now_epoch_ms())?;
    }
    activity::emit_full(&app);
    Ok(())
}

#[tauri::command]
pub fn set_idle_threshold(app: AppHandle, minutes: u64) -> Result<(), String> {
    {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|error| error.to_string())?;
        store.set_idle_threshold_ms(minutes.saturating_mul(60_000))?;
    }
    activity::emit_full(&app);
    Ok(())
}

#[tauri::command]
pub fn start_session(app: AppHandle, kind: String, label: Option<String>) -> Result<(), String> {
    let now = now_epoch_ms();
    let changed = {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|error| error.to_string())?;
        store.start_session(&kind, label.as_deref(), now)?;
        store.rebuild_range(
            now.saturating_sub(crate::activity::REBUILD_WINDOW_MS),
            now,
            now,
        )?
    };
    activity::emit_full(&app);
    if changed {
        entries_changed(&app);
    }
    crate::ai::nudge(&app);
    Ok(())
}

#[tauri::command]
pub fn stop_session(app: AppHandle) -> Result<(), String> {
    let now = now_epoch_ms();
    let changed = {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|error| error.to_string())?;
        store.stop_session(now)?;
        store.rebuild_range(
            now.saturating_sub(crate::activity::REBUILD_WINDOW_MS),
            now,
            now,
        )?
    };
    activity::emit_full(&app);
    if changed {
        entries_changed(&app);
    }
    crate::ai::nudge(&app);
    Ok(())
}

#[tauri::command]
pub fn mark_segment_reviewed(app: AppHandle, id: i64) -> Result<(), String> {
    {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|error| error.to_string())?;
        store.mark_reviewed(id)?;
    }
    activity::emit_full(&app);
    Ok(())
}

// --- P1: Categories -----------------------------------------------------

#[tauri::command]
pub fn list_categories(app: AppHandle) -> Result<Vec<Category>, String> {
    let state = app.state::<AppState>();
    let store = state.activity.lock().map_err(|e| e.to_string())?;
    store.list_categories()
}

#[tauri::command]
pub fn create_category(app: AppHandle, category: NewCategory) -> Result<Category, String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    store.create_category(category, now)
}

#[tauri::command]
pub fn update_category(
    app: AppHandle,
    id: String,
    patch: UpdateCategory,
) -> Result<Category, String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    store.update_category(&id, patch, now)
}

#[tauri::command]
pub fn delete_category(app: AppHandle, id: String) -> Result<(), String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    store.delete_category(&id, now)?;
    drop(store);
    entries_changed(&app);
    Ok(())
}

// --- P1: Projects -------------------------------------------------------

#[tauri::command]
pub fn list_projects(app: AppHandle) -> Result<Vec<Project>, String> {
    let state = app.state::<AppState>();
    let store = state.activity.lock().map_err(|e| e.to_string())?;
    store.list_projects()
}

#[tauri::command]
pub fn create_project(app: AppHandle, project: NewProject) -> Result<Project, String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    store.create_project(project, now)
}

#[tauri::command]
pub fn update_project(app: AppHandle, id: String, patch: UpdateProject) -> Result<Project, String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    store.update_project(&id, patch, now)
}

#[tauri::command]
pub fn delete_project(app: AppHandle, id: String) -> Result<(), String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    store.delete_project(&id, now)?;
    drop(store);
    entries_changed(&app);
    Ok(())
}

// --- Work management: tasks and labels ---------------------------------

#[tauri::command]
pub fn list_tasks(app: AppHandle, project_id: Option<String>) -> Result<Vec<Task>, String> {
    let state = app.state::<AppState>();
    let store = state.activity.lock().map_err(|e| e.to_string())?;
    store.list_tasks(project_id.as_deref())
}

#[tauri::command]
pub fn create_task(app: AppHandle, task: NewTask) -> Result<Task, String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    let result = store.create_task(task, now);
    drop(store);
    if result.is_ok() {
        entries_changed(&app);
    }
    result
}

#[tauri::command]
pub fn update_task(app: AppHandle, id: String, patch: UpdateTask) -> Result<Task, String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    let result = store.update_task(&id, patch, now);
    drop(store);
    if result.is_ok() {
        entries_changed(&app);
    }
    result
}

#[tauri::command]
pub fn delete_task(app: AppHandle, id: String) -> Result<(), String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    let result = store.delete_task(&id, now);
    drop(store);
    if result.is_ok() {
        entries_changed(&app);
    }
    result
}

#[tauri::command]
pub fn list_labels(app: AppHandle) -> Result<Vec<Label>, String> {
    let state = app.state::<AppState>();
    let store = state.activity.lock().map_err(|e| e.to_string())?;
    store.list_labels()
}

#[tauri::command]
pub fn create_label(app: AppHandle, label: NewLabel) -> Result<Label, String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    store.create_label(label, now)
}

#[tauri::command]
pub fn update_label(app: AppHandle, id: String, patch: UpdateLabel) -> Result<Label, String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    store.update_label(&id, patch, now)
}

#[tauri::command]
pub fn delete_label(app: AppHandle, id: String) -> Result<(), String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    store.delete_label(&id, now)
}

// --- External integrations ---------------------------------------------

#[tauri::command]
pub fn list_integrations(app: AppHandle) -> Result<Vec<IntegrationConnection>, String> {
    let state = app.state::<AppState>();
    let store = state.activity.lock().map_err(|e| e.to_string())?;
    store.list_integrations()
}

/// Persists only non-secret connection metadata. `secret_ref` points to an OS
/// keychain entry; the actual provider token never crosses this command.
#[tauri::command]
pub fn save_integration(
    app: AppHandle,
    connection: IntegrationConnection,
) -> Result<IntegrationConnection, String> {
    if connection.secret_ref.trim().is_empty() {
        return Err("An OS keychain reference is required.".to_string());
    }
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    store.upsert_integration(connection)
}

// --- P1: Clients --------------------------------------------------------

#[tauri::command]
pub fn list_clients(app: AppHandle) -> Result<Vec<Client>, String> {
    let state = app.state::<AppState>();
    let store = state.activity.lock().map_err(|e| e.to_string())?;
    store.list_clients()
}

#[tauri::command]
pub fn create_client(app: AppHandle, client: NewClient) -> Result<Client, String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    store.create_client(client, now)
}

#[tauri::command]
pub fn update_client(app: AppHandle, id: String, patch: UpdateClient) -> Result<Client, String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    store.update_client(&id, patch, now)
}

#[tauri::command]
pub fn delete_client(app: AppHandle, id: String) -> Result<(), String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    store.delete_client(&id, now)
}

// --- Personal timeline -------------------------------------------------

#[tauri::command]
pub fn timeline_day(
    app: AppHandle,
    start_ms: u64,
    end_ms: u64,
    workspace_id: Option<String>,
) -> Result<Vec<TimelineEvent>, String> {
    with_database(&app, |db| {
        timeline::list_events(
            db,
            start_ms,
            end_ms,
            now_epoch_ms(),
            workspace_id.as_deref(),
        )
    })
}

#[tauri::command]
pub fn publish_timeline_segment(
    app: AppHandle,
    input: PublishInput,
) -> Result<TimelineEvent, String> {
    with_database(&app, |db| timeline::publish(db, input, now_epoch_ms()))
}

#[tauri::command]
pub fn unpublish_timeline_segment(app: AppHandle, segment_id: i64) -> Result<(), String> {
    with_database(&app, |db| {
        timeline::unpublish(db, segment_id, now_epoch_ms())
    })
}

#[tauri::command]
pub fn list_timeline_devices(app: AppHandle) -> Result<Vec<TimelineDevice>, String> {
    with_database(&app, timeline::list_devices)
}

#[tauri::command]
pub fn timeline_sync_state(app: AppHandle) -> Result<TimelineSyncState, String> {
    with_database(&app, timeline::sync_state)
}

#[tauri::command]
pub async fn sync_timeline(
    app: AppHandle,
    input: TimelineSyncInput,
) -> Result<TimelineSyncResult, String> {
    if input.server_url.trim().is_empty()
        || input.bearer_token.trim().is_empty()
        || input.device_id.trim().is_empty()
        || input.workspace_id.trim().is_empty()
    {
        return Err("server URL, token, device id and workspace id are required".into());
    }
    with_database(&app, |db| {
        timeline::register_device(db, &input.device_id, now_epoch_ms())
    })?;
    with_database(&app, |db| {
        timeline::set_sync_connecting(db, &input.server_url)
    })?;
    let (operations, cursor) = with_database(&app, |db| {
        let operations = timeline::pending_operations(db, &input.workspace_id)?;
        let cursor = timeline::sync_cursor(db, &input.workspace_id)?;
        Ok((operations, cursor))
    })?;
    let base_url = input.server_url.trim_end_matches('/');
    let client = reqwest::Client::new();
    let push: SyncPushResponse = client
        .post(format!("{base_url}/api/v1/sync/push"))
        .bearer_auth(&input.bearer_token)
        .json(&SyncPushRequest {
            device_id: input.device_id,
            operations,
        })
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    let mut pull_url = url::Url::parse(&format!("{base_url}/api/v1/sync/pull"))
        .map_err(|error| error.to_string())?;
    pull_url
        .query_pairs_mut()
        .append_pair("workspace_id", &input.workspace_id)
        .append_pair("cursor", &cursor.to_string());
    let pull: SyncPullResponse = client
        .get(pull_url)
        .bearer_auth(&input.bearer_token)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    let applied = with_database(&app, |db| {
        timeline::apply_sync_response(db, &input.workspace_id, &push, &pull, now_epoch_ms())
    })?;
    Ok(TimelineSyncResult {
        accepted: push.accepted.len(),
        conflicts: push.conflicts.len(),
        applied,
        cursor: pull.cursor,
    })
}

// --- Invoices -----------------------------------------------------------

fn database_for(
    app: &AppHandle,
) -> Result<aevio_persistence::database::DatabaseConnection, String> {
    let state = app.state::<AppState>();
    aevio_persistence::database::connect(&state.database_path).map_err(|error| error.to_string())
}

pub(crate) fn with_database<T>(
    app: &AppHandle,
    run: impl FnOnce(&aevio_persistence::database::DatabaseConnection) -> Result<T, String>,
) -> Result<T, String> {
    let state = app.state::<AppState>();
    let _writer = state.activity.lock().map_err(|error| error.to_string())?;
    let database = database_for(app)?;
    run(&database)
}

/// Runs an invoice mutation on the writer connection.
#[tauri::command]
pub fn list_invoices(app: AppHandle) -> Result<Vec<InvoiceSummary>, String> {
    with_database_reader(&app, invoices::list_with_database)
}

#[tauri::command]
pub fn get_invoice(app: AppHandle, id: String) -> Result<Invoice, String> {
    with_database_reader(&app, |database| invoices::get_with_database(database, &id))
}

#[tauri::command]
pub fn list_billable_entries(
    app: AppHandle,
    client_id: String,
    start_ms: u64,
    end_ms: u64,
    invoice_id: Option<String>,
) -> Result<Vec<BillableEntry>, String> {
    with_database_reader(&app, |database| {
        invoices::billable_entries_with_database(
            database,
            &client_id,
            start_ms,
            end_ms,
            invoice_id.as_deref(),
        )
    })
}

/// The draft resolved and priced, without storing it.
#[tauri::command]
pub fn quote_invoice(app: AppHandle, draft: DraftInput) -> Result<Invoice, String> {
    with_database_reader(&app, |database| {
        invoices::quote_with_database(database, &draft)
    })
}

/// The DRAFT-watermarked PDF bytes for an unsaved or saved draft.
#[tauri::command]
pub fn render_invoice_preview(app: AppHandle, draft: DraftInput) -> Result<Response, String> {
    with_database_reader(&app, |database| {
        invoices::render_draft_with_database(database, &draft)
    })
    .map(Response::new)
}

/// The archived PDF bytes of a finalized invoice.
#[tauri::command]
pub fn get_invoice_pdf(app: AppHandle, id: String) -> Result<Response, String> {
    with_database_reader(&app, |database| {
        invoices::stored_pdf_with_database(database, &id)
    })
    .map(Response::new)
}

#[tauri::command]
pub fn save_invoice_draft(app: AppHandle, draft: DraftInput) -> Result<Invoice, String> {
    let saved = with_database(&app, |database| {
        invoices::save_draft_with_database(database, &draft, now_epoch_ms())
    })?;
    entries_changed(&app);
    Ok(saved)
}

#[tauri::command]
pub fn finalize_invoice(app: AppHandle, id: String) -> Result<Invoice, String> {
    let done = with_database(&app, |database| {
        invoices::finalize_with_database(database, &id, now_epoch_ms())
    })?;
    entries_changed(&app);
    Ok(done)
}

#[tauri::command]
pub fn set_invoice_paid(app: AppHandle, id: String, paid: bool) -> Result<(), String> {
    with_database(&app, |database| {
        aevio_persistence::database::repositories::invoices::InvoiceRepository::new(database)
            .set_paid(&id, paid, now_epoch_ms() as i64)
            .map_err(|error| error.to_string())
    })?;
    entries_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn void_invoice(app: AppHandle, id: String) -> Result<(), String> {
    with_database(&app, |database| {
        aevio_persistence::database::repositories::invoices::InvoiceRepository::new(database)
            .void(&id, now_epoch_ms() as i64)
            .map_err(|error| error.to_string())
    })?;
    entries_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_draft_invoice(app: AppHandle, id: String) -> Result<(), String> {
    with_database(&app, |database| {
        aevio_persistence::database::repositories::invoices::InvoiceRepository::new(database)
            .delete_draft(&id)
            .map_err(|error| error.to_string())
    })?;
    entries_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_void_invoice(app: AppHandle, id: String) -> Result<(), String> {
    with_database(&app, |database| {
        aevio_persistence::database::repositories::invoices::InvoiceRepository::new(database)
            .delete_void(&id)
            .map_err(|error| error.to_string())
    })?;
    entries_changed(&app);
    Ok(())
}

/// Asks where to save an invoice PDF and writes it: a finalized invoice's
/// archived bytes (`id`), or a fresh DRAFT render of `draft`. Returns the saved
/// path, or `None` when the user cancels.
#[tauri::command]
pub async fn export_invoice_pdf(
    app: AppHandle,
    id: Option<String>,
    draft: Option<DraftInput>,
) -> Result<Option<String>, String> {
    let (bytes, name) = match (&id, &draft) {
        (Some(id), _) => with_database_reader(&app, |database| {
            let invoice = invoices::get_with_database(database, id)?;
            let name = invoices::file_name(
                invoice.summary.number.as_deref(),
                &invoice.summary.client_name,
            );
            Ok((invoices::stored_pdf_with_database(database, id)?, name))
        })?,
        (None, Some(draft)) => with_database_reader(&app, |database| {
            Ok((
                invoices::render_draft_with_database(database, draft)?,
                invoices::file_name(None, &draft.bill_to_name),
            ))
        })?,
        (None, None) => return Err("Nothing to export".to_string()),
    };
    let Some(picked) = app
        .dialog()
        .file()
        .set_file_name(&name)
        .add_filter("PDF", &["pdf"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let mut path = picked.into_path().map_err(|e| e.to_string())?;
    if path.extension().is_none() {
        path.set_extension("pdf");
    }
    let temp = path.with_extension("pdf.aevio-tmp");
    std::fs::write(&temp, &bytes).map_err(|e| format!("Could not save the PDF: {e}"))?;
    std::fs::rename(&temp, &path).map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        format!("Could not save the PDF: {e}")
    })?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

#[tauri::command]
pub fn get_invoice_profile(app: AppHandle) -> Result<profile::InvoiceProfile, String> {
    with_database_reader(&app, profile::get_with_database)
}

#[tauri::command]
pub fn update_invoice_profile(
    app: AppHandle,
    profile: profile::ProfileInput,
) -> Result<profile::InvoiceProfile, String> {
    with_database(&app, |database| {
        profile::update_with_database(database, profile, now_epoch_ms())
    })
}

/// The logo image bytes (PNG), or an empty response when none is set.
#[tauri::command]
pub fn get_invoice_logo(app: AppHandle) -> Result<Response, String> {
    with_database_reader(&app, |database| {
        Ok(profile::logo_bytes_with_database(database)?.unwrap_or_default())
    })
    .map(Response::new)
}

/// Stores the raw bytes of the request body as the logo (PNG or JPEG in).
#[tauri::command]
pub fn set_invoice_logo(app: AppHandle, request: Request<'_>) -> Result<(), String> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err("Expected the logo image bytes".into());
    };
    with_database(&app, |database| {
        profile::set_logo_with_database(database, bytes, now_epoch_ms())
    })
}

#[tauri::command]
pub fn clear_invoice_logo(app: AppHandle) -> Result<(), String> {
    with_database(&app, |database| {
        profile::clear_logo_with_database(database, now_epoch_ms())
    })
}

// --- P1: Time Entries ---------------------------------------------------

/// Tells views to refetch, and wakes the AI worker: an entry mutation can
/// queue work (an approval queues its embedding, a rebuild a classification).
pub(crate) fn entries_changed(app: &AppHandle) {
    let _ = app.emit(crate::EVENT_ENTRIES_CHANGED, ());
    ai::nudge(app);
}

#[tauri::command]
pub fn list_time_entries(
    app: AppHandle,
    start_ms: u64,
    end_ms: u64,
) -> Result<Vec<TimeEntry>, String> {
    let state = app.state::<AppState>();
    let store = state.activity.lock().map_err(|e| e.to_string())?;
    store.list_time_entries(start_ms, end_ms)
}

#[tauri::command]
pub fn get_entry_detail(app: AppHandle, id: String) -> Result<EntryDetail, String> {
    let state = app.state::<AppState>();
    let store = state.activity.lock().map_err(|e| e.to_string())?;
    store.get_entry_detail(&id)
}

#[tauri::command]
pub fn update_time_entry(
    app: AppHandle,
    id: String,
    patch: UpdateTimeEntry,
) -> Result<TimeEntry, String> {
    let now = now_epoch_ms();
    let entry = {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|e| e.to_string())?;
        store.update_time_entry(&id, patch, now)?
    };
    entries_changed(&app);
    Ok(entry)
}

/// Returns the approved entries, so a caller adopts them without a refetch.
#[tauri::command]
pub fn approve_time_entries(app: AppHandle, ids: Vec<String>) -> Result<Vec<TimeEntry>, String> {
    with_database(&app, |db| {
        governance::require(db, "member-local", "approve_time")
    })?;
    let now = now_epoch_ms();
    let entries = {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|e| e.to_string())?;
        store.approve_time_entries(&ids, "user", now)?;
        store.time_entries(&ids)?
    };
    entries_changed(&app);
    Ok(entries)
}

/// Returns the entries, back to pending, so a caller adopts them without a
/// refetch.
#[tauri::command]
pub fn unapprove_time_entries(app: AppHandle, ids: Vec<String>) -> Result<Vec<TimeEntry>, String> {
    with_database(&app, |db| {
        governance::require(db, "member-local", "approve_time")
    })?;
    let now = now_epoch_ms();
    let entries = {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|e| e.to_string())?;
        store.unapprove_time_entries(&ids, now)?;
        store.time_entries(&ids)?
    };
    entries_changed(&app);
    Ok(entries)
}

/// One patch applied to many entries (My Timesheet's bulk bar). Returns the
/// updated entries.
#[tauri::command]
pub fn update_time_entries(
    app: AppHandle,
    ids: Vec<String>,
    patch: UpdateTimeEntry,
) -> Result<Vec<TimeEntry>, String> {
    with_database(&app, |db| {
        governance::require(db, "member-local", "bulk_edit")
    })?;
    let now = now_epoch_ms();
    let entries = {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|e| e.to_string())?;
        store.update_time_entries(&ids, patch, now)?
    };
    entries_changed(&app);
    Ok(entries)
}

#[tauri::command]
pub fn reject_time_entry(app: AppHandle, id: String) -> Result<(), String> {
    let now = now_epoch_ms();
    {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|e| e.to_string())?;
        store.reject_time_entry(&id, now)?;
    }
    entries_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn split_time_entry(
    app: AppHandle,
    id: String,
    at_ms: u64,
) -> Result<(TimeEntry, TimeEntry), String> {
    let now = now_epoch_ms();
    let res = {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|e| e.to_string())?;
        store.split_time_entry(&id, at_ms, now)?
    };
    entries_changed(&app);
    Ok(res)
}

#[tauri::command]
pub fn merge_time_entries(
    app: AppHandle,
    primary_id: String,
    secondary_id: String,
) -> Result<TimeEntry, String> {
    let now = now_epoch_ms();
    let res = {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|e| e.to_string())?;
        store.merge_time_entries(&primary_id, &secondary_id, now)?
    };
    entries_changed(&app);
    Ok(res)
}

#[tauri::command]
pub fn delete_time_entry(app: AppHandle, id: String) -> Result<(), String> {
    delete_time_entries(app, vec![id])
}

#[tauri::command]
pub fn delete_time_entries(app: AppHandle, ids: Vec<String>) -> Result<(), String> {
    with_database(&app, |db| {
        governance::require(db, "member-local", "bulk_delete")
    })?;
    let now = now_epoch_ms();
    {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|e| e.to_string())?;
        store.delete_time_entries(&ids, now)?;
    }
    entries_changed(&app);
    Ok(())
}

/// Undoes (or redoes) the last change to entries or the catalog. Returns
/// what it was ("Split", "Accept", ...), or `None` when there was nothing.
#[tauri::command]
pub fn undo(app: AppHandle, redo: bool) -> Result<Option<String>, String> {
    let step = {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|error| error.to_string())?;
        if redo {
            store.redo(now_epoch_ms())?
        } else {
            store.undo(now_epoch_ms())?
        }
    };
    if step.is_some() {
        entries_changed(&app);
    }
    Ok(step.map(str::to_string))
}

#[tauri::command]
pub fn create_time_entry(app: AppHandle, entry: NewTimeEntry) -> Result<TimeEntry, String> {
    let now = now_epoch_ms();
    let created = {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|e| e.to_string())?;
        store.create_manual_entry(entry, now)?
    };
    entries_changed(&app);
    Ok(created)
}

#[tauri::command]
pub fn rebuild_time_entries(
    app: AppHandle,
    start_ms: u64,
    end_ms: u64,
) -> Result<Vec<TimeEntry>, String> {
    let now = now_epoch_ms();
    let entries = {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|e| e.to_string())?;
        store.rebuild_time_entries_in_range(start_ms, end_ms, now)?
    };
    entries_changed(&app);
    Ok(entries)
}

// --- P1: Apps -----------------------------------------------------------

#[tauri::command]
pub fn list_apps(app: AppHandle) -> Result<Vec<AppRecord>, String> {
    let state = app.state::<AppState>();
    let store = state.activity.lock().map_err(|e| e.to_string())?;
    store.list_apps()
}

#[tauri::command]
pub fn update_app(
    app: AppHandle,
    id: String,
    default_category_id: Option<String>,
    default_project_id: Option<String>,
    excluded: Option<bool>,
) -> Result<AppRecord, String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    store.update_app(&id, default_category_id, default_project_id, excluded, now)
}

// --- P2: AI suggestions --------------------------------------------------

#[tauri::command]
pub fn work_ask(
    app: AppHandle,
    question: String,
) -> Result<crate::work_intelligence::WorkAnswer, String> {
    crate::work_intelligence::ask(&app, &question)
}

#[tauri::command]
pub fn work_relationships_list(
    app: AppHandle,
) -> Result<Vec<crate::work_intelligence::WorkRelationship>, String> {
    crate::work_intelligence::list_relationships(&app)
}

#[tauri::command]
pub fn work_traverse(
    app: AppHandle,
    start_type: String,
    start_id: String,
    max_depth: Option<u32>,
    max_results: Option<u32>,
) -> Result<crate::work_intelligence::GraphTraversal, String> {
    crate::work_intelligence::traverse(
        &app,
        &start_type,
        &start_id,
        max_depth.unwrap_or(3),
        max_results.unwrap_or(100),
    )
}

#[tauri::command]
pub fn work_project_suggestions(
    app: AppHandle,
    since_ms: u64,
) -> Result<Vec<crate::work_intelligence::ProjectSuggestion>, String> {
    crate::work_intelligence::project_suggestions(&app, since_ms)
}

#[tauri::command]
pub fn work_dismiss_suggestion(app: AppHandle, key: String) -> Result<(), String> {
    with_database(&app, |database| {
        aevio_persistence::projects::dismiss_suggestion_with_database(database, &key)
    })
}

#[tauri::command]
pub fn work_match_existing(app: AppHandle, key: String, project_id: String) -> Result<(), String> {
    with_database(&app, |db| {
        aevio_persistence::database::repositories::project_rules::ProjectRulesRepository::new(db)
            .attach_manual_rule(&key, &project_id, now_epoch_ms() as i64)
            .map_err(|error| match error {
                aevio_persistence::database::sea_orm::DbErr::RecordNotFound(_) => {
                    "project not found".into()
                }
                other => other.to_string(),
            })
    })
}

#[tauri::command]
pub fn work_relationship_add(
    app: AppHandle,
    from_type: String,
    from_id: String,
    relation: String,
    to_type: String,
    to_id: String,
    provenance: Option<String>,
) -> Result<crate::work_intelligence::WorkRelationship, String> {
    crate::work_intelligence::add_relationship(
        &app,
        &from_type,
        &from_id,
        &relation,
        &to_type,
        &to_id,
        provenance.as_deref().unwrap_or("user"),
    )
}

#[tauri::command]
pub fn ai_status(app: AppHandle) -> AiStatus {
    app.state::<AiRuntime>().status()
}

/// Answers a bounded, read-only agent question from the local workspace.
///
/// The local context is passed to the on-device sidecar when available. The
/// deterministic answer remains the honest fallback, and this command never
/// allows the model to perform actions or mutate durable state.
#[tauri::command]
pub fn agent_chat(app: AppHandle, message: String) -> Result<AgentChatReply, String> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return Err("Message cannot be empty".to_string());
    }
    if trimmed.chars().count() > 2000 {
        return Err("Message is too long (maximum 2,000 characters)".to_string());
    }

    let (project_names, category_names) = {
        let state = app.state::<AppState>();
        let store = state.activity.lock().map_err(|e| e.to_string())?;
        let projects = store.list_projects()?;
        let project_names = projects
            .iter()
            .filter(|project| project.status != "archived")
            .map(|project| project.name.clone())
            .take(20)
            .collect::<Vec<_>>();
        let category_names = store
            .list_categories()?
            .into_iter()
            .map(|category| category.name)
            .take(30)
            .collect::<Vec<_>>();
        (project_names, category_names)
    };
    let (pending_entries, pending_preview) = with_database(&app, |db| {
        aevio_persistence::database::repositories::agent_context::AgentContextRepository::new(db)
            .pending_context()
            .map_err(|error| error.to_string())
    })?;
    let active_projects = project_names.len() as u32;
    let categories = category_names.len() as u32;

    let status = app.state::<AiRuntime>().status();
    let context = AgentChatContext {
        pending_entries,
        active_projects,
        categories,
        ai_engine: status.engine.clone(),
        pending_preview: pending_preview.clone(),
        project_names: project_names.clone(),
        category_names: category_names.clone(),
    };
    let lower = trimmed.to_lowercase();
    let fallback = if lower.contains("review") || lower.contains("offen") {
        format!("Du hast aktuell {pending_entries} offene Zeitbuchungen zur Prüfung.")
    } else if lower.contains("projekt") {
        format!("Aevio kennt aktuell {active_projects} aktive Projekte.")
    } else if lower.contains("ki") || lower.contains("ai") || lower.contains("status") {
        format!("Der lokale AI-Engine-Status ist `{}`.", status.engine)
    } else {
        "Ich kann im ersten Agent-Slice deinen lokalen Aevio-Kontext read-only zusammenfassen. Frage zum Beispiel nach offenen Reviews, Projekten oder dem AI-Status.".to_string()
    };

    let context_value = serde_json::json!({
        "pendingEntries": pending_entries.to_string(),
        "activeProjects": active_projects.to_string(),
        "categories": categories.to_string(),
        "aiEngine": status.engine.clone(),
        "pendingPreview": pending_preview.join("\n"),
        "projects": project_names.join(", "),
        "categoriesList": category_names.join(", "),
    });
    let settings = app.state::<AppState>().settings_snapshot();
    let (answer, mode) = if !settings.ai_provider.is_empty() {
        match crate::ai::provider::answer(
            trimmed,
            &context_value,
            &settings.ai_provider,
            &settings.ai_base_url,
            &settings.ai_model,
            &settings.ai_api_key,
        ) {
            Ok(reply) => (reply.answer, "external-ai".to_string()),
            Err(_) => match crate::ai::agent::answer(trimmed, &context_value) {
                Ok(reply) => (reply.answer, "local-ai".to_string()),
                Err(_) => (fallback, "local-read-only".to_string()),
            },
        }
    } else {
        match crate::ai::agent::answer(trimmed, &context_value) {
            Ok(reply) => (reply.answer, "local-ai".to_string()),
            Err(_) => (fallback, "local-read-only".to_string()),
        }
    };

    Ok(AgentChatReply {
        answer,
        mode,
        context,
    })
}

// --- Agent routines, reports and dashboards ----------------------------

#[tauri::command]
pub fn automation_snapshot(app: AppHandle) -> Result<AutomationSnapshot, String> {
    with_database(&app, |db| {
        automation::list(db).map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn daily_workflow_snapshot(app: AppHandle) -> Result<DailyWorkflowSnapshot, String> {
    with_database(&app, |db| {
        automation::daily_workflow_snapshot(db, now_epoch_ms() as i64)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn run_daily_workflow(app: AppHandle) -> Result<DailyWorkflowRun, String> {
    let result = with_database(&app, |db| {
        automation::run_daily_workflow(db, now_epoch_ms()).map_err(|error| error.to_string())
    })?;
    entries_changed(&app);
    Ok(result)
}

#[tauri::command]
pub fn create_routine(app: AppHandle, input: NewRoutine) -> Result<Routine, String> {
    with_database(&app, |db| {
        automation::create_routine(
            db,
            &input.name,
            &input.prompt,
            input.cadence.as_deref().unwrap_or("on-demand"),
            input.send_at.as_deref(),
            &input.delivery.unwrap_or_else(|| vec!["in-app".into()]),
        )
        .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn create_report(app: AppHandle, input: NewReport) -> Result<ReportDefinition, String> {
    with_database(&app, |db| {
        automation::create_report(
            db,
            &input.name,
            &input.filter,
            input.cadence.as_deref().unwrap_or("on-demand"),
            &input.delivery.unwrap_or_else(|| vec!["in-app".into()]),
        )
        .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn create_dashboard(app: AppHandle, input: NewDashboard) -> Result<Dashboard, String> {
    with_database(&app, |db| {
        automation::create_dashboard(db, &input.name, &input.filter)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn run_routine(app: AppHandle, id: String) -> Result<Run, String> {
    with_database(&app, |db| {
        automation::run_routine(db, &id).map_err(|error| error.to_string())
    })
}

#[tauri::command]
pub fn run_report(app: AppHandle, id: String) -> Result<Run, String> {
    with_database(&app, |db| {
        automation::run_report(db, &id).map_err(|error| error.to_string())
    })
}

// --- Financial analysis and planning ----------------------------------

#[tauri::command]
pub fn finance_snapshot(app: AppHandle, filter: FinanceFilter) -> Result<FinanceSnapshot, String> {
    with_database(&app, |db| finance::snapshot(db, &filter))
}

#[tauri::command]
pub fn list_finance_rate_cards(app: AppHandle) -> Result<Vec<RateCard>, String> {
    with_database(&app, finance::list_rate_cards)
}

#[tauri::command]
pub fn save_finance_rate_card(app: AppHandle, input: RateCardInput) -> Result<RateCard, String> {
    with_database(&app, |db| finance::save_rate_card(db, input))
}

#[tauri::command]
pub fn list_finance_contracts(app: AppHandle) -> Result<Vec<Contract>, String> {
    with_database(&app, finance::list_contracts)
}

#[tauri::command]
pub fn save_finance_contract(app: AppHandle, input: ContractInput) -> Result<Contract, String> {
    with_database(&app, |db| finance::save_contract(db, input))
}

#[tauri::command]
pub fn share_finance_dashboard(
    app: AppHandle,
    dashboard_id: String,
    expires_at: Option<i64>,
) -> Result<DashboardShare, String> {
    with_database(&app, |db| {
        finance::share_dashboard(db, &dashboard_id, expires_at)
    })
}

#[tauri::command]
pub fn list_finance_capacity_plans(app: AppHandle) -> Result<Vec<CapacityPlan>, String> {
    with_database(&app, finance::list_capacity_plans)
}

#[tauri::command]
pub fn save_finance_capacity_plan(
    app: AppHandle,
    input: CapacityPlanInput,
) -> Result<CapacityPlan, String> {
    with_database(&app, |db| finance::save_capacity_plan(db, input))
}

#[tauri::command]
pub fn list_finance_dashboard_shares(
    app: AppHandle,
    dashboard_id: String,
) -> Result<Vec<DashboardShare>, String> {
    with_database(&app, |db| finance::list_dashboard_shares(db, &dashboard_id))
}

#[tauri::command]
pub fn revoke_finance_dashboard_share(app: AppHandle, id: String) -> Result<(), String> {
    with_database(&app, |db| finance::revoke_dashboard_share(db, &id))
}

// --- Staffing and freelancers ----------------------------------------

#[tauri::command]
pub fn staffing_snapshot(app: AppHandle) -> Result<StaffingSnapshot, String> {
    with_database(&app, staffing::snapshot)
}

#[tauri::command]
pub fn list_staffing_freelancers(app: AppHandle) -> Result<Vec<Freelancer>, String> {
    with_database(&app, staffing::list_freelancers)
}

#[tauri::command]
pub fn save_staffing_freelancer(
    app: AppHandle,
    input: FreelancerInput,
) -> Result<Freelancer, String> {
    with_database(&app, |db| staffing::save_freelancer(db, input))
}

#[tauri::command]
pub fn save_staffing_record(app: AppHandle, kind: String, payload: Value) -> Result<(), String> {
    with_database(&app, |db| staffing::save_json_record(db, &kind, &payload))
}

#[tauri::command]
pub fn staffing_matches(app: AppHandle, request_id: String) -> Result<Vec<StaffingMatch>, String> {
    with_database(&app, |db| staffing::match_request(db, &request_id))
}

/// "Couldn't categoaevio · Retry", or re-running a suggestion on demand.
#[tauri::command]
pub fn retry_classification(app: AppHandle, id: String) -> Result<(), String> {
    let now = now_epoch_ms();
    {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|e| e.to_string())?;
        store.queue_classification(&id, now)?;
    }
    entries_changed(&app);
    Ok(())
}

/// Accepts (creating a rule) or dismisses an inline rule suggestion.
#[tauri::command]
pub fn resolve_rule_suggestion(
    app: AppHandle,
    suggestion: RuleSuggestion,
    accept: bool,
) -> Result<(), String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    store.resolve_rule_suggestion(&suggestion, accept, now)
}

// --- P4: Learning loop -----------------------------------------------------

/// Settings → Categories & AI: effectiveness, calibration, the threshold
/// preview, and personal-model versions over the last `days` days.
#[tauri::command]
pub fn ai_metrics(app: AppHandle, days: u32) -> Result<AiMetrics, String> {
    with_database_reader(&app, |database| {
        ai::metrics::metrics_with_database(database, days, now_epoch_ms())
    })
}

/// "Retrain now": queues a personal-model retrain that skips the idle/power
/// gate. The result arrives as `ai-status-changed`.
#[tauri::command]
pub fn ai_retrain(app: AppHandle) -> AiStatus {
    ai::update_status(&app, |s| {
        s.retrain = "queued".to_string();
        s.retrain_note = None;
    });
    app.state::<AiRuntime>().request_retrain();
    app.state::<AiRuntime>().status()
}

/// "Reset learned data": forgets the personal models, calibration, and the
/// kNN store, keeping entries, categories, projects, and rules. Returns the
/// updated metrics.
#[tauri::command]
pub fn ai_reset_learned(app: AppHandle, days: u32) -> Result<AiMetrics, String> {
    let now = now_epoch_ms();
    let metrics = {
        let paths = with_database(&app, |database| {
            aevio_persistence::database::repositories::ai_artifacts::AiArtifactsRepository::new(
                database,
            )
            .reset_learned(now)
            .map_err(|error| error.to_string())
        })?;
        for path in paths {
            if let Err(error) = std::fs::remove_dir_all(&path) {
                if error.kind() != std::io::ErrorKind::NotFound {
                    eprintln!("could not delete model {path}: {error}");
                }
            }
        }
        let database = database_for(&app)?;
        ai::metrics::metrics_with_database(&database, days, now)?
    };
    ai::update_status(&app, |s| {
        s.outcomes = 0;
        s.calibrated = false;
        s.personal_model = false;
        s.retrain_note = None;
    });
    ai::nudge(&app);
    Ok(metrics)
}

// --- P3: Reports (Time Entries, My Timesheet, Calendar Month) -----------
//
// Read-only queries use the SeaORM-sync connection without borrowing the
// activity writer mutex.

pub(crate) fn with_database_reader<T>(
    app: &AppHandle,
    read: impl FnOnce(&aevio_persistence::database::DatabaseConnection) -> Result<T, String>,
) -> Result<T, String> {
    let database = database_for(app)?;
    read(&database)
}

#[tauri::command]
pub fn query_time_entries(
    app: AppHandle,
    filter: EntryFilter,
    limit: Option<u32>,
) -> Result<Vec<TimeEntry>, String> {
    with_database_reader(&app, |database| {
        reports::query_entries_with_database(
            database,
            &filter,
            limit.unwrap_or(reports::MAX_QUERY_ROWS),
        )
    })
}

#[tauri::command]
pub fn entry_rollup(
    app: AppHandle,
    filter: EntryFilter,
    boundaries: Vec<u64>,
    group_by: String,
) -> Result<Vec<RollupCell>, String> {
    let group_by = GroupBy::parse(&group_by)?;
    with_database_reader(&app, |database| {
        reports::rollup_with_database(database, &filter, &boundaries, group_by)
    })
}

/// Writes the filtered entries to the Downloads folder as CSV or JSON.
#[tauri::command]
pub fn export_time_entries(
    app: AppHandle,
    filter: EntryFilter,
    format: String,
) -> Result<ExportResult, String> {
    with_database(&app, |db| {
        governance::require(db, "member-local", "export_data")
    })?;
    let format = ExportFormat::parse(&format)?;
    let dir = app
        .path()
        .download_dir()
        .or_else(|_| app.path().home_dir())
        .map_err(|e| e.to_string())?;
    with_database_reader(&app, |database| {
        reports::export_entries_with_database(database, &filter, format, &dir)
    })
}

// --- P3: Projects ------------------------------------------------------

#[tauri::command]
pub fn project_stats(
    app: AppHandle,
    range_start: u64,
    range_end: u64,
    month_start: u64,
) -> Result<Vec<ProjectStats>, String> {
    with_database_reader(&app, |database| {
        crate::projects::project_stats_with_database(database, range_start, range_end, month_start)
    })
}

#[tauri::command]
pub fn project_rules(app: AppHandle, project_id: String) -> Result<Vec<ProjectRule>, String> {
    with_database_reader(&app, |database| {
        crate::projects::project_rules_with_database(database, &project_id)
    })
}

/// "Would have matched 14h in the last 30 days", per hint.
#[tauri::command]
pub fn preview_project_hints(
    app: AppHandle,
    hints: String,
    since_ms: u64,
) -> Result<HintPreview, String> {
    with_database_reader(&app, |database| {
        crate::projects::preview_hints_with_database(database, &hints, since_ms)
    })
}

#[tauri::command]
pub fn discover_projects(app: AppHandle, since_ms: u64) -> Result<Vec<ProjectSuggestion>, String> {
    with_database(&app, |database| {
        crate::projects::discover_with_database(database, since_ms)
    })
}

#[tauri::command]
pub fn dismiss_project_suggestion(app: AppHandle, key: String) -> Result<(), String> {
    with_database(&app, |database| {
        crate::projects::dismiss_suggestion_with_database(database, &key)
    })
}

#[tauri::command]
pub fn import_projects_csv(app: AppHandle, text: String) -> Result<ImportSummary, String> {
    let now = now_epoch_ms();
    let state = app.state::<AppState>();
    let mut store = state.activity.lock().map_err(|e| e.to_string())?;
    crate::projects::import_csv(&mut store, &text, now)
}

// --- Pulse panel -------------------------------------------------------

#[tauri::command]
pub fn resize_widget(app: AppHandle, height: f64) {
    crate::widget::resize(&app, height);
}

#[tauri::command]
pub fn resize_pulse_panel(app: AppHandle, height: f64) {
    crate::pulse::resize(&app, height);
}

#[tauri::command]
pub fn hide_pulse_panel(app: AppHandle) {
    crate::pulse::hide(&app);
}

#[tauri::command]
pub fn open_productivity_summary(app: AppHandle) -> Result<(), String> {
    crate::summary::open(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hide_productivity_summary(app: AppHandle) {
    crate::summary::hide(&app);
}

/// The panel's doors into the app: closes the panel and brings the main
/// window forward, on today's review queue when `review` is set.
#[tauri::command]
pub fn open_main_window(app: AppHandle, review: bool, timer_id: Option<String>) {
    crate::pulse::hide(&app);
    tray::show_main_window(&app);
    if review {
        let _ = app.emit_to("main", crate::EVENT_OPEN_REVIEW, ());
    }
    if let Some(id) = timer_id {
        let _ = app.emit_to("main", "open-timer", id);
    }
}

// --- Preferences -------------------------------------------------------

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    app.state::<AppState>().settings_snapshot()
}

#[tauri::command]
pub fn update_settings(app: AppHandle, settings: Settings) -> Result<Settings, String> {
    let (previous, next) = {
        let state = app.state::<AppState>();
        let mut store = state.settings.lock().map_err(|error| error.to_string())?;
        let previous = store.snapshot();
        let next = store.set(settings)?;
        (previous, next)
    };
    apply_settings_change(&app, &previous, &next);
    Ok(next)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionHealth {
    pub rules: String,
    pub laya: String,
    pub jev: String,
    pub cloud_configured: bool,
}

#[tauri::command]
pub fn get_decision_settings(app: AppHandle) -> DecisionSettings {
    app.state::<AppState>().decision_settings_snapshot()
}

#[tauri::command]
pub fn update_decision_settings(
    app: AppHandle,
    settings: DecisionSettings,
) -> Result<DecisionSettings, String> {
    let state = app.state::<AppState>();
    let mut store = state
        .decision_settings
        .lock()
        .map_err(|error| error.to_string())?;
    store.set(settings)
}

#[tauri::command]
pub fn decision_health(app: AppHandle) -> DecisionHealth {
    let settings = app.state::<AppState>().decision_settings_snapshot();
    let cloud_configured = std::env::var_os("AEVIO_JEV_API_KEY").is_some();
    DecisionHealth {
        rules: "available".into(),
        laya: if settings.laya_endpoint.is_empty() {
            "notConfigured"
        } else {
            "configured"
        }
        .into(),
        jev: if cloud_configured && settings.allow_cloud {
            "configured"
        } else {
            "disabled"
        }
        .into(),
        cloud_configured,
    }
}

pub(crate) fn configured_decision_router(app: &AppHandle) -> Result<DecisionRouter, String> {
    let settings = app.state::<AppState>().decision_settings_snapshot();
    if !settings.enabled {
        return Err("decision engine is disabled".into());
    }
    let rules: std::sync::Arc<dyn DecisionProvider> = std::sync::Arc::new(RuleProvider);
    let local = if settings.laya_endpoint.is_empty() {
        None
    } else {
        Some(std::sync::Arc::new(
            RemoteProvider::new(
                settings.laya_endpoint,
                settings.laya_checkpoint,
                None,
                RemoteProtocol::Laya,
                std::time::Duration::from_millis(u64::from(settings.timeout_ms)),
            )
            .map_err(|error| error.to_string())?,
        ) as std::sync::Arc<dyn DecisionProvider>)
    };
    let cloud = if settings.allow_cloud {
        std::env::var("AEVIO_JEV_API_KEY")
            .ok()
            .map(|key| {
                RemoteProvider::new(
                    "https://thejevai.com/v1/systemone".into(),
                    settings.jev_model,
                    Some(key),
                    RemoteProtocol::Jev,
                    std::time::Duration::from_millis(u64::from(settings.timeout_ms)),
                )
            })
            .transpose()
            .map_err(|error| error.to_string())?
            .map(|provider| std::sync::Arc::new(provider) as std::sync::Arc<dyn DecisionProvider>)
    } else {
        None
    };
    let router = DecisionRouter {
        mode: settings.mode,
        policy: DecisionPolicy {
            minimum_confidence: f64::from(settings.minimum_confidence_percent) / 100.0,
            remote_allowed: settings.allow_cloud,
            human_review_below_threshold: settings.human_review,
        },
        rules,
        local,
        cloud,
    };
    Ok(router)
}

#[tauri::command]
pub async fn evaluate_decision(
    app: AppHandle,
    request: DecisionRequest,
) -> Result<DecisionResult, String> {
    configured_decision_router(&app)?
        .decide(&request)
        .await
        .map_err(|error| error.to_string())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DecisionUseCaseInput {
    pub kind: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub options: Vec<String>,
    pub recommended: Option<String>,
    #[serde(default)]
    pub facts: Vec<(String, String)>,
}

fn decision_use_case_fact(facts: &[(String, String)], key: &str) -> Result<String, String> {
    facts
        .iter()
        .find(|(fact_key, _)| fact_key == key)
        .map(|(_, value)| value.clone())
        .ok_or_else(|| format!("missing decision use-case fact: {key}"))
}

fn decision_use_case_bool(facts: &[(String, String)], key: &str) -> Result<bool, String> {
    decision_use_case_fact(facts, key)?
        .parse::<bool>()
        .map_err(|_| format!("decision use-case fact {key} must be a boolean"))
}

fn decision_use_case_u32(facts: &[(String, String)], key: &str) -> Result<u32, String> {
    decision_use_case_fact(facts, key)?
        .parse::<u32>()
        .map_err(|_| format!("decision use-case fact {key} must be an unsigned integer"))
}

fn decision_use_case_optional_score(
    facts: &[(String, String)],
    key: &str,
) -> Result<Option<f64>, String> {
    let Some((_, value)) = facts.iter().find(|(fact_key, _)| fact_key == key) else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    value
        .parse::<f64>()
        .map(Some)
        .map_err(|_| format!("decision use-case fact {key} must be a number"))
}

fn build_decision_use_case(input: DecisionUseCaseInput) -> Result<DecisionRequest, String> {
    let DecisionUseCaseInput {
        kind,
        summary: _summary,
        options,
        recommended,
        facts,
    } = input;
    let fact = |key: &str| decision_use_case_fact(&facts, key);
    let bool_fact = |key: &str| decision_use_case_bool(&facts, key);
    let u32_fact = |key: &str| decision_use_case_u32(&facts, key);

    Ok(match kind.as_str() {
        "interruption_risk" => use_cases::interruption_risk(
            bool_fact("inFocus")?,
            bool_fact("meetingSoon")?,
            u32_fact("workMinutes")?,
        ),
        "meeting_recovery" => use_cases::meeting_recovery(options, recommended),
        "energy_task_selection" => {
            let energy_level = u32_fact("energyLevel")?.min(100) as u8;
            use_cases::energy_task_selection(options, recommended, energy_level)
        }
        "deadline_risk" => use_cases::deadline_risk(
            &fact("taskId")?,
            u32_fact("remainingMinutes")?,
            u32_fact("estimateMinutes")?,
            bool_fact("dueSoon")?,
        ),
        "scope_creep" => use_cases::scope_creep(
            &fact("plannedTask")?,
            &fact("observedContext")?,
            u32_fact("minutesOffTask")?,
        ),
        "focus_start" => use_cases::focus_start(options, recommended),
        "daily_review" => use_cases::daily_review(&fact("outcome")?, bool_fact("hasPlan")?),
        "invoice_anomaly" => use_cases::invoice_anomaly(
            &fact("invoiceId")?,
            decision_use_case_optional_score(&facts, "anomalyScore")?,
        ),
        "project_health" => use_cases::project_health(
            &fact("projectId")?,
            decision_use_case_optional_score(&facts, "healthScore")?,
        ),
        "break_timing" => use_cases::break_timing(
            &fact("focusState")?,
            u32_fact("workMinutes")?,
            bool_fact("meetingSoon")?,
        ),
        "task_prioritization" => use_cases::task_prioritization(options, recommended),
        other => return Err(format!("unsupported decision use-case: {other}")),
    })
}

#[tauri::command]
pub async fn evaluate_decision_use_case(
    app: AppHandle,
    input: DecisionUseCaseInput,
) -> Result<DecisionResult, String> {
    let request = build_decision_use_case(input)?;
    configured_decision_router(&app)?
        .decide(&request)
        .await
        .map_err(|error| error.to_string())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CoachDecisionInput {
    pub kind: String,
    pub day: String,
    pub time_zone: String,
    #[serde(default)]
    pub options: Vec<String>,
    pub recommended: Option<String>,
}

#[tauri::command]
pub async fn coach_decision(
    app: AppHandle,
    input: CoachDecisionInput,
) -> Result<DecisionResult, String> {
    let request = {
        let state = app.state::<AppState>();
        let _store = state.activity.lock().map_err(|error| error.to_string())?;
        let database = aevio_persistence::database::connect(&state.database_path)
            .map_err(|error| error.to_string())?;
        let snapshot = aevio_persistence::coach::service::day_with_database(
            &database,
            &input.day,
            &input.time_zone,
        )?;
        match input.kind.as_str() {
            "focus_recovery" => {
                let focus =
                    aevio_persistence::database::repositories::coach::CoachTimeRepository::new(
                        &database,
                    )
                    .current_focus(aevio_core::timers::now_epoch_ms() as i64)?
                    .ok_or_else(|| "no current focus session".to_string())?;
                use_cases::focus_recovery(&focus.state, &focus.title, &focus.next_step, false)
            }
            "daily_review" => {
                use_cases::daily_review(snapshot.review.outcome.as_str(), snapshot.plan.is_some())
            }
            "task_prioritization" => {
                use_cases::task_prioritization(input.options, input.recommended)
            }
            other => return Err(format!("unsupported coach decision: {other}")),
        }
    };
    configured_decision_router(&app)?
        .decide(&request)
        .await
        .map_err(|error| error.to_string())
}

/// Carries a settings change out to the parts of the app that hold a copy.
pub(crate) fn apply_settings_change(app: &AppHandle, previous: &Settings, next: &Settings) {
    if next.tray_enabled != previous.tray_enabled {
        let timers = {
            let state = app.state::<AppState>();
            state
                .store
                .lock()
                .ok()
                .and_then(|store| store.snapshot().ok())
        };
        if let Some(timers) = timers {
            if let Err(error) = tray::set_enabled(app, next.tray_enabled, &timers) {
                eprintln!("could not toggle the tray: {error}");
            }
        }
    }

    if next.retention_days != previous.retention_days {
        let _ = sweep_retention(app);
    }

    if next.tracking_hours != previous.tracking_hours {
        let state = app.state::<AppState>();
        if let Ok(mut store) = state.activity.lock() {
            store.set_tracking_hours(next.tracking_hours.clone());
        }
        activity::emit_full(app);
    }

    if next.notification_placement != previous.notification_placement {
        breaks::surface::reposition(app);
    }
    if next.language != previous.language {
        let timers = {
            let state = app.state::<AppState>();
            state
                .store
                .lock()
                .ok()
                .and_then(|store| store.snapshot().ok())
        };
        if let Some(timers) = timers {
            if let Err(error) = tray::refresh(app, &timers) {
                eprintln!("could not refresh localized tray: {error}");
            }
        }
    }
    let _ = app.emit(crate::EVENT_SETTINGS_CHANGED, next);
}

pub fn sweep_retention(app: &AppHandle) -> Result<u64, String> {
    let days = app.state::<AppState>().settings_snapshot().retention_days;
    let removed = {
        let state = app.state::<AppState>();
        let mut store = state.activity.lock().map_err(|error| error.to_string())?;
        store.purge_older_than(days, now_epoch_ms())?
    };
    let _ = with_database(app, |database| {
        crate::energy::purge_older_than(database, days, now_epoch_ms())
    });
    if removed > 0 {
        activity::emit_full(app);
    }
    Ok(removed)
}

pub use aevio_infrastructure::state::StoragePaths;

/// Whether macOS will launch Aevio at login (see login_item.rs).
#[tauri::command]
pub fn launch_at_login() -> LoginItemState {
    login_item::state()
}

#[tauri::command]
pub fn set_launch_at_login(enabled: bool) -> Result<LoginItemState, String> {
    login_item::set(enabled)
}

/// Current authorization status of required/requested system permissions.
#[tauri::command]
pub fn check_permissions() -> Vec<crate::permissions::PermissionInfo> {
    crate::permissions::check_permissions()
}

#[tauri::command]
pub fn storage_paths(app: AppHandle) -> Result<StoragePaths, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    Ok(StoragePaths::new(&data_dir))
}

#[tauri::command]
pub fn get_energy_summary(
    app: AppHandle,
    days: Option<u32>,
) -> Result<crate::energy::EnergySummary, String> {
    with_database(&app, |database| {
        crate::energy::get_summary(
            database,
            days.unwrap_or(crate::energy::DEFAULT_HISTORY_DAYS),
            None,
            None,
        )
    })
}

#[tauri::command]
pub fn query_energy_history(
    app: AppHandle,
    since_ms: Option<u64>,
    limit: Option<u32>,
) -> Result<Vec<crate::energy::EnergySample>, String> {
    with_database(&app, |database| {
        crate::energy::query_history(database, since_ms, limit)
    })
}

#[tauri::command]
pub fn reset_energy_history(app: AppHandle) -> Result<crate::energy::EnergySummary, String> {
    let summary = with_database(&app, |database| {
        crate::energy::reset_history(database)?;
        crate::energy::get_summary(database, crate::energy::DEFAULT_HISTORY_DAYS, None, None)
    })?;
    let _ = app.emit(crate::energy::EVENT_ENERGY_CHANGED, &summary);
    Ok(summary)
}

// --- Updates -----------------------------------------------------------

#[tauri::command]
pub fn update_status(app: AppHandle) -> Result<UpdateStatus, String> {
    app.state::<UpdaterState>().snapshot()
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<UpdateStatus, String> {
    updater::check(&app, true).await;
    app.state::<UpdaterState>().snapshot()
}

/// Downloads and installs the available update, then restarts into it.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    updater::install(&app).await
}

// --- Break reminders ----------------------------------------------------

#[tauri::command]
pub fn break_state(app: AppHandle) -> Result<breaks::engine::BreakState, String> {
    breaks::current_state(&app)
}

#[tauri::command]
pub fn dismiss_stopwatch_reminder(
    app: AppHandle,
    id: String,
    started_at: u64,
) -> Result<breaks::engine::BreakState, String> {
    breaks::dismiss_stopwatch(&app, &id, started_at)
}

/// Starts the pending reminder's break, or a manual one when none is pending.
#[tauri::command]
pub fn start_break(app: AppHandle) -> Result<breaks::engine::BreakState, String> {
    breaks::command(&app, breaks::engine::Command::StartBreak)
}

#[tauri::command]
pub fn end_break(app: AppHandle) -> Result<breaks::engine::BreakState, String> {
    breaks::command(&app, breaks::engine::Command::EndBreak)
}

#[tauri::command]
pub fn snooze_break(app: AppHandle, minutes: u16) -> Result<breaks::engine::BreakState, String> {
    if !crate::settings::SNOOZE_CHOICES.contains(&minutes) {
        return Err(format!("cannot snooze for {minutes} minutes"));
    }
    breaks::command(&app, breaks::engine::Command::Snooze(minutes))
}

#[tauri::command]
pub fn skip_break(app: AppHandle) -> Result<breaks::engine::BreakState, String> {
    breaks::command(&app, breaks::engine::Command::Skip)
}

#[tauri::command]
pub fn expand_break_reminder(app: AppHandle) -> Result<breaks::engine::BreakState, String> {
    breaks::command(&app, breaks::engine::Command::Expand)
}

#[tauri::command]
pub fn extend_break(app: AppHandle) -> Result<breaks::engine::BreakState, String> {
    breaks::command(
        &app,
        breaks::engine::Command::Extend(breaks::EXTEND_MINUTES),
    )
}

/// Silences reminders until `until` (epoch ms); `None` turns them back on.
#[tauri::command]
pub fn pause_break_reminders(
    app: AppHandle,
    until: Option<u64>,
) -> Result<breaks::engine::BreakState, String> {
    breaks::pause_reminders(&app, until)
}

#[tauri::command]
pub fn list_breaks(
    app: AppHandle,
    since_ms: u64,
    until_ms: u64,
) -> Result<Vec<breaks::store::BreakEntry>, String> {
    breaks::list_breaks(&app, since_ms, until_ms)
}

#[tauri::command]
pub fn resize_reminder_panel(app: AppHandle, width: f64, height: f64) {
    breaks::surface::resize(&app, width, height);
}

/// The reminder's "Reminder settings…": brings the main window forward on
/// Settings > Notifications.
#[tauri::command]
pub fn open_break_settings(app: AppHandle) {
    tray::show_main_window(&app);
    let _ = app.emit_to("main", crate::EVENT_OPEN_BREAK_SETTINGS, ());
}

/// Dev builds only: raises a sample reminder (Settings > Notifications).
#[cfg(debug_assertions)]
#[tauri::command]
pub fn dev_sample_break_reminder(app: AppHandle) -> Result<breaks::engine::BreakState, String> {
    breaks::dev_sample(&app)
}

#[tauri::command]
pub fn preview_break_chime() {
    breaks::chime();
}
