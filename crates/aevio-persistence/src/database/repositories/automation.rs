use sea_orm::entity::prelude::*;
use sea_orm::{DatabaseConnection, DbErr, QueryFilter, QueryOrder, QuerySelect};

use crate::database::entities::automation::{
    agent_routines, entry_events, report_definitions, report_runs, routine_runs, saved_dashboards,
    time_entries,
};

pub struct AutomationRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> AutomationRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn routines(&self) -> Result<Vec<agent_routines::Model>, DbErr> {
        agent_routines::Entity::find()
            .order_by_desc(agent_routines::Column::UpdatedAt)
            .all(self.db)
    }

    pub fn reports(&self) -> Result<Vec<report_definitions::Model>, DbErr> {
        report_definitions::Entity::find()
            .order_by_desc(report_definitions::Column::UpdatedAt)
            .all(self.db)
    }

    pub fn dashboards(&self) -> Result<Vec<saved_dashboards::Model>, DbErr> {
        saved_dashboards::Entity::find()
            .order_by_desc(saved_dashboards::Column::Pinned)
            .order_by_desc(saved_dashboards::Column::UpdatedAt)
            .all(self.db)
    }

    pub fn routine_runs(&self) -> Result<Vec<routine_runs::Model>, DbErr> {
        routine_runs::Entity::find()
            .order_by_desc(routine_runs::Column::CreatedAt)
            .limit(100)
            .all(self.db)
    }

    pub fn report_runs(&self) -> Result<Vec<report_runs::Model>, DbErr> {
        report_runs::Entity::find()
            .order_by_desc(report_runs::Column::CreatedAt)
            .limit(100)
            .all(self.db)
    }

    pub fn insert_routine(&self, model: agent_routines::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn insert_report(&self, model: report_definitions::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn insert_dashboard(&self, model: saved_dashboards::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn routine(&self, id: &str) -> Result<Option<agent_routines::Model>, DbErr> {
        agent_routines::Entity::find_by_id(id.to_owned()).one(self.db)
    }

    pub fn report(&self, id: &str) -> Result<Option<report_definitions::Model>, DbErr> {
        report_definitions::Entity::find_by_id(id.to_owned()).one(self.db)
    }

    pub fn insert_routine_run(&self, model: routine_runs::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn insert_report_run(&self, model: report_runs::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn advance_routine(
        &self,
        id: &str,
        next_run_at: Option<i64>,
        now: i64,
    ) -> Result<(), DbErr> {
        agent_routines::Entity::update_many()
            .col_expr(agent_routines::Column::NextRunAt, Expr::value(next_run_at))
            .col_expr(agent_routines::Column::UpdatedAt, Expr::value(now))
            .filter(agent_routines::Column::Id.eq(id.to_owned()))
            .exec(self.db)
            .map(|_| ())
    }

    pub fn advance_report(
        &self,
        id: &str,
        next_run_at: Option<i64>,
        now: i64,
    ) -> Result<(), DbErr> {
        report_definitions::Entity::update_many()
            .col_expr(
                report_definitions::Column::NextRunAt,
                Expr::value(next_run_at),
            )
            .col_expr(report_definitions::Column::UpdatedAt, Expr::value(now))
            .filter(report_definitions::Column::Id.eq(id.to_owned()))
            .exec(self.db)
            .map(|_| ())
    }

    pub fn due_routines(&self, now: i64) -> Result<Vec<agent_routines::Model>, DbErr> {
        agent_routines::Entity::find()
            .filter(agent_routines::Column::Active.eq(1_i64))
            .filter(agent_routines::Column::NextRunAt.is_not_null())
            .filter(agent_routines::Column::NextRunAt.lte(now))
            .all(self.db)
    }

    pub fn due_reports(&self, now: i64) -> Result<Vec<report_definitions::Model>, DbErr> {
        report_definitions::Entity::find()
            .filter(report_definitions::Column::Active.eq(1_i64))
            .filter(report_definitions::Column::NextRunAt.is_not_null())
            .filter(report_definitions::Column::NextRunAt.lte(now))
            .all(self.db)
    }

    pub fn overlapping_entries(
        &self,
        start: i64,
        end: i64,
    ) -> Result<Vec<time_entries::Model>, DbErr> {
        time_entries::Entity::find()
            .filter(time_entries::Column::DeletedAt.is_null())
            .filter(time_entries::Column::StartedAt.lt(end))
            .filter(time_entries::Column::EndedAt.gt(start))
            .all(self.db)
    }

    pub fn auto_approved_count(&self, start: i64, end: i64) -> Result<u32, DbErr> {
        let count = entry_events::Entity::find()
            .filter(entry_events::Column::Kind.eq("accepted"))
            .filter(entry_events::Column::Actor.eq("auto"))
            .filter(entry_events::Column::At.gte(start))
            .filter(entry_events::Column::At.lt(end))
            .count(self.db)?;
        Ok(count as u32)
    }

    pub fn report_entries(&self, start: i64, end: i64) -> Result<Vec<time_entries::Model>, DbErr> {
        time_entries::Entity::find()
            .filter(time_entries::Column::StartedAt.lt(end))
            .filter(time_entries::Column::EndedAt.gt(start))
            .all(self.db)
    }
}
