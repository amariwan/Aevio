//! The activity store: folds the app's foreground-window samples into
//! *segments* (one row per contiguous run of the same app + window title +
//! url), and owns the entries, catalog and queries built on them. The app's
//! sampler feeds it; `aevio` reads and edits it directly.

use std::fs;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::time::Duration;

#[cfg(test)]
use rusqlite::Connection;
#[cfg(test)]
use rusqlite::OptionalExtension;
#[cfg(test)]
use rusqlite::Row;
#[cfg(test)]
use rusqlite::params;
use serde::Serialize;

use aevio_core::capture::WindowSample;
use aevio_core::integrations::IntegrationConnection;
#[cfg(test)]
use aevio_core::integrations::{Provider, SyncDirection};
#[cfg(test)]
use aevio_core::models::EntryEvent;
use aevio_core::models::{
    AppContribution, AppRecord, Category, Client, EntryDetail, Label, NewCategory, NewClient,
    NewLabel, NewProject, NewTask, NewTimeEntry, Project, Task, TimeEntry, TitleItem,
    UpdateCategory, UpdateClient, UpdateLabel, UpdateProject, UpdateTask, UpdateTimeEntry,
};

pub const SAMPLE_SECS: u64 = 1;
pub const APP_SWITCH_GRACE_MS: u64 = 20_000;
pub const DEFAULT_IDLE_THRESHOLD_MS: u64 = 5 * 60 * 1000;
pub const DB_FILE: &str = crate::DATABASE_FILE;

const ONE_DAY_MS: u64 = 24 * 60 * 60 * 1000;
const RETENTION_DAYS: u64 = 90;
const RETENTION_MS: u64 = RETENTION_DAYS * ONE_DAY_MS;

#[cfg(test)]
use aevio_core::timers::now_epoch_ms;

pub use aevio_core::entries::models::{KIND_ACTIVITY, KIND_BREAK, KIND_FOCUS, KIND_MEETING};

pub const IDLE_LABEL: &str = "Idle";

/// Why every change to an approved entry is refused. Approval freezes an
/// entry; unapproving is the only way back to changing it.
pub const LOCKED: &str = "Approved entries are locked. Unapprove to change them.";

pub const APPROVED_OVERLAP: &str = "This time range overlaps approved entries. Unapprove those entries before creating a manual entry here.";

#[cfg(test)]
fn ensure_no_approved_overlap(conn: &Connection, start: u64, end: u64) -> Result<(), String> {
    let overlaps: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM time_entries WHERE deleted_at IS NULL AND status = 'approved' AND started_at < ?2 AND ended_at > ?1);",
        params![start as i64, end as i64],
        |row| row.get(0),
    ).map_err(|error| error.to_string())?;
    if overlaps {
        return Err(APPROVED_OVERLAP.to_string());
    }
    Ok(())
}

/// How long a statement waits on another connection's write lock. The AI
/// worker reads on its own connection, so contention is brief but real.
#[cfg(test)]
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// The window the background worker keeps rebuilt (see ai/worker.rs).
pub const REBUILD_WINDOW_MS: u64 = ONE_DAY_MS;

