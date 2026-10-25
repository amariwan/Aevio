use std::collections::HashMap;

use sea_orm::entity::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveValue::Set, DatabaseConnection, DatabaseTransaction, DbErr, QueryOrder, QuerySelect,
    TransactionError,
};

use crate::database::entities::{ai, ai_artifacts, settings};

pub const LEARNING_RESET_KEY: &str = "ai_learning_reset_at";
pub const KIND_CALIBRATION: &str = "calibration";
pub const AUTO_TRUSTED_AFTER_MS: u64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, PartialEq)]
pub struct Artifact {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactVersion {
    pub id: String,
    pub trained_at: u64,
    pub examples: u32,
    pub holdout_accuracy: Option<f64>,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricRow {
    pub entry_id: String,
    pub confidence: f64,
    pub raw_confidence: Option<f64>,
    pub engine: String,
    pub tier: Option<String>,
    pub outcome: Option<String>,
    pub updated_at: u64,
    pub status: String,
    pub approved_by: Option<String>,
}

pub struct AiArtifactsRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> AiArtifactsRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn active_artifact(&self, kind: &str) -> Result<Option<Artifact>, DbErr> {
        ai_artifacts::model_artifacts::Entity::find()
            .filter(ai_artifacts::model_artifacts::Column::Kind.eq(kind.to_owned()))
            .filter(ai_artifacts::model_artifacts::Column::Active.eq(1))
            .order_by_desc(ai_artifacts::model_artifacts::Column::TrainedAt)
            .one(self.db)
            .map(|row| {
                row.map(|row| Artifact {
                    id: row.id,
                    path: row.path,
                })
            })
    }

    pub fn history(&self, kind: &str, limit: u32) -> Result<Vec<ArtifactVersion>, DbErr> {
        ai_artifacts::model_artifacts::Entity::find()
            .filter(ai_artifacts::model_artifacts::Column::Kind.eq(kind.to_owned()))
            .order_by_desc(ai_artifacts::model_artifacts::Column::TrainedAt)
            .order_by_desc(ai_artifacts::model_artifacts::Column::Active)
            .limit(limit as u64)
            .all(self.db)
            .map(|rows| {
                rows.into_iter()
                    .map(|row| ArtifactVersion {
                        id: row.id,
                        trained_at: row.trained_at.max(0) as u64,
                        examples: row.n_examples.max(0) as u32,
                        holdout_accuracy: row.holdout_acc,
                        active: row.active != 0,
                    })
                    .collect()
            })
    }

    pub fn last_trained_at(&self, kind: &str) -> Result<Option<u64>, DbErr> {
        ai_artifacts::model_artifacts::Entity::find()
            .filter(ai_artifacts::model_artifacts::Column::Kind.eq(kind.to_owned()))
            .order_by_desc(ai_artifacts::model_artifacts::Column::TrainedAt)
            .one(self.db)
            .map(|row| row.map(|row| row.trained_at.max(0) as u64))
    }

    pub fn active_calibration(&self, kind: &str) -> Result<Option<String>, DbErr> {
        ai_artifacts::model_artifacts::Entity::find()
            .filter(ai_artifacts::model_artifacts::Column::Kind.eq(kind.to_owned()))
            .filter(ai_artifacts::model_artifacts::Column::Active.eq(1))
            .order_by_desc(ai_artifacts::model_artifacts::Column::TrainedAt)
            .one(self.db)
            .map(|row| row.and_then(|row| row.calibration))
    }

    pub fn latest_metric_rows(&self, since: u64) -> Result<Vec<MetricRow>, DbErr> {
        let suggestions = ai_artifacts::suggestions::Entity::find()
            .filter(ai_artifacts::suggestions::Column::DeletedAt.is_null())
            .filter(ai_artifacts::suggestions::Column::CreatedAt.gte(since as i64))
            .all(self.db)?;
        let mut latest = HashMap::<(String, String), i64>::new();
        for row in &suggestions {
            let key = (row.entry_id.clone(), row.field.clone());
            latest
                .entry(key)
                .and_modify(|at| *at = (*at).max(row.created_at))
                .or_insert(row.created_at);
        }
        let entries = ai_artifacts::entries::Entity::find()
            .filter(ai_artifacts::entries::Column::DeletedAt.is_null())
            .all(self.db)?
            .into_iter()
            .map(|row| (row.id.clone(), row))
            .collect::<HashMap<_, _>>();
        Ok(suggestions
            .into_iter()
            .filter(|row| {
                latest.get(&(row.entry_id.clone(), row.field.clone())) == Some(&row.created_at)
            })
            .filter_map(|row| {
                let entry = entries.get(&row.entry_id)?;
                Some(MetricRow {
                    entry_id: row.entry_id,
                    confidence: row.confidence,
                    raw_confidence: row.raw_confidence,
                    engine: row.engine,
                    tier: row.tier,
                    outcome: row.outcome,
                    updated_at: row.updated_at.max(0) as u64,
                    status: entry.status.clone(),
                    approved_by: entry.approved_by.clone(),
                })
            })
            .collect())
    }

