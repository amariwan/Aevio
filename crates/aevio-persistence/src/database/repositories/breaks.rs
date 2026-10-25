use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, QueryFilter, QueryOrder};

use crate::database::entities::breaks::{self, segments};

pub struct BreakRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> BreakRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn insert(&self, row: breaks::ActiveModel) -> Result<(), DbErr> {
        row.insert(self.db).map(|_| ())
    }

    pub fn set_ended(&self, id: &str, ended_at: i64, now: i64) -> Result<(), DbErr> {
        breaks::Entity::update_many()
            .col_expr(breaks::Column::EndedAt, Expr::value(ended_at))
            .col_expr(breaks::Column::UpdatedAt, Expr::value(now))
            .filter(breaks::Column::Id.eq(id.to_owned()))
            .exec(self.db)
            .map(|_| ())
    }

    pub fn set_planned(&self, id: &str, planned_ms: i64, now: i64) -> Result<(), DbErr> {
        breaks::Entity::update_many()
            .col_expr(breaks::Column::PlannedMs, Expr::value(planned_ms))
            .col_expr(breaks::Column::UpdatedAt, Expr::value(now))
            .filter(breaks::Column::Id.eq(id.to_owned()))
            .exec(self.db)
            .map(|_| ())
    }

    pub fn close_orphans(&self, now: i64) -> Result<(), DbErr> {
        let rows = breaks::Entity::find()
            .filter(breaks::Column::Status.eq("taken"))
            .filter(breaks::Column::StartedAt.is_not_null())
            .filter(breaks::Column::EndedAt.is_null())
            .all(self.db)?;
        let ids: Vec<i64> = rows.iter().filter_map(|row| row.segment_id).collect();
        let segments = if ids.is_empty() {
            Vec::new()
        } else {
            segments::Entity::find()
                .filter(segments::Column::Id.is_in(ids))
                .all(self.db)?
        };
        for row in rows {
            let ended_at = row
                .segment_id
                .and_then(|id| segments.iter().find(|segment| segment.id == id))
                .and_then(|segment| segment.ended_at)
                .or(row.started_at);
            if let Some(ended_at) = ended_at {
                self.set_ended(&row.id, ended_at, now)?;
            }
        }
        Ok(())
    }

    pub fn list(&self, since: i64, until: i64) -> Result<Vec<breaks::Model>, DbErr> {
        let mut rows = breaks::Entity::find().all(self.db)?;
        rows.retain(|row| {
            let event_at = row.started_at.or(row.due_at).unwrap_or(row.created_at);
            event_at >= since && event_at < until
        });
        rows.sort_by(|left, right| {
            left.started_at
                .or(left.due_at)
                .unwrap_or(left.created_at)
                .cmp(
                    &right
                        .started_at
                        .or(right.due_at)
                        .unwrap_or(right.created_at),
                )
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(rows)
    }

    pub fn segments_since(&self, day_start: i64, now: i64) -> Result<Vec<segments::Model>, DbErr> {
        let mut segments = segments::Entity::find()
            .order_by_asc(segments::Column::StartedAt)
            .order_by_asc(segments::Column::Id)
            .all(self.db)?;
        segments.retain(|segment| segment.ended_at.unwrap_or(now) > day_start);
        Ok(segments)
    }
}
