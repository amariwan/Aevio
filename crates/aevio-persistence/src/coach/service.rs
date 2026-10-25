#[cfg(test)]
use super::repository::{self, plans, tasks, time};
use aevio_core::coach::{dates, models::*, policies};
#[cfg(test)]
use rusqlite::Connection;

use crate::database::DatabaseConnection;
use sea_orm::{ActiveModelTrait, EntityTrait};

fn db_error(error: String) -> sea_orm::DbErr {
    sea_orm::DbErr::Custom(error)
}

pub fn day_with_database(
    database: &DatabaseConnection,
    date: &str,
    zone: &str,
) -> Result<Day, String> {
    crate::database::repositories::coach_day::CoachDayRepository::new(database).day(date, zone)
}

pub fn apply_with_database(
    database: &DatabaseConnection,
    date: &str,
    zone: &str,
    command: Command,
    now: u64,
) -> Result<Day, String> {
    dates::bounds(date, zone)?;
    let before = day_with_database(database, date, zone)?;
    let now = now as i64;
    database
        .transaction(|tx| {
            let items = &before.tasks;
            let event = match command {
                Command::SavePlan { plan } => {
                    policies::plan(&plan).map_err(db_error)?;
                    if plan.day != date {
                        return Err(sea_orm::DbErr::Custom("Plan day does not match request".into()));
                    }
                    crate::database::repositories::coach_commands::CoachCommandRepository::save_plan(tx, &plan, now)?;
                    "plan.changed"
                }
                Command::SaveReview { review, revision } => {
                    policies::review(&review).map_err(db_error)?;
                    crate::database::repositories::coach_commands::CoachCommandRepository::ensure_plan(tx, date, zone, now)?;
                    crate::database::repositories::coach_commands::CoachCommandRepository::save_review(tx, date, &review, revision, now)?;
                    "review.saved"
                }
                Command::CreateTask { reminder_at, title, notes, priority, estimate_minutes, important } => {
                    task_fields(&title, &notes, priority, estimate_minutes).map_err(db_error)?;
                    if reminder_at.is_some_and(|at| at > 253402300799999) {
                        return Err(sea_orm::DbErr::Custom("Reminder date is outside supported range".into()));
                    }
                    crate::database::repositories::coach_commands::CoachCommandRepository::ensure_plan(tx, date, zone, now)?;
                    let task = crate::database::repositories::coach_commands::CoachCommandRepository::create_personal(tx, &title, &notes, priority, estimate_minutes, reminder_at, now)?;
                    crate::database::repositories::coach_commands::CoachCommandRepository::add_task(tx, date, &task, estimate_minutes, important)?;
                    "task.added"
                }
                Command::AddTask { task, estimate_minutes, important } => {
                    estimate(estimate_minutes).map_err(db_error)?;
                    crate::database::repositories::coach_commands::CoachCommandRepository::ensure_plan(tx, date, zone, now)?;
                    crate::database::repositories::coach_commands::CoachCommandRepository::add_task(tx, date, &task, estimate_minutes, important)?;
                    "task.added"
                }
                Command::EditTask { reminder_at, id, title, notes, priority, estimate_minutes, important } => {
                    task_fields(&title, &notes, priority, estimate_minutes).map_err(db_error)?;
                    if reminder_at.is_some_and(|at| at > 253402300799999) {
                        return Err(sea_orm::DbErr::Custom("Reminder date is outside supported range".into()));
                    }
                    let task = item(items, &id).map_err(sea_orm::DbErr::Custom)?;
                    crate::database::repositories::coach_commands::CoachCommandRepository::edit_task(tx, crate::database::repositories::coach_commands::EditTaskInput { item: task, title: &title, notes: &notes, priority, minutes: estimate_minutes, important, reminder_at, now })?;
                    "coach.changed"
                }
                Command::CompleteTask { id, done } => {
                    let task = item(items, &id).map_err(sea_orm::DbErr::Custom)?;
                    crate::database::repositories::coach_commands::CoachCommandRepository::complete(tx, task, done, now)?;
                    "task.completed"
                }
                Command::MoveTask { id, target_day } => {
                    dates::bounds(&target_day, zone).map_err(sea_orm::DbErr::Custom)?;
                    let moving = items.iter().chain(before.carryover.iter()).find(|task| task.id == id).ok_or_else(|| sea_orm::DbErr::Custom("Task is not available for carryover".into()))?;
                    if target_day == date && items.iter().any(|task| task.id == id) {
                        return Err(sea_orm::DbErr::Custom("Choose another day".into()));
                    }
                    if moving.done {
                        return Err(sea_orm::DbErr::Custom("Completed tasks stay in their daily history".into()));
                    }
                    crate::database::repositories::coach_commands::CoachCommandRepository::ensure_plan(tx, &target_day, zone, now)?;
                    crate::database::repositories::coach_commands::CoachCommandRepository::move_task(tx, &id, &target_day)?;
                    "task.moved"
                }
                Command::RemoveTask { id } => {
                    item(items, &id).map_err(sea_orm::DbErr::Custom)?;
                    crate::database::repositories::coach_commands::CoachCommandRepository::remove_task(tx, &id)?;
                    "task.removed"
                }
                Command::Reorder { ids } => {
                    let unique = ids.iter().collect::<std::collections::HashSet<_>>();
                    if unique.len() != items.len() || ids.len() != items.len() || ids.iter().any(|id| item(items, id).is_err()) {
                        return Err(sea_orm::DbErr::Custom("Reorder must contain every daily assignment exactly once".into()));
                    }
                    crate::database::repositories::coach_commands::CoachCommandRepository::reorder(tx, &ids)?;
                    "tasks.reordered"
                }
                Command::SaveSteps { id, steps } => {
                    item(items, &id).map_err(sea_orm::DbErr::Custom)?;
                    if steps.len() > 30 { return Err(sea_orm::DbErr::Custom("Keep at most 30 steps per task".into())); }
                    for step in &steps { policies::title(&step.title).map_err(db_error)?; }
                    crate::database::repositories::coach_commands::CoachCommandRepository::save_steps(tx, &id, &steps)?;
                    "steps.saved"
                }
                Command::LinkTime { id, entry_id } => {
                    item(items, &id).map_err(sea_orm::DbErr::Custom)?;
                    let entry_zone = before.plan.as_ref().map(|plan| plan.time_zone.as_str()).unwrap_or(zone);
                    let (start, end) = dates::bounds(date, entry_zone).map_err(sea_orm::DbErr::Custom)?;
                    crate::database::repositories::coach_commands::CoachCommandRepository::link_time(tx, &id, &entry_id, start, end)?;
                    "coach.changed"
                }
                Command::UnlinkTime { entry_id } => {
                    crate::database::repositories::coach_commands::CoachCommandRepository::unlink_time(tx, &entry_id, date)?;
                    "coach.changed"
                }
                Command::SavePreferences { preferences } => {
                    policies::preferences(&preferences).map_err(db_error)?;
                    let raw = serde_json::to_string(&preferences).map_err(|error| sea_orm::DbErr::Custom(error.to_string()))?;
                    let current = crate::database::entities::coach::preferences::Entity::find_by_id(1).one(tx)?;
                    if let Some(row) = current { let mut model: crate::database::entities::coach::preferences::ActiveModel = row.into(); model.value = sea_orm::ActiveValue::Set(raw); model.update(tx)?; } else { crate::database::entities::coach::preferences::ActiveModel { id: sea_orm::ActiveValue::Set(1), value: sea_orm::ActiveValue::Set(raw) }.insert(tx)?; }
                    "coach.changed"
                }
                Command::EraseCoach => {
                    crate::database::repositories::coach_commands::CoachCommandRepository::erase(tx)?;
                    "coach.erased"
                }
            };
            if event != "coach.erased" {
                crate::database::repositories::coach_commands::CoachCommandRepository::audit(tx, date, event, now)?;
            }
            Ok(())
        })
        .map_err(|error| error.to_string())?;
    day_with_database(database, date, zone)
}

