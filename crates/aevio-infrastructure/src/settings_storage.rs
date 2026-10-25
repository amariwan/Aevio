use std::fs;
use std::path::{Path, PathBuf};

use aevio_core::settings::Settings;
use aevio_core::timers::now_epoch_ms;

pub const FILE_NAME: &str = "settings.json";

pub fn config_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("aevio")
}

pub struct SettingsStore {
    path: PathBuf,
    settings: Settings,
}

pub fn appearance_init_script(dir: &Path) -> String {
    let settings = fs::read(dir.join(FILE_NAME))
        .ok()
        .and_then(|bytes| decode_settings(&bytes).ok())
        .unwrap_or_default();
    let palette = serde_json::to_string(&settings.palette).unwrap_or_default();
    let mode = serde_json::to_string(&settings.mode).unwrap_or_default();
    let shape = serde_json::to_string(&settings.shape).unwrap_or_default();
    format!(
        "try {{ const seed = (key, value) => {{ if (localStorage.getItem(key) === null) localStorage.setItem(key, value); }}; seed(\"aevio.theme\", {palette}); seed(\"aevio.mode\", {mode}); seed(\"aevio.shape\", {shape}); }} catch (_) {{}}"
    )
}

/// Read-time compatibility for pre-palette settings. Canonical fields win.
fn decode_settings(bytes: &[u8]) -> Result<Settings, serde_json::Error> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes)?;
    if let Some(fields) = value.as_object_mut() {
        if !fields.contains_key("palette") {
            let palette = match fields.get("accent").and_then(serde_json::Value::as_str) {
                Some("green") => Some("forest"),
                Some("blue") => Some("ocean"),
                Some("purple") => Some("violet"),
                Some("orange") => Some("sunset"),
                _ => None,
            };
            if let Some(palette) = palette {
                fields.insert("palette".into(), palette.into());
            }
        }
        if !fields.contains_key("mode")
            && let Some(mode) = fields
                .get("theme")
                .and_then(serde_json::Value::as_str)
                .filter(|mode| matches!(*mode, "light" | "dark" | "system"))
                .map(str::to_owned)
        {
            fields.insert("mode".into(), mode.into());
        }
    }
    serde_json::from_value(value)
}

impl SettingsStore {
    pub fn load(dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|error| format!("could not create config dir: {error}"))?;
        let path = dir.join(FILE_NAME);
        let settings = fs::read(&path)
            .ok()
            .and_then(|bytes| decode_settings(&bytes).ok())
            .unwrap_or_else(|| {
                if path.exists() {
                    let quarantine = path.with_extension(format!("corrupt-{}", now_epoch_ms()));
                    let _ = fs::rename(&path, &quarantine);
                    eprintln!(
                        "settings.json was unreadable; moved to {}",
                        quarantine.display()
                    );
                }
                Settings::default()
            });

        Ok(Self {
            path,
            settings: settings.normalized(),
        })
    }

    pub fn snapshot(&self) -> Settings {
        self.settings.clone()
    }

    pub fn set(&mut self, next: Settings) -> Result<Settings, String> {
        let normalized = next.normalized();
        self.persist(&normalized)?;
        self.settings = normalized;
        Ok(self.snapshot())
    }

    pub fn patch(&mut self, key: &str, value: serde_json::Value) -> Result<Settings, String> {
        let mut document = serde_json::to_value(&self.settings).map_err(|e| e.to_string())?;
        let parts: Vec<&str> = key.split('.').collect();
        let (last, parents) = parts.split_last().ok_or("empty settings key")?;
        let mut slot = &mut document;
        for part in parents {
            slot = slot
                .get_mut(*part)
                .filter(|value| value.is_object())
                .ok_or_else(|| format!("unknown setting {key}"))?;
        }
        let fields = slot
            .as_object_mut()
            .ok_or_else(|| format!("unknown setting {key}"))?;
        if !fields.contains_key(*last) {
            return Err(format!("unknown setting {key}"));
        }
        fields.insert((*last).to_string(), value);
        let next: Settings = serde_json::from_value(document)
            .map_err(|error| format!("invalid value for {key}: {error}"))?;
        self.set(next)
    }

    fn persist(&self, settings: &Settings) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
        let temp = self.path.with_extension("json.tmp");
        fs::write(&temp, json).map_err(|error| format!("could not write settings: {error}"))?;
        fs::rename(&temp, &self.path)
            .map_err(|error| format!("could not replace settings: {error}"))
    }
}

