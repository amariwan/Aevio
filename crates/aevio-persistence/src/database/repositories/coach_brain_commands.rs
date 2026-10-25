use sea_orm::entity::prelude::*;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::coach::brain::{self, Command};
use crate::database::entities::coach::{brain_entries, routine_items, routine_runs, routines};
use crate::database::repositories::coach_commands::CoachCommandRepository;

pub struct CoachBrainCommandRepository;

impl CoachBrainCommandRepository {
    pub fn apply<C: ConnectionTrait>(
        db: &C,
        day: &str,
        zone: &str,
        command: Command,
        now: i64,
    ) -> Result<(), String> {
        match command {
            Command::SaveEntry { entry } => {
                aevio_core::coach::policies::text(&entry.content, 10_000)?;
                if entry.content.trim().is_empty() || !brain::valid_category(&entry.category) {
                    return Err("Enter a thought and select a supported category".into());
                }
                let id = if entry.id.is_empty() {
                    uuid::Uuid::now_v7().to_string()
                } else {
                    entry.id
                };
                let existing = brain_entries::Entity::find_by_id(id.clone())
                    .one(db)
                    .map_err(|error| error.to_string())?;
                if existing.is_none()
                    && brain_entries::Entity::find()
                        .count(db)
                        .map_err(|error| error.to_string())?
                        >= 500
                {
                    return Err("Keep at most 500 thoughts; remove an older entry first".into());
                }
                if let Some(row) = existing {
                    let mut model: brain_entries::ActiveModel = row.into();
                    model.content = Set(entry.content.trim().to_owned());
                    model.category = Set(entry.category);
                    model.updated_at = Set(now);
                    model.update(db).map_err(|error| error.to_string())?;
                } else {
                    brain_entries::ActiveModel {
                        id: Set(id),
                        content: Set(entry.content.trim().to_owned()),
                        category: Set(entry.category),
                        converted_task_id: Set(entry.converted_task_id),
                        updated_at: Set(now),
                    }
                    .insert(db)
                    .map_err(|error| error.to_string())?;
                }
            }
            Command::RemoveEntry { id } => {
                brain_entries::Entity::delete_by_id(id)
                    .exec(db)
                    .map_err(|error| error.to_string())?;
            }
            Command::Convert { id } => {
                let Some(row) = brain_entries::Entity::find_by_id(id.clone())
                    .one(db)
                    .map_err(|error| error.to_string())?
                else {
                    return Err("Thought not found".into());
                };
                if row.converted_task_id.is_none() {
                    CoachCommandRepository::ensure_plan(db, day, zone, now)
                        .map_err(|error| error.to_string())?;
                    let title = row
                        .content
                        .lines()
                        .next()
                        .unwrap_or(&row.content)
                        .chars()
                        .take(300)
                        .collect::<String>();
                    let task = CoachCommandRepository::create_personal(
                        db,
                        &title,
                        &row.content,
                        3,
                        0,
                        None,
                        now,
                    )
                    .map_err(|error| error.to_string())?;
                    CoachCommandRepository::add_task(db, day, &task, 0, false)
                        .map_err(|error| error.to_string())?;
                    let mut model: brain_entries::ActiveModel = row.into();
                    model.converted_task_id = Set(Some(task.id));
                    model.updated_at = Set(now);
                    model.update(db).map_err(|error| error.to_string())?;
                }
            }
            Command::SaveRoutine { routine } => {
                aevio_core::coach::policies::title(&routine.title)?;
                if routine.tasks.is_empty() || routine.tasks.len() > 30 {
                    return Err("A routine needs between 1 and 30 tasks".into());
                }
                let id = if routine.id.is_empty() {
                    uuid::Uuid::now_v7().to_string()
                } else {
                    routine.id
                };
                let exists = routines::Entity::find_by_id(id.clone())
                    .one(db)
                    .map_err(|error| error.to_string())?
                    .is_some();
                if !exists
                    && routines::Entity::find()
                        .count(db)
                        .map_err(|error| error.to_string())?
                        >= 100
                {
                    return Err("Keep at most 100 routines".into());
                }
                if let Some(row) = routines::Entity::find_by_id(id.clone())
                    .one(db)
                    .map_err(|error| error.to_string())?
                {
                    let mut model: routines::ActiveModel = row.into();
                    model.title = Set(routine.title.trim().to_owned());
                    model.update(db).map_err(|error| error.to_string())?;
                } else {
                    routines::ActiveModel {
                        id: Set(id.clone()),
                        title: Set(routine.title.trim().to_owned()),
                    }
                    .insert(db)
                    .map_err(|error| error.to_string())?;
                }
                routine_items::Entity::delete_many()
                    .filter(routine_items::Column::RoutineId.eq(id.clone()))
                    .exec(db)
                    .map_err(|error| error.to_string())?;
                for (position, item) in routine.tasks.iter().enumerate() {
                    aevio_core::coach::policies::title(&item.title)?;
                    if item.estimate_minutes > 1440 {
                        return Err("Routine task estimate exceeds one day".into());
                    }
                    routine_items::ActiveModel {
                        routine_id: Set(id.clone()),
                        position: Set(position as i32),
                        title: Set(item.title.trim().to_owned()),
                        estimate_minutes: Set(item.estimate_minutes as i32),
                    }
                    .insert(db)
                    .map_err(|error| error.to_string())?;
                }
            }
            Command::RemoveRoutine { id } => {
                routines::Entity::delete_by_id(id)
                    .exec(db)
                    .map_err(|error| error.to_string())?;
            }
            Command::ApplyRoutine { id } => {
                let Some(routine) = routines::Entity::find_by_id(id.clone())
                    .one(db)
                    .map_err(|error| error.to_string())?
                else {
                    return Err("Routine no longer exists".into());
                };
                CoachCommandRepository::ensure_plan(db, day, zone, now)
                    .map_err(|error| error.to_string())?;
                let already_applied =
                    routine_runs::Entity::find_by_id((id.clone(), day.to_owned()))
                        .one(db)
                        .map_err(|error| error.to_string())?
                        .is_some();
                if !already_applied {
                    routine_runs::ActiveModel {
                        routine_id: Set(id),
                        day: Set(day.to_owned()),
                    }
                    .insert(db)
                    .map_err(|error| error.to_string())?;
                    let items = routine_items::Entity::find()
                        .filter(routine_items::Column::RoutineId.eq(routine.id))
                        .order_by_asc(routine_items::Column::Position)
                        .all(db)
                        .map_err(|error| error.to_string())?;
                    for item in items {
                        let task = CoachCommandRepository::create_personal(
                            db,
                            &item.title,
                            "",
                            3,
                            item.estimate_minutes.max(0) as u32,
                            None,
                            now,
                        )
                        .map_err(|error| error.to_string())?;
                        CoachCommandRepository::add_task(
                            db,
                            day,
                            &task,
                            item.estimate_minutes.max(0) as u32,
                            false,
                        )
                        .map_err(|error| error.to_string())?;
                    }
                }
            }
        }
        Ok(())
    }
}
