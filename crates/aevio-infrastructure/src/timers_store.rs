//! Timer state and clock policy. Database access lives in the infrastructure
//! repository; this module contains validation and public model mapping.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use aevio_persistence::database::sea_orm::ActiveValue::Set;
use serde::Serialize;

use aevio_persistence::database::connect;
use aevio_persistence::database::entities::{timer_segments, timers};
use aevio_persistence::database::repositories::timers::TimerRepository;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timer {
    pub id: String,
    pub label: String,
    pub accumulated_ms: u64,
    pub segments: Vec<TimerSegment>,
    pub started_at: Option<u64>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerSegment {
    pub id: i64,
    pub duration_ms: u64,
    pub recorded_at: u64,
}

impl Timer {
    pub fn is_running(&self) -> bool {
        self.started_at.is_some()
    }
}

#[derive(Debug)]
struct Clock {
    origin: Instant,
    origin_epoch_ms: u64,
}

impl Clock {
    fn new() -> Self {
        Self {
            origin: Instant::now(),
            origin_epoch_ms: aevio_core::timers::now_epoch_ms(),
        }
    }
    fn now_ms(&self) -> u64 {
        self.origin_epoch_ms + self.origin.elapsed().as_millis() as u64
    }
}

pub struct TimerStore {
    clock: Clock,
    database_path: PathBuf,
}

impl TimerStore {
    pub fn load(dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|error| format!("could not create data dir: {error}"))?;
        Ok(Self {
            clock: Clock::new(),
            database_path: dir.join(aevio_core::DATABASE_FILE),
        })
    }

    pub fn snapshot(&self) -> Result<Vec<Timer>, String> {
        let database = self.database()?;
        let (timer_rows, segment_rows) =
            TimerRepository::new(&database).list().map_err(db_error)?;
        let mut by_timer = HashMap::<String, Vec<TimerSegment>>::new();
        for segment in segment_rows {
            let timer_id = segment.timer_id.clone();
            by_timer
                .entry(timer_id)
                .or_default()
                .push(segment_from_model(segment));
        }
        Ok(timer_rows
            .into_iter()
            .map(|timer| {
                let segments = by_timer.remove(&timer.id).unwrap_or_default();
                timer_from_model(timer, segments)
            })
            .collect())
    }

    pub fn create(&mut self, label: &str) -> Result<Vec<Timer>, String> {
        self.create_at(label, self.clock.now_ms())
    }
    pub fn create_at(&mut self, label: &str, now: u64) -> Result<Vec<Timer>, String> {
        let label = required_label(label)?;
        let database = self.database()?;
        let repository = TimerRepository::new(&database);
        let next = self.take_next_id(&repository)?;
        repository
            .insert(timers::ActiveModel {
                id: Set(format!("t{next}")),
                label: Set(label.to_owned()),
                accumulated_ms: Set(0),
                started_at: Set(None),
                created_at: Set(now as i64),
            })
            .map_err(db_error)?;
        self.snapshot()
    }

    pub fn start(&mut self, id: &str) -> Result<Vec<Timer>, String> {
        self.start_at(id, self.clock.now_ms())
    }
    pub fn start_at(&mut self, id: &str, now: u64) -> Result<Vec<Timer>, String> {
        let database = self.database()?;
        let repository = TimerRepository::new(&database);
        if self.find(&repository, id)?.started_at.is_none() {
            repository
                .set_started_at(id, Some(now as i64))
                .map_err(db_error)?;
        }
        self.snapshot()
    }

    pub fn pause(&mut self, id: &str) -> Result<Vec<Timer>, String> {
        self.pause_at(id, self.clock.now_ms())
    }
    pub fn pause_at(&mut self, id: &str, now: u64) -> Result<Vec<Timer>, String> {
        let database = self.database()?;
        let repository = TimerRepository::new(&database);
        if let Some(started) = self
            .find(&repository, id)?
            .started_at
            .map(|value| value as u64)
        {
            repository
                .pause(id, now.saturating_sub(started) as i64, now as i64)
                .map_err(db_error)?;
        }
        self.snapshot()
    }

    pub fn reset(&mut self, id: &str) -> Result<Vec<Timer>, String> {
        let database = self.database()?;
        let repository = TimerRepository::new(&database);
        self.find(&repository, id)?;
        repository.reset(id).map_err(db_error)?;
        self.snapshot()
    }

    pub fn rename(&mut self, id: &str, label: &str) -> Result<Vec<Timer>, String> {
        let label = required_label(label)?;
        let database = self.database()?;
        let repository = TimerRepository::new(&database);
        self.find(&repository, id)?;
        repository.rename(id, label).map_err(db_error)?;
        self.snapshot()
    }

    pub fn add_segment(&mut self, id: &str, duration_ms: u64) -> Result<Vec<Timer>, String> {
        self.add_segment_at(id, duration_ms, self.clock.now_ms())
    }
    pub fn add_segment_at(
        &mut self,
        id: &str,
        duration_ms: u64,
        now: u64,
    ) -> Result<Vec<Timer>, String> {
        check_duration(duration_ms)?;
        let database = self.database()?;
        let repository = TimerRepository::new(&database);
        self.find(&repository, id)?;
        repository
            .insert_segment(id, duration_ms as i64, now as i64)
            .map_err(db_error)?;
        self.snapshot()
    }

    pub fn update_segment(
        &mut self,
        id: &str,
        segment_id: i64,
        duration_ms: u64,
    ) -> Result<Vec<Timer>, String> {
        check_duration(duration_ms)?;
        let database = self.database()?;
        let repository = TimerRepository::new(&database);
        self.find(&repository, id)?;
        if !repository
            .update_segment(id, segment_id, duration_ms as i64)
            .map_err(db_error)?
        {
            return Err(unknown_segment(segment_id));
        }
        self.snapshot()
    }

    pub fn delete_segment(&mut self, id: &str, segment_id: i64) -> Result<Vec<Timer>, String> {
        let database = self.database()?;
        let repository = TimerRepository::new(&database);
        self.find(&repository, id)?;
        if !repository
            .delete_segment(id, segment_id)
            .map_err(db_error)?
        {
            return Err(unknown_segment(segment_id));
        }
        self.snapshot()
    }

    pub fn delete(&mut self, id: &str) -> Result<Vec<Timer>, String> {
        let database = self.database()?;
        if !TimerRepository::new(&database)
            .delete(id)
            .map_err(db_error)?
        {
            return Err(unknown(id));
        }
        self.snapshot()
    }

    fn database(&self) -> Result<aevio_persistence::database::sea_orm::DatabaseConnection, String> {
        connect(&self.database_path).map_err(|error| error.to_string())
    }
    fn find(&self, repository: &TimerRepository<'_>, id: &str) -> Result<timers::Model, String> {
        repository
            .find(id)
            .map_err(db_error)?
            .ok_or_else(|| unknown(id))
    }
    fn take_next_id(&self, repository: &TimerRepository<'_>) -> Result<u64, String> {
        let next = repository
            .read_setting("timer_next_id")
            .map_err(db_error)?
            .and_then(|value| value.parse().ok())
            .unwrap_or(1);
        repository
            .write_setting("timer_next_id", &(next + 1).to_string())
            .map_err(db_error)?;
        Ok(next)
    }
}

