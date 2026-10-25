//! The classification worker: one background thread that keeps recent
//! entries built, drains `classify_jobs` one job at a time, owns the Swift
//! sidecar, retrains the personal models, and refits confidence calibration.
//!
//! Reads use the worker's own SQLite connection, so a slow query never
//! blocks the 1 Hz sampler. Writes go through the `ActivityStore` writer and
//! hold its lock only for the few statements that persist a result; no lock
//! is held while the sidecar thinks.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use aevio_persistence::database::connect;
use aevio_persistence::database::repositories::ai::{AiRepository, Job as QueueJob};
use aevio_persistence::database::repositories::ai_artifacts::AiArtifactsRepository;
use aevio_persistence::database::repositories::ai_worker::{
    AiWorkerRepository, ClassificationUpdate, EventInput, SuggestionInput,
};
#[cfg(test)]
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

use super::arbiter::{self, Decision, Evidence, Label, Votes};
use super::calibration::Calibrator;
use super::features::{self, EntryFeatures, PreviousEntry};
use super::knn;
use super::power::Conditions;
use super::rules;
use super::sidecar::{self, Capabilities, Sidecar};
use super::{update_status, AiRuntime, Field, EVENT_SUGGESTION_READY, PREFILL_THRESHOLD};
use crate::activity::{self, ActivitySegment};
use crate::models::TimeEntry;
use crate::settings::{AiSuggest, Settings};
use crate::timers::now_epoch_ms;
use crate::AppState;
use aevio_persistence::database::repositories::ai::{Job, JOB_CLASSIFY};

/// Fallback poll when nothing nudges the worker.
const POLL: Duration = Duration::from_secs(30);
/// A burst of window switches rebuilds at most this often.
const REBUILD_DEBOUNCE: Duration = Duration::from_secs(3);
/// While the Foundation Model is missing, check again this often (it may
/// finish downloading, or the user may turn Apple Intelligence on).
const REPROBE_EVERY: Duration = Duration::from_secs(30 * 60);
const RETRAIN_CHECK_EVERY: Duration = Duration::from_secs(10 * 60);
/// New labeled entries that trigger retraining the personal models, and the
/// fewest a model is trained on.
pub const RETRAIN_AFTER_LABELS: u32 = 20;
/// With fewer new labels, the models still retrain once this long after the
/// last attempt ("nightly").
const RETRAIN_AT_LEAST_EVERY_MS: u64 = 24 * 60 * 60 * 1000;
/// One entry in `HOLDOUT_ONE_IN` is held out to judge a retrained model.
const HOLDOUT_ONE_IN: u64 = 5;
const CALIBRATION_CHECK_EVERY: Duration = Duration::from_secs(60);
/// New verdicts that trigger refitting the calibration curve.
const CALIBRATION_REFIT_AFTER: u32 = 10;
/// With more choices than this, only the fast tiers' top picks are sent to
/// the model, to stay inside its 4,096-token context.
const MAX_LLM_CATEGORIES: usize = 16;
const MAX_LLM_PROJECTS: usize = 15;
const LLM_SHORTLIST: usize = 8;
const FEW_SHOT_EXAMPLES: usize = 4;
/// A previous entry this close before the current one is context.
const CONTEXT_GAP_MS: u64 = 15 * 60 * 1000;
const BACKFILL_WINDOW_MS: u64 = 7 * 24 * 60 * 60 * 1000;

fn label_from_model(label: &str) -> Option<String> {
    (label != "none").then(|| label.to_owned())
}

fn why_text(signals: &[aevio_core::models::Signal]) -> String {
    let joined = signals
        .iter()
        .map(|signal| signal.text.as_str())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    let mut chars = joined.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => joined,
    }
}

fn queue_job(job: QueueJob) -> Job {
    Job {
        id: job.id,
        entry_id: job.entry_id,
        kind: job.kind,
        attempts: job.attempts,
    }
}

fn queue_job_input(job: &Job) -> QueueJob {
    QueueJob {
        id: job.id.clone(),
        entry_id: job.entry_id.clone(),
        kind: job.kind.clone(),
        attempts: job.attempts,
    }
}

pub fn spawn(app: AppHandle, db_path: PathBuf, ml_dir: PathBuf) {
    std::thread::Builder::new()
        .name("ai-worker".to_string())
        .spawn(move || Worker::new(app, db_path, ml_dir).run())
        .expect("spawn ai worker");
}

struct Choice {
    id: String,
    name: String,
    description: Option<String>,
    hint: Option<String>,
}

struct Worker {
    app: AppHandle,
    database_path: PathBuf,
    sidecar: Sidecar,
    ml_dir: PathBuf,
    last_rebuild: Option<Instant>,
    last_probe: Option<Instant>,
    last_retrain_check: Option<Instant>,
    last_calibration_check: Option<Instant>,
    applied: Option<Capabilities>,
}

/// What one personal-model retrain did.
enum Retrained {
    /// Not enough labeled data to train on yet.
    Skipped(String),
    /// The new model beat (or tied) the old one on the holdout and replaced it.
    Swapped { accuracy: Option<f64> },
    /// The new model did worse on the holdout; the old one stays.
    Kept { new: f64, old: f64 },
}

impl Worker {
    fn new(app: AppHandle, database_path: PathBuf, ml_dir: PathBuf) -> Self {
        Self {
            app,
            database_path,
            sidecar: Sidecar::new(),
            ml_dir,
            last_rebuild: None,
            last_probe: None,
            last_retrain_check: None,
            last_calibration_check: None,
            applied: None,
        }
    }

    fn run(mut self) {
        if let Err(error) = self.write_database(|database| {
            AiRepository::new(database)
                .requeue_interrupted()
                .map_err(|error| error.to_string())
        }) {
            eprintln!("ai worker: {error}");
        }
        self.probe();
        loop {
            self.rebuild_if_due();
            let now = now_epoch_ms();
            match self.write_database(|database| {
                AiRepository::new(database)
                    .enqueue_live_due(now as i64)
                    .map_err(|error| error.to_string())
            }) {
                Ok(count) if count > 0 => self.emit_entries_changed(),
                Err(error) => eprintln!("ai worker: live classification: {error}"),
                _ => {}
            }
            // Drain what's due, but come back to rebuild between jobs so a
            // long backlog never delays a block that just closed.
            for _ in 0..10 {
                match self.next_due(now_epoch_ms()) {
                    Ok(Some(job)) => self.run_job(job),
                    Ok(None) => break,
                    Err(error) => {
                        eprintln!("ai worker: {error}");
                        break;
                    }
                }
            }
            if self.sidecar.stop_if_idle() {
                update_status(&self.app, |s| s.sidecar = "stopped".to_string());
            }
            self.reprobe_if_due();
            self.retrain_if_due();
            self.refit_calibration_if_due(false);
            self.refresh_counts();

            let due_now = self.next_due(now_epoch_ms()).ok().flatten().is_some();
            if !due_now {
                self.app.state::<AiRuntime>().wait(POLL);
            }
        }
    }

