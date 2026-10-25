use sea_orm::entity::prelude::*;
use sea_orm::sea_query::Condition;
use sea_orm::{ActiveValue::Set, DatabaseConnection, DbErr, QueryOrder, TransactionError};

use crate::database::entities::ai_worker::{
    categories, entries, events, projects, segments, suggestions,
};
use aevio_core::entries::models::{ActivitySegment, TimeEntry};

#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceRecord {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub hint: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SuggestionInput {
    pub field: String,
    pub value_id: Option<String>,
    pub confidence: f64,
    pub raw_confidence: f64,
    pub tier: String,
    pub signals: String,
    pub rationale: String,
    pub alternatives: String,
    pub engine: String,
    pub model_version: String,
    pub outcome: Option<String>,
}

#[derive(Clone, Debug)]
pub struct EventInput {
    pub kind: String,
    pub actor: String,
    pub payload: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ClassificationUpdate {
    pub category_id: Option<String>,
    pub project_id: Option<String>,
    pub status: String,
    pub approved_by: Option<String>,
    pub description: Option<String>,
    pub suggestions: Vec<SuggestionInput>,
    pub events: Vec<EventInput>,
    pub at: i64,
}

pub struct AiWorkerRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> AiWorkerRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn entry(&self, id: &str) -> Result<Option<TimeEntry>, DbErr> {
        entries::Entity::find_by_id(id.to_owned())
            .one(self.db)
            .map(|row| row.map(entry_from_model))
    }

    pub fn segments(&self, entry: &TimeEntry) -> Result<Vec<ActivitySegment>, DbErr> {
        segments::Entity::find()
            .filter(
                Condition::any()
                    .add(segments::Column::EntryId.eq(entry.id.clone()))
                    .add(
                        Condition::all()
                            .add(segments::Column::EntryId.is_null())
                            .add(segments::Column::StartedAt.gte(entry.started_at as i64))
                            .add(segments::Column::EndedAt.lte(entry.ended_at as i64))
                            .add(segments::Column::Kind.ne("break")),
                    ),
            )
            .order_by_asc(segments::Column::StartedAt)
            .all(self.db)
            .map(|rows| rows.into_iter().map(segment_from_model).collect())
    }

    pub fn categories(&self) -> Result<Vec<ChoiceRecord>, DbErr> {
        categories::Entity::find()
            .filter(categories::Column::DeletedAt.is_null())
            .filter(categories::Column::Archived.eq(0))
            .order_by_asc(categories::Column::Sort)
            .order_by_asc(categories::Column::Name)
            .all(self.db)
            .map(|rows| {
                rows.into_iter()
                    .map(|row| ChoiceRecord {
                        id: row.id,
                        name: row.name,
                        description: row.description,
                        hint: row.ai_prompt,
                    })
                    .collect()
            })
    }

    pub fn projects(&self) -> Result<Vec<ChoiceRecord>, DbErr> {
        projects::Entity::find()
            .filter(projects::Column::DeletedAt.is_null())
            .filter(projects::Column::Status.eq("active"))
            .order_by_asc(projects::Column::Name)
            .all(self.db)
            .map(|rows| {
                rows.into_iter()
                    .map(|row| ChoiceRecord {
                        id: row.id,
                        name: row.name,
                        description: row.description,
                        hint: row.ai_hints,
                    })
                    .collect()
            })
    }

    pub fn previous_category(
        &self,
        entry: &TimeEntry,
        gap_ms: u64,
    ) -> Result<Option<(String, u64, u64)>, DbErr> {
        let previous = entries::Entity::find()
            .filter(entries::Column::DeletedAt.is_null())
            .filter(entries::Column::Id.ne(entry.id.clone()))
            .filter(entries::Column::EndedAt.lte(entry.started_at as i64 + 60_000))
            .filter(entries::Column::EndedAt.gte(entry.started_at.saturating_sub(gap_ms) as i64))
            .filter(entries::Column::CategoryId.is_not_null())
            .order_by_desc(entries::Column::EndedAt)
            .one(self.db)?;
        let Some(previous) = previous else {
            return Ok(None);
        };
        let category_id = previous.category_id.expect("filtered category id");
        let category = categories::Entity::find_by_id(category_id)
            .one(self.db)?
            .map(|category| category.name);
        Ok(category.map(|name| {
            (
                name,
                previous.started_at.max(0) as u64,
                previous.ended_at.max(0) as u64,
            )
        }))
    }