    pub fn learning_since(&self) -> Result<u64, DbErr> {
        let value = settings::Entity::find_by_id(LEARNING_RESET_KEY.to_owned())
            .one(self.db)?
            .map(|row| row.value);
        Ok(value.and_then(|value| value.parse().ok()).unwrap_or(0))
    }

    pub fn training_rows(&self, field: &str) -> Result<Vec<(String, String, String)>, DbErr> {
        let entries = ai_artifacts::entries::Entity::find()
            .filter(ai_artifacts::entries::Column::DeletedAt.is_null())
            .filter(ai_artifacts::entries::Column::Status.eq("approved"))
            .filter(ai_artifacts::entries::Column::CategoryId.is_not_null())
            .all(self.db)?;
        let embeddings = ai_artifacts::embeddings::Entity::find()
            .all(self.db)?
            .into_iter()
            .map(|row| (row.entry_id, row.features))
            .collect::<HashMap<_, _>>();
        Ok(entries
            .into_iter()
            .filter_map(|entry| {
                let features = embeddings.get(&entry.id)?.clone();
                let label = if field == "project" {
                    entry.project_id.unwrap_or_else(|| "none".to_owned())
                } else {
                    entry.category_id?
                };
                Some((entry.id, features, label))
            })
            .collect())
    }

    pub fn labeled_since(&self, since: u64) -> Result<u32, DbErr> {
        let entries = ai_artifacts::entries::Entity::find()
            .filter(ai_artifacts::entries::Column::DeletedAt.is_null())
            .filter(ai_artifacts::entries::Column::Status.eq("approved"))
            .filter(ai_artifacts::entries::Column::CategoryId.is_not_null())
            .filter(ai_artifacts::entries::Column::UpdatedAt.gte(since as i64))
            .all(self.db)?;
        let ids = entries
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>();
        let count = ai_artifacts::embeddings::Entity::find()
            .filter(ai_artifacts::embeddings::Column::EntryId.is_in(ids))
            .count(self.db)?;
        Ok(count as u32)
    }

    pub fn verdicts_since(&self, after: u64, now: u64) -> Result<u32, DbErr> {
        let since = self.learning_since()?;
        let cutoff = now.saturating_sub(AUTO_TRUSTED_AFTER_MS) as i64;
        let entries = ai_artifacts::entries::Entity::find()
            .filter(ai_artifacts::entries::Column::DeletedAt.is_null())
            .select_only()
            .column(ai_artifacts::entries::Column::Id)
            .into_tuple::<String>()
            .all(self.db)?;
        let valid = entries
            .into_iter()
            .collect::<std::collections::HashSet<_>>();
        let suggestions = ai_artifacts::suggestions::Entity::find()
            .filter(ai_artifacts::suggestions::Column::DeletedAt.is_null())
            .filter(ai_artifacts::suggestions::Column::Engine.ne("rules"))
            .filter(ai_artifacts::suggestions::Column::RawConfidence.is_not_null())
            .filter(ai_artifacts::suggestions::Column::CreatedAt.gte(since as i64))
            .filter(ai_artifacts::suggestions::Column::UpdatedAt.gt(after as i64))
            .all(self.db)?;
        Ok(suggestions
            .into_iter()
            .filter(|row| valid.contains(&row.entry_id))
            .filter(|row| {
                matches!(
                    row.outcome.as_deref(),
                    Some("accepted" | "changed" | "rejected")
                ) || (row.outcome.as_deref() == Some("auto") && row.updated_at <= cutoff)
            })
            .count() as u32)
    }

    pub fn calibration_samples(&self, now: u64) -> Result<Vec<(f64, bool)>, DbErr> {
        let since = self.learning_since()? as i64;
        let cutoff = now.saturating_sub(AUTO_TRUSTED_AFTER_MS) as i64;
        let entries = ai_artifacts::entries::Entity::find()
            .filter(ai_artifacts::entries::Column::DeletedAt.is_null())
            .select_only()
            .column(ai_artifacts::entries::Column::Id)
            .into_tuple::<String>()
            .all(self.db)?
            .into_iter()
            .collect::<std::collections::HashSet<_>>();
        let rows = ai_artifacts::suggestions::Entity::find()
            .filter(ai_artifacts::suggestions::Column::DeletedAt.is_null())
            .filter(ai_artifacts::suggestions::Column::Engine.ne("rules"))
            .filter(ai_artifacts::suggestions::Column::RawConfidence.is_not_null())
            .filter(ai_artifacts::suggestions::Column::CreatedAt.gte(since))
            .order_by_desc(ai_artifacts::suggestions::Column::CreatedAt)
            .limit(2_000)
            .all(self.db)?;
        Ok(rows
            .into_iter()
            .filter(|row| entries.contains(&row.entry_id))
            .filter_map(|row| {
                let valid = matches!(
                    row.outcome.as_deref(),
                    Some("accepted" | "changed" | "rejected")
                ) || (row.outcome.as_deref() == Some("auto")
                    && row.updated_at <= cutoff);
                valid.then(|| {
                    (
                        row.raw_confidence.expect("filtered raw confidence"),
                        matches!(row.outcome.as_deref(), Some("accepted" | "auto")),
                    )
                })
            })
            .collect())
    }