    // --- Plumbing -----------------------------------------------------------

    fn database(&self) -> Result<aevio_persistence::database::DatabaseConnection, String> {
        connect(&self.database_path).map_err(|error| error.to_string())
    }

    fn write_database<T>(
        &self,
        f: impl FnOnce(&aevio_persistence::database::DatabaseConnection) -> Result<T, String>,
    ) -> Result<T, String> {
        let state = self.app.state::<AppState>();
        let _writer = state.activity.lock().map_err(|error| error.to_string())?;
        let database = self.database()?;
        f(&database)
    }

    fn next_due(&self, now: u64) -> Result<Option<Job>, String> {
        let database = self.database()?;
        AiRepository::new(&database)
            .next_due(now as i64)
            .map(|job| job.map(queue_job))
            .map_err(|error| error.to_string())
    }

    fn emit_entries_changed(&self) {
        let _ = self.app.emit(crate::EVENT_ENTRIES_CHANGED, ());
    }

    fn settings(&self) -> Settings {
        self.app.state::<AppState>().settings_snapshot()
    }

    fn rebuild_if_due(&mut self) {
        if self
            .last_rebuild
            .is_some_and(|at| at.elapsed() < REBUILD_DEBOUNCE)
        {
            return;
        }
        self.last_rebuild = Some(Instant::now());
        let now = now_epoch_ms();
        let changed = {
            let state = self.app.state::<AppState>();
            let Ok(mut store) = state.activity.lock() else {
                return;
            };
            store.rebuild_range(now.saturating_sub(activity::REBUILD_WINDOW_MS), now, now)
        };
        match changed {
            Ok(true) => self.emit_entries_changed(),
            Ok(false) => {}
            Err(error) => eprintln!("ai worker: rebuild failed: {error}"),
        }
    }

    // --- Engine status -----------------------------------------------------

    fn probe(&mut self) {
        self.last_probe = Some(Instant::now());
        if !self.sidecar.is_installed() {
            update_status(&self.app, |s| {
                s.engine = "rules".to_string();
                s.llm = "unsupportedOS".to_string();
                s.sidecar = "unavailable".to_string();
            });
            return;
        }
        match self.sidecar.probe() {
            Ok(_) => self.apply_capabilities(),
            Err(error) => update_status(&self.app, |s| {
                s.engine = "rules".to_string();
                s.sidecar = "unavailable".to_string();
                s.last_error = Some(error);
            }),
        }
    }

    fn reprobe_if_due(&mut self) {
        let llm_ready = self.applied.as_ref().is_some_and(|c| c.llm == "available");
        if llm_ready || !self.sidecar.is_installed() {
            return;
        }
        if self
            .last_probe
            .is_none_or(|at| at.elapsed() >= REPROBE_EVERY)
        {
            self.probe();
        }
    }

    /// Mirrors the sidecar's latest capabilities into `AiStatus`, and when the
    /// Foundation Model has just become available, re-queues the entries the
    /// fallback engine handled.
    fn apply_capabilities(&mut self) {
        let Some(caps) = self.sidecar.capabilities().cloned() else {
            return;
        };
        let was_ready = self.applied.as_ref().map(|c| c.llm == "available");
        let running = self.sidecar.is_running();
        update_status(&self.app, |s| {
            s.engine = if caps.llm == "available" {
                "full"
            } else if caps.embed {
                "fallback"
            } else {
                "rules"
            }
            .to_string();
            s.llm = caps.llm.clone();
            s.embed = caps.embed;
            s.os = Some(caps.os.clone());
            s.sidecar = if running { "running" } else { "stopped" }.to_string();
        });
        if was_ready == Some(false) && caps.llm == "available" {
            let now = now_epoch_ms();
            let requeued = self.write_database(|database| {
                AiRepository::new(database)
                    .requeue_fallback((now - BACKFILL_WINDOW_MS) as i64, now as i64)
                    .map_err(|error| error.to_string())
            });
            if matches!(requeued, Ok(n) if n > 0) {
                self.emit_entries_changed();
            }
        }
        self.applied = Some(caps);
    }

    fn refresh_counts(&self) {
        let queued = self
            .database()
            .ok()
            .and_then(|database| AiRepository::new(&database).queued_count().ok())
            .unwrap_or(0);
        let outcomes = self
            .database()
            .ok()
            .and_then(|database| {
                let repository = AiArtifactsRepository::new(&database);
                repository
                    .learning_since()
                    .ok()
                    .and_then(|since| repository.verdicts_since(since, now_epoch_ms()).ok())
            })
            .unwrap_or(0);
        let artifacts = self.database().ok();
        let repository = artifacts.as_ref().map(AiArtifactsRepository::new);
        let calibrated = repository
            .and_then(|repository| {
                repository
                    .active_artifact(
                        aevio_persistence::database::repositories::ai_artifacts::KIND_CALIBRATION,
                    )
                    .ok()
            })
            .flatten()
            .is_some();
        let personal = artifacts
            .as_ref()
            .and_then(|database| {
                AiArtifactsRepository::new(database)
                    .active_artifact(Field::Category.as_str())
                    .ok()
            })
            .flatten()
            .is_some();
        let running = self.sidecar.is_running();
        let installed = self.sidecar.is_installed();
        update_status(&self.app, |s| {
            s.queued = queued;
            s.outcomes = outcomes;
            s.calibrated = calibrated;
            s.personal_model = personal;
            s.sidecar = if !installed {
                "unavailable"
            } else if running {
                "running"
            } else {
                "stopped"
            }
            .to_string();
        });
    }

    /// Capabilities, starting the sidecar if needed. `None` means only T0.
    fn capabilities(&mut self) -> Option<Capabilities> {
        if !self.sidecar.is_installed() {
            return None;
        }
        match self.sidecar.probe() {
            Ok(caps) => {
                if self.applied.as_ref() != Some(&caps) || !self.sidecar.is_running() {
                    self.apply_capabilities();
                } else {
                    update_status(&self.app, |s| s.sidecar = "running".to_string());
                }
                Some(caps)
            }
            Err(error) => {
                update_status(&self.app, |s| {
                    s.engine = "rules".to_string();
                    s.sidecar = "unavailable".to_string();
                    s.last_error = Some(error);
                });
                None
            }
        }
    }

    // --- Jobs --------------------------------------------------------------