pub use aevio_core::models::ActivitySegment;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivitySnapshot {
    pub current: Option<ActivitySegment>,
    pub segments: Vec<ActivitySegment>,
    pub tracked_ms: u64,
    pub focus_ms: u64,
    pub break_ms: u64,
    pub unreviewed: u64,
    pub idle_ms: u64,
    pub idle_threshold_ms: u64,
    pub capture_enabled: bool,
    pub in_tracking_hours: bool,
    pub tracking_active: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityTick {
    pub current: Option<ActivitySegment>,
    pub tracked_ms: u64,
    pub focus_ms: u64,
    pub break_ms: u64,
    pub unreviewed: u64,
    pub idle_ms: u64,
    pub idle_threshold_ms: u64,
    pub capture_enabled: bool,
    pub in_tracking_hours: bool,
    pub tracking_active: bool,
}

#[derive(Debug, Clone)]
struct Current {
    id: i64,
    app: String,
    title: String,
    kind: String,
    label: Option<String>,
    started_at: u64,
    reviewed: bool,
    app_id: Option<String>,
    bundle_id: Option<String>,
    url: Option<String>,
    domain: Option<String>,
    entry_id: Option<String>,
}

impl Current {
    fn as_segment(&self) -> ActivitySegment {
        ActivitySegment {
            id: self.id,
            app: self.app.clone(),
            title: self.title.clone(),
            kind: self.kind.clone(),
            label: self.label.clone(),
            started_at: self.started_at,
            ended_at: None,
            reviewed: self.reviewed,
            app_id: self.app_id.clone(),
            bundle_id: self.bundle_id.clone(),
            url: self.url.clone(),
            domain: self.domain.clone(),
            entry_id: self.entry_id.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct PendingSwitch {
    sample: WindowSample,
    started_at: u64,
    started_instant: Option<std::time::Instant>,
}

fn is_same_app(cur_app: &str, cur_bundle_id: Option<&str>, sample: &WindowSample) -> bool {
    if let (Some(cur_bid), Some(sample_bid)) = (cur_bundle_id, sample.bundle_id.as_deref()) {
        cur_bid == sample_bid && cur_app == sample.app
    } else {
        cur_app == sample.app
    }
}

pub struct LiveState {
    current: Option<Current>,
    capture_enabled: bool,
    pub idle_threshold_ms: u64,
    pub last_idle_ms: u64,
    pub watch_since_ms: Option<u64>,
    in_tracking_hours: bool,
    pub tracking_active: bool,
}

/// What the break engine needs to know about the open segment.
#[derive(Debug, Clone)]
pub struct SegmentInfo {
    pub id: i64,
    pub kind: String,
    pub label: Option<String>,
}

impl LiveState {
    pub fn segment_info(&self) -> Option<SegmentInfo> {
        self.current.as_ref().map(|current| SegmentInfo {
            id: current.id,
            kind: current.kind.clone(),
            label: current.label.clone(),
        })
    }
}

#[cfg(test)]
struct Totals {
    tracked_ms: u64,
    focus_ms: u64,
    break_ms: u64,
    unreviewed: u64,
}

#[cfg(test)]
#[path = "approval_lock.rs"]
mod approval_lock;
#[cfg(test)]
#[path = "break_regressions.rs"]
mod break_regressions;
#[cfg(test)]
#[path = "deletions.rs"]
mod deletions;
#[path = "history.rs"]
mod history;
#[cfg(test)]
#[path = "split_regressions.rs"]
mod split_regressions;
#[path = "typed_deletions.rs"]
mod typed_deletions;
#[path = "typed_history.rs"]
mod typed_history;
#[path = "typed_structure.rs"]
mod typed_structure;

pub struct ActivityStore {
    #[cfg(test)]
    conn: Connection,
    database_path: Option<PathBuf>,
    undo_stack: Vec<history::Command>,
    redo_stack: Vec<history::Command>,
    current: Option<Current>,
    pending_switch: Option<PendingSwitch>,
    capture_enabled: bool,
    idle_threshold_ms: u64,
    last_idle_ms: u64,
    /// When the foreground app was last seen keeping the display awake.
    watched_at_ms: Option<u64>,
    pub watch_since_ms: Option<u64>,
    tracking_hours: aevio_core::settings::TrackingHours,
    manual_tracking: bool,
}

impl ActivityStore {
    #[cfg(test)]
    pub fn load(dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|error| format!("could not create data dir: {error}"))?;
        let path: PathBuf = dir.join(DB_FILE);
        let conn = Connection::open(&path).map_err(|error| error.to_string())?;
        let mut store = Self::from_conn(conn)?;
        store.set_database_path(path)?;
        Ok(store)
    }

    #[cfg(not(test))]
    pub fn load(dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|error| format!("could not create data dir: {error}"))?;
        let path = dir.join(DB_FILE);
        crate::bootstrap::migrations::initialize_database(&path)?;
        let mut store = Self {
            database_path: Some(path.clone()),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current: None,
            pending_switch: None,
            capture_enabled: true,
            idle_threshold_ms: DEFAULT_IDLE_THRESHOLD_MS,
            last_idle_ms: 0,
            watched_at_ms: None,
            watch_since_ms: None,
            tracking_hours: aevio_core::settings::TrackingHours::default(),
            manual_tracking: false,
        };
        store.set_database_path(path)?;
        Ok(store)
    }

    #[cfg(not(test))]
    pub fn open_existing(path: PathBuf) -> Result<Self, String> {
        if !path.is_file() {
            return Err("database file does not exist".into());
        }
        let mut store = Self {
            database_path: Some(path.clone()),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current: None,
            pending_switch: None,
            capture_enabled: true,
            idle_threshold_ms: DEFAULT_IDLE_THRESHOLD_MS,
            last_idle_ms: 0,
            watched_at_ms: None,
            watch_since_ms: None,
            tracking_hours: aevio_core::settings::TrackingHours::default(),
            manual_tracking: false,
        };
        store.set_database_path(path)?;
        Ok(store)
    }

    #[cfg(test)]
    pub fn open_reader(dir: &Path) -> Result<Connection, String> {
        let path = dir.join(DB_FILE);
        let conn = Connection::open(&path).map_err(|error| error.to_string())?;
        conn.busy_timeout(BUSY_TIMEOUT)
            .map_err(|error| error.to_string())?;
        Ok(conn)
    }

    #[cfg(test)]
    pub fn from_conn(mut conn: Connection) -> Result<Self, String> {
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA optimize;")
            .map_err(|error| error.to_string())?;
        conn.busy_timeout(BUSY_TIMEOUT)
            .map_err(|error| error.to_string())?;

        crate::bootstrap::migrations::run_migrations(&mut conn)
            .map_err(|error| error.to_string())?;

        let mut store = Self {
            conn,
            database_path: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current: None,
            pending_switch: None,
            capture_enabled: true,
            idle_threshold_ms: DEFAULT_IDLE_THRESHOLD_MS,
            last_idle_ms: 0,
            watched_at_ms: None,
            watch_since_ms: None,
            tracking_hours: aevio_core::settings::TrackingHours::default(),
            manual_tracking: false,
        };
        store.capture_enabled = store
            .read_setting("capture_enabled")
            .and_then(|value| value.parse().ok())
            .unwrap_or(true);
        store.idle_threshold_ms = store
            .read_setting("idle_threshold_ms")
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_IDLE_THRESHOLD_MS);

        Ok(store)
    }

    pub fn set_database_path(&mut self, path: PathBuf) -> Result<(), String> {
        self.database_path = Some(path);
        self.capture_enabled = self
            .read_setting("capture_enabled")
            .and_then(|value| value.parse().ok())
            .unwrap_or(self.capture_enabled);
        self.idle_threshold_ms = self
            .read_setting("idle_threshold_ms")
            .and_then(|value| value.parse().ok())
            .unwrap_or(self.idle_threshold_ms);
        let database = self.database().map_err(|error| error.to_string())?;
        let repository =
            crate::database::repositories::activity::ActivityRepository::new(&database);
        repository
            .recover_open_segments()
            .and_then(|_| {
                repository.rollup_old_segments(
                    aevio_core::timers::now_epoch_ms().saturating_sub(RETENTION_MS) as i64,
                    ONE_DAY_MS as i64,
                )
            })
            .map_err(|error| error.to_string())
    }

    fn database(&self) -> Result<sea_orm::DatabaseConnection, sea_orm::DbErr> {
        let path = self
            .database_path
            .as_ref()
            .ok_or_else(|| sea_orm::DbErr::Custom("database path unavailable".into()))?;
        crate::database::connect(path)
    }

    /// The writer connection, for the AI worker's writes (see ai/store.rs).
    #[cfg(test)]
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    #[cfg(test)]
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    pub fn read_setting(&self, key: &str) -> Option<String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).ok()?;
            return crate::database::repositories::settings::SettingsRepository::new(&database)
                .value(key)
                .ok()
                .flatten();
        }
        #[cfg(not(test))]
        return None;
        #[cfg(test)]
        self.conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .ok()
    }

    pub fn write_setting(&self, key: &str, value: &str) -> Result<(), String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            return crate::database::repositories::settings::SettingsRepository::new(&database)
                .set(key, value)
                .map_err(|error| error.to_string());
        }
        #[cfg(not(test))]
        return Err("database path unavailable for settings write".into());
        #[cfg(test)]
        self.conn
            .execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub fn set_tracking_hours(&mut self, hours: aevio_core::settings::TrackingHours) {
        self.tracking_hours = hours;
    }

    pub fn is_in_tracking_window(&self, now: u64) -> bool {
        self.tracking_hours.is_inside_window(now)
    }

    pub fn is_sample_excluded(&self, sample: &WindowSample) -> bool {
        let domain = sample.domain.as_deref();
        let bundle_id = sample.bundle_id.as_deref();
        let app = sample.app.as_str();
        let identifier = domain.or(bundle_id).unwrap_or(app);

        if let Some(path) = self.database_path.as_ref() {
            let Ok(database) = crate::database::connect(path) else {
                return false;
            };
            return crate::database::repositories::apps::AppRepository::new(&database)
                .is_excluded(identifier, bundle_id, app)
                .unwrap_or(false);
        }
        #[cfg(test)]
        {
            let excluded: Result<i64, _> = self.conn.query_row(
                "SELECT excluded FROM apps WHERE deleted_at IS NULL AND (identifier = ?1 OR identifier = ?2 OR identifier = ?3 OR display_name = ?3) AND excluded != 0 LIMIT 1;",
                params![identifier, bundle_id.unwrap_or(identifier), app],
                |row| row.get(0),
            );
            excluded.map(|v| v != 0).unwrap_or(false)
        }
        #[cfg(not(test))]
        {
            false
        }
    }
}

