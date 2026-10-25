use sea_orm::entity::prelude::*;
use sea_orm::sea_query::Condition;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, DatabaseConnection, DbErr, QueryFilter, QueryOrder,
};
use std::collections::HashMap;

use crate::database::entities::ai::{
    apps, classify_jobs, embeddings, projects, rules, suggestions, time_entries,
};
use aevio_core::ai::knn::Labeled;
use aevio_core::ai::rules::Rule;
use aevio_core::models::{Alternative, ClassifyJob, FieldSuggestion, Signal};
use aevio_core::models::{EntryAi, RuleSuggestion};

pub const JOB_CLASSIFY: &str = "classify";
pub const JOB_EMBED: &str = "embed";
pub const RULE_SUGGESTION_MIN: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub id: String,
    pub entry_id: String,
    pub kind: String,
    pub attempts: u32,
}

pub struct AiRepository<'db> {
    db: &'db DatabaseConnection,
}

pub struct RuleInput<'a> {
    pub match_kind: &'a str,
    pub pattern: &'a str,
    pub category_id: Option<&'a str>,
    pub project_id: Option<&'a str>,
    pub origin: &'a str,
    pub enabled: bool,
    pub now: i64,
}

impl<'db> AiRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn enqueue_unclassified(&self, start: i64, end: i64, now: i64) -> Result<usize, DbErr> {
        let entries = time_entries::Entity::find()
            .filter(time_entries::Column::DeletedAt.is_null())
            .filter(time_entries::Column::Status.eq("pending"))
            .filter(
                Condition::any()
                    .add(time_entries::Column::Source.eq("auto"))
                    .add(
                        Condition::all()
                            .add(time_entries::Column::Source.eq("manual"))
                            .add(time_entries::Column::Description.eq(""))
                            .add(time_entries::Column::EndedAt.lte(now)),
                    ),
            )
            .filter(time_entries::Column::EndedAt.gte(start))
            .filter(time_entries::Column::StartedAt.lte(end))
            .all(self.db)?;
        let suggestions = suggestions::Entity::find().all(self.db)?;
        let jobs = classify_jobs::Entity::find()
            .filter(classify_jobs::Column::Kind.eq(JOB_CLASSIFY))
            .all(self.db)?;
        let suggested: std::collections::HashSet<_> =
            suggestions.into_iter().map(|row| row.entry_id).collect();
        let classified: std::collections::HashSet<_> =
            jobs.into_iter().map(|row| row.entry_id).collect();

        let mut queued = 0;
        for entry in entries {
            if suggested.contains(&entry.id) || classified.contains(&entry.id) {
                continue;
            }
            self.enqueue(&entry.id, JOB_CLASSIFY, now)?;
            queued += 1;
        }
        Ok(queued)
    }

    pub fn enqueue(&self, entry_id: &str, kind: &str, now: i64) -> Result<(), DbErr> {
        if let Some(existing) = classify_jobs::Entity::find()
            .filter(classify_jobs::Column::EntryId.eq(entry_id.to_owned()))
            .filter(classify_jobs::Column::Kind.eq(kind.to_owned()))
            .one(self.db)?
        {
            let mut model: classify_jobs::ActiveModel = existing.into();
            model.state = Set("queued".to_owned());
            model.attempts = Set(0);
            model.next_at = Set(now);
            model.last_error = Set(None);
            model.created_at = Set(now);
            model.updated_at = Set(now);
            model.update(self.db)?;
        } else {
            classify_jobs::ActiveModel {
                id: Set(uuid::Uuid::now_v7().to_string()),
                entry_id: Set(entry_id.to_owned()),
                kind: Set(kind.to_owned()),
                state: Set("queued".to_owned()),
                attempts: Set(0),
                next_at: Set(now),
                last_error: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(self.db)?;
        }
        time_entries::Entity::update_many()
            .col_expr(time_entries::Column::Status, Expr::value("processing"))
            .col_expr(time_entries::Column::UpdatedAt, Expr::value(now))
            .filter(time_entries::Column::Id.eq(entry_id.to_owned()))
            .filter(time_entries::Column::Status.eq("pending"))
            .exec(self.db)?;
        Ok(())
    }

    pub fn requeue_interrupted(&self) -> Result<(), DbErr> {
        classify_jobs::Entity::update_many()
            .col_expr(classify_jobs::Column::State, Expr::value("queued"))
            .filter(classify_jobs::Column::State.eq("running"))
            .exec(self.db)
            .map(|_| ())
    }

    pub fn enqueue_live_due(&self, now: i64) -> Result<usize, DbErr> {
        const MINUTE: i64 = 60_000;
        let entries = time_entries::Entity::find()
            .filter(time_entries::Column::DeletedAt.is_null())
            .filter(time_entries::Column::Status.eq("building"))
            .filter(time_entries::Column::Source.eq("auto"))
            .filter(time_entries::Column::EndedAt.gte(now - 2 * MINUTE))
            .filter(time_entries::Column::StartedAt.lte(now - 30 * MINUTE))
            .all(self.db)?;
        let jobs = classify_jobs::Entity::find()
            .filter(classify_jobs::Column::Kind.eq(JOB_CLASSIFY))
            .all(self.db)?;
        let mut queued = 0;
        for entry in entries {
            let due = jobs
                .iter()
                .find(|job| job.entry_id == entry.id)
                .is_none_or(|job| {
                    job.state != "queued"
                        && job.state != "running"
                        && job.created_at
                            < entry.started_at
                                + 30 * MINUTE
                                + ((now - entry.started_at - 30 * MINUTE) / (15 * MINUTE))
                                    * 15
                                    * MINUTE
                });
            if due {
                self.enqueue(&entry.id, JOB_CLASSIFY, now)?;
                queued += 1;
            }
        }
        Ok(queued)
    }

    pub fn requeue_fallback(&self, since: i64, now: i64) -> Result<usize, DbErr> {
        let entries = time_entries::Entity::find()
            .filter(time_entries::Column::DeletedAt.is_null())
            .filter(time_entries::Column::Status.eq("pending"))
            .filter(time_entries::Column::StartedAt.gte(since))
            .all(self.db)?;
        let entry_ids: std::collections::HashSet<_> =
            entries.into_iter().map(|row| row.id).collect();
        let mut latest = std::collections::HashMap::<(String, String), i64>::new();
        let mut fallback = Vec::new();
        for suggestion in suggestions::Entity::find().all(self.db)? {
            if !entry_ids.contains(&suggestion.entry_id)
                || suggestion.engine != "fallback"
                || suggestion.outcome.is_some()
                || suggestion.created_at < since
                || suggestion.deleted_at.is_some()
            {
                continue;
            }
            let key = (suggestion.entry_id.clone(), suggestion.field.clone());
            let is_latest = latest
                .get(&key)
                .is_none_or(|created_at| *created_at <= suggestion.created_at);
            if is_latest {
                latest.insert(key.clone(), suggestion.created_at);
                fallback.push(key.0);
            }
        }
        fallback.sort();
        fallback.dedup();
        for entry_id in &fallback {
            self.enqueue(entry_id, JOB_CLASSIFY, now)?;
        }
        Ok(fallback.len())
    }

    pub fn next_due(&self, now: i64) -> Result<Option<Job>, DbErr> {
        Ok(classify_jobs::Entity::find()
            .filter(classify_jobs::Column::State.eq("queued"))
            .filter(classify_jobs::Column::NextAt.lte(now))
            .order_by_desc(classify_jobs::Column::CreatedAt)
            .one(self.db)?
            .map(|row| Job {
                id: row.id,
                entry_id: row.entry_id,
                kind: row.kind,
                attempts: row.attempts.max(0) as u32,
            }))
    }

    pub fn set_job_state(&self, id: &str, state: &str, now: i64) -> Result<(), DbErr> {
        classify_jobs::Entity::update_many()
            .col_expr(classify_jobs::Column::State, Expr::value(state))
            .col_expr(classify_jobs::Column::UpdatedAt, Expr::value(now))
            .filter(classify_jobs::Column::Id.eq(id.to_owned()))
            .exec(self.db)
            .map(|_| ())
    }

    pub fn fail_job(&self, job: &Job, error: &str, now: i64) -> Result<bool, DbErr> {
        let attempts = job.attempts + 1;
        let gave_up = attempts >= 3;
        let next_at = now + 30_000 * (1_i64 << attempts.min(6));
        classify_jobs::Entity::update_many()
            .col_expr(
                classify_jobs::Column::State,
                Expr::value(if gave_up { "failed" } else { "queued" }),
            )
            .col_expr(
                classify_jobs::Column::Attempts,
                Expr::value(attempts as i64),
            )
            .col_expr(classify_jobs::Column::NextAt, Expr::value(next_at))
            .col_expr(classify_jobs::Column::LastError, Expr::value(Some(error)))
            .col_expr(classify_jobs::Column::UpdatedAt, Expr::value(now))
            .filter(classify_jobs::Column::Id.eq(job.id.clone()))
            .exec(self.db)?;
        if gave_up && job.kind == JOB_CLASSIFY {
            time_entries::Entity::update_many()
                .col_expr(time_entries::Column::Status, Expr::value("pending"))
                .col_expr(time_entries::Column::UpdatedAt, Expr::value(now))
                .filter(time_entries::Column::Id.eq(job.entry_id.clone()))
                .filter(time_entries::Column::Status.eq("processing"))
                .exec(self.db)?;
        }
        Ok(gave_up)
    }

    pub fn queued_count(&self) -> Result<u32, DbErr> {
        let rows = classify_jobs::Entity::find()
            .filter(
                Condition::any()
                    .add(classify_jobs::Column::State.eq("queued"))
                    .add(classify_jobs::Column::State.eq("running")),
            )
            .all(self.db)?;
        Ok(rows.len() as u32)
    }

    pub fn embedding_for(
        &self,
        entry_id: &str,
        text_hash: &str,
    ) -> Result<Option<Vec<f32>>, DbErr> {
        embeddings::Entity::find_by_id(entry_id.to_owned())
            .filter(embeddings::Column::TextHash.eq(text_hash.to_owned()))
            .one(self.db)
            .map(|row| row.map(|row| aevio_core::ai::knn::decode(&row.vec)))
    }

    pub fn has_embedding(&self, entry_id: &str) -> Result<bool, DbErr> {
        embeddings::Entity::find_by_id(entry_id.to_owned())
            .one(self.db)
            .map(|row| row.is_some())
    }

    pub fn upsert_embedding(
        &self,
        entry_id: &str,
        vector: &[f32],
        text_hash: &str,
        features: &str,
        model: &str,
        now: i64,
    ) -> Result<(), DbErr> {
        if let Some(existing) = embeddings::Entity::find_by_id(entry_id.to_owned()).one(self.db)? {
            let mut row: embeddings::ActiveModel = existing.into();
            row.vec = Set(aevio_core::ai::knn::encode(vector));
            row.text_hash = Set(text_hash.to_owned());
            row.features = Set(features.to_owned());
            row.model = Set(model.to_owned());
            row.created_at = Set(now);
            row.update(self.db).map(|_| ())
        } else {
            embeddings::ActiveModel {
                entry_id: Set(entry_id.to_owned()),
                vec: Set(aevio_core::ai::knn::encode(vector)),
                text_hash: Set(text_hash.to_owned()),
                features: Set(features.to_owned()),
                model: Set(model.to_owned()),
                created_at: Set(now),
            }
            .insert(self.db)
            .map(|_| ())
        }
    }

    pub fn labeled_pool(&self, since: i64, exclude: &str) -> Result<Vec<Labeled>, DbErr> {
        let entries = time_entries::Entity::find()
            .filter(time_entries::Column::DeletedAt.is_null())
            .filter(time_entries::Column::Status.eq("approved"))
            .filter(time_entries::Column::StartedAt.gte(since))
            .filter(time_entries::Column::Id.ne(exclude.to_owned()))
            .all(self.db)?;
        let ids = entries.iter().map(|row| row.id.clone()).collect::<Vec<_>>();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let corrected = suggestions::Entity::find()
            .filter(suggestions::Column::EntryId.is_in(ids.clone()))
            .filter(suggestions::Column::Outcome.is_in(["changed", "rejected"]))
            .all(self.db)?
            .into_iter()
            .map(|row| row.entry_id)
            .collect::<std::collections::HashSet<_>>();
        let entry_map = entries
            .into_iter()
            .map(|row| (row.id.clone(), row))
            .collect::<std::collections::HashMap<_, _>>();
        Ok(embeddings::Entity::find()
            .filter(embeddings::Column::EntryId.is_in(ids))
            .all(self.db)?
            .into_iter()
            .filter_map(|row| {
                let entry = entry_map.get(&row.entry_id)?;
                Some(Labeled {
                    vector: aevio_core::ai::knn::decode(&row.vec),
                    category_id: entry.category_id.clone(),
                    project_id: entry.project_id.clone(),
                    features: row.features,
                    corrected: corrected.contains(&row.entry_id),
                })
            })
            .collect())
    }

    pub fn labeled_count(&self) -> Result<u32, DbErr> {
        let entries = time_entries::Entity::find()
            .filter(time_entries::Column::DeletedAt.is_null())
            .filter(time_entries::Column::Status.eq("approved"))
            .all(self.db)?;
        let ids = entries.into_iter().map(|row| row.id).collect::<Vec<_>>();
        if ids.is_empty() {
            return Ok(0);
        }
        embeddings::Entity::find()
            .filter(embeddings::Column::EntryId.is_in(ids))
            .count(self.db)
            .map(|count| count as u32)
    }

    pub fn load_rules(&self) -> Result<Vec<Rule>, DbErr> {
        let active_projects = projects::Entity::find()
            .filter(projects::Column::DeletedAt.is_null())
            .filter(projects::Column::Status.eq("active"))
            .all(self.db)?
            .into_iter()
            .map(|row| row.id)
            .collect::<std::collections::HashSet<_>>();
        let mut loaded = rules::Entity::find()
            .filter(rules::Column::DeletedAt.is_null())
            .filter(rules::Column::Enabled.eq(1))
            .all(self.db)?
            .into_iter()
            .filter(|row| {
                row.project_id
                    .as_ref()
                    .is_none_or(|id| active_projects.contains(id))
            })
            .map(|row| Rule {
                id: row.id,
                match_kind: row.match_kind,
                pattern: row.pattern,
                category_id: row.category_id,
                project_id: row.project_id,
                priority: row.priority,
                origin: row.origin,
            })
            .collect::<Vec<_>>();
        loaded.extend(
            apps::Entity::find()
                .filter(apps::Column::DeletedAt.is_null())
                .filter(apps::Column::Excluded.eq(0))
                .all(self.db)?
                .into_iter()
                .filter(|row| row.default_category_id.is_some() || row.default_project_id.is_some())
                .map(|row| Rule {
                    id: format!("app:{}", row.id),
                    match_kind: if row.kind == "site" { "domain" } else { "app" }.to_owned(),
                    pattern: row.identifier,
                    category_id: row.default_category_id,
                    project_id: row.default_project_id,
                    priority: -1,
                    origin: "app".to_owned(),
                }),
        );
        Ok(loaded)
    }

    pub fn summaries(&self, start: i64, end: i64) -> Result<HashMap<String, EntryAi>, DbErr> {
        let entries = time_entries::Entity::find()
            .filter(time_entries::Column::DeletedAt.is_null())
            .filter(time_entries::Column::EndedAt.gte(start))
            .filter(time_entries::Column::StartedAt.lte(end))
            .all(self.db)?;
        let jobs = classify_jobs::Entity::find()
            .filter(classify_jobs::Column::Kind.eq(JOB_CLASSIFY))
            .all(self.db)?
            .into_iter()
            .map(|job| (job.entry_id, job.state))
            .collect::<HashMap<_, _>>();
        let mut latest = HashMap::<(String, String), suggestions::Model>::new();
        for row in suggestions::Entity::find()
            .filter(suggestions::Column::DeletedAt.is_null())
            .all(self.db)?
        {
            let key = (row.entry_id.clone(), row.field.clone());
            if latest
                .get(&key)
                .is_none_or(|current| current.created_at <= row.created_at)
            {
                latest.insert(key, row);
            }
        }
        let mut result = HashMap::new();
        for entry in entries {
            let category_confidence = latest
                .get(&(entry.id.clone(), "category".to_owned()))
                .and_then(|row| {
                    (row.outcome.as_deref() != Some("rejected")).then_some(row.confidence)
                });
            let project_confidence = latest
                .get(&(entry.id.clone(), "project".to_owned()))
                .and_then(|row| {
                    (row.outcome.as_deref() != Some("rejected")).then_some(row.confidence)
                });
            let state = jobs.get(&entry.id).cloned();
            if state.is_some() || category_confidence.is_some() || project_confidence.is_some() {
                result.insert(
                    entry.id,
                    EntryAi {
                        state,
                        category_confidence,
                        project_confidence,
                    },
                );
            }
        }
        Ok(result)
    }

    pub fn job_for(&self, entry_id: &str) -> Result<Option<ClassifyJob>, DbErr> {
        classify_jobs::Entity::find()
            .filter(classify_jobs::Column::EntryId.eq(entry_id.to_owned()))
            .filter(classify_jobs::Column::Kind.eq(JOB_CLASSIFY))
            .one(self.db)
            .map(|row| {
                row.map(|row| ClassifyJob {
                    state: row.state,
                    attempts: row.attempts.max(0) as u32,
                    last_error: row.last_error,
                })
            })
    }

    pub fn latest_suggestions(&self, entry_id: &str) -> Result<Vec<FieldSuggestion>, DbErr> {
        let mut rows = suggestions::Entity::find()
            .filter(suggestions::Column::EntryId.eq(entry_id.to_owned()))
            .filter(suggestions::Column::DeletedAt.is_null())
            .all(self.db)?;
        let latest = rows
            .iter()
            .map(|row| (row.field.clone(), row.created_at))
            .fold(
                HashMap::<String, i64>::new(),
                |mut map, (field, created_at)| {
                    map.entry(field)
                        .and_modify(|current| *current = (*current).max(created_at))
                        .or_insert(created_at);
                    map
                },
            );
        rows.retain(|row| latest.get(&row.field) == Some(&row.created_at));
        rows.sort_by_key(|row| (row.field != "category", row.created_at));
        Ok(rows
            .into_iter()
            .map(|row| {
                let signals = row
                    .signals
                    .as_deref()
                    .and_then(|json| serde_json::from_str::<StoredSignals>(json).ok())
                    .unwrap_or_default();
                let alternatives = row
                    .alternatives
                    .as_deref()
                    .and_then(|json| serde_json::from_str::<Vec<Alternative>>(json).ok())
                    .unwrap_or_default();
                FieldSuggestion {
                    id: row.id,
                    entry_id: row.entry_id,
                    field: row.field,
                    value_id: row.value_id,
                    confidence: row.confidence,
                    signals: signals.items,
                    dominant: signals.dominant,
                    rationale: row.rationale.unwrap_or_default(),
                    alternatives,
                    engine: row.engine,
                    model_version: row.model_version,
                    outcome: row.outcome,
                    created_at: row.created_at.max(0) as u64,
                }
            })
            .collect())
    }

    pub fn rule_suggestion(
        &self,
        entry_id: &str,
        category_id: Option<&str>,
        project_id: Option<&str>,
        suggestions: &[FieldSuggestion],
    ) -> Result<Option<RuleSuggestion>, DbErr> {
        let rules = rules::Entity::find()
            .filter(rules::Column::DeletedAt.is_null())
            .all(self.db)?;
        let approved = time_entries::Entity::find()
            .filter(time_entries::Column::DeletedAt.is_null())
            .filter(time_entries::Column::Status.eq("approved"))
            .all(self.db)?;
        let stored = suggestions::Entity::find()
            .filter(suggestions::Column::Outcome.is_in(["changed", "rejected"]))
            .all(self.db)?;
        for suggestion in suggestions {
            let (field, current) = match suggestion.field.as_str() {
                "category" => ("category_id", category_id),
                "project" => ("project_id", project_id),
                _ => continue,
            };
            let (Some(current), Some(dominant)) = (current, suggestion.dominant.as_ref()) else {
                continue;
            };
            let corrected_here = suggestion.value_id.as_deref() != Some(current)
                && matches!(
                    suggestion.outcome.as_deref(),
                    None | Some("changed" | "rejected")
                );
            if !corrected_here
                || rules.iter().any(|rule| {
                    rule.deleted_at.is_none()
                        && rule.match_kind == dominant.kind
                        && rule.pattern.eq_ignore_ascii_case(&dominant.key)
                        && ((field == "category_id" && rule.category_id.is_some())
                            || (field == "project_id" && rule.project_id.is_some()))
                })
            {
                continue;
            }
            let corrections = approved
                .iter()
                .filter(|entry| entry.id != entry_id)
                .filter(|entry| {
                    if field == "category_id" {
                        entry.category_id.as_deref() == Some(current)
                    } else {
                        entry.project_id.as_deref() == Some(current)
                    }
                })
                .filter(|entry| {
                    stored.iter().any(|row| {
                        row.entry_id == entry.id
                            && row.field == suggestion.field
                            && row.value_id.as_deref() != Some(current)
                            && row
                                .signals
                                .as_deref()
                                .and_then(|json| serde_json::from_str::<StoredSignals>(json).ok())
                                .and_then(|signals| signals.dominant)
                                .is_some_and(|value| {
                                    value.kind == dominant.kind && value.key == dominant.key
                                })
                    })
                })
                .count() as u32
                + 1;
            if corrections >= RULE_SUGGESTION_MIN {
                return Ok(Some(RuleSuggestion {
                    field: suggestion.field.clone(),
                    match_kind: dominant.kind.clone(),
                    pattern: dominant.key.clone(),
                    label: dominant.label.clone(),
                    value_id: Some(current.to_owned()),
                    corrections,
                }));
            }
        }
        Ok(None)
    }

    pub fn record_approval(
        &self,
        entry_id: &str,
        category_id: Option<&str>,
        project_id: Option<&str>,
        now: i64,
    ) -> Result<(), DbErr> {
        for suggestion in self.latest_suggestions(entry_id)? {
            if suggestion.outcome.is_some() {
                continue;
            }
            let current = if suggestion.field == "category" {
                category_id
            } else {
                project_id
            };
            let outcome = if suggestion.value_id.as_deref() == current {
                "accepted"
            } else {
                "changed"
            };
            self.set_suggestion_outcome(&suggestion.id, outcome, now)?;
        }
        Ok(())
    }

    pub fn record_rejection(
        &self,
        entry_id: &str,
        now: i64,
    ) -> Result<Vec<FieldSuggestion>, DbErr> {
        let suggestions = self.latest_suggestions(entry_id)?;
        for suggestion in &suggestions {
            if suggestion.outcome.is_none() {
                self.set_suggestion_outcome(&suggestion.id, "rejected", now)?;
            }
        }
        Ok(suggestions
            .into_iter()
            .filter(|suggestion| suggestion.outcome.is_none())
            .collect())
    }

    pub fn record_edit(
        &self,
        entry_id: &str,
        category_id: Option<&str>,
        project_id: Option<&str>,
        now: i64,
    ) -> Result<(), DbErr> {
        for suggestion in self.latest_suggestions(entry_id)? {
            if !matches!(suggestion.outcome.as_deref(), Some("accepted" | "auto")) {
                continue;
            }
            let current = if suggestion.field == "category" {
                category_id
            } else {
                project_id
            };
            if suggestion.value_id.as_deref() != current {
                self.set_suggestion_outcome(&suggestion.id, "changed", now)?;
            }
        }
        Ok(())
    }

    pub fn create_rule(&self, input: RuleInput<'_>) -> Result<String, DbErr> {
        let id = uuid::Uuid::now_v7().to_string();
        rules::ActiveModel {
            id: Set(id.clone()),
            match_kind: Set(input.match_kind.to_owned()),
            pattern: Set(input.pattern.to_owned()),
            category_id: Set(input.category_id.map(str::to_owned)),
            project_id: Set(input.project_id.map(str::to_owned)),
            priority: Set(0),
            enabled: Set(input.enabled as i64),
            origin: Set(input.origin.to_owned()),
            created_at: Set(input.now),
            updated_at: Set(input.now),
            deleted_at: Set(None),
        }
        .insert(self.db)
        .map(|_| id)
    }

    fn set_suggestion_outcome(&self, id: &str, outcome: &str, now: i64) -> Result<(), DbErr> {
        suggestions::Entity::update_many()
            .col_expr(suggestions::Column::Outcome, Expr::value(outcome))
            .col_expr(suggestions::Column::UpdatedAt, Expr::value(now))
            .filter(suggestions::Column::Id.eq(id.to_owned()))
            .filter(suggestions::Column::Outcome.is_null())
            .exec(self.db)
            .map(|_| ())
    }
}

#[derive(Debug, Default, serde::Deserialize)]
struct StoredSignals {
    dominant: Option<aevio_core::models::Dominant>,
    #[serde(default)]
    items: Vec<Signal>,
}
