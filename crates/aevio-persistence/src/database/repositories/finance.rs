use crate::database::entities::finance::{
    capacity_plans, clients, contracts, dashboard_shares, projects, rate_cards, time_entries,
};
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder};

pub struct FinanceRepository<'db> {
    db: &'db DatabaseConnection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStatsRow {
    pub project_id: String,
    pub entries: u32,
    pub total_ms: u64,
    pub range_ms: u64,
    pub month_ms: u64,
    pub billable_ms: u64,
    pub billable_month_ms: u64,
    pub last_activity: Option<u64>,
    pub unbilled_ms: u64,
    pub unbilled_entries: u32,
}
impl<'db> FinanceRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }
    pub fn entries(&self, start: i64, end: i64) -> Result<Vec<time_entries::Model>, DbErr> {
        time_entries::Entity::find()
            .filter(time_entries::Column::DeletedAt.is_null())
            .filter(time_entries::Column::EndedAt.gt(start))
            .filter(time_entries::Column::StartedAt.lt(end))
            .all(self.db)
    }

    pub fn project_stats(
        &self,
        range_start: i64,
        range_end: i64,
        month_start: i64,
    ) -> Result<Vec<ProjectStatsRow>, DbErr> {
        let rows = self
            .entries(i64::MIN, i64::MAX)
            .map(|rows| rows.into_iter().filter(|row| row.project_id.is_some()))?
            .fold(
                std::collections::HashMap::<String, ProjectStatsRow>::new(),
                |mut acc, row| {
                    let Some(project_id) = row.project_id else {
                        return acc;
                    };
                    let duration = row.ended_at.saturating_sub(row.started_at).max(0) as u64;
                    let in_range = row.started_at >= range_start && row.started_at < range_end;
                    let in_month = row.started_at >= month_start;
                    let billable = row.billable != 0;
                    let entry = acc.entry(project_id.clone()).or_insert(ProjectStatsRow {
                        project_id,
                        entries: 0,
                        total_ms: 0,
                        range_ms: 0,
                        month_ms: 0,
                        billable_ms: 0,
                        billable_month_ms: 0,
                        last_activity: None,
                        unbilled_ms: 0,
                        unbilled_entries: 0,
                    });
                    entry.entries += 1;
                    entry.total_ms += duration;
                    if in_range {
                        entry.range_ms += duration;
                    }
                    if in_month {
                        entry.month_ms += duration;
                    }
                    if billable {
                        entry.billable_ms += duration;
                        if in_month {
                            entry.billable_month_ms += duration;
                        }
                    }
                    entry.last_activity = Some(
                        entry
                            .last_activity
                            .unwrap_or(0)
                            .max(row.ended_at.max(0) as u64),
                    );
                    if row.status == "approved" && billable && row.invoice_id.is_none() {
                        entry.unbilled_ms += duration;
                        entry.unbilled_entries += 1;
                    }
                    acc
                },
            );
        let mut rows = rows.into_values().collect::<Vec<_>>();
        rows.sort_by(|left, right| left.project_id.cmp(&right.project_id));
        Ok(rows)
    }
    pub fn projects(&self) -> Result<Vec<projects::Model>, DbErr> {
        projects::Entity::find().all(self.db)
    }
    pub fn clients(&self) -> Result<Vec<clients::Model>, DbErr> {
        clients::Entity::find().all(self.db)
    }
    pub fn rates(&self) -> Result<Vec<rate_cards::Model>, DbErr> {
        rate_cards::Entity::find()
            .order_by_asc(rate_cards::Column::ProjectId)
            .all(self.db)
    }
    pub fn capacity(&self, start: i64, end: i64) -> Result<Vec<capacity_plans::Model>, DbErr> {
        capacity_plans::Entity::find()
            .filter(capacity_plans::Column::PeriodEnd.gt(start))
            .filter(capacity_plans::Column::PeriodStart.lt(end))
            .all(self.db)
    }
    pub fn contracts(&self) -> Result<Vec<contracts::Model>, DbErr> {
        contracts::Entity::find()
            .order_by_desc(contracts::Column::StartDate)
            .all(self.db)
    }
    pub fn shares(&self, dashboard_id: &str) -> Result<Vec<dashboard_shares::Model>, DbErr> {
        dashboard_shares::Entity::find()
            .filter(dashboard_shares::Column::DashboardId.eq(dashboard_id.to_owned()))
            .order_by_desc(dashboard_shares::Column::CreatedAt)
            .all(self.db)
    }
    pub fn save_rate(&self, row: rate_cards::ActiveModel) -> Result<(), DbErr> {
        rate_cards::Entity::insert(row)
            .on_conflict(
                OnConflict::column(rate_cards::Column::ProjectId)
                    .update_columns([
                        rate_cards::Column::BillRateCents,
                        rate_cards::Column::CostRateCents,
                        rate_cards::Column::Currency,
                        rate_cards::Column::EffectiveFrom,
                    ])
                    .to_owned(),
            )
            .exec(self.db)
            .map(|_| ())
    }
    pub fn save_contract(&self, row: contracts::ActiveModel) -> Result<(), DbErr> {
        contracts::Entity::insert(row)
            .on_conflict(
                OnConflict::column(contracts::Column::Id)
                    .update_columns([
                        contracts::Column::ClientId,
                        contracts::Column::ProjectId,
                        contracts::Column::Kind,
                        contracts::Column::Currency,
                        contracts::Column::StartDate,
                        contracts::Column::EndDate,
                        contracts::Column::RecurringCents,
                        contracts::Column::IncludedHoursHundredths,
                        contracts::Column::OverageRateCents,
                        contracts::Column::Status,
                    ])
                    .to_owned(),
            )
            .exec(self.db)
            .map(|_| ())
    }
    pub fn save_capacity(&self, row: capacity_plans::ActiveModel) -> Result<(), DbErr> {
        capacity_plans::Entity::insert(row)
            .on_conflict(
                OnConflict::column(capacity_plans::Column::Id)
                    .update_columns([
                        capacity_plans::Column::ProjectId,
                        capacity_plans::Column::PersonKey,
                        capacity_plans::Column::PeriodStart,
                        capacity_plans::Column::PeriodEnd,
                        capacity_plans::Column::PlannedMinutes,
                        capacity_plans::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(self.db)
            .map(|_| ())
    }
    pub fn save_share(&self, row: dashboard_shares::ActiveModel) -> Result<(), DbErr> {
        row.insert(self.db).map(|_| ())
    }
    pub fn delete_share(&self, id: &str) -> Result<(), DbErr> {
        dashboard_shares::Entity::delete_by_id(id.to_owned())
            .exec(self.db)
            .map(|_| ())
    }
}