    pub fn insert_calibration(&self, calibration: &str, now: u64) -> Result<(), DbErr> {
        self.db
            .transaction::<_, _, DbErr>(|transaction| {
                ai_artifacts::model_artifacts::Entity::update_many()
                    .col_expr(
                        ai_artifacts::model_artifacts::Column::Active,
                        Expr::value(0),
                    )
                    .filter(ai_artifacts::model_artifacts::Column::Kind.eq("calibration"))
                    .exec(transaction)?;
                ai_artifacts::model_artifacts::ActiveModel {
                    id: Set(uuid::Uuid::now_v7().to_string()),
                    kind: Set("calibration".to_owned()),
                    path: Set(String::new()),
                    trained_at: Set(now as i64),
                    n_examples: Set(0),
                    holdout_acc: Set(None),
                    calibration: Set(Some(calibration.to_owned())),
                    active: Set(1),
                }
                .insert(transaction)?;
                self.prune(transaction, "calibration")
            })
            .map_err(|error| match error {
                TransactionError::Connection(error) | TransactionError::Transaction(error) => error,
            })
    }

    pub fn insert_artifact(
        &self,
        kind: &str,
        path: &str,
        examples: u32,
        holdout_accuracy: Option<f64>,
        active: bool,
        now: u64,
    ) -> Result<(), DbErr> {
        self.db
            .transaction::<_, _, DbErr>(|transaction| {
                if active {
                    ai_artifacts::model_artifacts::Entity::update_many()
                        .col_expr(
                            ai_artifacts::model_artifacts::Column::Active,
                            Expr::value(0),
                        )
                        .filter(ai_artifacts::model_artifacts::Column::Kind.eq(kind.to_owned()))
                        .exec(transaction)?;
                }
                ai_artifacts::model_artifacts::ActiveModel {
                    id: Set(uuid::Uuid::now_v7().to_string()),
                    kind: Set(kind.to_owned()),
                    path: Set(path.to_owned()),
                    trained_at: Set(now as i64),
                    n_examples: Set(examples as i64),
                    holdout_acc: Set(holdout_accuracy),
                    calibration: Set(None),
                    active: Set(active as i64),
                }
                .insert(transaction)?;
                self.prune(transaction, kind)
            })
            .map_err(|error| match error {
                TransactionError::Connection(error) | TransactionError::Transaction(error) => error,
            })?;
        Ok(())
    }

    pub fn reset_learned(&self, now: u64) -> Result<Vec<String>, DbErr> {
        let paths = ai_artifacts::model_artifacts::Entity::find()
            .filter(ai_artifacts::model_artifacts::Column::Path.ne(""))
            .select_only()
            .column(ai_artifacts::model_artifacts::Column::Path)
            .into_tuple::<String>()
            .all(self.db)?;
        self.db
            .transaction::<_, _, DbErr>(|transaction| {
                ai_artifacts::model_artifacts::Entity::delete_many().exec(transaction)?;
                ai_artifacts::embeddings::Entity::delete_many().exec(transaction)?;
                ai::classify_jobs::Entity::delete_many()
                    .filter(ai::classify_jobs::Column::Kind.eq("embed"))
                    .filter(
                        ai::classify_jobs::Column::State
                            .is_in(["queued".to_owned(), "failed".to_owned()]),
                    )
                    .exec(transaction)?;
                settings::Entity::insert(settings::ActiveModel {
                    key: Set(LEARNING_RESET_KEY.to_owned()),
                    value: Set(now.to_string()),
                })
                .on_conflict(
                    OnConflict::column(settings::Column::Key)
                        .update_column(settings::Column::Value)
                        .to_owned(),
                )
                .exec(transaction)?;
                Ok(())
            })
            .map_err(|error| match error {
                TransactionError::Connection(error) | TransactionError::Transaction(error) => error,
            })?;
        Ok(paths)
    }

    fn prune(&self, db: &DatabaseTransaction, kind: &str) -> Result<(), DbErr> {
        let inactive = ai_artifacts::model_artifacts::Entity::find()
            .filter(ai_artifacts::model_artifacts::Column::Kind.eq(kind.to_owned()))
            .filter(ai_artifacts::model_artifacts::Column::Active.eq(0))
            .order_by_desc(ai_artifacts::model_artifacts::Column::TrainedAt)
            .all(db)?;
        for row in inactive.into_iter().skip(20) {
            ai_artifacts::model_artifacts::Entity::delete_by_id(row.id).exec(db)?;
        }
        Ok(())
    }
}
