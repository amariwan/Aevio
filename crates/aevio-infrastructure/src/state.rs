//! The open stores one process works on, and the lock that makes it the only
//! one. The app holds the lock for its whole life; `aevio` takes it for one
//! request while the app is closed.

use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use rusqlite::{Connection, OpenFlags};
use serde::Serialize;

use crate::decision_settings::{DecisionSettings, DecisionSettingsStore};
use crate::paths;
use crate::protocol::{ApiError, code};
use crate::settings_storage::{self, SettingsStore};
use crate::timers_store::TimerStore;
use aevio_core::settings::Settings;
use aevio_persistence::DATABASE_FILE as DB_FILE;
use aevio_persistence::activity::ActivityStore;
use aevio_persistence::bootstrap::migrations;

pub const NO_DATA: &str =
    "This laptop doesn't have any aevio data. Are you sure you've installed the app before?";

pub struct AppState {
    pub database_path: std::path::PathBuf,
    pub device_ref: String,
    pub store: Mutex<TimerStore>,
    pub activity: Mutex<ActivityStore>,
    /// Whether an Aevio window (main or the Pulse panel) is focused.
    /// Drives the push cadence to the frontend; the underlying sampling rate
    /// is unaffected.
    pub foreground: AtomicBool,
    pub settings: Mutex<SettingsStore>,
    pub decision_settings: Mutex<DecisionSettingsStore>,
}

impl AppState {
    /// Opens every store in `dir`, creating and migrating the database as
    /// needed. Only the app does this. The caller must hold the stores' lock
    /// (`lock_stores`): loading closes any segment left open.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let activity = ActivityStore::load(dir)?;
        Self::with(dir, activity)
    }

    /// Opens the stores the app already made, for `aevio` while the app is
    /// closed. Creates nothing and migrates nothing: a missing database means
    /// the app never ran here, and one at another schema version belongs to a
    /// different release than this `aevio`. The caller must hold the stores'
    /// lock.
    pub fn open_existing(dir: &Path) -> Result<Self, ApiError> {
        let path = dir.join(DB_FILE);
        if !path.is_file() {
            return Err(ApiError::new(code::NO_DATA, NO_DATA));
        }
        let failed = |error: String| ApiError::new(code::FAILED, error);
        let conn = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| failed(error.to_string()))?;
        let version: i32 = conn
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .map_err(|error| failed(error.to_string()))?;
        match version.cmp(&migrations::SCHEMA_VERSION) {
            std::cmp::Ordering::Equal => {}
            std::cmp::Ordering::Less => {
                return Err(ApiError::new(
                    code::INCOMPATIBLE,
                    "Your Aevio data is from an older version of the app. Update Aevio and open it once, then try again.",
                ));
            }
            std::cmp::Ordering::Greater => {
                return Err(ApiError::new(
                    code::INCOMPATIBLE,
                    "Your Aevio data is from a newer version of the app than this aevio. Update aevio to match the app.",
                ));
            }
        }
        drop(conn);
        let activity = ActivityStore::open_existing(path.clone()).map_err(failed)?;
        Self::with(dir, activity).map_err(failed)
    }

    fn with(dir: &Path, mut activity: ActivityStore) -> Result<Self, String> {
        activity.set_database_path(dir.join(DB_FILE))?;
        let store = TimerStore::load(dir)?;
        let settings = SettingsStore::load(&settings_storage::config_dir())?;
        let decision_settings = DecisionSettingsStore::load(&settings_storage::config_dir())?;
        let device_ref = load_or_create_device_ref(dir)?;
        activity.set_tracking_hours(settings.snapshot().tracking_hours.clone());
        Ok(Self {
            database_path: dir.join(DB_FILE),
            device_ref,
            store: Mutex::new(store),
            activity: Mutex::new(activity),
            foreground: AtomicBool::new(true),
            settings: Mutex::new(settings),
            decision_settings: Mutex::new(decision_settings),
        })
    }

    pub fn settings_snapshot(&self) -> Settings {
        self.settings
            .lock()
            .map(|store| store.snapshot())
            .unwrap_or_default()
    }

    pub fn decision_settings_snapshot(&self) -> DecisionSettings {
        self.decision_settings
            .lock()
            .map(|store| store.snapshot())
            .unwrap_or_default()
    }
}

fn load_or_create_device_ref(dir: &Path) -> Result<String, String> {
    let path = dir.join("device.id");
    if let Ok(value) = std::fs::read_to_string(&path) {
        let value = value.trim();
        if !value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control) {
            return Ok(value.to_owned());
        }
    }
    let value = uuid::Uuid::now_v7().to_string();
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut file) => {
            file.write_all(value.as_bytes())
                .map_err(|error| error.to_string())?;
            file.sync_all().map_err(|error| error.to_string())?;
            Ok(value)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            std::fs::read_to_string(path)
                .map(|value| value.trim().to_owned())
                .map_err(|read_error| read_error.to_string())
        }
        Err(error) => Err(error.to_string()),
    }
}

/// Holds the stores for this process. `wait` blocks until a `aevio` edit
/// finishes (the app at launch); otherwise `None` means another process has
/// them (`aevio` finding the app open). Only the lock file is created: the
/// caller decides whether the data directory may be.
pub fn lock_stores(data_dir: &Path, wait: bool) -> io::Result<Option<File>> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(data_dir.join(paths::LOCK_FILE))?;
    if wait {
        file.lock()?;
        return Ok(Some(file));
    }
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(error),
    }
}

/// Where Aevio keeps its files, as Settings and `aevio paths` show them.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoragePaths {
    pub config_file: String,
    pub data_dir: String,
    pub database_file: String,
}

impl StoragePaths {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            config_file: settings_storage::config_dir()
                .join("settings.json")
                .display()
                .to_string(),
            data_dir: data_dir.display().to_string(),
            database_file: data_dir.join(DB_FILE).display().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("aevio-state-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn open_existing_never_creates_the_database() {
        let dir = dir("missing");
        let error = AppState::open_existing(&dir).err().unwrap();
        assert_eq!(error.code, code::NO_DATA);
        assert!(!dir.join(DB_FILE).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn open_existing_refuses_another_schema_version() {
        let dir = dir("version");
        Connection::open(dir.join(DB_FILE))
            .unwrap()
            .execute_batch("PRAGMA user_version = 1;")
            .unwrap();
        let error = AppState::open_existing(&dir).err().unwrap();
        assert_eq!(error.code, code::INCOMPATIBLE);
        assert!(error.message.contains("older"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