#[cfg(test)]
pub fn segment_from_row(row: &Row<'_>) -> rusqlite::Result<ActivitySegment> {
    Ok(ActivitySegment {
        id: row.get(0)?,
        app: row.get(1)?,
        title: row.get(2)?,
        kind: row.get(3)?,
        label: row.get(4)?,
        started_at: row.get::<_, i64>(5)? as u64,
        ended_at: row.get::<_, Option<i64>>(6)?.map(|ms| ms as u64),
        reviewed: row.get::<_, i64>(7)? != 0,
        app_id: row.get(8).ok(),
        bundle_id: row.get(9).ok(),
        url: row.get(10).ok(),
        domain: row.get(11).ok(),
        entry_id: row.get(12).ok(),
    })
}

/// The `clients` columns `client_from_row` reads, in its order.
#[cfg(test)]
const CLIENT_COLUMNS: &str = "id, name, email, address, default_rate, currency, notes, archived_at, created_at, updated_at, deleted_at";

#[cfg(test)]
fn client_from_row(row: &Row<'_>) -> rusqlite::Result<Client> {
    Ok(Client {
        id: row.get(0)?,
        name: row.get(1)?,
        email: row.get(2)?,
        address: row.get(3)?,
        default_rate: row.get(4)?,
        currency: row.get(5)?,
        notes: row.get(6)?,
        archived_at: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
        created_at: row.get::<_, i64>(8)? as u64,
        updated_at: row.get::<_, i64>(9)? as u64,
        deleted_at: row.get::<_, Option<i64>>(10)?.map(|v| v as u64),
    })
}

