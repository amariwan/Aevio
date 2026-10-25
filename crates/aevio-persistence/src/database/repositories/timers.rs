use sea_orm::entity::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue::Set, DatabaseConnection, DbErr, QueryOrder, TransactionTrait};

use crate::database::entities::{settings, timer_segments, timers};

pub struct TimerRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> TimerRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn list(&self) -> Result<(Vec<timers::Model>, Vec<timer_segments::Model>), DbErr> {
        let timer_rows = timers::Entity::find()
            .order_by_asc(timers::Column::CreatedAt)
            .order_by_asc(timers::Column::Id)
            .all(self.db)?;
        let ids = timer_rows
            .iter()
            .map(|timer| timer.id.clone())
            .collect::<Vec<_>>();
        let segment_rows = if ids.is_empty() {
            Vec::new()
        } else {
            timer_segments::Entity::find()
                .filter(timer_segments::Column::TimerId.is_in(ids))
                .order_by_asc(timer_segments::Column::RecordedAt)
                .order_by_asc(timer_segments::Column::Id)
                .all(self.db)?
        };
        Ok((timer_rows, segment_rows))
    }

    pub fn find(&self, id: &str) -> Result<Option<timers::Model>, DbErr> {
        timers::Entity::find_by_id(id.to_owned()).one(self.db)
    }

    pub fn insert(&self, model: timers::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn set_started_at(&self, id: &str, started_at: Option<i64>) -> Result<(), DbErr> {
        timers::Entity::update_many()
            .col_expr(timers::Column::StartedAt, Expr::value(started_at))
            .filter(timers::Column::Id.eq(id.to_owned()))
            .exec(self.db)
            .map(|_| ())
    }

    pub fn rename(&self, id: &str, label: &str) -> Result<(), DbErr> {
        timers::Entity::update_many()
            .col_expr(timers::Column::Label, Expr::value(label.to_owned()))
            .filter(timers::Column::Id.eq(id.to_owned()))
            .exec(self.db)
            .map(|_| ())
    }

    pub fn insert_segment(
        &self,
        timer_id: &str,
        duration_ms: i64,
        recorded_at: i64,
    ) -> Result<(), DbErr> {
        timer_segments::ActiveModel {
            timer_id: Set(timer_id.to_owned()),
            duration_ms: Set(duration_ms),
            recorded_at: Set(recorded_at),
            ..Default::default()
        }
        .insert(self.db)
        .map(|_| ())
    }

    pub fn update_segment(
        &self,
        timer_id: &str,
        segment_id: i64,
        duration_ms: i64,
    ) -> Result<bool, DbErr> {
        let result = timer_segments::Entity::update_many()
            .col_expr(timer_segments::Column::DurationMs, Expr::value(duration_ms))
            .filter(timer_segments::Column::Id.eq(segment_id))
            .filter(timer_segments::Column::TimerId.eq(timer_id.to_owned()))
            .exec(self.db)?;
        Ok(result.rows_affected > 0)
    }

    pub fn delete_segment(&self, timer_id: &str, segment_id: i64) -> Result<bool, DbErr> {
        let result = timer_segments::Entity::delete_many()
            .filter(timer_segments::Column::Id.eq(segment_id))
            .filter(timer_segments::Column::TimerId.eq(timer_id.to_owned()))
            .exec(self.db)?;
        Ok(result.rows_affected > 0)
    }

    pub fn reset(&self, id: &str) -> Result<(), DbErr> {
        let transaction = self.db.begin()?;
        timers::Entity::update_many()
            .col_expr(timers::Column::StartedAt, Expr::value(None::<i64>))
            .filter(timers::Column::Id.eq(id.to_owned()))
            .exec(&transaction)?;
        timer_segments::Entity::delete_many()
            .filter(timer_segments::Column::TimerId.eq(id.to_owned()))
            .exec(&transaction)?;
        transaction.commit()
    }

    pub fn pause(&self, id: &str, duration_ms: i64, recorded_at: i64) -> Result<(), DbErr> {
        let transaction = self.db.begin()?;
        timers::Entity::update_many()
            .col_expr(timers::Column::StartedAt, Expr::value(None::<i64>))
            .filter(timers::Column::Id.eq(id.to_owned()))
            .exec(&transaction)?;
        timer_segments::ActiveModel {
            timer_id: Set(id.to_owned()),
            duration_ms: Set(duration_ms),
            recorded_at: Set(recorded_at),
            ..Default::default()
        }
        .insert(&transaction)?;
        transaction.commit()
    }

    pub fn delete(&self, id: &str) -> Result<bool, DbErr> {
        let transaction = self.db.begin()?;
        let result = timers::Entity::delete_by_id(id.to_owned()).exec(&transaction)?;
        if result.rows_affected == 0 {
            transaction.rollback()?;
            return Ok(false);
        }
        timer_segments::Entity::delete_many()
            .filter(timer_segments::Column::TimerId.eq(id.to_owned()))
            .exec(&transaction)?;
        transaction.commit()?;
        Ok(true)
    }

    pub fn read_setting(&self, key: &str) -> Result<Option<String>, DbErr> {
        settings::Entity::find_by_id(key.to_owned())
            .one(self.db)
            .map(|model| model.map(|model| model.value))
    }

    pub fn write_setting(&self, key: &str, value: &str) -> Result<(), DbErr> {
        settings::Entity::insert(settings::ActiveModel {
            key: Set(key.to_owned()),
            value: Set(value.to_owned()),
        })
        .on_conflict(
            OnConflict::column(settings::Column::Key)
                .update_column(settings::Column::Value)
                .to_owned(),
        )
        .exec(self.db)
        .map(|_| ())
    }
}
