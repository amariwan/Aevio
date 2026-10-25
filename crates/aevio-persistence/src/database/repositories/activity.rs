use sea_orm::entity::prelude::*;
use sea_orm::{
    ActiveModelTrait, Condition, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    Set,
};

use crate::database::entities::activity::{daily_rollups, segments};
use crate::database::entities::entry_query::entries;
use aevio_core::entry_builder::{SegmentInput, describe};
use aevio_core::models::ActivitySegment;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ActivityTotals {
    pub tracked_ms: u64,
    pub focus_ms: u64,
    pub break_ms: u64,
    pub unreviewed: u64,
}

pub struct ActivityRepository<'db> {
    db: &'db DatabaseConnection,
}

pub struct NewSegment<'a> {
    pub app: &'a str,
    pub title: &'a str,
    pub kind: &'a str,
    pub label: Option<&'a str>,
    pub started_at: i64,
    pub bundle_id: Option<&'a str>,
    pub url: Option<&'a str>,
    pub domain: Option<&'a str>,
}

impl<'db> ActivityRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn open_segment(&self, input: NewSegment<'_>) -> Result<i64, DbErr> {
        let row = segments::ActiveModel {
            app: Set(input.app.to_owned()),
            title: Set(input.title.to_owned()),
            kind: Set(input.kind.to_owned()),
            label: Set(input.label.map(str::to_owned)),
            started_at: Set(input.started_at),
            ended_at: Set(None),
            reviewed: Set(0),
            app_id: Set(None),
            bundle_id: Set(input.bundle_id.map(str::to_owned)),
            url: Set(input.url.map(str::to_owned)),
            domain: Set(input.domain.map(str::to_owned)),
            entry_id: Set(None),
            ..Default::default()
        }
        .insert(self.db)?;
        Ok(row.id)
    }

    pub fn close_segment(&self, id: i64, ended_at: i64) -> Result<(), DbErr> {
        segments::Entity::update_many()
            .col_expr(segments::Column::EndedAt, Expr::value(ended_at))
            .filter(segments::Column::Id.eq(id))
            .exec(self.db)
            .map(|_| ())
    }

    /// Releases capture that continued past a frozen entry's end. The
    /// operation stays in one SeaORM transaction so the owner lookup, cut,
    /// and relink cannot expose a half-released segment.
    pub fn release_live_tail(
        &self,
        segment_id: i64,
        now: i64,
    ) -> Result<Option<(i64, i64)>, DbErr> {
        self.db
            .transaction(|tx| {
                let Some(segment) = segments::Entity::find_by_id(segment_id).one(tx)? else {
                    return Ok(None);
                };
                let Some(entry_id) = segment.entry_id.clone() else {
                    return Ok(None);
                };
                let Some(entry) =
                    crate::database::entities::entry_query::entries::Entity::find_by_id(
                        entry_id.clone(),
                    )
                    .one(tx)?
                else {
                    return Ok(None);
                };
                if segment.ended_at.is_some()
                    || entry.status == "building"
                    || entry.deleted_at.is_some()
                    || entry.ended_at >= now
                {
                    return Ok(None);
                }
                if segment.started_at >= entry.ended_at {
                    segments::Entity::update_many()
                        .col_expr(segments::Column::EntryId, Expr::value(None::<String>))
                        .filter(segments::Column::Id.eq(segment_id))
                        .exec(tx)?;
                    return Ok(None);
                }
                let mut original: segments::ActiveModel = segment.clone().into();
                original.ended_at = Set(Some(entry.ended_at));
                original.update(tx)?;
                let piece = segments::ActiveModel {
                    id: Default::default(),
                    app: Set(segment.app),
                    title: Set(segment.title),
                    kind: Set(segment.kind),
                    label: Set(segment.label),
                    started_at: Set(entry.ended_at),
                    ended_at: Set(segment.ended_at),
                    reviewed: Set(segment.reviewed),
                    app_id: Set(segment.app_id),
                    bundle_id: Set(segment.bundle_id),
                    url: Set(segment.url),
                    domain: Set(segment.domain),
                    entry_id: Set(None),
                }
                .insert(tx)?;
                Ok(Some((piece.id, entry.ended_at)))
            })
            .map_err(|error| match error {
                sea_orm::TransactionError::Connection(error)
                | sea_orm::TransactionError::Transaction(error) => error,
            })
    }

    pub fn recover_open_segments(&self) -> Result<(), DbErr> {
        let rows = segments::Entity::find()
            .filter(segments::Column::EndedAt.is_null())
            .all(self.db)?;
        for row in rows {
            let started_at = row.started_at;
            let mut model: segments::ActiveModel = row.into();
            model.ended_at = Set(Some(started_at));
            model.update(self.db)?;
        }
        Ok(())
    }

    pub fn rollup_old_segments(&self, cutoff: i64, day_ms: i64) -> Result<(), DbErr> {
        let rows = segments::Entity::find()
            .filter(segments::Column::EndedAt.is_not_null())
            .filter(segments::Column::EndedAt.lt(cutoff))
            .all(self.db)?;
        let mut totals = std::collections::HashMap::<(i64, String), i64>::new();
        for row in rows {
            let day = row.started_at.div_euclid(day_ms);
            let duration = row.ended_at.unwrap_or(row.started_at) - row.started_at;
            *totals.entry((day, row.kind)).or_default() += duration;
        }
        for ((day_epoch, kind), total_ms) in totals {
            if let Some(row) = daily_rollups::Entity::find()
                .filter(daily_rollups::Column::DayEpoch.eq(day_epoch))
                .filter(daily_rollups::Column::Kind.eq(kind.clone()))
                .one(self.db)?
            {
                let mut model: daily_rollups::ActiveModel = row.into();
                model.total_ms = Set(total_ms);
                model.update(self.db)?;
            } else {
                daily_rollups::ActiveModel {
                    day_epoch: Set(day_epoch),
                    kind: Set(kind),
                    total_ms: Set(total_ms),
                }
                .insert(self.db)?;
            }
        }
        Ok(())
    }

    pub fn mark_reviewed(&self, id: i64) -> Result<(), DbErr> {
        segments::Entity::update_many()
            .col_expr(segments::Column::Reviewed, Expr::value(1))
            .filter(segments::Column::Id.eq(id))
            .exec(self.db)
            .map(|_| ())
    }

    pub fn describe_closed(&self, id: &str) -> Result<(), DbErr> {
        let Some(entry) = entries::Entity::find_by_id(id.to_owned()).one(self.db)? else {
            return Ok(());
        };
        if entry.status == "building" || entry.description_origin != "template" {
            return Ok(());
        }
        let rows = segments::Entity::find()
            .filter(segments::Column::EntryId.eq(id.to_owned()))
            .order_by_asc(segments::Column::StartedAt)
            .all(self.db)?;
        if rows.iter().all(|segment| segment.kind == "break") {
            return Ok(());
        }
        let input = rows
            .into_iter()
            .map(|row| SegmentInput {
                id: row.id,
                app: row.app,
                title: row.title,
                kind: row.kind,
                label: row.label,
                started_at: row.started_at as u64,
                ended_at: Some(row.ended_at.unwrap_or(entry.ended_at) as u64),
                entry_id: row.entry_id,
            })
            .collect::<Vec<_>>();
        entries::Entity::update_many()
            .col_expr(entries::Column::Description, Expr::value(describe(&input)))
            .filter(entries::Column::Id.eq(id.to_owned()))
            .exec(self.db)?;
        Ok(())
    }

    pub fn snapshot_data(
        &self,
        since_ms: i64,
        now: i64,
    ) -> Result<(Vec<ActivitySegment>, ActivityTotals), DbErr> {
        let rows = segments::Entity::find()
            .filter(
                Condition::any()
                    .add(segments::Column::EndedAt.is_null())
                    .add(
                        Condition::all()
                            .add(segments::Column::EndedAt.gte(since_ms))
                            .add(segments::Column::StartedAt.lt(now)),
                    ),
            )
            .order_by_asc(segments::Column::StartedAt)
            .all(self.db)?;
        let mut totals = ActivityTotals::default();
        for row in rows.iter().filter(|row| row.ended_at.is_some()) {
            let duration = row
                .ended_at
                .unwrap_or_default()
                .saturating_sub(row.started_at.max(since_ms)) as u64;
            match row.kind.as_str() {
                "activity" => {
                    totals.tracked_ms += duration;
                    if row.reviewed == 0 {
                        totals.unreviewed += 1;
                    }
                }
                "focus" => {
                    totals.tracked_ms += duration;
                    totals.focus_ms += duration;
                }
                "meeting" => totals.tracked_ms += duration,
                "break" => totals.break_ms += duration,
                _ => {}
            }
        }
        let segments = rows.into_iter().map(to_segment).collect();
        Ok((segments, totals))
    }

    pub fn purge_older_than(&self, cutoff: i64, day_epoch: i64) -> Result<u64, DbErr> {
        self.db
            .transaction::<_, _, DbErr>(|txn| {
                let removed = segments::Entity::delete_many()
                    .filter(segments::Column::EndedAt.is_not_null())
                    .filter(segments::Column::EndedAt.lt(cutoff))
                    .exec(txn)?
                    .rows_affected;
                breaks::Entity::delete_many()
                    .filter(breaks::Column::CreatedAt.lt(cutoff))
                    .exec(txn)?;
                daily_rollups::Entity::delete_many()
                    .filter(daily_rollups::Column::DayEpoch.lt(day_epoch))
                    .exec(txn)?;
                Ok(removed)
            })
            .map_err(|error| DbErr::Custom(error.to_string()))
    }
}

