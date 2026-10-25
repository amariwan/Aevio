use sea_orm::entity::prelude::*;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    Set,
};

use crate::database::entities::coach::{daily_plans, daily_reviews, interventions, tasks};
use crate::database::entities::entry_query::segments;
use crate::database::repositories::coach_commands::CoachCommandRepository;
use crate::database::repositories::coach_focus::CoachFocusRepository;
use aevio_core::coach::models::Preferences;

pub struct CoachReminderRepository;

impl CoachReminderRepository {
    pub fn mark_task_and_sent(
        database: &DatabaseConnection,
        due_task: Option<(String, String)>,
        day: &str,
        kind: &str,
        now: i64,
    ) -> Result<(), String> {
        database
            .transaction(|tx| {
                if let Some((id, _)) = due_task {
                    Self::mark_task(tx, &id, now).map_err(sea_orm::DbErr::Custom)?;
                }
                Self::mark_sent(tx, day, kind, now).map_err(sea_orm::DbErr::Custom)?;
                Ok::<(), sea_orm::DbErr>(())
            })
            .map_err(|error| error.to_string())
    }

    pub fn review_due<C: ConnectionTrait>(
        db: &C,
        day: &str,
        minute: u16,
        preferences: &Preferences,
    ) -> Result<bool, String> {
        if minute < preferences.review_minute {
            return Ok(false);
        }
        let Some(plan) = daily_plans::Entity::find_by_id(day.to_owned())
            .one(db)
            .map_err(|error| error.to_string())?
        else {
            return Ok(false);
        };
        if plan.state != "planned" {
            return Ok(false);
        }
        let reviewed = daily_reviews::Entity::find_by_id(day.to_owned())
            .one(db)
            .map_err(|error| error.to_string())?
            .is_some_and(|review| review.outcome != "not_reviewed");
        let offered = interventions::Entity::find()
            .filter(interventions::Column::Day.eq(day.to_owned()))
            .filter(interventions::Column::Kind.eq("review.offered"))
            .one(db)
            .map_err(|error| error.to_string())?
            .is_some();
        Ok(!reviewed && !offered)
    }

    pub fn mark_sent<C: ConnectionTrait>(
        db: &C,
        day: &str,
        kind: &str,
        now: i64,
    ) -> Result<(), String> {
        CoachCommandRepository::audit(db, day, "reminder.sent", now)
            .map_err(|error| error.to_string())?;
        CoachCommandRepository::audit(db, day, kind, now).map_err(|error| error.to_string())
    }

    pub fn mark_task<C: ConnectionTrait>(db: &C, id: &str, now: i64) -> Result<(), String> {
        let Some(row) = tasks::Entity::find_by_id(id.to_owned())
            .one(db)
            .map_err(|error| error.to_string())?
        else {
            return Ok(());
        };
        let mut model: tasks::ActiveModel = row.into();
        model.notified_at = Set(Some(now));
        model.update(db).map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn guardian_due<C: ConnectionTrait>(db: &C, now: i64) -> Result<bool, String> {
        let rows = segments::Entity::find()
            .filter(segments::Column::Kind.eq("activity"))
            .filter(segments::Column::StartedAt.gte(now.saturating_sub(10 * 60_000)))
            .order_by_asc(segments::Column::StartedAt)
            .order_by_asc(segments::Column::Id)
            .all(db)
            .map_err(|error| error.to_string())?;
        let switches = rows
            .windows(2)
            .filter(|pair| pair[0].app != pair[1].app)
            .count();
        Ok(switches >= 8)
    }

    pub fn reconcile_and_attribute<C: ConnectionTrait>(db: &C, now: i64) -> Result<(), String> {
        CoachFocusRepository::reconcile(db, now)?;
        CoachFocusRepository::attribute_entries(db)
    }
}
