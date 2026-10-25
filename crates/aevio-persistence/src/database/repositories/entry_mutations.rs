use sea_orm::entity::prelude::*;
use sea_orm::{ActiveValue::Set, DatabaseConnection, DbErr, EntityTrait, QueryFilter};

use crate::activity::{APPROVED_OVERLAP, LOCKED};
use crate::database::entities::entry_query::entries;
use crate::database::repositories::ai::{AiRepository, JOB_CLASSIFY, JOB_EMBED};
use aevio_core::entries::models::{NewTimeEntry, UpdateTimeEntry};

pub struct EntryMutationRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> EntryMutationRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn ensure_no_approved_overlap(&self, start: u64, end: u64) -> Result<(), DbErr> {
        let overlap = entries::Entity::find()
            .filter(entries::Column::DeletedAt.is_null())
            .filter(entries::Column::Status.eq("approved"))
            .filter(entries::Column::StartedAt.lt(end as i64))
            .filter(entries::Column::EndedAt.gt(start as i64))
            .one(self.db)?
            .is_some();
        if overlap {
            return Err(DbErr::Custom(APPROVED_OVERLAP.to_owned()));
        }
        Ok(())
    }

    pub fn update(
        &self,
        id: &str,
        patch: UpdateTimeEntry,
        now: u64,
    ) -> Result<entries::Model, DbErr> {
        let row = self.current(id)?;
        if row.status == "approved" {
            return Err(DbErr::Custom(LOCKED.to_owned()));
        }
        let mut model: entries::ActiveModel = row.clone().into();
        let mut started_at = row.started_at;
        let mut ended_at = row.ended_at;
        let mut description = row.description.clone();
        let mut category_id = row.category_id.clone();
        let mut project_id = row.project_id.clone();
        let mut status = row.status.clone();
        let mut billable = row.billable;

        if let Some(value) = patch.description {
            description = value;
            model.description_origin = Set("user".to_owned());
        }
        if let Some(value) = patch.category_id {
            category_id = (!value.is_empty()).then_some(value);
        }
        if let Some(value) = patch.project_id {
            project_id = (!value.is_empty()).then_some(value);
        }
        if let Some(value) = patch.started_at {
            started_at = value as i64;
        }
        if let Some(value) = patch.ended_at {
            ended_at = value as i64;
        }
        if let Some(value) = patch.status {
            status = value;
        }
        if let Some(value) = patch.billable {
            billable = value as i64;
        }
        let was_live = row.status == "building";
        if was_live && (patch.started_at.is_some() || patch.ended_at.is_some()) {
            status = "pending".to_owned();
        }
        if ended_at <= started_at {
            return Err(DbErr::Custom("Entry end must be after its start".into()));
        }

        model.started_at = Set(started_at);
        model.ended_at = Set(ended_at);
        model.description = Set(description);
        model.category_id = Set(category_id);
        model.project_id = Set(project_id);
        model.status = Set(status);
        model.billable = Set(billable);
        model.updated_at = Set(now as i64);
        model.update(self.db)
    }

    pub fn create(
        &self,
        id: &str,
        input: &NewTimeEntry,
        now: u64,
    ) -> Result<entries::Model, DbErr> {
        if input.ended_at <= input.started_at {
            return Err(DbErr::Custom("Entry end must be after its start".into()));
        }
        let unclassified = input.description.trim().is_empty()
            && input.category_id.is_none()
            && input.project_id.is_none();
        if input.description.trim().is_empty() && !unclassified {
            return Err(DbErr::Custom("Entry description cannot be empty".into()));
        }
        self.ensure_no_approved_overlap(input.started_at, input.ended_at)?;
        let pending = unclassified || input.review;
        let status = if pending { "pending" } else { "approved" };
        let row = entries::ActiveModel {
            id: Set(id.to_owned()),
            started_at: Set(input.started_at as i64),
            ended_at: Set(input.ended_at as i64),
            description: Set(input.description.clone()),
            category_id: Set(input.category_id.clone()),
            project_id: Set(input.project_id.clone()),
            task_id: Set(None),
            status: Set(status.to_owned()),
            approved_by: Set((!pending).then_some("user".to_owned())),
            source: Set("manual".to_owned()),
            billable: Set(input.billable.unwrap_or(false) as i64),
            invoice_id: Set(None),
            created_at: Set(now as i64),
            updated_at: Set(now as i64),
            deleted_at: Set(None),
            description_origin: Set("user".to_owned()),
        }
        .insert(self.db)?;

        let ai = AiRepository::new(self.db);
        if unclassified && input.ended_at <= now {
            ai.enqueue(id, JOB_CLASSIFY, now as i64)?;
        } else if !pending {
            ai.enqueue(id, JOB_EMBED, now as i64)?;
        }
        Ok(row)
    }

    pub fn approve(
        &self,
        ids: &[String],
        approved_by: &str,
        now: u64,
    ) -> Result<Vec<entries::Model>, DbErr> {
        let ai = AiRepository::new(self.db);
        let mut result = Vec::with_capacity(ids.len());
        for id in ids {
            let row = self.current(id)?;
            if row.status == "approved" {
                result.push(row);
                continue;
            }
            let ended_at = if row.status == "building" {
                row.ended_at.max(now as i64)
            } else {
                row.ended_at
            };
            let mut model: entries::ActiveModel = row.clone().into();
            model.status = Set("approved".to_owned());
            model.approved_by = Set(Some(approved_by.to_owned()));
            model.ended_at = Set(ended_at);
            model.updated_at = Set(now as i64);
            let updated = model.update(self.db)?;
            ai.record_approval(
                id,
                updated.category_id.as_deref(),
                updated.project_id.as_deref(),
                now as i64,
            )?;
            if !ai.has_embedding(id)? {
                ai.enqueue(id, JOB_EMBED, now as i64)?;
            }
            result.push(updated);
        }
        Ok(result)
    }

    pub fn reject(&self, id: &str, now: u64) -> Result<entries::Model, DbErr> {
        let row = self.current(id)?;
        if row.status == "approved" {
            return Err(DbErr::Custom(LOCKED.to_owned()));
        }
        let rejected = AiRepository::new(self.db).record_rejection(id, now as i64)?;
        let prefilled_project = rejected.iter().any(|suggestion| {
            suggestion.field == aevio_core::ai::FIELD_PROJECT
                && suggestion.value_id.is_some()
                && suggestion.value_id == row.project_id
        });
        let was_building = row.status == "building";
        let mut model: entries::ActiveModel = row.into();
        model.category_id = Set(None);
        if prefilled_project {
            model.project_id = Set(None);
        }
        if !was_building {
            model.status = Set("pending".to_owned());
        }
        model.approved_by = Set(None);
        model.updated_at = Set(now as i64);
        model.update(self.db)
    }

    pub fn unapprove(&self, ids: &[String], now: u64) -> Result<Vec<entries::Model>, DbErr> {
        let mut result = Vec::with_capacity(ids.len());
        for id in ids {
            let row = self.current(id)?;
            if row.invoice_id.is_some() {
                return Err(DbErr::Custom(
                    "Invoiced entries cannot be unapproved".into(),
                ));
            }
            if row.status != "approved" {
                result.push(row);
                continue;
            }
            let mut model: entries::ActiveModel = row.into();
            model.status = Set("pending".to_owned());
            model.approved_by = Set(None);
            model.updated_at = Set(now as i64);
            result.push(model.update(self.db)?);
        }
        Ok(result)
    }

    fn current(&self, id: &str) -> Result<entries::Model, DbErr> {
        entries::Entity::find_by_id(id.to_owned())
            .one(self.db)?
            .ok_or_else(|| DbErr::RecordNotFound("time entry".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::migrations::run_migrations;
    use crate::database::connect;
    use rusqlite::Connection;

    fn database(name: &str) -> (std::path::PathBuf, DatabaseConnection) {
        let path = std::env::temp_dir().join(format!(
            "aevio-entry-mutations-{name}-{}.db",
            uuid::Uuid::now_v7()
        ));
        let mut sqlite = Connection::open(&path).unwrap();
        run_migrations(&mut sqlite).unwrap();
        drop(sqlite);
        let database = connect(&path).unwrap();
        (path, database)
    }

    #[test]
    fn creates_updates_and_approves_with_typed_models() {
        let (path, database) = database("round-trip");
        let repository = EntryMutationRepository::new(&database);
        let entry = repository
            .create(
                "entry-1",
                &NewTimeEntry {
                    started_at: 100,
                    ended_at: 200,
                    description: "Work".into(),
                    category_id: None,
                    project_id: None,
                    billable: Some(true),
                    review: true,
                },
                250,
            )
            .unwrap();
        assert_eq!(entry.status, "pending");
        let updated = repository
            .update(
                "entry-1",
                UpdateTimeEntry {
                    category_id: Some("category".into()),
                    ..Default::default()
                },
                300,
            )
            .unwrap();
        assert_eq!(updated.category_id.as_deref(), Some("category"));
        let approved = repository
            .approve(&["entry-1".into()], "snow", 400)
            .unwrap();
        assert_eq!(approved[0].status, "approved");
        assert_eq!(approved[0].approved_by.as_deref(), Some("snow"));
        drop(database);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn approved_entries_keep_the_lock_and_overlap_guard() {
        let (path, database) = database("locks");
        let repository = EntryMutationRepository::new(&database);
        repository
            .create(
                "entry-1",
                &NewTimeEntry {
                    started_at: 100,
                    ended_at: 200,
                    description: "Work".into(),
                    category_id: Some("category".into()),
                    project_id: None,
                    billable: None,
                    review: false,
                },
                250,
            )
            .unwrap();
        assert!(repository.ensure_no_approved_overlap(150, 160).is_err());
        assert!(
            repository
                .update("entry-1", UpdateTimeEntry::default(), 300)
                .unwrap_err()
                .to_string()
                .contains(LOCKED)
        );
        drop(database);
        std::fs::remove_file(path).unwrap();
    }
}