    fn run_job(&mut self, job: Job) {
        {
            let runtime = self.app.state::<AiRuntime>();
            runtime.set_busy(true);
            runtime.set_sidecar_pid(self.sidecar.pid());
        }

        let now = now_epoch_ms();
        if let Err(error) = self.write_database(|database| {
            AiRepository::new(database)
                .set_job_state(&job.id, "running", now as i64)
                .map_err(|error| error.to_string())
        }) {
            self.app.state::<AiRuntime>().set_busy(false);
            eprintln!("ai worker: {error}");
            return;
        }
        let result = if job.kind == JOB_CLASSIFY {
            self.classify(&job)
        } else {
            self.embed_only(&job)
        };
        {
            let runtime = self.app.state::<AiRuntime>();
            runtime.set_busy(false);
            runtime.set_sidecar_pid(self.sidecar.pid());
        }
        let now = now_epoch_ms();
        match result {
            Ok(()) => {
                if let Err(error) = self.write_database(|database| {
                    AiRepository::new(database)
                        .set_job_state(&job.id, "done", now as i64)
                        .map_err(|error| error.to_string())
                }) {
                    eprintln!("ai worker: {error}");
                }
            }
            Err(error) => {
                eprintln!("ai worker: {} {} failed: {error}", job.kind, job.entry_id);
                let gave_up = self
                    .write_database(|database| {
                        AiRepository::new(database)
                            .fail_job(&queue_job_input(&job), &error, now as i64)
                            .map_err(|error| error.to_string())
                    })
                    .unwrap_or(false);
                update_status(&self.app, |s| s.last_error = Some(error));
                if gave_up {
                    self.emit_entries_changed();
                    let _ = self
                        .app
                        .emit(EVENT_SUGGESTION_READY, json!({ "entryId": job.entry_id }));
                }
            }
        }
    }

    fn load_entry(&self, id: &str) -> Result<Option<TimeEntry>, String> {
        let database = self.database()?;
        AiWorkerRepository::new(&database)
            .entry(id)
            .map_err(|error| error.to_string())
    }

    fn load_segments(&self, entry: &TimeEntry) -> Result<Vec<ActivitySegment>, String> {
        let database = self.database()?;
        AiWorkerRepository::new(&database)
            .segments(entry)
            .map_err(|error| error.to_string())
    }

    fn load_categories(&self) -> Result<Vec<Choice>, String> {
        let database = self.database()?;
        AiWorkerRepository::new(&database)
            .categories()
            .map(|rows| {
                rows.into_iter()
                    .map(|row| Choice {
                        id: row.id,
                        name: row.name,
                        description: row.description,
                        hint: row.hint,
                    })
                    .collect()
            })
            .map_err(|error| error.to_string())
    }

    fn load_projects(&self) -> Result<Vec<Choice>, String> {
        let database = self.database()?;
        AiWorkerRepository::new(&database)
            .projects()
            .map(|rows| {
                rows.into_iter()
                    .map(|row| Choice {
                        id: row.id,
                        name: row.name,
                        description: row.description,
                        hint: row.hint,
                    })
                    .collect()
            })
            .map_err(|error| error.to_string())
    }

    fn previous_category(&self, entry: &TimeEntry) -> Option<(String, u64, u64)> {
        let database = self.database().ok()?;
        AiWorkerRepository::new(&database)
            .previous_category(entry, CONTEXT_GAP_MS)
            .ok()
            .flatten()
    }

    /// Embeds the entry's content (reusing a stored vector for identical
    /// text) and saves it to the kNN store.
    fn vector_for(
        &mut self,
        entry_id: &str,
        features: &EntryFeatures,
        caps: Option<&Capabilities>,
    ) -> Result<Option<Vec<f32>>, String> {
        let hash = text_hash(&features.content);
        let database = self.database()?;
        if let Some(vector) = AiRepository::new(&database)
            .embedding_for(entry_id, &hash)
            .map_err(|error| error.to_string())?
        {
            return Ok(Some(vector));
        }
        let Some(caps) = caps.filter(|c| c.embed) else {
            return Ok(None);
        };
        #[derive(Deserialize)]
        struct Embedded {
            vectors: Vec<Option<Vec<f32>>>,
        }
        let reply: Embedded = self.sidecar.call(
            "embed",
            json!({ "texts": [features.content] }),
            sidecar::TIMEOUT_QUICK,
        )?;
        let Some(vector) = reply.vectors.into_iter().next().flatten() else {
            return Ok(None);
        };
        let model = format!("nlembedding-en-r{}", caps.embed_revision);
        let now = now_epoch_ms();
        self.write_database(|database| {
            AiRepository::new(database)
                .upsert_embedding(
                    entry_id,
                    &vector,
                    &hash,
                    &features.content,
                    &model,
                    now as i64,
                )
                .map_err(|error| error.to_string())
        })?;
        Ok(Some(vector))
    }

    fn personal_predict(&mut self, field: Field, text: &str) -> Option<HashMap<Label, f64>> {
        let database = self.database().ok()?;
        let artifact = AiArtifactsRepository::new(&database)
            .active_artifact(field.as_str())
            .ok()??;
        #[derive(Deserialize)]
        struct Predicted {
            hypotheses: Vec<HashMap<String, f64>>,
        }
        let reply: Result<Predicted, String> = self.sidecar.call(
            "personal_predict",
            json!({ "model": artifact.path, "texts": [text] }),
            sidecar::TIMEOUT_QUICK,
        );
        match reply {
            Ok(reply) => reply.hypotheses.into_iter().next().map(|h| {
                h.into_iter()
                    .map(|(label, p)| (label_from_model(&label), p))
                    .collect()
            }),
            Err(error) => {
                eprintln!("ai worker: personal model: {error}");
                None
            }
        }
    }

    fn embed_only(&mut self, job: &Job) -> Result<(), String> {
        let Some(entry) = self.load_entry(&job.entry_id)? else {
            return Ok(());
        };
        if entry.deleted_at.is_some() {
            return Ok(());
        }
        let segments = self.load_segments(&entry)?;
        let mut features = features::extract(entry.started_at, entry.ended_at, &segments, None);
        if features.active_ms == 0 {
            // A hand-made entry with no captured activity: its description
            // is all there is to embed.
            features.content = entry.description.clone();
        }
        let caps = self.capabilities();
        if caps.as_ref().is_none_or(|c| !c.embed) {
            return Err("no embedding model available".to_string());
        }
        self.vector_for(&entry.id, &features, caps.as_ref())
            .map(|_| ())
    }

