use sea_orm::entity::prelude::*;
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    Condition, DatabaseConnection, DbErr, QueryFilter, QueryOrder, QuerySelect,
};

use crate::coach::brain::{BrainEntry, Routine, RoutineTask};
use crate::database::entities::catalog::categories;
use crate::database::entities::coach::{
    brain_entries, daily_tasks, focus, interventions, memory, preferences, routine_items, routines,
    task_time_links, tasks,
};
use crate::database::entities::entry_query::entries;
use crate::database::entities::entry_query::segments;
use crate::database::entities::tasks::tasks as project_tasks;
use aevio_core::coach::adaptive::Memory;
use aevio_core::coach::legacy::{CoachTask, NewCoachTask, UpdateCoachTask};
use aevio_core::coach::models::Preferences;
use aevio_core::coach::models::{PlannedTask, TimeEntry};
use aevio_core::coach::policies;

pub struct CoachTaskRepository<'db> {
    db: &'db DatabaseConnection,
}

pub struct CoachMetadataRepository<'db> {
    db: &'db DatabaseConnection,
}

pub struct CoachTimeRepository<'db> {
    db: &'db DatabaseConnection,
}

pub struct CoachBrainRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> CoachBrainRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn entries(&self) -> Result<Vec<BrainEntry>, String> {
        brain_entries::Entity::find()
            .order_by_desc(brain_entries::Column::UpdatedAt)
            .order_by_desc(brain_entries::Column::Id)
            .limit(500)
            .all(self.db)
            .map(|rows| {
                rows.into_iter()
                    .map(|row| BrainEntry {
                        id: row.id,
                        content: row.content,
                        category: row.category,
                        converted_task_id: row.converted_task_id,
                        updated_at: row.updated_at.max(0) as u64,
                    })
                    .collect()
            })
            .map_err(|error| error.to_string())
    }

    pub fn routines(&self) -> Result<Vec<Routine>, String> {
        let rows = routines::Entity::find()
            .order_by_asc(routines::Column::Title)
            .limit(100)
            .all(self.db)
            .map_err(|error| error.to_string())?;
        let ids = rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>();
        let items = routine_items::Entity::find()
            .filter(routine_items::Column::RoutineId.is_in(ids))
            .order_by_asc(routine_items::Column::Position)
            .all(self.db)
            .map_err(|error| error.to_string())?;
        Ok(rows
            .into_iter()
            .map(|row| Routine {
                id: row.id.clone(),
                title: row.title,
                tasks: items
                    .iter()
                    .filter(|item| item.routine_id == row.id)
                    .map(|item| RoutineTask {
                        title: item.title.clone(),
                        estimate_minutes: item.estimate_minutes.max(0) as u32,
                    })
                    .collect(),
            })
            .collect())
    }
}

impl<'db> CoachTimeRepository<'db> {
    pub fn current_focus(
        &self,
        now: i64,
    ) -> Result<Option<aevio_core::coach::focus::Focus>, String> {
        let row = focus::Entity::find()
            .filter(focus::Column::State.ne("ended"))
            .order_by_desc(focus::Column::StartedAt)
            .order_by_desc(focus::Column::Id)
            .one(self.db)
            .map_err(|error| error.to_string())?;
        let Some(row) = row else { return Ok(None) };
        let daily_task = daily_tasks::Entity::find_by_id(row.daily_task_id.clone())
            .one(self.db)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "Focus task is missing".to_owned())?;
        let title = if let Some(id) = daily_task.personal_task_id {
            tasks::Entity::find_by_id(id)
                .one(self.db)
                .map_err(|error| error.to_string())?
                .map(|task| task.title)
        } else if let Some(id) = daily_task.project_task_id {
            project_tasks::Entity::find_by_id(id)
                .one(self.db)
                .map_err(|error| error.to_string())?
                .map(|task| task.title)
        } else {
            None
        }
        .unwrap_or_default();
        let segment = segments::Entity::find_by_id(row.segment_id)
            .one(self.db)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "Focus segment is missing".to_owned())?;
        let elapsed_ms = (segment.ended_at.unwrap_or(now) - segment.started_at).max(0) as u64;
        Ok(Some(aevio_core::coach::focus::Focus {
            id: row.id,
            task_id: row.daily_task_id,
            segment_id: row.segment_id,
            title,
            goal: row.goal,
            next_step: row.next_step,
            planned_minutes: row.planned_minutes.max(0) as u32,
            body_doubling: row.body_doubling != 0,
            state: row.state,
            started_at: row.started_at.max(0) as u64,
            ended_at: row.ended_at.map(|value| value.max(0) as u64),
            elapsed_ms,
        }))
    }
}

