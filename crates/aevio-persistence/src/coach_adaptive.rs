//! SeaORM repository for coach adaptive insights.
//!
//! Moved out of `aevio-core` so the domain kernel no longer owns persistence.
//! The repository lives in `aevio-persistence` and operates on the shared
//! `DatabaseConnection`; domain types and helper functions are imported from
//! `aevio_core`.

use crate::database::sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
};

use crate::database::entities::coach::tasks;
use crate::database::entities::coach::{daily_tasks, focus, task_time_links};
use crate::database::entities::entry_query::entries;
use crate::database::entities::tasks::tasks as project_tasks;
use crate::database::repositories::coach::CoachMetadataRepository;
use crate::database::repositories::coach_day::CoachDayRepository;
use aevio_core::coach::adaptive::{Estimate, WeekDay, Weekly};
use aevio_core::coach::dates;

pub struct CoachAdaptiveRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> CoachAdaptiveRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn estimates(&self) -> Result<Vec<Estimate>, String> {
        if !CoachMetadataRepository::new(self.db)
            .preferences()?
            .memory_enabled
        {
            return Ok(Vec::new());
        }
        let links = task_time_links::Entity::find()
            .all(self.db)
            .map_err(|error| error.to_string())?;
        let entries = entries::Entity::find()
            .filter(entries::Column::DeletedAt.is_null())
            .all(self.db)
            .map_err(|error| error.to_string())?;
        let entry_durations = entries
            .into_iter()
            .map(|entry| {
                (
                    entry.id,
                    std::cmp::max(entry.ended_at - entry.started_at, 0),
                )
            })
            .collect::<std::collections::HashMap<_, _>>();
        let daily_tasks = daily_tasks::Entity::find()
            .all(self.db)
            .map_err(|error| error.to_string())?;
        let task_by_id = daily_tasks
            .into_iter()
            .map(|task| (task.id.clone(), task))
            .collect::<std::collections::HashMap<_, _>>();
        let mut actual_by_task = std::collections::HashMap::<String, i64>::new();
        for link in links {
            if let Some(duration) = entry_durations.get(&link.entry_id) {
                *actual_by_task.entry(link.daily_task_id).or_default() += duration;
            }
        }
        let mut grouped = std::collections::HashMap::<(String, String), (i64, u32)>::new();
        for (daily_task_id, actual_ms) in actual_by_task {
            if actual_ms <= 0 {
                continue;
            }
            let Some(task) = task_by_id.get(&daily_task_id) else {
                continue;
            };
            let identity = task
                .personal_task_id
                .as_ref()
                .map(|id| ("personal".to_owned(), id.clone()))
                .or_else(|| {
                    task.project_task_id
                        .as_ref()
                        .map(|id| ("project".to_owned(), id.clone()))
                });
            let Some(identity) = identity else { continue };
            let aggregate = grouped.entry(identity).or_default();
            aggregate.0 += actual_ms;
            aggregate.1 += 1;
        }
        Ok(grouped
            .into_iter()
            .filter(|(_, (_, samples))| *samples >= 2)
            .take(200)
            .map(|((source, task_id), (total_ms, samples))| Estimate {
                source,
                task_id,
                minutes: ((total_ms as f64 / samples as f64 / 60_000.0).round() as u32)
                    .clamp(1, 1440),
                samples,
            })
            .collect())
    }

    pub fn weekly(&self, day: &str, zone: &str) -> Result<Weekly, String> {
        use chrono::{Datelike, Duration};

        let date = dates::parse(day)?;
        let monday = date - Duration::days(date.weekday().num_days_from_monday() as i64);
        let mut weeks = Vec::new();
        for offset in [-7_i64, 0] {
            let mut rows = Vec::new();
            for index in 0..7_i64 {
                let current = (monday + Duration::days(offset + index))
                    .format("%Y-%m-%d")
                    .to_string();
                let snapshot = CoachDayRepository::new(self.db).day(&current, zone)?;
                let (start, end) = dates::bounds(&current, &snapshot.time_zone)?;
                let focus_sessions = focus::Entity::find()
                    .filter(focus::Column::StartedAt.gte(start))
                    .filter(focus::Column::StartedAt.lt(end))
                    .count(self.db)
                    .map_err(|error| error.to_string())?
                    as u32;
                let completed = self.completed_tasks(start, end)?;
                rows.push(WeekDay {
                    day: current,
                    completed,
                    planned_minutes: snapshot.planned_minutes,
                    actual_ms: snapshot.tracked_ms,
                    outcome: snapshot.review.outcome,
                    focus_sessions,
                    postponed: snapshot
                        .tasks
                        .iter()
                        .filter(|item| item.postponements > 0)
                        .count() as u32,
                });
            }
            weeks.push(rows);
        }
        let days = weeks.pop().ok_or("Missing current week")?;
        let previous = weeks.pop().ok_or("Missing previous week")?;
        let mut recommendations = vec![
            "Erfasste Zeit und erledigte Aufgaben beschreiben unterschiedliche Dinge. Deine Zielbewertung bleibt maßgeblich.".into(),
        ];
        let planned: u32 = days.iter().map(|item| item.planned_minutes).sum();
        let actual: u64 = days.iter().map(|item| item.actual_ms).sum();
        if planned > 0 && actual > 0 {
            recommendations.push(format!(
                "Diese Woche sind {planned} Minuten geplant und {} Minuten erfasst. Nutze die Abweichung als Gesprächsanlass für deine nächste Planung.",
                actual / 60_000
            ));
        }
        if days.iter().any(|item| item.postponed > 0) {
            recommendations.push(
                "Falls eine Aufgabe mehrfach weiterwandert, kannst du ihren ersten Schritt verkleinern oder ihre Relevanz neu entscheiden.".into(),
            );
        }
        Ok(Weekly {
            starts_on: monday.format("%Y-%m-%d").to_string(),
            days,
            previous,
            recommendations,
            source: "local_aggregates".into(),
        })
    }

    fn completed_tasks(&self, start: i64, end: i64) -> Result<u32, String> {
        let personal = tasks::Entity::find()
            .filter(tasks::Column::CompletedAt.gte(start))
            .filter(tasks::Column::CompletedAt.lt(end))
            .all(self.db)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|task| format!("personal:{}", task.id));
        let project = project_tasks::Entity::find()
            .filter(project_tasks::Column::CompletedAt.gte(start))
            .filter(project_tasks::Column::CompletedAt.lt(end))
            .all(self.db)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|task| format!("project:{}", task.id));
        Ok(personal
            .chain(project)
            .collect::<std::collections::HashSet<_>>()
            .len() as u32)
    }
}
