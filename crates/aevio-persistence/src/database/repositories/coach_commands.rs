use sea_orm::entity::prelude::*;
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set},
    ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder,
};

use crate::database::entities::coach::{
    brain_entries, daily_plans, daily_reviews, daily_tasks, focus, focus_interruptions,
    interventions, memory, preferences, routine_items, routine_runs, routines, task_steps,
    task_time_links, tasks,
};
use crate::database::entities::entry_query::entries;
use crate::database::entities::tasks::tasks as project_tasks;
use aevio_core::coach::models::{Plan, Review, Step, TaskRef};

pub struct CoachCommandRepository;

pub struct EditTaskInput<'a> {
    pub item: &'a aevio_core::coach::models::PlannedTask,
    pub title: &'a str,
    pub notes: &'a str,
    pub priority: u8,
    pub minutes: u32,
    pub important: bool,
    pub reminder_at: Option<u64>,
    pub now: i64,
}

impl CoachCommandRepository {
    pub fn save_plan<C: ConnectionTrait>(db: &C, plan: &Plan, now: i64) -> Result<(), DbErr> {
        let existing = daily_plans::Entity::find_by_id(plan.day.clone()).one(db)?;
        let Some(existing) = existing else {
            if plan.revision != 0 {
                return Err(DbErr::Custom(
                    "The plan changed in another window. Reload before saving.".into(),
                ));
            }
            daily_plans::ActiveModel {
                day: Set(plan.day.clone()),
                time_zone: Set(plan.time_zone.clone()),
                goal: Set(plan.goal.trim().to_owned()),
                notes: Set(plan.notes.clone()),
                available_minutes: Set(plan.available_minutes as i64),
                energy: Set(plan.energy.map(i64::from)),
                state: Set(plan.state.clone()),
                revision: Set(1),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(db)?;
            return Ok(());
        };
        if existing.revision != plan.revision as i64 {
            return Err(DbErr::Custom(
                "The plan changed in another window. Reload before saving.".into(),
            ));
        }
        let mut model: daily_plans::ActiveModel = existing.into();
        model.time_zone = Set(plan.time_zone.clone());
        model.goal = Set(plan.goal.trim().to_owned());
        model.notes = Set(plan.notes.clone());
        model.available_minutes = Set(plan.available_minutes as i64);
        model.energy = Set(plan.energy.map(i64::from));
        model.state = Set(plan.state.clone());
        model.revision = Set(plan.revision as i64 + 1);
        model.updated_at = Set(now);
        model.update(db)?;
        Ok(())
    }

    pub fn ensure_plan<C: ConnectionTrait>(
        db: &C,
        day: &str,
        zone: &str,
        now: i64,
    ) -> Result<(), DbErr> {
        if daily_plans::Entity::find_by_id(day.to_owned())
            .one(db)?
            .is_none()
        {
            daily_plans::ActiveModel {
                day: Set(day.to_owned()),
                time_zone: Set(zone.to_owned()),
                goal: Set(String::new()),
                notes: Set(String::new()),
                available_minutes: Set(0),
                energy: Set(None),
                state: Set("planned".into()),
                revision: Set(1),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(db)?;
        }
        Ok(())
    }

    pub fn save_review<C: ConnectionTrait>(
        db: &C,
        day: &str,
        review: &Review,
        revision: u32,
        now: i64,
    ) -> Result<(), DbErr> {
        let current = daily_reviews::Entity::find_by_id(day.to_owned()).one(db)?;
        if let Some(row) = current.as_ref()
            && row.outcome == review.outcome
            && row.worked == review.worked
            && row.obstacles == review.obstacles
            && row.tomorrow == review.tomorrow
        {
            return Ok(());
        }
        if current.as_ref().map_or(0, |row| row.revision.max(0) as u32) != revision {
            return Err(DbErr::Custom(
                "The review changed in another window. Reload before saving.".into(),
            ));
        }
        if let Some(row) = current {
            let mut model: daily_reviews::ActiveModel = row.into();
            model.outcome = Set(review.outcome.clone());
            model.worked = Set(review.worked.clone());
            model.obstacles = Set(review.obstacles.clone());
            model.tomorrow = Set(review.tomorrow.clone());
            model.revision = Set(revision as i64 + 1);
            model.updated_at = Set(now);
            model.update(db)?;
        } else {
            daily_reviews::ActiveModel {
                day: Set(day.to_owned()),
                outcome: Set(review.outcome.clone()),
                worked: Set(review.worked.clone()),
                obstacles: Set(review.obstacles.clone()),
                tomorrow: Set(review.tomorrow.clone()),
                revision: Set(1),
                updated_at: Set(now),
            }
            .insert(db)?;
        }
        Ok(())
    }

    pub fn add_task<C: ConnectionTrait>(
        db: &C,
        day: &str,
        task: &TaskRef,
        minutes: u32,
        important: bool,
    ) -> Result<(), DbErr> {
        let valid = match task.source.as_str() {
            "personal" => tasks::Entity::find_by_id(task.id.clone())
                .one(db)?
                .is_some(),
            "project" => project_tasks::Entity::find_by_id(task.id.clone())
                .filter(project_tasks::Column::DeletedAt.is_null())
                .one(db)?
                .is_some(),
            _ => false,
        };
        if !valid {
            return Err(DbErr::Custom(
                "Task no longer exists in this local catalog".into(),
            ));
        }
        let exists = daily_tasks::Entity::find()
            .filter(daily_tasks::Column::Day.eq(day.to_owned()))
            .filter(match task.source.as_str() {
                "personal" => daily_tasks::Column::PersonalTaskId.eq(task.id.clone()),
                _ => daily_tasks::Column::ProjectTaskId.eq(task.id.clone()),
            })
            .one(db)?
            .is_some();
        if exists {
            return Ok(());
        }
        let position = daily_tasks::Entity::find()
            .filter(daily_tasks::Column::Day.eq(day.to_owned()))
            .order_by_desc(daily_tasks::Column::Position)
            .one(db)?
            .map_or(0, |row| row.position + 1);
        daily_tasks::ActiveModel {
            id: Set(uuid::Uuid::now_v7().to_string()),
            day: Set(day.to_owned()),
            personal_task_id: Set((task.source == "personal").then(|| task.id.clone())),
            project_task_id: Set((task.source == "project").then(|| task.id.clone())),
            estimate_minutes: Set(minutes as i64),
            important: Set(i64::from(important)),
            position: Set(position),
            postponements: Set(0),
        }
        .insert(db)?;
        Ok(())
    }

    pub fn create_personal<C: ConnectionTrait>(
        db: &C,
        title: &str,
        notes: &str,
        priority: u8,
        minutes: u32,
        reminder_at: Option<u64>,
        now: i64,
    ) -> Result<TaskRef, DbErr> {
        let id = uuid::Uuid::now_v7().to_string();
        tasks::ActiveModel {
            id: Set(id.clone()),
            title: Set(title.trim().to_owned()),
            notes: Set(Some(notes.to_owned())),
            priority: Set(i64::from(priority)),
            status: Set("open".into()),
            due_at: Set(None),
            reminder_at: Set(reminder_at.map(|value| value as i64)),
            estimate_ms: Set(Some(minutes as i64 * 60_000)),
            created_at: Set(now),
            updated_at: Set(now),
            completed_at: Set(None),
            notified_at: Set(None),
        }
        .insert(db)?;
        Ok(TaskRef {
            source: "personal".into(),
            id,
        })
    }

    pub fn edit_task<C: ConnectionTrait>(db: &C, input: EditTaskInput<'_>) -> Result<(), DbErr> {
        if input.item.task.source == "personal" {
            if let Some(row) = tasks::Entity::find_by_id(input.item.task.id.clone()).one(db)? {
                let reminder_changed = row.reminder_at != input.reminder_at.map(|v| v as i64);
                let mut model: tasks::ActiveModel = row.into();
                if reminder_changed {
                    model.notified_at = Set(None);
                }
                model.title = Set(input.title.trim().to_owned());
                model.notes = Set(Some(input.notes.to_owned()));
                model.priority = Set(i64::from(input.priority));
                model.estimate_ms = Set(Some(input.minutes as i64 * 60_000));
                model.reminder_at = Set(input.reminder_at.map(|v| v as i64));
                model.updated_at = Set(input.now);
                model.update(db)?;
            }
        } else if let Some(row) = project_tasks::Entity::find_by_id(input.item.task.id.clone())
            .filter(project_tasks::Column::DeletedAt.is_null())
            .one(db)?
        {
            let mut model: project_tasks::ActiveModel = row.into();
            model.title = Set(input.title.trim().to_owned());
            model.description = Set(Some(input.notes.to_owned()));
            model.priority = Set(project_priority(input.priority));
            model.updated_at = Set(input.now);
            model.update(db)?;
        }
        let Some(row) = daily_tasks::Entity::find_by_id(input.item.id.clone()).one(db)? else {
            return Err(DbErr::RecordNotFound("daily task".into()));
        };
        let mut model: daily_tasks::ActiveModel = row.into();
        model.estimate_minutes = Set(input.minutes as i64);
        model.important = Set(i64::from(input.important));
        model.update(db)?;
        Ok(())
    }

    pub fn complete<C: ConnectionTrait>(
        db: &C,
        item: &aevio_core::coach::models::PlannedTask,
        done: bool,
        now: i64,
    ) -> Result<(), DbErr> {
        if item.task.source == "personal" {
            if let Some(row) = tasks::Entity::find_by_id(item.task.id.clone()).one(db)? {
                let mut model: tasks::ActiveModel = row.into();
                model.status = Set(if done { "done" } else { "open" }.into());
                model.completed_at = Set(done.then_some(now));
                model.updated_at = Set(now);
                model.update(db)?;
            }
        } else if let Some(row) = project_tasks::Entity::find_by_id(item.task.id.clone())
            .filter(project_tasks::Column::DeletedAt.is_null())
            .one(db)?
        {
            let mut model: project_tasks::ActiveModel = row.into();
            model.status = Set(if done { "done" } else { "todo" }.into());
            model.completed_at = Set(done.then_some(now));
            model.updated_at = Set(now);
            model.update(db)?;
        }
        Ok(())
    }

    pub fn move_task<C: ConnectionTrait>(db: &C, id: &str, day: &str) -> Result<(), DbErr> {
        let position = daily_tasks::Entity::find()
            .filter(daily_tasks::Column::Day.eq(day.to_owned()))
            .order_by_desc(daily_tasks::Column::Position)
            .one(db)?
            .map_or(0, |row| row.position + 1);
        let Some(row) = daily_tasks::Entity::find_by_id(id.to_owned()).one(db)? else {
            return Ok(());
        };
        let next_postponements = row.postponements + 1;
        let mut model: daily_tasks::ActiveModel = row.into();
        model.day = Set(day.to_owned());
        model.position = Set(position);
        model.postponements = Set(next_postponements);
        model.update(db)?;
        Ok(())
    }

    pub fn remove_task<C: ConnectionTrait>(db: &C, id: &str) -> Result<(), DbErr> {
        daily_tasks::Entity::delete_by_id(id.to_owned()).exec(db)?;
        Ok(())
    }

    pub fn reorder<C: ConnectionTrait>(db: &C, ids: &[String]) -> Result<(), DbErr> {
        for (position, id) in ids.iter().enumerate() {
            daily_tasks::Entity::update_many()
                .col_expr(daily_tasks::Column::Position, Expr::value(position as i64))
                .filter(daily_tasks::Column::Id.eq(id.clone()))
                .exec(db)?;
        }
        Ok(())
    }

    pub fn save_steps<C: ConnectionTrait>(db: &C, id: &str, steps: &[Step]) -> Result<(), DbErr> {
        task_steps::Entity::delete_many()
            .filter(task_steps::Column::DailyTaskId.eq(id.to_owned()))
            .exec(db)?;
        for (position, step) in steps.iter().enumerate() {
            task_steps::ActiveModel {
                daily_task_id: Set(id.to_owned()),
                position: Set(position as i64),
                title: Set(step.title.trim().to_owned()),
                done: Set(i64::from(step.done)),
            }
            .insert(db)?;
        }
        Ok(())
    }

    pub fn link_time<C: ConnectionTrait>(
        db: &C,
        id: &str,
        entry_id: &str,
        start: i64,
        end: i64,
    ) -> Result<(), DbErr> {
        let entry = entries::Entity::find_by_id(entry_id.to_owned())
            .filter(entries::Column::DeletedAt.is_null())
            .filter(entries::Column::StartedAt.lt(end))
            .filter(entries::Column::EndedAt.gt(start))
            .one(db)?;
        if entry.is_none() {
            return Err(DbErr::Custom("Select a time entry from this day".into()));
        }
        if let Some(row) = crate::database::entities::coach::task_time_links::Entity::find_by_id(
            entry_id.to_owned(),
        )
        .one(db)?
        {
            let mut model: crate::database::entities::coach::task_time_links::ActiveModel =
                row.into();
            model.daily_task_id = Set(id.to_owned());
            model.update(db)?;
        } else {
            crate::database::entities::coach::task_time_links::ActiveModel {
                entry_id: Set(entry_id.to_owned()),
                daily_task_id: Set(id.to_owned()),
            }
            .insert(db)?;
        }
        Ok(())
    }

    pub fn unlink_time<C: ConnectionTrait>(db: &C, entry_id: &str, day: &str) -> Result<(), DbErr> {
        let ids = crate::database::entities::coach::daily_tasks::Entity::find()
            .filter(crate::database::entities::coach::daily_tasks::Column::Day.eq(day.to_owned()))
            .all(db)?
            .into_iter()
            .map(|row| row.id)
            .collect::<Vec<_>>();
        crate::database::entities::coach::task_time_links::Entity::delete_many()
            .filter(
                crate::database::entities::coach::task_time_links::Column::EntryId
                    .eq(entry_id.to_owned()),
            )
            .filter(
                crate::database::entities::coach::task_time_links::Column::DailyTaskId.is_in(ids),
            )
            .exec(db)?;
        Ok(())
    }

    pub fn audit<C: ConnectionTrait>(db: &C, day: &str, kind: &str, now: i64) -> Result<(), DbErr> {
        interventions::ActiveModel {
            id: NotSet,
            day: Set(day.to_owned()),
            kind: Set(kind.to_owned()),
            at: Set(now),
        }
        .insert(db)?;
        interventions::Entity::delete_many()
            .filter(interventions::Column::At.lt(now.saturating_sub(90 * 86_400_000)))
            .exec(db)?;
        Ok(())
    }

    pub fn erase<C: ConnectionTrait>(db: &C) -> Result<(), DbErr> {
        routine_runs::Entity::delete_many().exec(db)?;
        routine_items::Entity::delete_many().exec(db)?;
        routines::Entity::delete_many().exec(db)?;
        brain_entries::Entity::delete_many().exec(db)?;
        memory::Entity::delete_many().exec(db)?;
        focus_interruptions::Entity::delete_many().exec(db)?;
        focus::Entity::delete_many().exec(db)?;
        task_time_links::Entity::delete_many().exec(db)?;
        task_steps::Entity::delete_many().exec(db)?;
        daily_tasks::Entity::delete_many().exec(db)?;
        daily_reviews::Entity::delete_many().exec(db)?;
        daily_plans::Entity::delete_many().exec(db)?;
        preferences::Entity::delete_many().exec(db)?;
        interventions::Entity::delete_many().exec(db)?;
        tasks::Entity::delete_many().exec(db)?;
        Ok(())
    }
}

fn project_priority(priority: u8) -> String {
    match priority {
        5 => "urgent",
        4 => "high",
        1 | 2 => "low",
        _ => "normal",
    }
    .into()
}