#[cfg(test)]
fn compute_totals(conn: &Connection, since_ms: u64, now: u64) -> Result<Totals, String> {
    let mut statement = conn
        .prepare(
            "SELECT kind,
                    COALESCE(SUM(ended_at - MAX(started_at, ?1)), 0),
                    COALESCE(SUM(CASE WHEN reviewed = 0 THEN 1 ELSE 0 END), 0)
             FROM segments
             WHERE ended_at IS NOT NULL AND ended_at >= ?1 AND started_at < ?2
             GROUP BY kind",
        )
        .map_err(|error| error.to_string())?;

    let mut totals = Totals {
        tracked_ms: 0,
        focus_ms: 0,
        break_ms: 0,
        unreviewed: 0,
    };

    let rows = statement
        .query_map(params![since_ms as i64, now as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)? as u64,
                row.get::<_, i64>(2)? as u64,
            ))
        })
        .map_err(|error| error.to_string())?;

    for row in rows {
        let (kind, duration_ms, unreviewed) = row.map_err(|error| error.to_string())?;
        match kind.as_str() {
            KIND_ACTIVITY => {
                totals.tracked_ms += duration_ms;
                totals.unreviewed += unreviewed;
            }
            KIND_FOCUS => {
                totals.tracked_ms += duration_ms;
                totals.focus_ms += duration_ms;
            }
            KIND_MEETING => {
                totals.tracked_ms += duration_ms;
            }
            KIND_BREAK => {
                totals.break_ms += duration_ms;
            }
            _ => {}
        }
    }
    Ok(totals)
}