#[cfg(test)]
pub fn day(conn: &Connection, date: &str, zone: &str) -> Result<Day, String> {
    let plan = plans::get(conn, date)?;
    let zone = plan
        .as_ref()
        .map(|plan| plan.time_zone.as_str())
        .unwrap_or(zone);
    let (start, end) = dates::bounds(date, zone)?;
    let mut items = tasks::list(conn, date, false)?;
    let entries = time::entries(conn, start, end, &items)?;
    let tracked_ms = entries.iter().map(|entry| entry.duration_ms).sum();
    for item in &mut items {
        item.actual_ms = entries
            .iter()
            .filter(|entry| entry.assigned_to.as_deref() == Some(&item.id))
            .map(|entry| entry.duration_ms)
            .sum();
    }
    let mut focus_ms = 0;
    for (task_id, entry_id, ms) in time::focus_allocations(conn, date, start, end)? {
        if entries
            .iter()
            .any(|entry| entry.id == entry_id && entry.assigned_to.is_none())
            && let Some(item) = items.iter_mut().find(|item| item.id == task_id)
        {
            item.actual_ms += ms;
            focus_ms += ms;
        }
    }
    let unassigned_ms = entries
        .iter()
        .filter(|entry| {
            !items
                .iter()
                .any(|item| Some(&item.id) == entry.assigned_to.as_ref())
        })
        .map(|entry| entry.duration_ms)
        .sum::<u64>()
        .saturating_sub(focus_ms);
    let planned_minutes = items.iter().map(|item| item.estimate_minutes).sum::<u32>();
    let overbooked_minutes = plan
        .as_ref()
        .map(|plan| planned_minutes.saturating_sub(plan.available_minutes))
        .unwrap_or(0);
    Ok(Day {
        day: date.into(),
        time_zone: zone.into(),
        plan,
        review: plans::review(conn, date)?,
        tasks: items,
        catalog: tasks::catalog(conn)?,
        carryover: tasks::list(conn, date, true)?,
        preferences: repository::preferences(conn)?,
        tracked_ms,
        unassigned_ms,
        planned_minutes,
        overbooked_minutes,
        changes: plans::changes(conn, date)?,
        time_entries: entries,
    })
}