    fn classify(&mut self, job: &Job) -> Result<(), String> {
        let Some(entry) = self.load_entry(&job.entry_id)? else {
            return Ok(());
        };
        if entry.deleted_at.is_some() || entry.status == "approved" {
            return Ok(());
        }
        let settings = self.settings();
        let now = now_epoch_ms();

        let segments = self.load_segments(&entry)?;
        let categories = self.load_categories()?;
        let projects = self.load_projects()?;
        if categories.is_empty() {
            return Err("there are no categories to suggest".to_string());
        }
        let want_project =
            settings.ai_suggest == AiSuggest::CategoryProject && !projects.is_empty();

        let previous = self.previous_category(&entry);
        let features = features::extract(
            entry.started_at,
            entry.ended_at,
            &segments,
            previous
                .as_ref()
                .map(|(category, start, end)| PreviousEntry {
                    category,
                    started_at: *start,
                    ended_at: *end,
                }),
        );

        // T0
        let database = self.database()?;
        let ai_repository = AiRepository::new(&database);
        let rule_set = ai_repository
            .load_rules()
            .map_err(|error| error.to_string())?;
        let rule_category = rules::evaluate(&rule_set, &segments, Field::Category);
        let rule_project = want_project
            .then(|| rules::evaluate(&rule_set, &segments, Field::Project))
            .flatten();

        // T1
        let caps = self.capabilities();
        let vector = match self.vector_for(&entry.id, &features, caps.as_ref()) {
            Ok(vector) => vector,
            Err(error) => {
                eprintln!("ai worker: embedding failed: {error}");
                None
            }
        };
        let pool = ai_repository
            .labeled_pool(now.saturating_sub(knn::LOOKBACK_MS) as i64, &entry.id)
            .map_err(|error| error.to_string())?;
        let neighbors = vector
            .as_deref()
            .map(|v| knn::nearest(v, &pool, knn::K))
            .unwrap_or_default();
        let category_neighbors: Vec<(Label, f32)> = neighbors
            .iter()
            .filter(|n| n.item.category_id.is_some())
            .map(|n| (n.item.category_id.clone(), n.similarity))
            .collect();
        let project_neighbors: Vec<(Label, f32)> = neighbors
            .iter()
            .map(|n| (n.item.project_id.clone(), n.similarity))
            .collect();
        let personal_category = caps
            .is_some()
            .then(|| self.personal_predict(Field::Category, &features.content))
            .flatten();
        let personal_project = (caps.is_some() && want_project)
            .then(|| self.personal_predict(Field::Project, &features.content))
            .flatten();
        let labeled = ai_repository
            .labeled_count()
            .map_err(|error| error.to_string())?;
        let calibrator = AiArtifactsRepository::new(&database)
            .active_calibration(
                aevio_persistence::database::repositories::ai_artifacts::KIND_CALIBRATION,
            )
            .map_err(|error| error.to_string())?
            .and_then(|json| serde_json::from_str(&json).ok());

        // T2
        let t1_category = arbiter::t1_top(&category_neighbors, personal_category.as_ref(), labeled);
        let t1_project = arbiter::t1_top(&project_neighbors, personal_project.as_ref(), labeled);
        let mut llm_category: Option<Votes> = None;
        let mut llm_project: Option<Votes> = None;
        let mut description: Option<String> = None;
        if caps.as_ref().is_some_and(|c| c.llm == "available") {
            let request = LlmRequest {
                features: &features.text,
                categories: shortlist(&categories, MAX_LLM_CATEGORIES, &category_neighbors),
                projects: want_project
                    .then(|| shortlist(&projects, MAX_LLM_PROJECTS, &project_neighbors)),
                examples: few_shot(&neighbors, &categories, &projects, want_project),
                custom: &settings.ai_custom_prompt,
            };
            match self.llm(&request, 1, None) {
                Ok(first) => {
                    let (mut category_votes, mut project_votes, first_description) = first;
                    description = first_description;
                    // Adaptive sampling: one call normally, two more when the
                    // model disagrees with (or has nothing to check against)
                    // the fast tiers, so vote share can measure its certainty.
                    let disagrees = |votes: &Votes, t1: &Option<Label>| match t1 {
                        Some(t1) => votes.top() != Some(t1),
                        None => true,
                    };
                    let unsure = (rule_category.is_none()
                        && disagrees(&category_votes, &t1_category))
                        || (want_project
                            && rule_project.is_none()
                            && disagrees(&project_votes, &t1_project));
                    if unsure {
                        match self.llm(&request, 2, Some(0.9)) {
                            Ok((more_category, more_project, _)) => {
                                merge_votes(&mut category_votes, more_category);
                                merge_votes(&mut project_votes, more_project);
                            }
                            Err(error) => eprintln!("ai worker: extra samples failed: {error}"),
                        }
                    }
                    llm_category = Some(category_votes);
                    llm_project = want_project.then_some(project_votes);
                }
                Err(error) => {
                    eprintln!("ai worker: foundation model failed: {error}");
                    update_status(&self.app, |s| s.last_error = Some(error));
                }
            }
        }

        // Arbiter
        let names: HashMap<&str, &str> = categories
            .iter()
            .chain(projects.iter())
            .map(|c| (c.id.as_str(), c.name.as_str()))
            .collect();
        let name = |label: &Label| match label {
            Some(id) => names
                .get(id.as_str())
                .copied()
                .unwrap_or("Unknown")
                .to_string(),
            None => "No project".to_string(),
        };
        let category_decision = arbiter::decide(
            &Evidence {
                field: Field::Category,
                rule: rule_category.as_ref(),
                neighbors: category_neighbors,
                personal: personal_category,
                llm: llm_category,
                mentions: HashMap::new(),
                labeled,
                calibration: calibrator.as_ref(),
                dominant: features.dominant.as_ref(),
            },
            &name,
        );
        let project_decision = if want_project {
            arbiter::decide(
                &Evidence {
                    field: Field::Project,
                    rule: rule_project.as_ref(),
                    neighbors: project_neighbors,
                    personal: personal_project,
                    llm: llm_project,
                    mentions: mentions(&projects, &segments),
                    labeled,
                    calibration: calibrator.as_ref(),
                    dominant: features.dominant.as_ref(),
                },
                &name,
            )
        } else {
            None
        };

        let database = self.database().ok();
        let model_version = model_version(caps.as_ref(), database.as_ref());
        let outcome = Outcome {
            category: category_decision,
            project: project_decision,
            want_project,
            description,
            rule_category: rule_category.is_some(),
            rule_project: rule_project.is_some(),
        };
        let now = now_epoch_ms();
        self.write_database(|database| {
            persist(
                database,
                &entry.id,
                &outcome,
                features.dominant.as_ref(),
                &model_version,
                &settings,
                now,
            )
        })?;
        self.emit_entries_changed();
        let _ = self
            .app
            .emit(EVENT_SUGGESTION_READY, json!({ "entryId": entry.id }));
        Ok(())
    }