#[cfg(test)]
fn fetch_segments(
    conn: &Connection,
    since_ms: u64,
    now: u64,
) -> Result<Vec<ActivitySegment>, String> {
    let mut statement = conn
        .prepare(
            "SELECT id, app, title, kind, label, started_at, ended_at, reviewed, app_id, bundle_id, url, domain, entry_id
             FROM segments
             WHERE ended_at IS NULL OR (ended_at >= ?1 AND started_at < ?2)
             ORDER BY started_at ASC",
        )
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map(params![since_ms as i64, now as i64], segment_from_row)
        .map_err(|error| error.to_string())?;

    let mut segments = Vec::new();
    for row in rows {
        segments.push(row.map_err(|error| error.to_string())?);
    }
    Ok(segments)
}

#[cfg(test)]
fn add_current(totals: &mut Totals, current: Option<&Current>, since_ms: u64, now: u64) {
    if let Some(current) = current {
        let effective_start = current.started_at.max(since_ms);
        let active_ms = now.saturating_sub(effective_start);
        match current.kind.as_str() {
            KIND_ACTIVITY => {
                totals.tracked_ms += active_ms;
                if !current.reviewed {
                    totals.unreviewed += 1;
                }
            }
            KIND_FOCUS => {
                totals.tracked_ms += active_ms;
                totals.focus_ms += active_ms;
            }
            KIND_MEETING => {
                totals.tracked_ms += active_ms;
            }
            KIND_BREAK => {
                totals.break_ms += active_ms;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
pub fn build_snapshot(
    conn: &Connection,
    live: &LiveState,
    since_ms: u64,
    now: u64,
) -> Result<ActivitySnapshot, String> {
    let mut totals = compute_totals(conn, since_ms, now)?;
    add_current(&mut totals, live.current.as_ref(), since_ms, now);

    let mut segments = fetch_segments(conn, since_ms, now)?;
    if let Some(current) = &live.current
        && !segments.iter().any(|s| s.id == current.id)
    {
        segments.push(current.as_segment());
    }

    Ok(ActivitySnapshot {
        current: live.current.as_ref().map(Current::as_segment),
        segments,
        tracked_ms: totals.tracked_ms,
        focus_ms: totals.focus_ms,
        break_ms: totals.break_ms,
        unreviewed: totals.unreviewed,
        idle_ms: live.last_idle_ms,
        idle_threshold_ms: live.idle_threshold_ms,
        capture_enabled: live.capture_enabled,
        in_tracking_hours: live.in_tracking_hours,
        tracking_active: live.tracking_active,
    })
}

#[cfg(test)]
pub fn build_tick(
    conn: &Connection,
    live: &LiveState,
    since_ms: u64,
    now: u64,
) -> Result<ActivityTick, String> {
    let mut totals = compute_totals(conn, since_ms, now)?;
    add_current(&mut totals, live.current.as_ref(), since_ms, now);

    Ok(ActivityTick {
        current: live.current.as_ref().map(Current::as_segment),
        tracked_ms: totals.tracked_ms,
        focus_ms: totals.focus_ms,
        break_ms: totals.break_ms,
        unreviewed: totals.unreviewed,
        idle_ms: live.last_idle_ms,
        idle_threshold_ms: live.idle_threshold_ms,
        capture_enabled: live.capture_enabled,
        in_tracking_hours: live.in_tracking_hours,
        tracking_active: live.tracking_active,
    })
}

#[path = "store/apps.rs"]
mod apps;
#[path = "store/catalog.rs"]
mod catalog;
#[path = "store/entries.rs"]
mod entries;
#[path = "store/integrations.rs"]
mod integrations;
#[path = "store/tasks.rs"]
mod tasks;
#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
#[path = "store/tracking.rs"]
mod tracking;
