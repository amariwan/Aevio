//! Financial analysis and planning. All durable access is delegated to the
//! SeaORM-sync finance repository; aggregation is deliberately performed in
//! Rust so the legacy overlap and NULL semantics are explicit and testable.

use aevio_core::timers::now_epoch_ms;
use aevio_persistence::database::DatabaseConnection;
use aevio_persistence::database::entities::finance::{
    capacity_plans, contracts, dashboard_shares, rate_cards,
};
use aevio_persistence::database::repositories::finance::FinanceRepository;
use sea_orm::{ActiveValue::Set, EntityTrait, QueryOrder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinanceFilter {
    pub start_ms: u64,
    pub end_ms: u64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancePivotRow {
    pub project_id: Option<String>,
    pub project_name: String,
    pub client_id: Option<String>,
    pub client_name: Option<String>,
    pub tracked_ms: u64,
    pub planned_ms: u64,
    pub billable_ms: u64,
    pub revenue_cents: i64,
    pub cost_cents: i64,
    pub margin_cents: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_hours_hundredths: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_amount_cents: Option<i64>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinanceWarning {
    pub project_id: Option<String>,
    pub kind: String,
    pub message: String,
    pub severity: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinanceSnapshot {
    pub start_ms: u64,
    pub end_ms: u64,
    pub tracked_ms: u64,
    pub planned_ms: u64,
    pub billable_ms: u64,
    pub revenue_cents: i64,
    pub cost_cents: i64,
    pub margin_cents: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_percent: Option<f64>,
    pub rows: Vec<FinancePivotRow>,
    pub warnings: Vec<FinanceWarning>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateCard {
    pub project_id: String,
    pub bill_rate_cents: i64,
    pub cost_rate_cents: i64,
    pub currency: String,
    pub effective_from: i64,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateCardInput {
    pub project_id: String,
    pub bill_rate_cents: i64,
    pub cost_rate_cents: i64,
    pub currency: String,
    pub effective_from: Option<i64>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Contract {
    pub id: String,
    pub client_id: String,
    pub project_id: Option<String>,
    pub kind: String,
    pub currency: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub recurring_cents: i64,
    pub included_hours_hundredths: i64,
    pub overage_rate_cents: i64,
    pub status: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractInput {
    pub id: Option<String>,
    pub client_id: String,
    pub project_id: Option<String>,
    pub kind: String,
    pub currency: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub recurring_cents: i64,
    pub included_hours_hundredths: i64,
    pub overage_rate_cents: i64,
    pub status: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardShare {
    pub id: String,
    pub dashboard_id: String,
    pub token: String,
    pub permission: String,
    pub expires_at: Option<i64>,
    pub created_at: i64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapacityPlan {
    pub id: String,
    pub project_id: Option<String>,
    pub person_key: String,
    pub period_start: i64,
    pub period_end: i64,
    pub planned_minutes: i64,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapacityPlanInput {
    pub id: Option<String>,
    pub project_id: Option<String>,
    pub person_key: String,
    pub period_start: i64,
    pub period_end: i64,
    pub planned_minutes: i64,
}

fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn overlap(start: i64, end: i64, lo: i64, hi: i64) -> u64 {
    (end.min(hi) - start.max(lo)).max(0) as u64
}
fn rate(
    row: &aevio_persistence::database::entities::finance::projects::Model,
    cards: &HashMap<String, rate_cards::Model>,
) -> (i64, i64) {
    let card = cards.get(&row.id);
    (
        card.map(|v| v.bill_rate_cents)
            .unwrap_or_else(|| (row.hourly_rate.unwrap_or(0.0) * 100.0).round() as i64)
            .max(0),
        card.map(|v| v.cost_rate_cents).unwrap_or(0).max(0),
    )
}

pub fn snapshot(
    db: &DatabaseConnection,
    filter: &FinanceFilter,
) -> Result<FinanceSnapshot, String> {
    if filter.end_ms <= filter.start_ms {
        return Err("The finance range must end after it starts".into());
    }
    let start = filter.start_ms as i64;
    let end = filter.end_ms as i64;
    let repository = FinanceRepository::new(db);
    let entries = repository.entries(start, end).map_err(error)?;
    let projects = repository
        .projects()
        .map_err(error)?
        .into_iter()
        .map(|v| (v.id.clone(), v))
        .collect::<HashMap<_, _>>();
    let clients = repository
        .clients()
        .map_err(error)?
        .into_iter()
        .map(|v| (v.id.clone(), v))
        .collect::<HashMap<_, _>>();
    let cards = repository
        .rates()
        .map_err(error)?
        .into_iter()
        .map(|v| (v.project_id.clone(), v))
        .collect::<HashMap<_, _>>();
    let plans = repository.capacity(start, end).map_err(error)?;
    struct Acc {
        project_id: Option<String>,
        tracked: u64,
        planned: u64,
        billable: u64,
    }
    let mut grouped: HashMap<Option<String>, Acc> = HashMap::new();
    for entry in entries {
        let tracked = overlap(entry.started_at, entry.ended_at, start, end);
        let slot = grouped.entry(entry.project_id.clone()).or_insert(Acc {
            project_id: entry.project_id.clone(),
            tracked: 0,
            planned: 0,
            billable: 0,
        });
        slot.tracked += tracked;
        if entry.billable != 0 {
            slot.billable += tracked;
        }
    }
    for item in grouped.values_mut() {
        item.planned = plans
            .iter()
            .filter(|plan| plan.project_id == item.project_id)
            .map(|plan| {
                let period = (plan.period_end - plan.period_start).max(0) as u64;
                if period == 0 {
                    0
                } else {
                    (plan.planned_minutes.max(0) as u64
                        * 60_000
                        * overlap(plan.period_start, plan.period_end, start, end))
                    .checked_div(period)
                    .unwrap_or(0)
                }
            })
            .sum();
    }
    let mut rows = grouped
        .into_values()
        .map(|item| {
            let project = item.project_id.as_ref().and_then(|id| projects.get(id));
            let project_name = project
                .map(|v| v.name.clone())
                .unwrap_or_else(|| "No project".into());
            let client_id = project.and_then(|v| v.client_id.clone());
            let client_name = client_id
                .as_ref()
                .and_then(|id| clients.get(id))
                .map(|v| v.name.clone());
            let (bill_rate, cost_rate) = project.map(|v| rate(v, &cards)).unwrap_or((0, 0));
            let revenue = (item.billable as i128 * bill_rate as i128 / 3_600_000) as i64;
            let cost = (item.tracked as i128 * cost_rate as i128 / 3_600_000) as i64;
            let budget_hours_hundredths = project.and_then(|v| {
                (v.budget_kind == "hours")
                    .then(|| (v.budget_value.unwrap_or(0.0) * 100.0).round() as i64)
            });
            let budget_amount_cents = project.and_then(|v| {
                (v.budget_kind == "amount")
                    .then(|| (v.budget_value.unwrap_or(0.0) * 100.0).round() as i64)
            });
            FinancePivotRow {
                project_id: item.project_id,
                project_name,
                client_id,
                client_name,
                tracked_ms: item.tracked,
                planned_ms: item.planned,
                billable_ms: item.billable,
                revenue_cents: revenue,
                cost_cents: cost,
                margin_cents: revenue - cost,
                margin_percent: (revenue != 0)
                    .then(|| (revenue - cost) as f64 * 100.0 / revenue as f64),
                budget_hours_hundredths,
                budget_amount_cents,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| std::cmp::Reverse(row.tracked_ms));
    let tracked_ms = rows.iter().map(|r| r.tracked_ms).sum();
    let planned_ms = rows.iter().map(|r| r.planned_ms).sum();
    let billable_ms = rows.iter().map(|r| r.billable_ms).sum();
    let revenue_cents = rows.iter().map(|r| r.revenue_cents).sum();
    let cost_cents = rows.iter().map(|r| r.cost_cents).sum();
    let margin_cents = revenue_cents - cost_cents;
    let warnings = rows
        .iter()
        .flat_map(|row| {
            let mut out = Vec::new();
            if row.margin_cents < 0 {
                out.push(FinanceWarning {
                    project_id: row.project_id.clone(),
                    kind: "margin".into(),
                    message: format!("{} is below cost", row.project_name),
                    severity: "critical".into(),
                });
            }
            if row
                .budget_hours_hundredths
                .is_some_and(|budget| row.tracked_ms > budget as u64 * 36_000)
            {
                out.push(FinanceWarning {
                    project_id: row.project_id.clone(),
                    kind: "budget-hours".into(),
                    message: format!("{} exceeded its planned hours", row.project_name),
                    severity: "warning".into(),
                });
            }
            if row
                .budget_amount_cents
                .is_some_and(|budget| row.revenue_cents > budget)
            {
                out.push(FinanceWarning {
                    project_id: row.project_id.clone(),
                    kind: "budget-amount".into(),
                    message: format!("{} exceeded its amount budget", row.project_name),
                    severity: "warning".into(),
                });
            }
            out
        })
        .collect();
    Ok(FinanceSnapshot {
        start_ms: filter.start_ms,
        end_ms: filter.end_ms,
        tracked_ms,
        planned_ms,
        billable_ms,
        revenue_cents,
        cost_cents,
        margin_cents,
        margin_percent: (revenue_cents != 0)
            .then(|| margin_cents as f64 * 100.0 / revenue_cents as f64),
        rows,
        warnings,
    })
}

pub fn list_rate_cards(db: &DatabaseConnection) -> Result<Vec<RateCard>, String> {
    FinanceRepository::new(db)
        .rates()
        .map_err(error)
        .map(|rows| {
            rows.into_iter()
                .map(|r| RateCard {
                    project_id: r.project_id,
                    bill_rate_cents: r.bill_rate_cents,
                    cost_rate_cents: r.cost_rate_cents,
                    currency: r.currency,
                    effective_from: r.effective_from,
                })
                .collect()
        })
}
pub fn save_rate_card(db: &DatabaseConnection, input: RateCardInput) -> Result<RateCard, String> {
    let card = RateCard {
        project_id: input.project_id,
        bill_rate_cents: input.bill_rate_cents.max(0),
        cost_rate_cents: input.cost_rate_cents.max(0),
        currency: input.currency,
        effective_from: input
            .effective_from
            .unwrap_or_else(|| now_epoch_ms() as i64),
    };
    FinanceRepository::new(db)
        .save_rate(rate_cards::ActiveModel {
            project_id: Set(card.project_id.clone()),
            bill_rate_cents: Set(card.bill_rate_cents),
            cost_rate_cents: Set(card.cost_rate_cents),
            currency: Set(card.currency.clone()),
            effective_from: Set(card.effective_from),
        })
        .map_err(error)?;
    Ok(card)
}
pub fn list_contracts(db: &DatabaseConnection) -> Result<Vec<Contract>, String> {
    FinanceRepository::new(db)
        .contracts()
        .map_err(error)
        .map(|rows| rows.into_iter().map(contract).collect())
}
fn contract(r: contracts::Model) -> Contract {
    Contract {
        id: r.id,
        client_id: r.client_id,
        project_id: r.project_id,
        kind: r.kind,
        currency: r.currency,
        start_date: r.start_date,
        end_date: r.end_date,
        recurring_cents: r.recurring_cents,
        included_hours_hundredths: r.included_hours_hundredths,
        overage_rate_cents: r.overage_rate_cents,
        status: r.status,
    }
}
pub fn save_contract(db: &DatabaseConnection, input: ContractInput) -> Result<Contract, String> {
    let c = Contract {
        id: input.id.unwrap_or_else(|| uuid::Uuid::now_v7().to_string()),
        client_id: input.client_id,
        project_id: input.project_id,
        kind: input.kind,
        currency: input.currency,
        start_date: input.start_date,
        end_date: input.end_date,
        recurring_cents: input.recurring_cents.max(0),
        included_hours_hundredths: input.included_hours_hundredths.max(0),
        overage_rate_cents: input.overage_rate_cents.max(0),
        status: input.status,
    };
    FinanceRepository::new(db)
        .save_contract(contracts::ActiveModel {
            id: Set(c.id.clone()),
            client_id: Set(c.client_id.clone()),
            project_id: Set(c.project_id.clone()),
            kind: Set(c.kind.clone()),
            currency: Set(c.currency.clone()),
            start_date: Set(c.start_date.clone()),
            end_date: Set(c.end_date.clone()),
            recurring_cents: Set(c.recurring_cents),
            included_hours_hundredths: Set(c.included_hours_hundredths),
            overage_rate_cents: Set(c.overage_rate_cents),
            status: Set(c.status.clone()),
        })
        .map_err(error)?;
    Ok(c)
}
pub fn share_dashboard(
    db: &DatabaseConnection,
    dashboard_id: &str,
    expires_at: Option<i64>,
) -> Result<DashboardShare, String> {
    let share = DashboardShare {
        id: uuid::Uuid::now_v7().to_string(),
        dashboard_id: dashboard_id.into(),
        token: uuid::Uuid::now_v7().to_string(),
        permission: "view".into(),
        expires_at,
        created_at: now_epoch_ms() as i64,
    };
    FinanceRepository::new(db)
        .save_share(dashboard_shares::ActiveModel {
            id: Set(share.id.clone()),
            dashboard_id: Set(share.dashboard_id.clone()),
            token: Set(share.token.clone()),
            permission: Set(share.permission.clone()),
            expires_at: Set(share.expires_at),
            created_at: Set(share.created_at),
        })
        .map_err(error)?;
    Ok(share)
}
pub fn list_capacity_plans(db: &DatabaseConnection) -> Result<Vec<CapacityPlan>, String> {
    capacity_plans::Entity::find()
        .order_by_desc(capacity_plans::Column::PeriodStart)
        .all(db)
        .map_err(error)
        .map(|rows| {
            rows.into_iter()
                .map(|r| CapacityPlan {
                    id: r.id,
                    project_id: r.project_id,
                    person_key: r.person_key,
                    period_start: r.period_start,
                    period_end: r.period_end,
                    planned_minutes: r.planned_minutes,
                })
                .collect()
        })
}
pub fn save_capacity_plan(
    db: &DatabaseConnection,
    input: CapacityPlanInput,
) -> Result<CapacityPlan, String> {
    if input.period_end <= input.period_start {
        return Err("Capacity period must end after it starts".into());
    }
    let p = CapacityPlan {
        id: input.id.unwrap_or_else(|| uuid::Uuid::now_v7().to_string()),
        project_id: input.project_id,
        person_key: if input.person_key.trim().is_empty() {
            "default".into()
        } else {
            input.person_key
        },
        period_start: input.period_start,
        period_end: input.period_end,
        planned_minutes: input.planned_minutes.max(0),
    };
    let ts = now_epoch_ms() as i64;
    FinanceRepository::new(db)
        .save_capacity(capacity_plans::ActiveModel {
            id: Set(p.id.clone()),
            project_id: Set(p.project_id.clone()),
            person_key: Set(p.person_key.clone()),
            period_start: Set(p.period_start),
            period_end: Set(p.period_end),
            planned_minutes: Set(p.planned_minutes),
            created_at: Set(ts),
            updated_at: Set(ts),
        })
        .map_err(error)?;
    Ok(p)
}
pub fn list_dashboard_shares(
    db: &DatabaseConnection,
    dashboard_id: &str,
) -> Result<Vec<DashboardShare>, String> {
    FinanceRepository::new(db)
        .shares(dashboard_id)
        .map_err(error)
        .map(|rows| {
            rows.into_iter()
                .map(|r| DashboardShare {
                    id: r.id,
                    dashboard_id: r.dashboard_id,
                    token: r.token,
                    permission: r.permission,
                    expires_at: r.expires_at,
                    created_at: r.created_at,
                })
                .collect()
        })
}
pub fn revoke_dashboard_share(db: &DatabaseConnection, id: &str) -> Result<(), String> {
    FinanceRepository::new(db).delete_share(id).map_err(error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aevio_persistence::database::entities::finance::{
        capacity_plans, clients, projects, rate_cards, time_entries,
    };
    use aevio_persistence::database::sea_orm::{ConnectionTrait, Database, DbBackend, Schema};
    use sea_orm::{ActiveValue::Set, EntityTrait};

    #[test]
    fn snapshot_uses_capacity_plan_period_columns() {
        let db = Database::connect("sqlite::memory:").unwrap();
        let schema = Schema::new(DbBackend::Sqlite);
        for statement in [
            schema.create_table_from_entity(time_entries::Entity),
            schema.create_table_from_entity(projects::Entity),
            schema.create_table_from_entity(clients::Entity),
            schema.create_table_from_entity(rate_cards::Entity),
            schema.create_table_from_entity(capacity_plans::Entity),
        ] {
            db.execute(&statement).unwrap();
        }
        time_entries::Entity::insert(time_entries::ActiveModel {
            id: Set("entry".into()),
            started_at: Set(0),
            ended_at: Set(3_600_000),
            project_id: Set(None),
            billable: Set(1),
            status: Set("pending".into()),
            invoice_id: Set(None),
            deleted_at: Set(None),
        })
        .exec(&db)
        .unwrap();
        capacity_plans::Entity::insert(capacity_plans::ActiveModel {
            id: Set("plan".into()),
            project_id: Set(None),
            person_key: Set("default".into()),
            period_start: Set(0),
            period_end: Set(7_200_000),
            planned_minutes: Set(120),
            created_at: Set(0),
            updated_at: Set(0),
        })
        .exec(&db)
        .unwrap();
        let result = snapshot(
            &db,
            &FinanceFilter {
                start_ms: 0,
                end_ms: 3_600_000,
            },
        )
        .unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].planned_ms, 3_600_000);
    }
}
