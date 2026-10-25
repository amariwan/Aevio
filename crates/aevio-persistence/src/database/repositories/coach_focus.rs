use std::collections::HashMap;

use sea_orm::entity::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ConnectionTrait, EntityTrait, ExprTrait, QueryFilter,
    QueryOrder, Set,
};

use crate::database::entities::coach::{focus, focus_interruptions, task_time_links};
use crate::database::entities::entry_query::{entries, segments};
use crate::database::repositories::coach_commands::CoachCommandRepository;
use aevio_core::coach::focus::Start;
use aevio_core::coach::models::Preferences;

pub struct CoachFocusRepository;

impl CoachFocusRepository {
    pub fn begin<C: ConnectionTrait>(
        db: &C,
        task_id: &str,
        start: Start<'_>,
        now: i64,
    ) -> Result<(), String> {
        if !(2..=180).contains(&start.minutes) {
            return Err("Choose between 2 and 180 focus minutes".into());
        }
        aevio_core::coach::policies::text(start.next_step, 1000)?;
        focus::Entity::update_many()
            .col_expr(focus::Column::State, Expr::value("ended"))
            .col_expr(focus::Column::EndedAt, Expr::value(now))
            .filter(focus::Column::State.ne("ended"))
            .exec(db)
            .map_err(|error| error.to_string())?;
        focus::ActiveModel {
            id: Set(uuid::Uuid::now_v7().to_string()),
            daily_task_id: Set(task_id.to_owned()),
            segment_id: Set(start.segment_id),
            goal: Set(start.goal.to_owned()),
            next_step: Set(start.next_step.to_owned()),
            planned_minutes: Set(start.minutes as i64),
            body_doubling: Set(i64::from(start.body)),
            state: Set("running".into()),
            started_at: Set(now),
            ended_at: Set(None),
            reminded_at: Set(None),
        }
        .insert(db)
        .map_err(|error| error.to_string())?;
        CoachCommandRepository::audit(db, "", "focus.started", now)
            .map_err(|error| error.to_string())
    }

    pub fn finish<C: ConnectionTrait>(db: &C, id: &str, now: i64) -> Result<(), String> {
        focus::Entity::update_many()
            .col_expr(focus::Column::State, Expr::value("ended"))
            .col_expr(focus::Column::EndedAt, Expr::value(now))
            .filter(focus::Column::Id.eq(id.to_owned()))
            .filter(focus::Column::State.ne("ended"))
            .exec(db)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub fn reconcile<C: ConnectionTrait>(db: &C, now: i64) -> Result<(), String> {
        let running = focus::Entity::find()
            .filter(focus::Column::State.eq("running"))
            .all(db)
            .map_err(|error| error.to_string())?;
        for row in running {
            let Some(segment) = segments::Entity::find_by_id(row.segment_id)
                .one(db)
                .map_err(|error| error.to_string())?
            else {
                continue;
            };
            let Some(ended_at) = segment.ended_at else {
                continue;
            };
            focus_interruptions::ActiveModel {
                id: NotSet,
                focus_id: Set(row.id.clone()),
                at: Set(ended_at),
                reason: Set("Activity session ended".into()),
            }
            .insert(db)
            .map_err(|error| error.to_string())?;
            let mut model: focus::ActiveModel = row.into();
            model.state = Set("interrupted".into());
            model.ended_at = Set(Some(ended_at));
            model.update(db).map_err(|error| error.to_string())?;
        }
        let _ = now;
        Ok(())
    }

    pub fn attribute_entries<C: ConnectionTrait>(db: &C) -> Result<(), String> {
        let focus_rows = focus::Entity::find()
            .all(db)
            .map_err(|error| error.to_string())?;
        let segments_by_id = segments::Entity::find()
            .all(db)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|row| (row.id, row))
            .collect::<HashMap<_, _>>();
        let entry_rows = entries::Entity::find()
            .filter(entries::Column::DeletedAt.is_null())
            .all(db)
            .map_err(|error| error.to_string())?;
        let mut existing = task_time_links::Entity::find()
            .all(db)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|row| row.entry_id)
            .collect::<std::collections::HashSet<_>>();
        for row in focus_rows {
            let Some(segment) = segments_by_id.get(&row.segment_id) else {
                continue;
            };
            let segment_end = segment.ended_at.unwrap_or(i64::MAX);
            for entry in &entry_rows {
                if existing.contains(&entry.id)
                    || entry.started_at < segment.started_at
                    || entry.ended_at > segment_end
                    || entry.started_at >= entry.ended_at
                {
                    continue;
                }
                task_time_links::ActiveModel {
                    entry_id: Set(entry.id.clone()),
                    daily_task_id: Set(row.daily_task_id.clone()),
                }
                .insert(db)
                .map_err(|error| error.to_string())?;
                existing.insert(entry.id.clone());
            }
        }
        Ok(())
    }

    pub fn due<C: ConnectionTrait>(db: &C, now: i64) -> Result<Option<String>, String> {
        focus::Entity::find()
            .filter(focus::Column::State.eq("running"))
            .filter(focus::Column::RemindedAt.is_null())
            .filter(
                Expr::col(focus::Column::StartedAt)
                    .add(Expr::col(focus::Column::PlannedMinutes).mul(60_000))
                    .lte(now),
            )
            .order_by_asc(focus::Column::StartedAt)
            .one(db)
            .map(|row| row.map(|value| value.id))
            .map_err(|error| error.to_string())
    }

    pub fn mark_reminded<C: ConnectionTrait>(db: &C, id: &str, now: i64) -> Result<(), String> {
        focus::Entity::update_many()
            .col_expr(focus::Column::RemindedAt, Expr::value(now))
            .filter(focus::Column::Id.eq(id.to_owned()))
            .exec(db)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub fn allowed<C: ConnectionTrait>(
        db: &C,
        preferences: &Preferences,
        minute: u16,
        now: i64,
    ) -> Result<bool, String> {
        if !preferences.enabled
            || !preferences.reminders
            || aevio_core::coach::policies::quiet(preferences, minute)
        {
            return Ok(false);
        }
        let last = crate::database::entities::coach::interventions::Entity::find()
            .filter(
                crate::database::entities::coach::interventions::Column::Kind.eq("reminder.sent"),
            )
            .order_by_desc(crate::database::entities::coach::interventions::Column::At)
            .one(db)
            .map_err(|error| error.to_string())?;
        Ok(last.is_none_or(|row| {
            now.saturating_sub(std::cmp::max(row.at, 0))
                >= preferences.cooldown_minutes as i64 * 60_000
        }))
    }

    pub fn due_task<C: ConnectionTrait>(
        db: &C,
        now: i64,
    ) -> Result<Option<(String, String)>, String> {
        crate::database::entities::coach::tasks::Entity::find()
            .filter(crate::database::entities::coach::tasks::Column::Status.eq("open"))
            .filter(crate::database::entities::coach::tasks::Column::ReminderAt.is_not_null())
            .filter(crate::database::entities::coach::tasks::Column::ReminderAt.lte(now))
            .filter(crate::database::entities::coach::tasks::Column::NotifiedAt.is_null())
            .order_by_asc(crate::database::entities::coach::tasks::Column::ReminderAt)
            .one(db)
            .map(|row| row.map(|value| (value.id, value.title)))
            .map_err(|error| error.to_string())
    }
}