    fn llm(
        &mut self,
        request: &LlmRequest<'_>,
        samples: u32,
        temperature: Option<f64>,
    ) -> Result<(Votes, Votes, Option<String>), String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Classified {
            category_votes: HashMap<String, u32>,
            project_votes: Option<HashMap<String, u32>>,
            description: Option<String>,
            samples: u32,
        }
        let choice = |c: &&Choice| json!({ "id": c.id, "name": c.name, "description": c.description, "hint": c.hint });
        let mut params = json!({
            "features": request.features,
            "categories": request.categories.iter().map(choice).collect::<Vec<Value>>(),
            "examples": request.examples,
            "customInstructions": request.custom,
            "samples": samples,
        });
        if let Some(projects) = &request.projects {
            params["projects"] = Value::Array(projects.iter().map(choice).collect());
        }
        if let Some(temperature) = temperature {
            params["temperature"] = json!(temperature);
        }
        let reply: Classified = self
            .sidecar
            .call("classify", params, sidecar::TIMEOUT_CLASSIFY)?;
        let votes = |counts: HashMap<String, u32>| Votes {
            counts: counts
                .into_iter()
                .map(|(id, n)| (label_from_model(&id), n))
                .collect(),
            samples: reply.samples,
        };
        Ok((
            votes(reply.category_votes),
            votes(reply.project_votes.unwrap_or_default()),
            reply.description.filter(|d| !d.is_empty()),
        ))
    }

    // --- Personal model retraining -------------------------------------------

    /// Retrains when "Retrain now" was pressed, or when a model is due (20+
    /// new labels, or any new label a day after the last attempt) and the
    /// Mac is idle, on AC power, and not in Low Power Mode. Calibration is
    /// refit right after, since the raw scores it maps just changed.
    fn retrain_if_due(&mut self) {
        let requested = self.app.state::<AiRuntime>().take_retrain_request();
        if !requested
            && self
                .last_retrain_check
                .is_some_and(|at| at.elapsed() < RETRAIN_CHECK_EVERY)
        {
            return;
        }
        self.last_retrain_check = Some(Instant::now());
        if !self.sidecar.is_installed() {
            if requested {
                update_status(&self.app, |s| {
                    s.retrain = "idle".to_string();
                    s.retrain_note = Some("On-device ML isn't available in this build".to_string());
                });
            }
            return;
        }

        let now = now_epoch_ms();
        let due: Vec<Field> = [Field::Category, Field::Project]
            .into_iter()
            .filter(|field| requested || self.retrain_due(*field, now))
            .collect();
        if due.is_empty() {
            return;
        }
        if !requested {
            if let Some(why) = Conditions::read().blocker() {
                update_status(&self.app, |s| {
                    s.retrain_note = Some(format!("Retrain due, {why}"));
                });
                return;
            }
        }

        update_status(&self.app, |s| s.retrain = "running".to_string());
        {
            let runtime = self.app.state::<AiRuntime>();
            runtime.set_busy(true);
            runtime.set_sidecar_pid(self.sidecar.pid());
        }
        let mut notes = Vec::new();
        for field in due {
            let what = format!("{} model", field.as_str());
            match self.retrain(field) {
                Ok(Retrained::Skipped(why)) if requested || field == Field::Category => {
                    notes.push(format!("{}: {why}", capitalize(&what)))
                }
                Ok(Retrained::Skipped(_)) => {}
                Ok(Retrained::Swapped { accuracy }) => notes.push(match accuracy {
                    Some(accuracy) => format!(
                        "{} updated ({}% on the holdout)",
                        capitalize(&what),
                        (accuracy * 100.0).round()
                    ),
                    None => format!("{} updated", capitalize(&what)),
                }),
                Ok(Retrained::Kept { new, old }) => notes.push(format!(
                    "Kept the current {what}: the new one scored {}% vs {}% on the holdout",
                    (new * 100.0).round(),
                    (old * 100.0).round()
                )),
                Err(error) => {
                    eprintln!("ai worker: retraining the {what} failed: {error}");
                    notes.push(format!("Retraining the {what} failed: {error}"));
                }
            }
        }
        self.refit_calibration_if_due(true);
        let note = (!notes.is_empty()).then(|| notes.join(" · "));
        update_status(&self.app, |s| {
            s.retrain = "idle".to_string();
            s.retrain_note = note;
        });
        {
            let runtime = self.app.state::<AiRuntime>();
            runtime.set_busy(false);
            runtime.set_sidecar_pid(self.sidecar.pid());
        }
        self.refresh_counts();
    }

    fn retrain_due(&self, field: Field, now: u64) -> bool {
        let database = match self.database() {
            Ok(database) => database,
            Err(_) => return false,
        };
        let repository = AiArtifactsRepository::new(&database);
        let last = repository
            .last_trained_at(field.as_str())
            .ok()
            .flatten()
            .unwrap_or(0);
        let new = repository.labeled_since(last).unwrap_or(0);
        new >= RETRAIN_AFTER_LABELS
            || (new > 0 && now.saturating_sub(last) >= RETRAIN_AT_LEAST_EVERY_MS)
    }

    /// Trains a new personal classifier on the approved entries minus a 20%
    /// holdout and swaps it in only if it scores at least as well as the
    /// current one on that same holdout. Every attempt is versioned in
    /// `model_artifacts`.
    fn retrain(&mut self, field: Field) -> Result<Retrained, String> {
        let kind = field.as_str();
        let database = self.database()?;
        let repository = AiArtifactsRepository::new(&database);
        let epoch = repository
            .learning_since()
            .map_err(|error| error.to_string())?;
        let rows = repository
            .training_rows(field.as_str())
            .map_err(|error| error.to_string())?;
        let labels: HashSet<&str> = rows.iter().map(|(_, _, label)| label.as_str()).collect();
        if rows.len() < RETRAIN_AFTER_LABELS as usize {
            return Ok(Retrained::Skipped(format!(
                "needs {RETRAIN_AFTER_LABELS} reviewed entries, has {}",
                rows.len()
            )));
        }
        if labels.len() < 2 {
            let what = match field {
                Field::Category => "at least two categories",
                Field::Project => "at least two projects (No project counts as one)",
            };
            return Ok(Retrained::Skipped(format!(
                "needs reviewed entries in {what}"
            )));
        }
        let active = repository
            .active_artifact(kind)
            .map_err(|error| error.to_string())?;

        std::fs::create_dir_all(&self.ml_dir).map_err(|e| e.to_string())?;
        let dataset = self.ml_dir.join(format!("{kind}-train.jsonl"));
        let mut file = std::fs::File::create(&dataset).map_err(|e| e.to_string())?;
        let mut holdout: Vec<(&str, &str)> = Vec::new();
        for (entry_id, text, label) in &rows {
            let is_holdout = in_holdout(entry_id);
            if is_holdout {
                holdout.push((text, label));
            }
            let line = json!({ "text": text, "label": label, "holdout": is_holdout });
            writeln!(file, "{line}").map_err(|e| e.to_string())?;
        }
        drop(file);

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Trained {
            path: String,
            train_examples: u32,
            holdout_accuracy: Option<f64>,
        }
        let out = self
            .ml_dir
            .join(format!("{kind}-{}.mlmodelc", uuid::Uuid::now_v7()));
        let trained: Result<Trained, String> = self.sidecar.call(
            "train",
            json!({ "dataset": dataset, "out": out }),
            sidecar::TIMEOUT_TRAIN,
        );
        let _ = std::fs::remove_file(&dataset);
        let trained = trained?;

        let old_accuracy = match &active {
            Some(artifact) if !holdout.is_empty() => {
                self.holdout_accuracy(&artifact.path, &holdout)
            }
            _ => None,
        };
        let better = match (trained.holdout_accuracy, old_accuracy) {
            (Some(new), Some(old)) => new >= old,
            _ => true,
        };
        let now = now_epoch_ms();
        let recorded = self.write_database(|database| {
            // "Reset learned data" ran while this model trained on the old
            // data: drop it instead of bringing that data back.
            if AiArtifactsRepository::new(database)
                .learning_since()
                .map_err(|error| error.to_string())?
                != epoch
            {
                return Ok(false);
            }
            AiArtifactsRepository::new(database)
                .insert_artifact(
                    kind,
                    &trained.path,
                    trained.train_examples,
                    trained.holdout_accuracy,
                    better,
                    now,
                )
                .map_err(|error| error.to_string())?;
            Ok(true)
        })?;
        if !recorded {
            let _ = std::fs::remove_dir_all(&trained.path);
            return Ok(Retrained::Skipped("learned data was reset".to_string()));
        }
        let retired = if better {
            active.map(|a| a.path)
        } else {
            Some(trained.path)
        };
        if let Some(path) = retired {
            let _ = std::fs::remove_dir_all(path);
        }
        Ok(match (better, trained.holdout_accuracy, old_accuracy) {
            (false, Some(new), Some(old)) => Retrained::Kept { new, old },
            (_, accuracy, _) => Retrained::Swapped { accuracy },
        })
    }

    // --- Calibration -----------------------------------------------------------

    /// Refits the isotonic calibration curve once `CALIBRATION_REFIT_AFTER`
    /// new verdicts exist (or the first time `MIN_SAMPLES` do), checked every
    /// minute. `force` refits now, after the models changed. Fitting is pure
    /// Rust over at most a few thousand rows, so it needs no power gate.
    fn refit_calibration_if_due(&mut self, force: bool) {
        if !force
            && self
                .last_calibration_check
                .is_some_and(|at| at.elapsed() < CALIBRATION_CHECK_EVERY)
        {
            return;
        }
        self.last_calibration_check = Some(Instant::now());
        if let Err(error) = self.refit_calibration(force) {
            eprintln!("ai worker: calibration refit failed: {error}");
        }
    }

    fn refit_calibration(&mut self, force: bool) -> Result<(), String> {
        let now = now_epoch_ms();
        let database = self.database()?;
        let repository = AiArtifactsRepository::new(&database);
        let epoch = repository
            .learning_since()
            .map_err(|error| error.to_string())?;
        let fitted_at = repository
            .last_trained_at(
                aevio_persistence::database::repositories::ai_artifacts::KIND_CALIBRATION,
            )
            .map_err(|error| error.to_string())?;
        let due = force
            || match fitted_at {
                None => true,
                Some(at) => {
                    repository
                        .verdicts_since(at, now)
                        .map_err(|error| error.to_string())?
                        >= CALIBRATION_REFIT_AFTER
                }
            };
        if !due {
            return Ok(());
        }
        let samples = repository
            .calibration_samples(now)
            .map_err(|error| error.to_string())?;
        let Some(calibrator) = Calibrator::fit(&samples) else {
            return Ok(());
        };
        let saved = self.write_database(|database| {
            let repository = AiArtifactsRepository::new(database);
            if repository
                .learning_since()
                .map_err(|error| error.to_string())?
                != epoch
            {
                return Ok(false);
            }
            let serialized =
                serde_json::to_string(&calibrator).map_err(|error| error.to_string())?;
            repository
                .insert_calibration(&serialized, now)
                .map_err(|error| error.to_string())?;
            Ok(true)
        })?;
        if saved {
            update_status(&self.app, |s| s.calibrated = true);
        }
        Ok(())
    }

    fn holdout_accuracy(&mut self, model: &str, holdout: &[(&str, &str)]) -> Option<f64> {
        #[derive(Deserialize)]
        struct Predicted {
            hypotheses: Vec<HashMap<String, f64>>,
        }
        let texts: Vec<&str> = holdout.iter().map(|(text, _)| *text).collect();
        let reply: Predicted = self
            .sidecar
            .call(
                "personal_predict",
                json!({ "model": model, "texts": texts }),
                sidecar::TIMEOUT_TRAIN,
            )
            .ok()?;
        let correct = reply
            .hypotheses
            .iter()
            .zip(holdout)
            .filter(|(hypotheses, (_, label))| {
                hypotheses
                    .iter()
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .is_some_and(|(top, _)| top == label)
            })
            .count();
        Some(correct as f64 / holdout.len() as f64)
    }
}