#[cfg(test)]
mod storage_tests {
    use super::*;
    use std::fs;

    #[test]
    fn legacy_appearance_migrates_without_overriding_explicit_preferences() {
        use aevio_core::settings::{Mode, Palette};
        let legacy = decode_settings(br#"{"theme":"dark","accent":"orange"}"#).unwrap();
        assert_eq!(legacy.palette, Palette::Sunset);
        assert_eq!(legacy.mode, Mode::Dark);
        let canonical = decode_settings(
            br#"{"palette":"ocean","mode":"light","theme":"dark","accent":"orange"}"#,
        )
        .unwrap();
        assert_eq!(canonical.palette, Palette::Ocean);
        assert_eq!(canonical.mode, Mode::Light);
    }

    #[test]
    fn failed_persistence_does_not_change_the_in_memory_snapshot() {
        let root =
            std::env::temp_dir().join(format!("aevio-settings-persist-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let mut store = SettingsStore::load(&root).unwrap();
        let before = store.snapshot();

        // A directory at the target path makes the atomic replacement fail.
        fs::create_dir(root.join(FILE_NAME)).unwrap();
        let result = store.set(Settings {
            retention_days: 30,
            ..before.clone()
        });

        assert!(result.is_err());
        assert_eq!(store.snapshot(), before);
        let _ = fs::remove_dir_all(root);
    }
}
#[cfg(test)]
mod tests {
    use crate::settings_storage::{FILE_NAME, SettingsStore, appearance_init_script};
    use aevio_core::settings::*;
    use aevio_core::timers::now_epoch_ms;
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aevio-settings-{name}-{}", now_epoch_ms()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn notification_placements_round_trip_and_legacy_center_migrates() {
        let dir = temp_dir("notification-placement");
        let mut store = SettingsStore::load(&dir).unwrap();
        for placement in [
            NotificationPlacement::TopLeft,
            NotificationPlacement::TopRight,
            NotificationPlacement::TopCenter,
            NotificationPlacement::BottomLeft,
            NotificationPlacement::BottomRight,
        ] {
            store
                .set(Settings {
                    notification_placement: placement,
                    ..Settings::default()
                })
                .unwrap();
            assert_eq!(
                SettingsStore::load(&dir)
                    .unwrap()
                    .snapshot()
                    .notification_placement,
                placement
            );
        }
        fs::write(
            dir.join(FILE_NAME),
            r#"{"notificationPlacement":"centerMiddle","accent":"orange"}"#,
        )
        .unwrap();
        let mut legacy = SettingsStore::load(&dir).unwrap();
        let migrated = legacy.snapshot();
        assert_eq!(
            migrated.notification_placement,
            NotificationPlacement::TopCenter
        );
        // Legacy accent values map to their palette on load.
        assert_eq!(migrated.palette, Palette::Sunset);
        legacy.set(migrated).unwrap();
        let saved: serde_json::Value =
            serde_json::from_slice(&fs::read(dir.join(FILE_NAME)).unwrap()).unwrap();
        assert_eq!(saved["notificationPlacement"], "topCenter");
        assert_eq!(
            SettingsStore::load(&dir)
                .unwrap()
                .snapshot()
                .notification_placement,
            NotificationPlacement::TopCenter
        );
    }

    #[test]
    fn stopwatch_reminder_defaults_bounds_and_saved_values() {
        let dir = temp_dir("stopwatch-reminder");
        let mut store = SettingsStore::load(&dir).unwrap();
        assert_eq!(
            store.snapshot().stopwatch_reminder,
            StopwatchReminder::default()
        );
        let old: Settings = serde_json::from_str(r#"{"mode":"light"}"#).unwrap();
        assert!(old.stopwatch_reminder.enabled);
        assert_eq!(old.stopwatch_reminder.after_minutes, 180);
        for (input, expected) in [(0, 30), (240, 240), (u32::MAX, 720)] {
            store
                .set(Settings {
                    stopwatch_reminder: StopwatchReminder {
                        enabled: false,
                        after_minutes: input,
                    },
                    ..Settings::default()
                })
                .unwrap();
            let saved = SettingsStore::load(&dir)
                .unwrap()
                .snapshot()
                .stopwatch_reminder;
            assert!(!saved.enabled);
            assert_eq!(saved.after_minutes, expected);
        }
    }

    #[test]
    fn patch_changes_one_field_and_validates_it() {
        let dir = temp_dir("patch");
        let mut store = SettingsStore::load(&dir).unwrap();
        store
            .set(Settings {
                retention_days: 30,
                ..Settings::default()
            })
            .unwrap();
        let next = store
            .patch("weeklyTargetHours", serde_json::json!(32))
            .unwrap();
        assert_eq!(next.weekly_target_hours, 32);
        assert_eq!(next.retention_days, 30);
        let breaks = !next.breaks.enabled;
        assert_eq!(
            store
                .patch("breaks.enabled", serde_json::json!(breaks))
                .unwrap()
                .breaks
                .enabled,
            breaks
        );
        // Clamped like any other write.
        assert_eq!(
            store
                .patch("weeklyTargetHours", serde_json::json!(1_000))
                .unwrap()
                .weekly_target_hours,
            MAX_WEEKLY_TARGET_HOURS
        );
        for (key, value) in [
            ("nope", serde_json::json!(1)),
            ("breaks.nope", serde_json::json!(1)),
            ("breaks.enabled", serde_json::json!("maybe")),
            ("weeklyTargetHours.deeper", serde_json::json!(1)),
        ] {
            assert!(store.patch(key, value).is_err(), "accepted {key}");
        }
        // Failed patches leave the file as it was.
        let reloaded = SettingsStore::load(&dir).unwrap().snapshot();
        assert_eq!(reloaded, store.snapshot());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn defaults_round_trip() {
        let dir = temp_dir("defaults");
        let store = SettingsStore::load(&dir).unwrap();
        assert_eq!(store.snapshot(), Settings::default());
    }

    #[test]
    fn values_survive_a_reload() {
        let dir = temp_dir("reload");
        {
            let mut store = SettingsStore::load(&dir).unwrap();
            store
                .set(Settings {
                    palette: Palette::Sunset,
                    mode: Mode::Light,
                    language: Language::De,
                    shape: Shape::Sharper,
                    size_mode: SizeMode::VeryRelaxed,
                    close_behavior: CloseBehavior::Quit,
                    tray_enabled: false,
                    retention_days: 30,
                    ai_suggest: AiSuggest::Category,
                    auto_accept: false,
                    auto_accept_percent: 90,
                    ai_custom_prompt: "Aevio is Coding".to_string(),
                    ai_provider: String::new(),
                    ai_base_url: String::new(),
                    ai_model: String::new(),
                    ai_api_key: String::new(),
                    weekly_target_hours: 32,
                    tracking_hours: TrackingHours {
                        enabled: true,
                        per_day: true,
                        default_start: "08:00".to_string(),
                        default_end: "17:00".to_string(),
                        monday: DaySchedule {
                            enabled: true,
                            start: "09:00".to_string(),
                            end: "18:00".to_string(),
                        },
                        tuesday: DaySchedule {
                            enabled: false,
                            start: "07:00".to_string(),
                            end: "19:00".to_string(),
                        },
                        wednesday: DaySchedule::default(),
                        thursday: DaySchedule::default(),
                        friday: DaySchedule::default(),
                        saturday: DaySchedule::default(),
                        sunday: DaySchedule::default(),
                    },
                    breaks: BreakSettings::default(),
                    stopwatch_reminder: StopwatchReminder::default(),
                    notification_placement: NotificationPlacement::TopLeft,
                })
                .unwrap();
        }
        let reloaded = SettingsStore::load(&dir).unwrap().snapshot();
        assert_eq!(reloaded.mode, Mode::Light);
        assert_eq!(reloaded.language, Language::De);
        assert_eq!(reloaded.palette, Palette::Sunset);
        assert_eq!(reloaded.shape, Shape::Sharper);
        assert_eq!(reloaded.size_mode, SizeMode::VeryRelaxed);
        assert_eq!(reloaded.close_behavior, CloseBehavior::Quit);
        assert!(!reloaded.tray_enabled);
        assert_eq!(reloaded.retention_days, 30);
        assert_eq!(reloaded.ai_suggest, AiSuggest::Category);
        assert!(!reloaded.auto_accept);
        assert_eq!(reloaded.auto_accept_percent, 90);
        assert_eq!(reloaded.ai_custom_prompt, "Aevio is Coding");
        assert_eq!(reloaded.weekly_target_hours, 32);
        assert_eq!(
            reloaded.notification_placement,
            NotificationPlacement::TopLeft
        );
        assert!(reloaded.tracking_hours.per_day);
        assert_eq!(reloaded.tracking_hours.monday.start, "09:00");
        assert!(!reloaded.tracking_hours.tuesday.enabled);
    }

    #[test]
    fn keys_from_removed_settings_are_ignored_and_dropped_on_the_next_save() {
        let dir = temp_dir("removed-keys");
        fs::write(
            dir.join(FILE_NAME),
            r#"{"weeklyTargetHours":32,"advancedWorkflowTracking":true,"extensions":{"tmux":false}}"#,
        )
        .unwrap();
        let mut store = SettingsStore::load(&dir).unwrap();
        assert_eq!(store.snapshot().weekly_target_hours, 32);

        store.patch("retentionDays", serde_json::json!(7)).unwrap();
        let saved = fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        assert!(!saved.contains("advancedWorkflowTracking"));
        assert!(!saved.contains("extensions"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn mode_defaults_to_system_but_keeps_saved_choices() {
        assert_eq!(Settings::default().mode, Mode::System);
        let unset: Settings = serde_json::from_str(r#"{"palette":"ocean"}"#).unwrap();
        assert_eq!(unset.mode, Mode::System);
        let system: Settings = serde_json::from_str(r#"{"mode":"system"}"#).unwrap();
        assert_eq!(system.mode, Mode::System);
        let light: Settings = serde_json::from_str(r#"{"mode":"light"}"#).unwrap();
        assert_eq!(light.mode, Mode::Light);
    }

    #[test]
    fn appearance_init_script_seeds_the_saved_choices() {
        let dir = temp_dir("init-script");
        let mut store = SettingsStore::load(&dir).unwrap();
        store
            .set(Settings {
                mode: Mode::Light,
                shape: Shape::Sharper,
                ..Settings::default()
            })
            .unwrap();
        let script = appearance_init_script(&dir);
        assert!(script.contains("seed(\"aevio.mode\", \"light\")"));
        assert!(script.contains("seed(\"aevio.shape\", \"sharper\")"));
        // Only fills a missing copy; an existing one is current.
        assert!(script.contains("localStorage.getItem(key) === null"));

        let system = temp_dir("init-script-system");
        fs::write(system.join(FILE_NAME), r#"{"mode":"system"}"#).unwrap();
        let script = appearance_init_script(&system);
        assert!(script.contains("\"system\""));
        assert!(script.contains("\"rounded\""));
    }

    #[test]
    fn appearance_init_script_falls_back_to_the_defaults() {
        let missing = temp_dir("init-script-missing");
        let script = appearance_init_script(&missing);
        assert!(script.contains("seed(\"aevio.mode\", \"system\")"));
        assert!(script.contains("seed(\"aevio.shape\", \"rounded\")"));

        let corrupt = temp_dir("init-script-corrupt");
        fs::write(corrupt.join(FILE_NAME), "not json").unwrap();
        let script = appearance_init_script(&corrupt);
        assert!(script.contains("\"system\""));
        // Reading must not quarantine the file; `load` owns that.
        assert!(corrupt.join(FILE_NAME).exists());
    }

    #[test]
    fn older_settings_default_to_rounded_shape() {
        let loaded: Settings = serde_json::from_str(r#"{"mode":"light"}"#).unwrap();
        assert_eq!(loaded.shape, Shape::Rounded);
        let sharper: Settings = serde_json::from_str(r#"{"shape":"sharper"}"#).unwrap();
        assert_eq!(sharper.shape, Shape::Sharper);
    }

    #[test]
    fn older_settings_default_to_normal_size() {
        let loaded: Settings =
            serde_json::from_str(r#"{"mode":"dark","palette":"ocean"}"#).unwrap();
        assert_eq!(loaded.size_mode, SizeMode::Normal);
    }

    #[test]
    fn auto_accept_threshold_is_clamped() {
        let dir = temp_dir("threshold");
        let mut store = SettingsStore::load(&dir).unwrap();
        let saved = store
            .set(Settings {
                auto_accept_percent: 5,
                ..Settings::default()
            })
            .unwrap();
        assert_eq!(saved.auto_accept_percent, MIN_AUTO_ACCEPT_PERCENT);
    }

    #[test]
    fn retention_is_clamped() {
        let dir = temp_dir("clamp");
        let mut store = SettingsStore::load(&dir).unwrap();
        let saved = store
            .set(Settings {
                retention_days: u32::MAX,
                ..Settings::default()
            })
            .unwrap();
        assert_eq!(saved.retention_days, MAX_RETENTION_DAYS);
    }

    #[test]
    fn weekly_target_is_clamped() {
        let dir = temp_dir("target");
        let mut store = SettingsStore::load(&dir).unwrap();
        let zero = store
            .set(Settings {
                weekly_target_hours: 0,
                ..Settings::default()
            })
            .unwrap();
        assert_eq!(zero.weekly_target_hours, 1);
        let huge = store
            .set(Settings {
                weekly_target_hours: 500,
                ..Settings::default()
            })
            .unwrap();
        assert_eq!(huge.weekly_target_hours, MAX_WEEKLY_TARGET_HOURS);
    }

    #[test]
    fn break_reminders_default_to_fifty_minutes_of_work_then_five_off_the_clock() {
        let defaults = Settings::default().breaks;
        assert!(defaults.enabled);
        assert_eq!(defaults.work_minutes, 50);
        assert_eq!(defaults.break_minutes, 5);
        assert_eq!(defaults.snooze_minutes, 5);
        assert_eq!(defaults.message, "");
        // Pausing stopwatches for a break is opt-in.
        assert!(!defaults.pause_stopwatches);
        assert!(defaults.schedules.is_empty());
        // An older settings file loads with the defaults filled in.
        let loaded: Settings = serde_json::from_str(r#"{"mode":"dark"}"#).unwrap();
        assert_eq!(loaded.breaks, defaults);
    }

    #[test]
    fn break_settings_are_clamped_and_schedules_get_ids() {
        let dir = temp_dir("breaks");
        let mut store = SettingsStore::load(&dir).unwrap();
        let saved = store
            .set(Settings {
                breaks: BreakSettings {
                    work_minutes: 1,
                    break_minutes: 500,
                    snooze_minutes: 7,
                    message: "  x".repeat(100),
                    schedules: vec![ScheduledBreak {
                        label: "   ".to_string(),
                        at: "25:99".to_string(),
                        minutes: 0,
                        ..ScheduledBreak::default()
                    }],
                    ..BreakSettings::default()
                },
                ..Settings::default()
            })
            .unwrap();
        assert_eq!(saved.breaks.work_minutes, 15);
        assert_eq!(saved.breaks.break_minutes, 60);
        assert_eq!(saved.breaks.snooze_minutes, DEFAULT_SNOOZE_MINUTES);
        assert!(saved.breaks.message.chars().count() <= MAX_BREAK_MESSAGE_CHARS);
        let schedule = &saved.breaks.schedules[0];
        assert!(!schedule.id.is_empty());
        assert_eq!(schedule.label, "Break");
        assert_eq!(schedule.at, "12:30");
        assert_eq!(schedule.minutes, 1);
        let reloaded = SettingsStore::load(&dir).unwrap().snapshot();
        assert_eq!(reloaded.breaks, saved.breaks);
    }

    #[test]
    fn a_corrupt_file_falls_back_to_defaults() {
        let dir = temp_dir("corrupt");
        fs::write(dir.join(FILE_NAME), b"{ not json").unwrap();
        let store = SettingsStore::load(&dir).unwrap();
        assert_eq!(store.snapshot(), Settings::default());
        // The bad file is preserved under a quarantine name, not deleted.
        assert!(
            fs::read_dir(&dir)
                .unwrap()
                .filter_map(Result::ok)
                .any(|entry| entry.file_name().to_string_lossy().contains("corrupt"))
        );
    }

    #[test]
    fn test_is_time_in_window_daytime() {
        // 07:00 to 19:00
        assert!(!is_time_in_window(6, 59, "07:00", "19:00"));
        assert!(is_time_in_window(7, 0, "07:00", "19:00"));
        assert!(is_time_in_window(12, 30, "07:00", "19:00"));
        assert!(is_time_in_window(18, 59, "07:00", "19:00"));
        assert!(!is_time_in_window(19, 0, "07:00", "19:00"));
        assert!(!is_time_in_window(23, 0, "07:00", "19:00"));
    }

    #[test]
    fn test_is_time_in_window_zero_length() {
        // Zero length window means no capture
        assert!(!is_time_in_window(7, 0, "07:00", "07:00"));
        assert!(!is_time_in_window(12, 0, "07:00", "07:00"));
    }

    #[test]
    fn test_is_time_in_window_overnight() {
        // 22:00 to 06:00
        assert!(!is_time_in_window(21, 59, "22:00", "06:00"));
        assert!(is_time_in_window(22, 0, "22:00", "06:00"));
        assert!(is_time_in_window(23, 59, "22:00", "06:00"));
        assert!(is_time_in_window(0, 0, "22:00", "06:00"));
        assert!(is_time_in_window(5, 59, "22:00", "06:00"));
        assert!(!is_time_in_window(6, 0, "22:00", "06:00"));
        assert!(!is_time_in_window(12, 0, "22:00", "06:00"));
    }

    #[test]
    fn test_tracking_hours_collapse_syncs_all_days() {
        let mut th = TrackingHours {
            per_day: true,
            default_start: "08:00".to_string(),
            default_end: "16:00".to_string(),
            monday: DaySchedule {
                enabled: true,
                start: "10:00".to_string(),
                end: "20:00".to_string(),
            },
            ..TrackingHours::default()
        };
        assert_eq!(th.monday.start, "10:00");
        // Collapsing to single window:
        th.per_day = false;
        let normalized = th.normalized();
        assert_eq!(normalized.monday.start, "08:00");
        assert_eq!(normalized.monday.end, "16:00");
        assert_eq!(normalized.sunday.start, "08:00");
        assert_eq!(normalized.sunday.end, "16:00");
    }

    #[test]
    fn test_tracking_hours_disabled_day() {
        use chrono::{Local, TimeZone};
        let th = TrackingHours {
            enabled: true,
            per_day: true,
            default_start: "07:00".to_string(),
            default_end: "19:00".to_string(),
            monday: DaySchedule {
                enabled: false,
                start: "07:00".to_string(),
                end: "19:00".to_string(),
            },
            ..TrackingHours::default()
        };
        // Construct a Monday at 12:00 local time
        let dt = Local
            .with_ymd_and_hms(2026, 9, 28, 12, 0, 0)
            .single()
            .expect("local dt"); // 2026-09-28 is a Monday
        let ms = dt.timestamp_millis() as u64;
        assert!(!th.is_inside_window(ms));
    }
}
