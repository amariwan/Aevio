use std::collections::{HashMap, HashSet};

use sea_orm::entity::prelude::*;
use sea_orm::{DatabaseConnection, PaginatorTrait, QueryFilter, QueryOrder};

use crate::database::entities::catalog::projects;
use crate::database::entities::coach::{
    daily_plans, daily_reviews, daily_tasks, focus, interventions, task_steps, tasks,
};
use crate::database::entities::entry_query::segments;
use crate::database::entities::tasks::tasks as project_tasks;
use crate::database::repositories::coach::{CoachMetadataRepository, CoachTimeRepository};
use aevio_core::coach::dates;
use aevio_core::coach::models::{CatalogTask, Day, Plan, PlannedTask, Review, Step, TaskRef};

pub struct CoachDayRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> CoachDayRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn day(&self, date: &str, requested_zone: &str) -> Result<Day, String> {
        let plan = self.plan(date)?;
        let zone = plan
            .as_ref()
            .map(|value| value.time_zone.as_str())
            .unwrap_or(requested_zone);
        let (start, end) = dates::bounds(date, zone)?;
        let tasks = self.planned_tasks(date, false)?;
        let carryover = self.planned_tasks(date, true)?;
        let entries = CoachTimeRepository::new(self.db).entries(start, end, &tasks)?;
        let focus_ms = self.focus_allocations(date, start, end, &entries, &tasks)?;
        let tracked_ms = entries.iter().map(|entry| entry.duration_ms).sum();
        let unassigned_ms = entries
            .iter()
            .filter(|entry| {
                !tasks
                    .iter()
                    .any(|task| Some(&task.id) == entry.assigned_to.as_ref())
            })
            .map(|entry| entry.duration_ms)
            .sum::<u64>()
            .saturating_sub(focus_ms);
        let planned_minutes = tasks.iter().map(|task| task.estimate_minutes).sum();
        let available = plan
            .as_ref()
            .map(|value| value.available_minutes)
            .unwrap_or(0);
        Ok(Day {
            day: date.to_owned(),
            time_zone: zone.to_owned(),
            plan,
            review: self.review(date)?,
            tasks,
            catalog: self.catalog()?,
            carryover,
            preferences: CoachMetadataRepository::new(self.db).preferences()?,
            tracked_ms,
            unassigned_ms,
            planned_minutes,
            overbooked_minutes: planned_minutes.saturating_sub(available),
            changes: self.changes(date)?,
            time_entries: entries,
        })
    }

    fn plan(&self, date: &str) -> Result<Option<Plan>, String> {
        daily_plans::Entity::find_by_id(date.to_owned())
            .one(self.db)
            .map_err(|error| error.to_string())
            .map(|row| {
                row.map(|value| Plan {
                    day: value.day,
                    time_zone: value.time_zone,
                    goal: value.goal,
                    notes: value.notes,
                    available_minutes: value.available_minutes.max(0) as u32,
                    energy: value.energy.map(|item| item.clamp(1, 5) as u8),
                    state: value.state,
                    revision: value.revision.max(0) as u32,
                })
            })
    }

    fn review(&self, date: &str) -> Result<Review, String> {
        daily_reviews::Entity::find_by_id(date.to_owned())
            .one(self.db)
            .map_err(|error| error.to_string())
            .map(|row| {
                row.map(|value| Review {
                    outcome: value.outcome,
                    worked: value.worked,
                    obstacles: value.obstacles,
                    tomorrow: value.tomorrow,
                    revision: value.revision.max(0) as u32,
                })
                .unwrap_or_default()
            })
    }

    fn changes(&self, date: &str) -> Result<u32, String> {
        interventions::Entity::find()
            .filter(interventions::Column::Day.eq(date.to_owned()))
            .filter(interventions::Column::Kind.is_in([
                "plan.changed",
                "task.moved",
                "task.added",
                "task.removed",
                "tasks.reordered",
            ]))
            .count(self.db)
            .map(|count| count as u32)
            .map_err(|error| error.to_string())
    }

    fn catalog(&self) -> Result<Vec<CatalogTask>, String> {
        let personal = tasks::Entity::find()
            .order_by_desc(tasks::Column::Priority)
            .order_by_asc(tasks::Column::Title)
            .all(self.db)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|task| CatalogTask {
                task: TaskRef {
                    source: "personal".into(),
                    id: task.id,
                },
                title: task.title,
                notes: task.notes.unwrap_or_default(),
                done: task.status == "done",
                priority: task.priority,
                estimate_minutes: task.estimate_ms.map(|value| (value.max(0) / 60_000) as u32),
                reminder_at: task.reminder_at.map(|value| value.max(0) as u64),
            })
            .collect::<Vec<_>>();
        let active_projects = projects::Entity::find()
            .filter(projects::Column::DeletedAt.is_null())
            .all(self.db)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|project| project.id)
            .collect::<HashSet<_>>();
        let mut result = personal;
        result.extend(
            project_tasks::Entity::find()
                .filter(project_tasks::Column::DeletedAt.is_null())
                .all(self.db)
                .map_err(|error| error.to_string())?
                .into_iter()
                .filter(|task| active_projects.contains(&task.project_id))
                .map(|task| CatalogTask {
                    task: TaskRef {
                        source: "project".into(),
                        id: task.id,
                    },
                    title: task.title,
                    notes: task.description.unwrap_or_default(),
                    done: task.status == "done",
                    priority: project_priority(&task.priority),
                    estimate_minutes: None,
                    reminder_at: None,
                }),
        );
        result.sort_by_key(|task| (-task.priority, task.title.to_lowercase()));
        Ok(result)
    }

    fn planned_tasks(&self, date: &str, carryover: bool) -> Result<Vec<PlannedTask>, String> {
        let rows = daily_tasks::Entity::find()
            .order_by_desc(daily_tasks::Column::Day)
            .order_by_asc(daily_tasks::Column::Position)
            .order_by_asc(daily_tasks::Column::Id)
            .all(self.db)
            .map_err(|error| error.to_string())?;
        let personal = tasks::Entity::find()
            .all(self.db)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|task| (task.id.clone(), task))
            .collect::<HashMap<_, _>>();
        let project = project_tasks::Entity::find()
            .all(self.db)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|task| (task.id.clone(), task))
            .collect::<HashMap<_, _>>();
        let today_ids = rows
            .iter()
            .filter(|row| row.day == date)
            .filter_map(|row| {
                row.personal_task_id
                    .as_ref()
                    .or(row.project_task_id.as_ref())
            })
            .cloned()
            .collect::<HashSet<_>>();
        let steps = task_steps::Entity::find()
            .order_by_asc(task_steps::Column::Position)
            .all(self.db)
            .map_err(|error| error.to_string())?;
        Ok(rows
            .into_iter()
            .filter(|row| {
                if carryover {
                    row.day.as_str() < date
                        && !today_ids.contains(
                            row.personal_task_id
                                .as_ref()
                                .or(row.project_task_id.as_ref())
                                .expect("daily task has a source"),
                        )
                } else {
                    row.day == date
                }
            })
            .filter_map(|row| {
                let (source, task_id, title, notes, done, priority, reminder_at) =
                    if let Some(id) = row.personal_task_id.as_ref() {
                        let task = personal.get(id)?;
                        (
                            "personal",
                            id.clone(),
                            task.title.clone(),
                            task.notes.clone().unwrap_or_default(),
                            task.status == "done",
                            task.priority,
                            task.reminder_at.map(|value| value.max(0) as u64),
                        )
                    } else {
                        let id = row.project_task_id.as_ref()?;
                        let task = project.get(id)?;
                        (
                            "project",
                            id.clone(),
                            task.title.clone(),
                            task.description.clone().unwrap_or_default(),
                            task.status == "done",
                            project_priority(&task.priority),
                            None,
                        )
                    };
                if carryover && done {
                    return None;
                }
                Some(PlannedTask {
                    id: row.id.clone(),
                    task: TaskRef {
                        source: source.into(),
                        id: task_id,
                    },
                    title,
                    notes,
                    done,
                    priority,
                    estimate_minutes: row.estimate_minutes.max(0) as u32,
                    important: row.important != 0,
                    position: row.position,
                    steps: steps
                        .iter()
                        .filter(|step| step.daily_task_id == row.id)
                        .map(|step| Step {
                            title: step.title.clone(),
                            done: step.done != 0,
                        })
                        .collect(),
                    actual_ms: 0,
                    postponements: row.postponements.max(0) as u32,
                    reminder_at,
                })
            })
            .collect())
    }

    fn focus_allocations(
        &self,
        date: &str,
        start: i64,
        end: i64,
        entries: &[aevio_core::coach::models::TimeEntry],
        tasks: &[PlannedTask],
    ) -> Result<u64, String> {
        let task_ids = tasks
            .iter()
            .map(|task| task.id.clone())
            .collect::<HashSet<_>>();
        let focus_rows = focus::Entity::find()
            .all(self.db)
            .map_err(|error| error.to_string())?;
        let daily = daily_tasks::Entity::find()
            .all(self.db)
            .map_err(|error| error.to_string())?;
        let daily_days = daily
            .into_iter()
            .map(|task| (task.id, task.day))
            .collect::<HashMap<_, _>>();
        let segment_rows = segments::Entity::find()
            .all(self.db)
            .map_err(|error| error.to_string())?;
        let segment_by_id = segment_rows
            .into_iter()
            .map(|segment| (segment.id, segment))
            .collect::<HashMap<_, _>>();
        let entry_ids = entries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<HashSet<_>>();
        Ok(focus_rows
            .into_iter()
            .filter(|row| {
                daily_days
                    .get(&row.daily_task_id)
                    .is_some_and(|day| day == date)
            })
            .filter_map(|row| {
                if !task_ids.contains(&row.daily_task_id) {
                    return None;
                }
                let segment = segment_by_id.get(&row.segment_id)?;
                let entry_id = segment.entry_id.as_ref()?;
                if !entry_ids.contains(entry_id.as_str()) {
                    return None;
                }
                let slice_start = segment.started_at.max(start);
                let slice_end = segment.ended_at.unwrap_or(end).min(end);
                Some(slice_end.saturating_sub(slice_start) as u64)
            })
            .sum())
    }
}