struct LlmRequest<'a> {
    features: &'a str,
    categories: Vec<&'a Choice>,
    projects: Option<Vec<&'a Choice>>,
    examples: Vec<Value>,
    custom: &'a str,
}

struct Outcome {
    category: Option<Decision>,
    project: Option<Decision>,
    want_project: bool,
    description: Option<String>,
    rule_category: bool,
    rule_project: bool,
}

/// Saves suggestions and applies them to the entry, all or nothing.
///
/// - A field the user hasn't set is pre-filled when its suggestion is at
///   least `PREFILL_THRESHOLD`; one the user already set is left alone.
/// - The AI sentence replaces the builder's template description only.
/// - The entry auto-approves when auto-accept is on, every suggested field
///   clears the threshold, and the user hasn't touched the entry.
fn persist(
    database: &aevio_persistence::database::DatabaseConnection,
    entry_id: &str,
    outcome: &Outcome,
    dominant: Option<&crate::models::Dominant>,
    model_version: &str,
    settings: &Settings,
    now: u64,
) -> Result<(), String> {
    let repository = AiWorkerRepository::new(database);
    let Some(current) = repository
        .entry(entry_id)
        .map_err(|error| error.to_string())?
    else {
        return Ok(());
    };
    if current.deleted_at.is_some() {
        return Ok(());
    }
    let status = current.status;
    let category_id = current.category_id;
    let project_id = current.project_id;
    let description_origin = current.description_origin;
    if status == "approved" {
        return Ok(());
    }

    let threshold = f64::from(settings.auto_accept_percent) / 100.0;
    let clears =
        |decision: &Option<Decision>| decision.as_ref().is_some_and(|d| d.confidence >= threshold);
    let untouched = category_id.is_none() && project_id.is_none();
    let auto = status != "building"
        && settings.auto_accept
        && untouched
        && clears(&outcome.category)
        && (!outcome.want_project || clears(&outcome.project));
    let auto_outcome = auto.then_some("auto");

    let mut payload = serde_json::Map::new();
    let mut suggestions = Vec::new();
    let mut next_category = category_id.clone();
    let mut next_project = project_id.clone();
    if let Some(decision) = &outcome.category {
        suggestions.push(suggestion_input(
            Field::Category,
            decision,
            dominant,
            model_version,
            auto_outcome,
        )?);
        payload.insert("category".into(), json!(decision.confidence));
        if next_category.is_none() && decision.confidence >= PREFILL_THRESHOLD {
            next_category = decision.value.clone();
        }
    }
    if let Some(decision) = &outcome.project {
        suggestions.push(suggestion_input(
            Field::Project,
            decision,
            dominant,
            model_version,
            auto_outcome,
        )?);
        payload.insert("project".into(), json!(decision.confidence));
        if next_project.is_none() && decision.confidence >= PREFILL_THRESHOLD {
            next_project = decision.value.clone();
        }
    }

    let (next_status, approved_by) = if auto {
        let by_rules = outcome.rule_category && (!outcome.want_project || outcome.rule_project);
        ("approved", Some(if by_rules { "rule" } else { "auto" }))
    } else if status == "processing" {
        ("pending", None)
    } else {
        (status.as_str(), None)
    };
    let description = outcome
        .description
        .as_ref()
        .filter(|_| description_origin == "template");
    let actor = if outcome.rule_category { "rule" } else { "ai" };
    let mut events = Vec::new();
    if outcome.category.is_some() || outcome.project.is_some() {
        events.push(EventInput {
            kind: "suggested".to_owned(),
            actor: actor.to_owned(),
            payload: Some(Value::Object(payload).to_string()),
        });
    }
    if auto {
        events.push(EventInput {
            kind: "auto_approved".to_owned(),
            actor: actor.to_owned(),
            payload: None,
        });
    }
    repository
        .persist_classification(
            entry_id,
            ClassificationUpdate {
                category_id: next_category,
                project_id: next_project,
                status: next_status.to_owned(),
                approved_by: approved_by.map(str::to_owned),
                description: description.cloned(),
                suggestions,
                events,
                at: now as i64,
            },
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn suggestion_input(
    field: Field,
    decision: &Decision,
    dominant: Option<&crate::models::Dominant>,
    model_version: &str,
    outcome: Option<&str>,
) -> Result<SuggestionInput, String> {
    let alternatives = decision
        .alternatives
        .iter()
        .map(
            |(value_id, confidence)| aevio_core::entries::models::Alternative {
                value_id: value_id.clone(),
                confidence: *confidence,
            },
        )
        .collect::<Vec<_>>();
    Ok(SuggestionInput {
        field: field.as_str().to_owned(),
        value_id: decision.value.clone(),
        confidence: decision.confidence,
        raw_confidence: decision.raw_confidence,
        tier: decision.tier.to_owned(),
        signals: serde_json::json!({ "dominant": dominant, "items": decision.signals }).to_string(),
        rationale: why_text(&decision.signals),
        alternatives: serde_json::to_string(&alternatives).map_err(|error| error.to_string())?,
        engine: decision.engine.to_owned(),
        model_version: model_version.to_owned(),
        outcome: outcome.map(str::to_owned),
    })
}

/// With too many choices for the model's context, keep the fast tiers' top
/// picks (in list order), padded with the rest of the list.
fn shortlist<'a>(choices: &'a [Choice], max: usize, neighbors: &[(Label, f32)]) -> Vec<&'a Choice> {
    if choices.len() <= max {
        return choices.iter().collect();
    }
    let mut score: HashMap<&str, f32> = HashMap::new();
    for (label, similarity) in neighbors {
        if let Some(id) = label {
            *score.entry(id.as_str()).or_default() += similarity;
        }
    }
    let mut ranked: Vec<&Choice> = choices.iter().collect();
    ranked.sort_by(|a, b| {
        let a = score.get(a.id.as_str()).copied().unwrap_or(0.0);
        let b = score.get(b.id.as_str()).copied().unwrap_or(0.0);
        b.total_cmp(&a)
    });
    ranked.truncate(LLM_SHORTLIST);
    ranked
}