    pub fn persist_classification(
        &self,
        entry_id: &str,
        update: ClassificationUpdate,
    ) -> Result<bool, DbErr> {
        let result = self.db.transaction::<_, _, DbErr>(|transaction| {
            let Some(entry) = entries::Entity::find_by_id(entry_id.to_owned())
                .filter(entries::Column::DeletedAt.is_null())
                .one(transaction)?
            else {
                return Ok(false);
            };
            if entry.status == "approved" {
                return Ok(false);
            }
            for suggestion in update.suggestions {
                suggestions::ActiveModel {
                    id: Set(uuid::Uuid::now_v7().to_string()),
                    entry_id: Set(entry_id.to_owned()),
                    field: Set(suggestion.field),
                    value_id: Set(suggestion.value_id),
                    confidence: Set(suggestion.confidence),
                    raw_confidence: Set(Some(suggestion.raw_confidence)),
                    tier: Set(Some(suggestion.tier)),
                    signals: Set(Some(suggestion.signals)),
                    rationale: Set(Some(suggestion.rationale)),
                    alternatives: Set(Some(suggestion.alternatives)),
                    engine: Set(suggestion.engine),
                    model_version: Set(Some(suggestion.model_version)),
                    outcome: Set(suggestion.outcome),
                    created_at: Set(update.at),
                    updated_at: Set(update.at),
                    deleted_at: Set(None),
                }
                .insert(transaction)?;
            }
            let mut entry: entries::ActiveModel = entry.into();
            entry.category_id = Set(update.category_id);
            entry.project_id = Set(update.project_id);
            entry.status = Set(update.status);
            if update.approved_by.is_some() {
                entry.approved_by = Set(update.approved_by);
            }
            if let Some(description) = update.description {
                entry.description = Set(description);
                entry.description_origin = Set("ai".to_owned());
            }
            entry.updated_at = Set(update.at);
            entry.update(transaction)?;
            for event in update.events {
                events::ActiveModel {
                    id: Set(uuid::Uuid::now_v7().to_string()),
                    entry_id: Set(entry_id.to_owned()),
                    kind: Set(event.kind),
                    actor: Set(event.actor),
                    payload: Set(event.payload),
                    at: Set(update.at),
                }
                .insert(transaction)?;
            }
            Ok(true)
        });
        result.map_err(transaction_error)
    }
}

fn transaction_error(error: TransactionError<DbErr>) -> DbErr {
    match error {
        TransactionError::Connection(error) | TransactionError::Transaction(error) => error,
    }
}

fn entry_from_model(row: entries::Model) -> TimeEntry {
    TimeEntry {
        id: row.id,
        started_at: row.started_at.max(0) as u64,
        ended_at: row.ended_at.max(0) as u64,
        description: row.description,
        category_id: row.category_id,
        project_id: row.project_id,
        status: row.status,
        approved_by: row.approved_by,
        source: row.source,
        billable: row.billable != 0,
        invoice_id: row.invoice_id,
        created_at: row.created_at.max(0) as u64,
        updated_at: row.updated_at.max(0) as u64,
        deleted_at: row.deleted_at.map(|value| value.max(0) as u64),
        description_origin: row.description_origin,
        ai: None,
        dominant_app: None,
    }
}