fn project_priority(priority: &str) -> i64 {
    match priority {
        "urgent" => 5,
        "high" => 4,
        "low" => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::migrations::run_migrations;
    use crate::database::connect;
    use rusqlite::Connection;

    #[test]
    fn day_projection_preserves_nullable_task_fields_and_counts_time() {
        let path =
            std::env::temp_dir().join(format!("aevio-coach-day-{}.db", uuid::Uuid::now_v7()));
        let mut sqlite = Connection::open(&path).unwrap();
        run_migrations(&mut sqlite).unwrap();
        sqlite
            .execute_batch(
                "INSERT INTO daily_plans(day,time_zone,available_minutes,state,created_at,updated_at)
                 VALUES('1970-01-01','Europe/Berlin',60,'planned',1,1);
                 INSERT INTO coach_tasks(id,title,priority,created_at,updated_at)
                 VALUES('task','Focus',3,1,1);
                 INSERT INTO daily_tasks(id,day,personal_task_id,estimate_minutes,important,position)
                 VALUES('daily','1970-01-01','task',30,1,0);
                 INSERT INTO time_entries(id,started_at,ended_at,description,source,created_at,updated_at)
                 VALUES('entry',0,120000,'Focus','manual',0,0);
                 INSERT INTO task_time_links(entry_id,daily_task_id) VALUES('entry','daily');
                 INSERT INTO daily_reviews(day,outcome,worked,obstacles,tomorrow,revision,updated_at)
                 VALUES('1970-01-01','achieved','','','',1,1);",
            )
            .unwrap();
        drop(sqlite);

        let database = connect(&path).unwrap();
        let day = CoachDayRepository::new(&database)
            .day("1970-01-01", "UTC")
            .unwrap();
        assert_eq!(day.time_zone, "Europe/Berlin");
        assert_eq!(day.tasks.len(), 1);
        assert_eq!(day.tasks[0].reminder_at, None);
        assert_eq!(day.time_entries[0].duration_ms, 120_000);
        assert_eq!(day.review.outcome, "achieved");

        drop(database);
        std::fs::remove_file(path).unwrap();
    }
}