/// The closest approved entries, as few-shot examples for the model.
/// Corrections come first: an entry the user had to fix is exactly where the
/// model needs the hint.
fn few_shot(
    neighbors: &[knn::Neighbor<'_>],
    categories: &[Choice],
    projects: &[Choice],
    want_project: bool,
) -> Vec<Value> {
    let name = |choices: &[Choice], id: &Option<String>| {
        id.as_ref()
            .and_then(|id| choices.iter().find(|c| &c.id == id))
            .map(|c| c.name.clone())
    };
    let (corrected, rest): (Vec<_>, Vec<_>) = neighbors.iter().partition(|n| n.item.corrected);
    corrected
        .into_iter()
        .chain(rest)
        .filter_map(|n| {
            let category = name(categories, &n.item.category_id)?;
            let project = want_project
                .then(|| name(projects, &n.item.project_id))
                .flatten();
            Some(json!({ "features": n.item.features, "category": category, "project": project }))
        })
        .take(FEW_SHOT_EXAMPLES)
        .collect()
}

/// Share of active time whose title, URL, or domain names each project (its
/// name, or one of its comma- or line-separated AI hints).
fn mentions(projects: &[Choice], segments: &[ActivitySegment]) -> HashMap<Label, f64> {
    let mut active_ms = 0u64;
    let mut covered: HashMap<Label, u64> = HashMap::new();
    let needles: Vec<(&Choice, Vec<String>)> = projects
        .iter()
        .map(|project| {
            let mut words = vec![project.name.to_lowercase()];
            if let Some(hints) = &project.hint {
                words.extend(
                    hints
                        .split([',', '\n'])
                        .map(|w| w.trim().to_lowercase())
                        .filter(|w| !w.is_empty()),
                );
            }
            words.retain(|w| w.chars().count() >= 3);
            (project, words)
        })
        .collect();
    for segment in segments.iter().filter(|s| s.kind != activity::KIND_BREAK) {
        let ms = segment
            .ended_at
            .unwrap_or(segment.started_at)
            .saturating_sub(segment.started_at);
        active_ms += ms;
        let haystack = format!(
            "{} {} {}",
            segment.title,
            segment.url.as_deref().unwrap_or(""),
            segment.domain.as_deref().unwrap_or("")
        )
        .to_lowercase();
        for (project, words) in &needles {
            if words.iter().any(|w| haystack.contains(w.as_str())) {
                *covered.entry(Some(project.id.clone())).or_default() += ms;
            }
        }
    }
    if active_ms == 0 {
        return HashMap::new();
    }
    covered
        .into_iter()
        .map(|(label, ms)| (label, ms as f64 / active_ms as f64))
        .collect()
}

fn merge_votes(into: &mut Votes, more: Votes) {
    for (label, count) in more.counts {
        *into.counts.entry(label).or_default() += count;
    }
    into.samples += more.samples;
}

fn model_version(
    caps: Option<&Capabilities>,
    database: Option<&aevio_persistence::database::DatabaseConnection>,
) -> String {
    let artifact = |kind: &str| {
        database
            .and_then(|database| {
                AiArtifactsRepository::new(database)
                    .active_artifact(kind)
                    .ok()
            })
            .flatten()
            .map(|a| a.id)
            .unwrap_or_else(|| "none".to_string())
    };
    let calibration =
        artifact(aevio_persistence::database::repositories::ai_artifacts::KIND_CALIBRATION);
    match caps {
        Some(caps) => format!(
            "fm:{}:{} emb:r{} pm:{} cal:{calibration}",
            caps.os,
            caps.llm,
            caps.embed_revision,
            artifact(Field::Category.as_str())
        ),
        None => format!("rules-only cal:{calibration}"),
    }
}

/// Whether an entry belongs to the 20% holdout. Keyed on the entry id, so an
/// entry stays in (or out of) the holdout across retrains and the old and
/// new models are always compared on data neither trained on.
fn in_holdout(entry_id: &str) -> bool {
    fnv1a(entry_id).is_multiple_of(HOLDOUT_ONE_IN)
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// FNV-1a, hex. Stable across Rust releases, unlike `DefaultHasher`.
fn text_hash(text: &str) -> String {
    format!("{:016x}", fnv1a(text))
}

fn fnv1a(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::arbiter::Decision;

    fn db() -> (Connection, aevio_persistence::database::DatabaseConnection) {
        let path =
            std::env::temp_dir().join(format!("aevio-worker-test-{}.db", uuid::Uuid::now_v7()));
        let mut conn = Connection::open(&path).unwrap();
        aevio_persistence::bootstrap::migrations::run_migrations(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO time_entries (id, started_at, ended_at, description, status, source, created_at, updated_at)
             VALUES ('e1', 0, 1000, 'Xcode: main.rs', 'processing', 'auto', 0, 0);",
            [],
        )
        .unwrap();
        let database = aevio_persistence::database::connect(&path).unwrap();
        (conn, database)
    }

    fn decision(value: &str, confidence: f64) -> Decision {
        Decision {
            value: Some(value.to_string()),
            confidence,
            raw_confidence: confidence,
            alternatives: vec![],
            signals: vec![],
            engine: "full",
            tier: "model",
        }
    }

    fn entry(conn: &Connection) -> (String, Option<String>, Option<String>, String) {
        conn.query_row(
            "SELECT status, category_id, approved_by, description FROM time_entries WHERE id = 'e1';",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap()
    }

    fn outcome(category: f64) -> Outcome {
        Outcome {
            category: Some(decision("coding", category)),
            project: None,
            want_project: false,
            description: Some("Refactored the activity store".to_string()),
            rule_category: false,
            rule_project: false,
        }
    }

    #[test]
    fn a_confident_suggestion_auto_approves() {
        let (conn, database) = db();
        persist(
            &database,
            "e1",
            &outcome(0.97),
            None,
            "v",
            &Settings::default(),
            5,
        )
        .unwrap();
        let (status, category, by, description) = entry(&conn);
        assert_eq!(status, "approved");
        assert_eq!(category.as_deref(), Some("coding"));
        assert_eq!(by.as_deref(), Some("auto"));
        assert_eq!(description, "Refactored the activity store");
        let outcome: String = conn
            .query_row("SELECT outcome FROM suggestions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(outcome, "auto");
    }

    #[test]
    fn a_medium_suggestion_prefills_and_waits_for_review() {
        let (conn, database) = db();
        persist(
            &database,
            "e1",
            &outcome(0.7),
            None,
            "v",
            &Settings::default(),
            5,
        )
        .unwrap();
        let (status, category, by, _) = entry(&conn);
        assert_eq!(status, "pending");
        assert_eq!(category.as_deref(), Some("coding"));
        assert_eq!(by, None);
    }

    #[test]
    fn a_low_suggestion_preselects_nothing() {
        let (conn, database) = db();
        persist(
            &database,
            "e1",
            &outcome(0.4),
            None,
            "v",
            &Settings::default(),
            5,
        )
        .unwrap();
        let (status, category, _, _) = entry(&conn);
        assert_eq!(status, "pending");
        assert_eq!(category, None);
    }

    #[test]
    fn a_user_choice_is_never_overwritten() {
        let (conn, database) = db();
        conn.execute(
            "UPDATE time_entries SET category_id = 'design', description_origin = 'user' WHERE id = 'e1';",
            [],
        )
        .unwrap();
        persist(
            &database,
            "e1",
            &outcome(0.99),
            None,
            "v",
            &Settings::default(),
            5,
        )
        .unwrap();
        let (status, category, _, description) = entry(&conn);
        assert_eq!(status, "pending");
        assert_eq!(category.as_deref(), Some("design"));
        assert_eq!(description, "Xcode: main.rs");
    }

    #[test]
    fn mentions_measure_time_naming_a_project() {
        let projects = vec![Choice {
            id: "p1".into(),
            name: "Aevio".into(),
            description: None,
            hint: Some("aevio, tauri".into()),
        }];
        let seg = |title: &str, ms: u64| ActivitySegment {
            id: 0,
            app: "Xcode".into(),
            title: title.into(),
            kind: "activity".into(),
            label: None,
            started_at: 0,
            ended_at: Some(ms),
            reviewed: false,
            app_id: None,
            bundle_id: None,
            url: None,
            domain: None,
            entry_id: None,
        };
        let found = mentions(
            &projects,
            &[seg("activity.rs - Aevio", 750), seg("Inbox", 250)],
        );
        assert_eq!(found.get(&Some("p1".to_string())), Some(&0.75));
    }
}