impl<'db> CoachTimeRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn entries(
        &self,
        start: i64,
        end: i64,
        tasks: &[PlannedTask],
    ) -> Result<Vec<TimeEntry>, String> {
        let rows = entries::Entity::find()
            .filter(entries::Column::DeletedAt.is_null())
            .filter(entries::Column::StartedAt.lt(end))
            .filter(entries::Column::EndedAt.gt(start))
            .all(self.db)
            .map_err(|error| error.to_string())?;
        let links = task_time_links::Entity::find()
            .all(self.db)
            .map_err(|error| error.to_string())?;
        let category_ids = rows
            .iter()
            .filter_map(|row| row.category_id.clone())
            .collect::<Vec<_>>();
        let categories = categories::Entity::find()
            .filter(categories::Column::Id.is_in(category_ids))
            .all(self.db)
            .map_err(|error| error.to_string())?;
        let counts_as_work = categories
            .into_iter()
            .map(|row| (row.id, row.counts_as_work != 0))
            .collect::<std::collections::HashMap<_, _>>();
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                if row
                    .category_id
                    .as_ref()
                    .and_then(|id| counts_as_work.get(id))
                    .is_some_and(|counts| !counts)
                {
                    return None;
                }
                let duration = row.ended_at.min(end) - row.started_at.max(start);
                let assigned_to = links
                    .iter()
                    .find(|link| link.entry_id == row.id)
                    .map(|link| link.daily_task_id.clone())
                    .or_else(|| {
                        row.task_id.as_ref().and_then(|task_id| {
                            tasks.iter().find_map(|item| {
                                (item.task.source == "project" && item.task.id == *task_id)
                                    .then(|| item.id.clone())
                            })
                        })
                    });
                Some(TimeEntry {
                    id: row.id,
                    description: row.description,
                    duration_ms: duration.max(0) as u64,
                    assigned_to,
                })
            })
            .collect())
    }
}

