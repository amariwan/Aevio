mod activity;
mod ai;
mod breaks;
mod capture;
mod coach;
mod commands;
mod energy;
mod i18n;
mod invoices;
mod login_item;
mod music_ui;
mod permissions;
mod plugin_network;
mod plugin_registry;
mod plugin_ui;
mod pulse;
mod rpc;
mod runtime;
mod summary;
mod tray;
mod updater;
mod usage;
mod watch_bridge;
mod widget;
mod work_intelligence;

use aevio_core::{models, settings, timers};
use aevio_persistence::projects;

use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::{Manager, RunEvent, WindowEvent};

use settings::CloseBehavior;

/// Emitted after every timer mutation so the frontend can adopt the snapshot
/// Rust already has — the tray can change state without the window asking.
pub const EVENT_TIMERS_CHANGED: &str = "timers-changed";

/// Emitted on every structural activity change (a segment opened or closed)
/// and on focus-regained reconciliation. Carries the full `ActivitySnapshot`
/// — see activity.rs's module doc for the push-model rationale.
pub const EVENT_ACTIVITY_CHANGED: &str = "activity-changed";

/// Emitted at 1Hz while Aevio is focused, or every 30s while it isn't and
/// nothing structural has changed. Carries the lighter `ActivityTick`.
pub const EVENT_ACTIVITY_TICK: &str = "activity-tick";

/// Emitted whenever preferences change, so any open view (and the tray) can
/// re-read them without polling.
pub const EVENT_SETTINGS_CHANGED: &str = "settings-changed";

/// Emitted whenever time entries are modified or rebuilt.
pub const EVENT_ENTRIES_CHANGED: &str = "entries-changed";

/// Sent to the main window when the break reminder asks for its settings.
pub const EVENT_OPEN_BREAK_SETTINGS: &str = "open-break-settings";

/// Sent to the main window when the Pulse panel's review button asks it to
/// open today's review queue.
pub const EVENT_OPEN_REVIEW: &str = "open-review";

/// Emitted after a plugin installation, permission or lifecycle mutation.
pub const EVENT_PLUGINS_CHANGED: &str = "plugins-changed";

pub use aevio_infrastructure::state::AppState;