fn timer_from_model(model: timers::Model, segments: Vec<TimerSegment>) -> Timer {
    Timer {
        id: model.id,
        label: model.label,
        accumulated_ms: segments.iter().map(|segment| segment.duration_ms).sum(),
        segments,
        started_at: model.started_at.map(|value| value as u64),
        created_at: model.created_at as u64,
    }
}
fn segment_from_model(model: timer_segments::Model) -> TimerSegment {
    TimerSegment {
        id: model.id,
        duration_ms: model.duration_ms as u64,
        recorded_at: model.recorded_at as u64,
    }
}
fn required_label(label: &str) -> Result<&str, String> {
    let label = label.trim();
    if label.is_empty() {
        Err("a tracker needs a name".to_string())
    } else {
        Ok(label)
    }
}
fn check_duration(duration_ms: u64) -> Result<(), String> {
    if duration_ms == 0 {
        Err("a segment needs some time in it".to_string())
    } else {
        Ok(())
    }
}
fn db_error(error: aevio_persistence::database::sea_orm::DbErr) -> String {
    error.to_string()
}
fn unknown_segment(segment_id: i64) -> String {
    format!("no time segment with id {segment_id}")
}
fn unknown(id: &str) -> String {
    format!("no tracker with id {id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "aevio-{name}-{}",
            aevio_core::timers::now_epoch_ms()
        ));
        fs::create_dir_all(&dir).unwrap();
        let conn = rusqlite::Connection::open(dir.join(aevio_core::DATABASE_FILE)).unwrap();
        conn.execute_batch("CREATE TABLE timers (id TEXT PRIMARY KEY,label TEXT NOT NULL,accumulated_ms INTEGER NOT NULL,started_at INTEGER,created_at INTEGER NOT NULL); CREATE TABLE timer_segments (id INTEGER PRIMARY KEY AUTOINCREMENT,timer_id TEXT NOT NULL,duration_ms INTEGER NOT NULL,recorded_at INTEGER NOT NULL); CREATE TABLE settings (key TEXT PRIMARY KEY,value TEXT NOT NULL);").unwrap();
        dir
    }
    fn store(name: &str) -> TimerStore {
        TimerStore::load(&temp_dir(name)).unwrap()
    }

    #[test]
    fn pause_accumulates_only_the_time_that_ran() {
        let mut s = store("pause");
        let id = s.create_at("Write", 1000).unwrap()[0].id.clone();
        s.start_at(&id, 1000).unwrap();
        s.pause_at(&id, 4500).unwrap();
        assert_eq!(s.snapshot().unwrap()[0].accumulated_ms, 3500);
        s.pause_at(&id, 9000).unwrap();
        assert_eq!(s.snapshot().unwrap()[0].accumulated_ms, 3500);
    }
    #[test]
    fn segments_are_the_only_time_and_can_be_edited() {
        let mut s = store("segments");
        let id = s.create_at("Edit", 1000).unwrap()[0].id.clone();
        s.start_at(&id, 1000).unwrap();
        s.pause_at(&id, 61000).unwrap();
        s.add_segment_at(&id, 30000, 70000).unwrap();
        let t = s.snapshot().unwrap().remove(0);
        assert_eq!(t.accumulated_ms, 90000);
        let a = t.segments[0].id;
        let b = t.segments[1].id;
        s.update_segment(&id, a, 10000).unwrap();
        assert_eq!(s.snapshot().unwrap()[0].accumulated_ms, 40000);
        s.delete_segment(&id, b).unwrap();
        assert_eq!(s.snapshot().unwrap()[0].accumulated_ms, 10000);
        assert!(s.add_segment_at(&id, 0, 80000).is_err());
        assert!(s.update_segment(&id, b, 5000).is_err());
    }
    #[test]
    fn reset_zeroes_and_stops_the_timer() {
        let mut s = store("reset");
        let id = s.create_at("Reset", 1000).unwrap()[0].id.clone();
        s.start_at(&id, 1000).unwrap();
        s.reset(&id).unwrap();
        let t = &s.snapshot().unwrap()[0];
        assert_eq!(t.accumulated_ms, 0);
        assert_eq!(t.started_at, None);
    }
    #[test]
    fn a_new_tracker_is_created_idle() {
        let mut s = store("idle");
        let t = &s.create_at("Idle", 1000).unwrap()[0];
        assert_eq!(t.started_at, None);
        assert_eq!(t.accumulated_ms, 0);
    }
    #[test]
    fn bad_input_returns_an_error_instead_of_panicking() {
        let mut s = store("bad");
        assert!(s.create_at("   ", 1000).is_err());
        assert!(s.pause("nope").is_err());
        assert!(s.delete("nope").is_err());
        assert!(s.rename("nope", "still nope").is_err());
    }
    #[test]
    fn a_running_timer_survives_a_reload() {
        let dir = temp_dir("reload");
        let mut first = TimerStore::load(&dir).unwrap();
        let id = first.create_at("Survive", 1000).unwrap()[0].id.clone();
        first.start_at(&id, 1000).unwrap();
        first.pause_at(&id, 4000).unwrap();
        let second = TimerStore::load(&dir).unwrap();
        let timers = second.snapshot().unwrap();
        assert_eq!(timers[0].label, "Survive");
        assert_eq!(timers[0].accumulated_ms, 3000);
        assert_eq!(timers.len(), 1);
    }
}