impl<'db> CoachMetadataRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn preferences(&self) -> Result<Preferences, String> {
        let raw = preferences::Entity::find_by_id(1)
            .one(self.db)
            .map_err(|error| error.to_string())?
            .map(|row| row.value);
        raw.map(|value| {
            serde_json::from_str(&value)
                .map_err(|error| format!("Coach preferences are invalid: {error}"))
        })
        .unwrap_or_else(|| Ok(policies::defaults()))
    }

    pub fn save_preferences(&self, value: &Preferences) -> Result<(), String> {
        let raw = serde_json::to_string(value).map_err(|error| error.to_string())?;
        if let Some(row) = preferences::Entity::find_by_id(1)
            .one(self.db)
            .map_err(|error| error.to_string())?
        {
            let mut model: preferences::ActiveModel = row.into();
            model.value = Set(raw);
            model.update(self.db).map_err(|error| error.to_string())?;
        } else {
            preferences::ActiveModel {
                id: Set(1),
                value: Set(raw),
            }
            .insert(self.db)
            .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn last_intervention(&self) -> Result<Option<u64>, String> {
        interventions::Entity::find()
            .order_by_desc(interventions::Column::At)
            .one(self.db)
            .map(|row| row.and_then(|value| (value.at >= 0).then_some(value.at as u64)))
            .map_err(|error| error.to_string())
    }

    pub fn audit(&self, day: &str, kind: &str, now: u64) -> Result<(), String> {
        interventions::ActiveModel {
            id: NotSet,
            day: Set(day.to_owned()),
            kind: Set(kind.to_owned()),
            at: Set(now as i64),
        }
        .insert(self.db)
        .map_err(|error| error.to_string())?;
        interventions::Entity::delete_many()
            .filter(interventions::Column::At.lt(now.saturating_sub(90 * 86_400_000) as i64))
            .exec(self.db)
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn memory(&self) -> Result<Vec<Memory>, String> {
        memory::Entity::find()
            .order_by_desc(memory::Column::UpdatedAt)
            .order_by_asc(memory::Column::Id)
            .limit(200)
            .all(self.db)
            .map(|rows| {
                rows.into_iter()
                    .map(|row| Memory {
                        id: row.id,
                        content: row.content,
                        kind: row.kind,
                        updated_at: row.updated_at.max(0) as u64,
                    })
                    .collect()
            })
            .map_err(|error| error.to_string())
    }

    pub fn save_memory(&self, value: &Memory, now: u64) -> Result<(), String> {
        policies::title(&value.content)?;
        if !matches!(value.kind.as_str(), "explicit" | "confirmed") {
            return Err("Memory must be explicitly authored or confirmed".into());
        }
        let existing = self.memory()?;
        if !existing.iter().any(|item| item.id == value.id) && existing.len() >= 200 {
            return Err("Keep at most 200 memory notes; remove an older note first".into());
        }
        let id = if value.id.is_empty() {
            uuid::Uuid::now_v7().to_string()
        } else {
            value.id.clone()
        };
        if let Some(row) = memory::Entity::find_by_id(id.clone())
            .one(self.db)
            .map_err(|error| error.to_string())?
        {
            let mut model: memory::ActiveModel = row.into();
            model.content = Set(value.content.trim().to_owned());
            model.kind = Set(value.kind.clone());
            model.updated_at = Set(now as i64);
            model.update(self.db).map_err(|error| error.to_string())?;
        } else {
            memory::ActiveModel {
                id: Set(id),
                content: Set(value.content.trim().to_owned()),
                kind: Set(value.kind.clone()),
                updated_at: Set(now as i64),
            }
            .insert(self.db)
            .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn delete_memory(&self, id: Option<&str>) -> Result<(), String> {
        let mut query = memory::Entity::delete_many();
        if let Some(id) = id {
            query = query.filter(memory::Column::Id.eq(id.to_owned()));
        }
        query
            .exec(self.db)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub fn reset_interventions(&self) -> Result<(), String> {
        let condition = Condition::any()
            .add(interventions::Column::Kind.starts_with("suppress:"))
            .add(interventions::Column::Kind.starts_with("suppress_observation:"))
            .add(interventions::Column::Kind.starts_with("feedback:"))
            .add(interventions::Column::Kind.starts_with("applied:"));
        interventions::Entity::delete_many()
            .filter(condition)
            .exec(self.db)
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn suppressed(&self, prefix: &str) -> Result<Vec<String>, String> {
        interventions::Entity::find()
            .filter(interventions::Column::Kind.starts_with(prefix))
            .all(self.db)
            .map(|rows| {
                rows.into_iter()
                    .map(|row| row.kind.trim_start_matches(prefix).to_owned())
                    .collect()
            })
            .map_err(|error| error.to_string())
    }
}

impl<'db> CoachTaskRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn list(&self) -> Result<Vec<CoachTask>, DbErr> {
        tasks::Entity::find()
            .order_by_asc(tasks::Column::Status)
            .order_by_desc(tasks::Column::Priority)
            .order_by_asc(tasks::Column::DueAt)
            .order_by_asc(tasks::Column::CreatedAt)
            .all(self.db)
            .map(|rows| rows.into_iter().map(to_task).collect())
    }

    pub fn create(&self, input: NewCoachTask, now: i64) -> Result<CoachTask, String> {
        let title = input.title.trim();
        if title.is_empty() {
            return Err("A task needs a title".into());
        }
        let id = uuid::Uuid::now_v7().to_string();
        tasks::ActiveModel {
            id: Set(id.clone()),
            title: Set(title.to_owned()),
            notes: Set(input.notes),
            priority: Set(input.priority.unwrap_or(3).clamp(1, 5)),
            status: Set("open".into()),
            due_at: Set(input.due_at.map(|value| value as i64)),
            reminder_at: Set(input.reminder_at.map(|value| value as i64)),
            estimate_ms: Set(input.estimate_ms.map(|value| value as i64)),
            created_at: Set(now),
            updated_at: Set(now),
            completed_at: Set(None),
            notified_at: Set(None),
        }
        .insert(self.db)
        .map(to_task)
        .map_err(|error| error.to_string())
    }

    pub fn update(&self, id: &str, patch: UpdateCoachTask, now: i64) -> Result<CoachTask, String> {
        let current = tasks::Entity::find_by_id(id.to_owned())
            .one(self.db)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "Task not found".to_string())?;
        let title = patch
            .title
            .unwrap_or_else(|| current.title.clone())
            .trim()
            .to_owned();
        if title.is_empty() {
            return Err("A task needs a title".into());
        }
        let status = patch.status.unwrap_or_else(|| current.status.clone());
        if !matches!(status.as_str(), "open" | "done") {
            return Err(format!("unsupported task status: {status}"));
        }
        let reminder_changed = patch.reminder_at.is_some();
        let completed_at = (status == "done").then(|| current.completed_at.unwrap_or(now));
        let notes = patch.notes.unwrap_or(current.notes.clone());
        let priority = patch.priority.unwrap_or(current.priority).clamp(1, 5);
        let due_at = patch
            .due_at
            .unwrap_or(current.due_at.map(|value| value.max(0) as u64));
        let reminder_at = patch
            .reminder_at
            .unwrap_or(current.reminder_at.map(|value| value.max(0) as u64));
        let estimate_ms = patch
            .estimate_ms
            .unwrap_or(current.estimate_ms.map(|value| value.max(0) as u64));
        let mut model: tasks::ActiveModel = current.into();
        model.title = Set(title);
        model.notes = Set(notes);
        model.priority = Set(priority);
        model.status = Set(status);
        model.due_at = Set(due_at.map(|value| value as i64));
        model.reminder_at = Set(reminder_at.map(|value| value as i64));
        model.estimate_ms = Set(estimate_ms.map(|value| value as i64));
        model.updated_at = Set(now);
        model.completed_at = Set(completed_at);
        if reminder_changed {
            model.notified_at = Set(None);
        }
        model
            .update(self.db)
            .map(to_task)
            .map_err(|error| error.to_string())
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let result = tasks::Entity::delete_by_id(id.to_owned())
            .exec(self.db)
            .map_err(|error| error.to_string())?;
        (result.rows_affected == 1)
            .then_some(())
            .ok_or_else(|| "Task not found".into())
    }
}

fn to_task(row: tasks::Model) -> CoachTask {
    CoachTask {
        id: row.id,
        title: row.title,
        notes: row.notes,
        priority: row.priority,
        status: row.status,
        due_at: row.due_at.map(|value| value.max(0) as u64),
        reminder_at: row.reminder_at.map(|value| value.max(0) as u64),
        estimate_ms: row.estimate_ms.map(|value| value.max(0) as u64),
        created_at: row.created_at.max(0) as u64,
        updated_at: row.updated_at.max(0) as u64,
        completed_at: row.completed_at.map(|value| value.max(0) as u64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::sea_orm::{ConnectionTrait, Database, DbBackend, Schema};

    #[test]
    fn task_crud_preserves_nullable_fields_and_reminder_reset() {
        let database = Database::connect("sqlite::memory:").unwrap();
        let schema = Schema::new(DbBackend::Sqlite);
        database
            .execute(&schema.create_table_from_entity(tasks::Entity))
            .unwrap();
        let repository = CoachTaskRepository::new(&database);
        let created = repository
            .create(
                NewCoachTask {
                    title: "Ship migration".into(),
                    notes: None,
                    priority: Some(4),
                    due_at: None,
                    reminder_at: Some(10),
                    estimate_ms: None,
                },
                1,
            )
            .unwrap();
        let updated = repository
            .update(
                &created.id,
                UpdateCoachTask {
                    reminder_at: Some(Some(20)),
                    ..Default::default()
                },
                2,
            )
            .unwrap();
        assert_eq!(updated.reminder_at, Some(20));
        assert_eq!(repository.list().unwrap().len(), 1);
        repository.delete(&created.id).unwrap();
        assert!(repository.list().unwrap().is_empty());
    }
}