/// Launched by `aevio app start`: run in the menu bar with no window until
/// one is asked for.
const BACKGROUND_ARG: &str = "--background";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let background = std::env::args().any(|arg| arg == BACKGROUND_ARG);
    tauri::Builder::default()
        // External plugin pages use a plugin-specific origin and are served
        // only from the active, staged UI bundle. Their window has no
        // matching capability file, so no Tauri IPC surface is injected.
        .register_uri_scheme_protocol("aevio-plugin", plugin_ui::response)
        .register_uri_scheme_protocol("aevio-music", music_ui::response)
        // Registered first so a second launch exits before it opens the
        // database; the running copy comes forward instead.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if !args.iter().any(|arg| arg == BACKGROUND_ARG) {
                tray::show_main_window(app);
            }
        }))
        .plugin(
            // Seeds the page's theme/shape hint before its first paint; see
            // `settings::appearance_init_script`.
            tauri::plugin::Builder::<tauri::Wry>::new("appearance")
                .js_init_script(
                    aevio_infrastructure::settings_storage::appearance_init_script(
                        &aevio_infrastructure::settings_storage::config_dir(),
                    ),
                )
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            runtime::install_bundled_cli();
            let dir = app.path().app_data_dir()?;
            // Waits out a `aevio` edit made while the app was closed, then
            // keeps the stores for as long as the app runs.
            std::fs::create_dir_all(&dir)?;
            let lock =
                aevio_infrastructure::state::lock_stores(&dir, true)?.expect("a blocking lock");
            app.manage(rpc::StoresLock(lock));

            let state = AppState::load(&dir)
                .map_err(|error| -> Box<dyn std::error::Error> { error.into() })?;
            let timers = state.store.lock().map_err(|e| e.to_string())?.snapshot()?;
            let preferences = state.settings_snapshot();
            app.manage(state);
            runtime::spawn_automation_scheduler(app.handle().clone());
            app.manage(ai::AiRuntime::default());
            app.manage(coach::provider::CoachAi::default());
            app.manage(pulse::PulseState::default());
            breaks::init(app.handle())?;
            coach::spawn_scheduler(app.handle().clone());
            app.manage(updater::UpdaterState::new(
                app.package_info().version.to_string(),
            ));

            if preferences.tray_enabled {
                tray::init(app.handle(), &timers)?;
            }
            activity::spawn_sampler(app.handle().clone());
            ai::worker::spawn(
                app.handle().clone(),
                dir.join(activity::DB_FILE),
                dir.join("ml"),
            );
            energy::spawn_sampler(app.handle().clone(), dir.join(activity::DB_FILE));
            capture::register_sleep_listeners(app.handle().clone());
            widget::init(app.handle())?;
            watch_bridge::start(app.handle().clone(), dir.clone());
            spawn_retention_sweeper(app.handle().clone());
            updater::spawn_scheduler(app.handle().clone());
            // One sweep on startup, so a long-dormant install is cleaned before
            // the first hour-long wait elapses.
            if let Err(error) = commands::sweep_retention(app.handle()) {
                eprintln!("retention sweep failed: {error}");
            }
            rpc::serve(app.handle().clone(), dir);
            // The window is created hidden (tauri.conf.json), so a background
            // launch never flashes it.
            if background {
                #[cfg(target_os = "macos")]
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            } else {
                tray::show_main_window(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // The panel has no close control; anything that asks just hides it.
            WindowEvent::CloseRequested { api, .. } if window.label() == pulse::LABEL => {
                api.prevent_close();
                let _ = window.hide();
            }
            WindowEvent::CloseRequested { api, .. } if window.label() == summary::LABEL => {
                api.prevent_close();
                let _ = window.hide();
            }
            // Closing is a preference: either end the app, or hide to the menu
            // bar so a background tracker keeps running. With the tray off a
            // hidden window would be unreachable, so that combination quits.
            WindowEvent::CloseRequested { api, .. } => {
                let app = window.app_handle();
                let preferences = app.state::<AppState>().settings_snapshot();
                api.prevent_close();
                if preferences.tray_enabled && preferences.close_behavior == CloseBehavior::Hide {
                    let _ = window.hide();
                } else {
                    app.exit(0);
                }
            }
            // Drives the activity push cadence (1Hz while any Aevio window
            // is focused / 30s otherwise — see activity.rs). Recomputed from
            // every window, so the order of one window's blur and the next
            // one's focus does not matter.
            WindowEvent::Focused(is_focused) => {
                let app = window.app_handle();
                if !*is_focused && window.label() == pulse::LABEL {
                    pulse::on_blur(app);
                }
                let focused = app
                    .webview_windows()
                    .values()
                    .any(|window| window.is_focused().unwrap_or(false));
                let state = window.state::<AppState>();
                let was_foreground = state.foreground.swap(focused, Ordering::SeqCst);
                if focused && !was_foreground {
                    activity::emit_full(app);
                    updater::on_focus(app);
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_timers,
            coach::focus_commands::coach_focus,
            coach::focus_commands::coach_start_focus,
            coach::focus_commands::coach_end_focus,
            coach::adaptive_commands::coach_plan_proposal,
            coach::adaptive_commands::coach_intervention,
            coach::adaptive_commands::coach_dismiss_intervention,
            coach::adaptive_commands::coach_snooze_intervention,
            coach::adaptive_commands::coach_feedback,
            coach::adaptive_commands::coach_intelligence,
            coach::adaptive_commands::coach_apply_recommendation,
            coach::adaptive_commands::coach_suppress_recommendation,
            coach::adaptive_commands::coach_suppress_observation,
            coach::adaptive_commands::coach_reset_personal_intelligence,
            coach::adaptive_commands::coach_insights,
            coach::adaptive_commands::coach_memory,
            coach::brain_commands::coach_brain,
            coach::brain_commands::coach_brain_apply,
            coach::brain_commands::coach_classify_thought,
            coach::commands::coach_day,
            coach::commands::coach_apply,
            coach::commands::coach_propose,
            commands::coach_snapshot,
            commands::list_coach_tasks,
            commands::create_coach_task,
            commands::update_coach_task,
            commands::complete_coach_task,
            commands::delete_coach_task,
            commands::create_timer,
            commands::start_timer,
            commands::pause_timer,
            commands::reset_timer,
            commands::rename_timer,
            commands::add_timer_segment,
            commands::update_timer_segment,
            commands::delete_timer_segment,
            commands::delete_timer,
            commands::watch_pairing,
            commands::list_music_tracks,
            commands::import_music_track,
            commands::plugin_catalog,
            commands::plugin_installed,
            commands::plugin_contributions,
            commands::plugin_health,
            commands::plugin_layout,
            commands::plugin_set_contribution_preference,
            commands::plugin_restore_layout,
            commands::plugin_update_policy,
            commands::plugin_set_update_policy,
            commands::plugin_pick_package,
            commands::plugin_execute_command,
            commands::plugin_open_ui,
            commands::plugin_search,
            commands::plugin_search_all,
            commands::plugin_publish_event,
            commands::plugin_subscribe_event,
            commands::plugin_show_notification,
            commands::plugin_schedule_job,
            commands::plugin_cancel_job,
            commands::plugin_workflow_run,
            commands::plugin_workflow_cancel,
            commands::plugin_workflow_history,
            commands::plugin_plan_install,
            commands::plugin_install,
            commands::plugin_grant_permissions,
            commands::plugin_revoke_permissions,
            commands::plugin_set_enabled,
            commands::plugin_uninstall,
            commands::plugin_delete_data,
            commands::plugin_storage_get,
            commands::plugin_storage_set,
            commands::plugin_storage_delete,
            commands::plugin_install_package,
            plugin_registry::catalog,
            plugin_registry::updates,
            plugin_registry::workspace_plugin_states,
            plugin_registry::set_workspace_plugin_state,
            plugin_registry::install,
            plugin_network::fetch,
            commands::plugin_rollback,
            commands::plugin_recover,
            commands::activity_snapshot,
            commands::usage_snapshot,
            commands::set_capture_enabled,
            commands::set_idle_threshold,
            commands::start_session,
            commands::stop_session,
            commands::mark_segment_reviewed,
            commands::get_settings,
            commands::update_settings,
            commands::get_decision_settings,
            commands::update_decision_settings,
            commands::decision_health,
            commands::evaluate_decision,
            commands::evaluate_decision_use_case,
            commands::coach_decision,
            commands::storage_paths,
            commands::launch_at_login,
            commands::set_launch_at_login,
            commands::check_permissions,
            commands::governance_snapshot,
            commands::save_governance_team,
            commands::save_governance_department,
            commands::save_governance_member,
            commands::save_governance_defaults,
            commands::authorize_governance,
            commands::list_categories,
            commands::create_category,
            commands::update_category,
            commands::delete_category,
            commands::list_projects,
            commands::create_project,
            commands::update_project,
            commands::delete_project,
            commands::list_documents,
            commands::create_document,
            commands::update_document,
            commands::delete_document,
            commands::list_notes,
            commands::create_note,
            commands::update_note,
            commands::delete_note,
            commands::list_comments,
            commands::create_comment,
            commands::list_attachments,
            commands::create_attachment,
            commands::list_mentions,
            commands::list_cycles,
            commands::create_cycle,
            commands::list_initiatives,
            commands::create_initiative,
            commands::list_tasks,
            commands::create_task,
            commands::update_task,
            commands::delete_task,
            commands::list_labels,
            commands::create_label,
            commands::update_label,
            commands::delete_label,
            commands::list_integrations,
            commands::save_integration,
            commands::list_clients,
            commands::create_client,
            commands::update_client,
            commands::delete_client,
            commands::timeline_day,
            commands::publish_timeline_segment,
            commands::unpublish_timeline_segment,
            commands::list_timeline_devices,
            commands::timeline_sync_state,
            commands::sync_timeline,
            // Local invoice lifecycle.
            commands::list_invoices,
            commands::get_invoice,
            commands::list_billable_entries,
            commands::quote_invoice,
            commands::render_invoice_preview,
            commands::get_invoice_pdf,
            commands::save_invoice_draft,
            commands::finalize_invoice,
            commands::set_invoice_paid,
            commands::void_invoice,
            commands::delete_draft_invoice,
            commands::delete_void_invoice,
            commands::export_invoice_pdf,
            commands::get_invoice_profile,
            commands::update_invoice_profile,
            commands::get_invoice_logo,
            commands::set_invoice_logo,
            commands::clear_invoice_logo,
            commands::list_time_entries,
            commands::get_entry_detail,
            commands::update_time_entry,
            commands::approve_time_entries,
            commands::unapprove_time_entries,
            commands::reject_time_entry,
            commands::split_time_entry,
            commands::merge_time_entries,
            commands::delete_time_entry,
            commands::delete_time_entries,
            commands::undo,
            commands::create_time_entry,
            commands::rebuild_time_entries,
            commands::list_apps,
            commands::update_app,
            commands::ai_status,
            commands::agent_chat,
            commands::work_ask,
            commands::work_relationships_list,
            commands::work_relationship_add,
            commands::work_traverse,
            commands::work_project_suggestions,
            commands::work_dismiss_suggestion,
            commands::work_match_existing,
            commands::automation_snapshot,
            commands::daily_workflow_snapshot,
            commands::run_daily_workflow,
            commands::create_routine,
            commands::create_report,
            commands::create_dashboard,
            commands::run_routine,
            commands::run_report,
            commands::finance_snapshot,
            commands::list_finance_rate_cards,
            commands::save_finance_rate_card,
            commands::list_finance_contracts,
            commands::save_finance_contract,
            commands::share_finance_dashboard,
            commands::list_finance_capacity_plans,
            commands::save_finance_capacity_plan,
            commands::list_finance_dashboard_shares,
            commands::revoke_finance_dashboard_share,
            commands::staffing_snapshot,
            commands::list_staffing_freelancers,
            commands::save_staffing_freelancer,
            commands::save_staffing_record,
            commands::staffing_matches,
            commands::retry_classification,
            commands::resolve_rule_suggestion,
            commands::ai_metrics,
            commands::ai_retrain,
            commands::ai_reset_learned,
            commands::query_time_entries,
            commands::entry_rollup,
            commands::export_time_entries,
            commands::update_time_entries,
            commands::project_stats,
            commands::project_rules,
            commands::preview_project_hints,
            commands::discover_projects,
            commands::dismiss_project_suggestion,
            commands::import_projects_csv,
            commands::get_energy_summary,
            commands::query_energy_history,
            commands::reset_energy_history,
            commands::resize_widget,
            commands::resize_pulse_panel,
            commands::hide_pulse_panel,
            commands::open_productivity_summary,
            commands::hide_productivity_summary,
            commands::break_state,
            commands::dismiss_stopwatch_reminder,
            commands::start_break,
            commands::end_break,
            commands::snooze_break,
            commands::skip_break,
            commands::expand_break_reminder,
            commands::extend_break,
            commands::pause_break_reminders,
            commands::list_breaks,
            commands::resize_reminder_panel,
            #[cfg(debug_assertions)]
            commands::dev_sample_break_reminder,
            commands::open_break_settings,
            commands::preview_break_chime,
            commands::open_main_window,
            commands::update_status,
            commands::check_for_updates,
            commands::install_update,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Aevio")
        .run(|app, event| match event {
            // macOS: clicking the dock icon with no visible window reopens it.
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => tray::show_main_window(app),
            // Close the open segment at the real quit time. Left open, the
            // next launch can only close it at its own start, losing its time.
            RunEvent::Exit => {
                breaks::on_exit(app);
                if let Ok(mut store) = app.state::<AppState>().activity.lock() {
                    let _ = store.close_active_segment(timers::now_epoch_ms());
                }
            }
            _ => {}
        });
}

/// Deletes activity history past the retention window once an hour. Startup
/// does the first pass; this keeps a process open.
fn spawn_retention_sweeper(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(3600));
        if let Err(error) = commands::sweep_retention(&app) {
            eprintln!("retention sweep failed: {error}");
        }
    });
}

#[cfg(test)]
mod cli_tests {
    #[test]
    fn independent_client_identity_matches_the_release_bundle() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(config["identifier"], aevio_core::APP_IDENTIFIER);
    }
}