fn item<'a>(items: &'a [PlannedTask], id: &str) -> Result<&'a PlannedTask, String> {
    items
        .iter()
        .find(|item| item.id == id)
        .ok_or_else(|| "Task is not assigned to the selected day".into())
}
fn estimate(minutes: u32) -> Result<(), String> {
    if minutes > 1440 {
        Err("A task estimate must be between 0 and 1440 minutes".into())
    } else {
        Ok(())
    }
}
fn task_fields(title: &str, notes: &str, priority: u8, minutes: u32) -> Result<(), String> {
    policies::title(title)?;
    policies::text(notes, 10000)?;
    estimate(minutes)?;
    if !(1..=5).contains(&priority) {
        return Err("Priority must be between 1 and 5".into());
    }
    Ok(())
}

/// One explicit action, one atomic write. Failure rolls back all effects.
#[cfg(test)]
pub fn apply(
    conn: &mut Connection,
    date: &str,
    zone: &str,
    command: Command,
    now: u64,
) -> Result<Day, String> {
    dates::bounds(date, zone)?;
    let tx = conn.transaction().map_err(crate::err)?;
    let items = tasks::list(&tx, date, false)?;
    let mut event = "coach.changed";
    match command {
        Command::SavePlan { plan } => {
            policies::plan(&plan)?;
            if plan.day != date {
                return Err("Plan day does not match request".into());
            }
            plans::save(&tx, &plan, now)?;
            event = "plan.changed";
        }
        Command::SaveReview { review, revision } => {
            policies::review(&review)?;
            plans::ensure(&tx, date, zone, now)?;
            plans::save_review(&tx, date, &review, revision, now)?;
            event = "review.saved";
        }
        Command::CreateTask {
            reminder_at,
            title,
            notes,
            priority,
            estimate_minutes,
            important,
        } => {
            task_fields(&title, &notes, priority, estimate_minutes)?;
            if reminder_at.is_some_and(|at| at > 253402300799999) {
                return Err("Reminder date is outside supported range".into());
            }
            plans::ensure(&tx, date, zone, now)?;
            let task = tasks::create_personal(
                &tx,
                &title,
                &notes,
                priority,
                estimate_minutes,
                reminder_at,
                now,
            )?;
            tasks::add(&tx, date, &task, estimate_minutes, important)?;
            event = "task.added";
        }
        Command::AddTask {
            task,
            estimate_minutes,
            important,
        } => {
            estimate(estimate_minutes)?;
            plans::ensure(&tx, date, zone, now)?;
            tasks::add(&tx, date, &task, estimate_minutes, important)?;
            event = "task.added";
        }
        Command::EditTask {
            reminder_at,
            id,
            title,
            notes,
            priority,
            estimate_minutes,
            important,
        } => {
            task_fields(&title, &notes, priority, estimate_minutes)?;
            if reminder_at.is_some_and(|at| at > 253402300799999) {
                return Err("Reminder date is outside supported range".into());
            }
            tasks::edit(
                &tx,
                item(&items, &id)?,
                &tasks::TaskEdit {
                    title: &title,
                    notes: &notes,
                    priority,
                    minutes: estimate_minutes,
                    important,
                    reminder_at,
                },
                now,
            )?;
        }
        Command::CompleteTask { id, done } => {
            tasks::complete(&tx, item(&items, &id)?, done, now)?;
            event = "task.completed";
        }
        Command::MoveTask { id, target_day } => {
            dates::bounds(&target_day, zone)?;
            // Carryover is explicit, and only unfinished assignments can be moved.
            let previous = tasks::list(&tx, date, true)?;
            let moving = items
                .iter()
                .chain(previous.iter())
                .find(|item| item.id == id)
                .ok_or("Task is not available for carryover")?;
            if target_day == date && items.iter().any(|item| item.id == id) {
                return Err("Choose another day".into());
            }
            if moving.done {
                return Err("Completed tasks stay in their daily history".into());
            }
            plans::ensure(&tx, &target_day, zone, now)?;
            tasks::move_to(&tx, &id, &target_day)?;
            event = "task.moved";
        }
        Command::RemoveTask { id } => {
            item(&items, &id)?;
            tasks::remove(&tx, &id)?;
            event = "task.removed";
        }
        Command::Reorder { ids } => {
            let unique = ids.iter().collect::<std::collections::HashSet<_>>();
            if unique.len() != items.len()
                || ids.len() != items.len()
                || ids.iter().any(|id| item(&items, id).is_err())
            {
                return Err("Reorder must contain every daily assignment exactly once".into());
            }
            tasks::reorder(&tx, &ids)?;
            event = "tasks.reordered";
        }
        Command::SaveSteps { id, steps } => {
            item(&items, &id)?;
            if steps.len() > 30 {
                return Err("Keep at most 30 steps per task".into());
            }
            for step in &steps {
                policies::title(&step.title)?;
            }
            tasks::save_steps(&tx, &id, &steps)?;
            event = "steps.saved";
        }
        Command::LinkTime { id, entry_id } => {
            item(&items, &id)?;
            let plan = plans::get(&tx, date)?;
            let tz = plan.as_ref().map(|p| p.time_zone.as_str()).unwrap_or(zone);
            let (start, end) = dates::bounds(date, tz)?;
            time::link(&tx, &id, &entry_id, start, end)?;
        }
        Command::UnlinkTime { entry_id } => {
            time::unlink(&tx, &entry_id, date)?;
        }
        Command::SavePreferences { preferences } => {
            policies::preferences(&preferences)?;
            repository::save_preferences(&tx, &preferences)?;
        }
        Command::EraseCoach => {
            repository::erase(&tx)?;
            event = "coach.erased";
        }
    }
    if event != "coach.erased" {
        repository::audit(&tx, date, event, now)?;
    }
    tx.commit().map_err(crate::err)?;
    day(conn, date, zone)
}

#[cfg(test)]
mod typed_tests {
    use super::*;
    use crate::bootstrap::migrations::run_migrations;
    use crate::database::connect;
    use rusqlite::Connection;

    #[test]
    fn typed_apply_rolls_back_ensure_when_review_revision_conflicts() {
        let path =
            std::env::temp_dir().join(format!("aevio-coach-apply-{}.db", uuid::Uuid::now_v7()));
        let mut sqlite = Connection::open(&path).unwrap();
        run_migrations(&mut sqlite).unwrap();
        drop(sqlite);
        let database = connect(&path).unwrap();
        let error = apply_with_database(
            &database,
            "1970-01-01",
            "UTC",
            Command::SaveReview {
                review: Review::default(),
                revision: 1,
            },
            1,
        )
        .unwrap_err();
        assert!(error.contains("review changed"));
        let plan = crate::database::entities::coach::daily_plans::Entity::find_by_id(
            "1970-01-01".to_owned(),
        )
        .one(&database)
        .unwrap();
        assert!(plan.is_none());
        drop(database);
        std::fs::remove_file(path).unwrap();
    }
}