fn segment_from_model(row: segments::Model) -> ActivitySegment {
    ActivitySegment {
        id: row.id,
        app: row.app,
        title: row.title,
        kind: row.kind,
        label: row.label,
        started_at: row.started_at.max(0) as u64,
        ended_at: row.ended_at.map(|value| value.max(0) as u64),
        reviewed: row.reviewed != 0,
        app_id: row.app_id,
        bundle_id: row.bundle_id,
        url: row.url,
        domain: row.domain,
        entry_id: row.entry_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::entities::ai_worker::{entries, events, suggestions};
    use crate::database::sea_orm::{ConnectionTrait, Database};

    fn entry(id: &str, status: &str) -> entries::ActiveModel {
        entries::ActiveModel {
            id: Set(id.to_owned()),
            started_at: Set(10),
            ended_at: Set(20),
            description: Set("template".to_owned()),
            category_id: Set(None),
            project_id: Set(None),
            status: Set(status.to_owned()),
            approved_by: Set(None),
            source: Set("auto".to_owned()),
            billable: Set(0),
            invoice_id: Set(None),
            created_at: Set(1),
            updated_at: Set(1),
            deleted_at: Set(None),
            description_origin: Set("template".to_owned()),
        }
    }

    #[test]
    fn classification_persists_suggestions_entry_and_events_atomically() {
        let db = Database::connect("sqlite::memory:").unwrap();
        db.execute_unprepared("CREATE TABLE time_entries (id TEXT PRIMARY KEY, started_at INTEGER NOT NULL, ended_at INTEGER NOT NULL, description TEXT NOT NULL, category_id TEXT, project_id TEXT, status TEXT NOT NULL, approved_by TEXT, source TEXT NOT NULL, billable INTEGER NOT NULL, invoice_id TEXT, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, deleted_at INTEGER, description_origin TEXT NOT NULL)").unwrap();
        db.execute_unprepared("CREATE TABLE suggestions (id TEXT PRIMARY KEY, entry_id TEXT NOT NULL, field TEXT NOT NULL, value_id TEXT, confidence REAL NOT NULL, raw_confidence REAL, tier TEXT, signals TEXT, rationale TEXT, alternatives TEXT, engine TEXT NOT NULL, model_version TEXT, outcome TEXT, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, deleted_at INTEGER)").unwrap();
        db.execute_unprepared("CREATE TABLE entry_events (id TEXT PRIMARY KEY, entry_id TEXT NOT NULL, kind TEXT NOT NULL, actor TEXT NOT NULL, payload TEXT, at INTEGER NOT NULL)").unwrap();
        entries::Entity::insert(entry("entry", "processing"))
            .exec(&db)
            .unwrap();
        AiWorkerRepository::new(&db)
            .persist_classification(
                "entry",
                ClassificationUpdate {
                    category_id: Some("category".into()),
                    project_id: None,
                    status: "approved".into(),
                    approved_by: Some("auto".into()),
                    description: Some("AI text".into()),
                    suggestions: vec![SuggestionInput {
                        field: "category".into(),
                        value_id: Some("category".into()),
                        confidence: 0.9,
                        raw_confidence: 0.8,
                        tier: "model".into(),
                        signals: "{}".into(),
                        rationale: "model".into(),
                        alternatives: "[]".into(),
                        engine: "full".into(),
                        model_version: "model".into(),
                        outcome: Some("auto".into()),
                    }],
                    events: vec![EventInput {
                        kind: "suggested".into(),
                        actor: "ai".into(),
                        payload: None,
                    }],
                    at: 30,
                },
            )
            .unwrap();
        let saved = entries::Entity::find_by_id("entry".to_owned())
            .one(&db)
            .unwrap()
            .unwrap();
        assert_eq!(saved.status, "approved");
        assert_eq!(saved.category_id.as_deref(), Some("category"));
        assert_eq!(saved.description, "AI text");
        assert_eq!(suggestions::Entity::find().all(&db).unwrap().len(), 1);
        assert_eq!(events::Entity::find().all(&db).unwrap().len(), 1);
    }
}