fn to_segment(row: segments::Model) -> ActivitySegment {
    ActivitySegment {
        id: row.id,
        app: row.app,
        title: row.title,
        kind: row.kind,
        label: row.label,
        started_at: row.started_at.max(0) as u64,
        ended_at: row.ended_at.map(|value| value.max(0) as u64),
        reviewed: row.reviewed != 0,
        app_id: row.app_id,
        bundle_id: row.bundle_id,
        url: row.url,
        domain: row.domain,
        entry_id: row.entry_id,
    }
}

use crate::database::entities::breaks;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::migrations::run_migrations;
    use crate::database::connect;
    use rusqlite::Connection;

    #[test]
    fn segment_lifecycle_uses_typed_models_and_preserves_nullable_fields() {
        let path = std::env::temp_dir().join(format!("aevio-activity-{}.db", uuid::Uuid::now_v7()));
        let mut sqlite = Connection::open(&path).unwrap();
        run_migrations(&mut sqlite).unwrap();
        drop(sqlite);

        let database = connect(&path).unwrap();
        let repository = ActivityRepository::new(&database);
        let id = repository
            .open_segment(NewSegment {
                app: "Editor",
                title: "main.rs",
                kind: "activity",
                label: None,
                started_at: 100,
                bundle_id: Some("com.example.Editor"),
                url: None,
                domain: None,
            })
            .unwrap();
        repository.mark_reviewed(id).unwrap();
        repository.close_segment(id, 200).unwrap();

        let row = segments::Entity::find_by_id(id)
            .one(&database)
            .unwrap()
            .unwrap();
        assert_eq!(row.ended_at, Some(200));
        assert_eq!(row.reviewed, 1);
        assert_eq!(row.bundle_id.as_deref(), Some("com.example.Editor"));

        drop(database);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn release_live_tail_cuts_and_unlinks_the_recording_piece_atomically() {
        let path =
            std::env::temp_dir().join(format!("aevio-activity-tail-{}.db", uuid::Uuid::now_v7()));
        let mut sqlite = Connection::open(&path).unwrap();
        run_migrations(&mut sqlite).unwrap();
        drop(sqlite);

        let database = connect(&path).unwrap();
        let repository = ActivityRepository::new(&database);
        let segment_id = repository
            .open_segment(NewSegment {
                app: "Editor",
                title: "main.rs",
                kind: "activity",
                label: None,
                started_at: 100,
                bundle_id: None,
                url: None,
                domain: None,
            })
            .unwrap();
        crate::database::entities::entry_query::entries::ActiveModel {
            id: Set("entry-1".into()),
            started_at: Set(0),
            ended_at: Set(500),
            description: Set("work".into()),
            category_id: Set(None),
            project_id: Set(None),
            task_id: Set(None),
            status: Set("pending".into()),
            approved_by: Set(None),
            source: Set("auto".into()),
            billable: Set(0),
            invoice_id: Set(None),
            created_at: Set(0),
            updated_at: Set(0),
            deleted_at: Set(None),
            description_origin: Set("template".into()),
        }
        .insert(&database)
        .unwrap();
        crate::database::entities::activity::segments::Entity::update_many()
            .col_expr(segments::Column::EntryId, Expr::value("entry-1"))
            .filter(segments::Column::Id.eq(segment_id))
            .exec(&database)
            .unwrap();

        let released = repository.release_live_tail(segment_id, 1_000).unwrap();
        let (new_id, started_at) = released.unwrap();
        assert_eq!(started_at, 500);
        assert_eq!(
            segments::Entity::find_by_id(segment_id)
                .one(&database)
                .unwrap()
                .unwrap()
                .ended_at,
            Some(500)
        );
        assert_eq!(
            segments::Entity::find_by_id(new_id)
                .one(&database)
                .unwrap()
                .unwrap()
                .entry_id,
            None
        );

        drop(database);
        std::fs::remove_file(path).unwrap();
    }
}
