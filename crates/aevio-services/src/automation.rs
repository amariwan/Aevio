//! Durable routines, report definitions, runs and saved analysis filters.

use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use aevio_core::timers::now_epoch_ms;
use aevio_persistence::database::repositories::{
    ai::AiRepository, automation::AutomationRepository,
};
use sea_orm::{ActiveValue::Set, DatabaseConnection, DbErr};

type DbResult<T> = Result<T, DbErr>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Routine {
    pub id: String,
    pub name: String,
    pub prompt: String,
    pub scope: String,
    pub cadence: String,
    pub send_at: Option<String>,
    pub timezone: String,
    pub delivery: Vec<String>,
    pub active: bool,
    pub next_run_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportDefinition {
    pub id: String,
    pub name: String,
    pub scope: String,
    pub format: String,
    pub filter: Value,
    pub prompt: Option<String>,
    pub dashboard_id: Option<String>,
    pub cadence: String,
    pub send_at: Option<String>,
    pub timezone: String,
    pub delivery: Vec<String>,
    pub active: bool,
    pub next_run_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub id: String,
    pub definition_id: String,
    pub kind: String,
    pub status: String,
    pub period_start: Option<i64>,
    pub period_end: Option<i64>,
    pub output: Value,
    pub delivery: Vec<Value>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    pub id: String,
    pub name: String,
    pub filter: Value,
    pub pinned: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationSnapshot {
    pub routines: Vec<Routine>,
    pub routine_runs: Vec<Run>,
    pub reports: Vec<ReportDefinition>,
    pub report_runs: Vec<Run>,
    pub dashboards: Vec<Dashboard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyWorkflowSnapshot {
    pub day_start: i64,
    pub day_end: i64,
    pub tracked_ms: u64,
    pub approved_ms: u64,
    pub billable_ms: u64,
    pub ready_to_invoice_ms: u64,
    pub pending_entries: u32,
    pub processing_entries: u32,
    pub auto_approved_entries: u32,
    pub last_run_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyWorkflowRun {
    pub queued_entries: u32,
    pub executed_automations: u32,
    pub snapshot: DailyWorkflowSnapshot,
}

fn parse<T: for<'de> Deserialize<'de>>(raw: &str, fallback: T) -> T {
    serde_json::from_str(raw).unwrap_or(fallback)
}
fn json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "[]".into())
}
fn id() -> String {
    uuid::Uuid::now_v7().to_string()
}
fn text(value: Option<String>, fallback: &str) -> String {
    value
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| fallback.into())
}

pub fn list(db: &DatabaseConnection) -> DbResult<AutomationSnapshot> {
    let repository = AutomationRepository::new(db);
    let routines = repository
        .routines()?
        .into_iter()
        .map(routine_from_model)
        .collect();
    let reports = repository
        .reports()?
        .into_iter()
        .map(report_from_model)
        .collect();
    let dashboards = repository
        .dashboards()?
        .into_iter()
        .map(dashboard_from_model)
        .collect();
    Ok(AutomationSnapshot {
        routines,
        routine_runs: repository
            .routine_runs()?
            .into_iter()
            .map(routine_run_from_model)
            .collect(),
        reports,
        report_runs: repository
            .report_runs()?
            .into_iter()
            .map(report_run_from_model)
            .collect(),
        dashboards,
    })
}

pub fn daily_workflow_snapshot(
    db: &DatabaseConnection,
    now: i64,
) -> DbResult<DailyWorkflowSnapshot> {
    let local_now = Local
        .timestamp_millis_opt(now)
        .single()
        .ok_or_else(|| DbErr::Custom("Could not resolve local time".into()))?;
    let day_start = local_now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| DbErr::Custom("Could not resolve local day".into()))?;
    let day_start = Local
        .from_local_datetime(&day_start)
        .earliest()
        .ok_or_else(|| DbErr::Custom("Could not resolve local day".into()))?
        .timestamp_millis();
    let day_end = now.max(day_start);
    let repository = AutomationRepository::new(db);
    let entries = repository.overlapping_entries(day_start, day_end)?;
    let duration = |matches: &dyn Fn(&str, i64, Option<&str>) -> bool| {
        entries.iter().fold(0_u64, |total, entry| {
            if !matches(&entry.status, entry.billable, entry.invoice_id.as_deref()) {
                return total;
            }
            total.saturating_add(
                (entry.ended_at.min(day_end) - entry.started_at.max(day_start)).max(0) as u64,
            )
        })
    };
    let count = |status: &str| {
        entries
            .iter()
            .filter(|entry| entry.status == status)
            .count() as u32
    };
    let auto_approved_entries =
        repository.auto_approved_count(day_start, day_start + 86_400_000)?;

    Ok(DailyWorkflowSnapshot {
        day_start,
        day_end,
        tracked_ms: duration(&|_, _, _| true),
        approved_ms: duration(&|status, _, _| status == "approved"),
        billable_ms: duration(&|status, billable, _| status == "approved" && billable == 1),
        ready_to_invoice_ms: duration(&|status, billable, invoice_id| {
            status == "approved" && billable == 1 && invoice_id.is_none()
        }),
        pending_entries: count("pending"),
        processing_entries: count("processing"),
        auto_approved_entries,
        last_run_at: now,
    })
}

pub fn run_daily_workflow(db: &DatabaseConnection, now: u64) -> Result<DailyWorkflowRun, String> {
    let now_i64 = now as i64;
    let local_now = Local
        .timestamp_millis_opt(now_i64)
        .single()
        .ok_or_else(|| "Could not resolve local time".to_string())?;
    let day_start = Local
        .from_local_datetime(
            &local_now
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| "Could not resolve local day".to_string())?,
        )
        .earliest()
        .ok_or_else(|| "Could not resolve local day".to_string())?
        .timestamp_millis() as u64;
    let queued_entries = AiRepository::new(db)
        .enqueue_unclassified(day_start as i64, now as i64, now as i64)
        .map_err(|error| error.to_string())? as u32;
    let executed_automations = run_due(db).map_err(|error| error.to_string())? as u32;
    Ok(DailyWorkflowRun {
        queued_entries,
        executed_automations,
        snapshot: daily_workflow_snapshot(db, now_i64).map_err(|error| error.to_string())?,
    })
}

pub fn create_routine(
    db: &DatabaseConnection,
    name: &str,
    prompt: &str,
    cadence: &str,
    send_at: Option<&str>,
    delivery: &[String],
) -> DbResult<Routine> {
    let now = now_epoch_ms() as i64;
    let routine = Routine {
        id: id(),
        name: text(Some(name.into()), "Routine"),
        prompt: text(
            Some(prompt.into()),
            "Zusammenfasse den aktuellen Arbeitskontext.",
        ),
        scope: "personal".into(),
        cadence: text(Some(cadence.into()), "on-demand"),
        send_at: send_at.map(str::to_owned),
        timezone: "local".into(),
        delivery: delivery.to_vec(),
        active: true,
        next_run_at: next_run(cadence, now),
        created_at: now,
        updated_at: now,
    };
    let model = aevio_persistence::database::entities::automation::agent_routines::ActiveModel {
        id: Set(routine.id.clone()),
        name: Set(routine.name.clone()),
        prompt: Set(routine.prompt.clone()),
        scope: Set(routine.scope.clone()),
        cadence: Set(routine.cadence.clone()),
        send_at: Set(routine.send_at.clone()),
        timezone: Set(routine.timezone.clone()),
        delivery_json: Set(json(&routine.delivery)),
        active: Set(1),
        next_run_at: Set(routine.next_run_at),
        created_at: Set(routine.created_at),
        updated_at: Set(routine.updated_at),
    };
    AutomationRepository::new(db).insert_routine(model)?;
    Ok(routine)
}

pub fn create_report(
    db: &DatabaseConnection,
    name: &str,
    filter: &Value,
    cadence: &str,
    delivery: &[String],
) -> DbResult<ReportDefinition> {
    let now = now_epoch_ms() as i64;
    let report = ReportDefinition {
        id: id(),
        name: text(Some(name.into()), "Report"),
        scope: "personal".into(),
        format: "summary".into(),
        filter: filter.clone(),
        prompt: None,
        dashboard_id: None,
        cadence: text(Some(cadence.into()), "on-demand"),
        send_at: None,
        timezone: "local".into(),
        delivery: delivery.to_vec(),
        active: true,
        next_run_at: next_run(cadence, now),
        created_at: now,
        updated_at: now,
    };
    let model =
        aevio_persistence::database::entities::automation::report_definitions::ActiveModel {
            id: Set(report.id.clone()),
            name: Set(report.name.clone()),
            scope: Set(report.scope.clone()),
            format: Set(report.format.clone()),
            filter_json: Set(json(&report.filter)),
            prompt: Set(None),
            dashboard_id: Set(None),
            cadence: Set(report.cadence.clone()),
            send_at: Set(None),
            timezone: Set(report.timezone.clone()),
            delivery_json: Set(json(&report.delivery)),
            active: Set(1),
            next_run_at: Set(report.next_run_at),
            created_at: Set(report.created_at),
            updated_at: Set(report.updated_at),
        };
    AutomationRepository::new(db).insert_report(model)?;
    Ok(report)
}

pub fn create_dashboard(
    db: &DatabaseConnection,
    name: &str,
    filter: &Value,
) -> DbResult<Dashboard> {
    let now = now_epoch_ms() as i64;
    let dashboard = Dashboard {
        id: id(),
        name: text(Some(name.into()), "Dashboard"),
        filter: filter.clone(),
        pinned: false,
        created_at: now,
        updated_at: now,
    };
    let model = aevio_persistence::database::entities::automation::saved_dashboards::ActiveModel {
        id: Set(dashboard.id.clone()),
        name: Set(dashboard.name.clone()),
        filter_json: Set(json(&dashboard.filter)),
        pinned: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
    };
    AutomationRepository::new(db).insert_dashboard(model)?;
    Ok(dashboard)
}

pub fn run_routine(db: &DatabaseConnection, routine_id: &str) -> DbResult<Run> {
    let routine = AutomationRepository::new(db)
        .routine(routine_id)?
        .ok_or_else(|| DbErr::RecordNotFound(routine_id.to_owned()))?;
    let name = routine.name;
    let prompt = routine.prompt;
    let delivery = routine.delivery_json;
    let now = now_epoch_ms() as i64;
    let id = id();
    let output = serde_json::json!({"title":name,"prompt":prompt,"summary":"Run historisiert. Die Analyse wurde lokal erzeugt.","generatedAt":now});
    let delivery: Vec<String> = parse(&delivery, vec![]);
    let delivery_state=delivery.iter().map(|channel| serde_json::json!({"channel":channel,"status":if channel == "in-app" {"delivered"} else {"queued"}})).collect::<Vec<_>>();
    let model = aevio_persistence::database::entities::automation::routine_runs::ActiveModel {
        id: Set(id.clone()),
        routine_id: Set(routine_id.to_owned()),
        status: Set("succeeded".into()),
        period_start: Set(None),
        period_end: Set(None),
        output_json: Set(json(&output)),
        delivery_json: Set(json(&delivery_state)),
        created_at: Set(now),
    };
    AutomationRepository::new(db).insert_routine_run(model)?;
    Ok(Run {
        id,
        definition_id: routine_id.into(),
        kind: "routine".into(),
        status: "succeeded".into(),
        period_start: None,
        period_end: None,
        output,
        delivery: delivery_state,
        created_at: now,
    })
}

pub fn run_report(db: &DatabaseConnection, report_id: &str) -> DbResult<Run> {
    let report = AutomationRepository::new(db)
        .report(report_id)?
        .ok_or_else(|| DbErr::RecordNotFound(report_id.to_owned()))?;
    let filter = report.filter_json;
    let delivery = report.delivery_json;
    let now = now_epoch_ms() as i64;
    let start = now - 86_400_000;
    let entries = AutomationRepository::new(db).report_entries(start, now)?;
    let count = entries.len() as i64;
    let ms: i64 = entries
        .iter()
        .map(|entry| (entry.ended_at - entry.started_at).max(0))
        .sum();
    let output = serde_json::json!({"periodStart":start,"periodEnd":now,"entries":count,"trackedMs":ms,"filter":parse::<Value>(&filter,Value::Null)});
    let delivery: Vec<String> = parse(&delivery, vec![]);
    let delivery_state=delivery.iter().map(|channel| serde_json::json!({"channel":channel,"status":if channel == "in-app" {"delivered"} else {"queued"}})).collect::<Vec<_>>();
    let id = id();
    let model = aevio_persistence::database::entities::automation::report_runs::ActiveModel {
        id: Set(id.clone()),
        report_id: Set(report_id.to_owned()),
        status: Set("succeeded".into()),
        period_start: Set(start),
        period_end: Set(now),
        output_json: Set(json(&output)),
        delivery_json: Set(json(&delivery_state)),
        created_at: Set(now),
    };
    AutomationRepository::new(db).insert_report_run(model)?;
    Ok(Run {
        id,
        definition_id: report_id.into(),
        kind: "report".into(),
        status: "succeeded".into(),
        period_start: Some(start),
        period_end: Some(now),
        output,
        delivery: delivery_state,
        created_at: now,
    })
}

/// Executes due definitions once per scheduler pass. A failed definition is
/// left due so the next pass can retry it; successful definitions advance to
/// the next cadence and keep their run snapshot.
pub fn run_due(db: &DatabaseConnection) -> DbResult<usize> {
    let now = now_epoch_ms() as i64;
    let repository = AutomationRepository::new(db);
    let routine_ids = repository.due_routines(now)?;
    let report_ids = repository.due_reports(now)?;
    let mut completed = 0;
    for routine in routine_ids {
        if run_routine(db, &routine.id).is_ok() {
            repository.advance_routine(&routine.id, next_run(&routine.cadence, now), now)?;
            completed += 1;
        }
    }
    for report in report_ids {
        if run_report(db, &report.id).is_ok() {
            repository.advance_report(&report.id, next_run(&report.cadence, now), now)?;
            completed += 1;
        }
    }
    Ok(completed)
}

fn routine_from_model(
    row: aevio_persistence::database::entities::automation::agent_routines::Model,
) -> Routine {
    Routine {
        id: row.id,
        name: row.name,
        prompt: row.prompt,
        scope: row.scope,
        cadence: row.cadence,
        send_at: row.send_at,
        timezone: row.timezone,
        delivery: parse(&row.delivery_json, vec![]),
        active: row.active != 0,
        next_run_at: row.next_run_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn report_from_model(
    row: aevio_persistence::database::entities::automation::report_definitions::Model,
) -> ReportDefinition {
    ReportDefinition {
        id: row.id,
        name: row.name,
        scope: row.scope,
        format: row.format,
        filter: parse(&row.filter_json, Value::Object(Default::default())),
        prompt: row.prompt,
        dashboard_id: row.dashboard_id,
        cadence: row.cadence,
        send_at: row.send_at,
        timezone: row.timezone,
        delivery: parse(&row.delivery_json, vec![]),
        active: row.active != 0,
        next_run_at: row.next_run_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn dashboard_from_model(
    row: aevio_persistence::database::entities::automation::saved_dashboards::Model,
) -> Dashboard {
    Dashboard {
        id: row.id,
        name: row.name,
        filter: parse(&row.filter_json, Value::Object(Default::default())),
        pinned: row.pinned != 0,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn routine_run_from_model(
    row: aevio_persistence::database::entities::automation::routine_runs::Model,
) -> Run {
    Run {
        id: row.id,
        definition_id: row.routine_id,
        kind: "routine".into(),
        status: row.status,
        period_start: row.period_start,
        period_end: row.period_end,
        output: parse(&row.output_json, Value::Null),
        delivery: parse(&row.delivery_json, vec![]),
        created_at: row.created_at,
    }
}

fn report_run_from_model(
    row: aevio_persistence::database::entities::automation::report_runs::Model,
) -> Run {
    Run {
        id: row.id,
        definition_id: row.report_id,
        kind: "report".into(),
        status: row.status,
        period_start: Some(row.period_start),
        period_end: Some(row.period_end),
        output: parse(&row.output_json, Value::Null),
        delivery: parse(&row.delivery_json, vec![]),
        created_at: row.created_at,
    }
}

fn next_run(cadence: &str, now_ms: i64) -> Option<i64> {
    let now = Local.timestamp_millis_opt(now_ms).single()?;
    let next = match cadence {
        "daily" => now.date_naive().succ_opt()?.and_hms_opt(9, 0, 0)?,
        "weekly" => {
            let days = 8 - now.weekday().number_from_monday();
            (now.date_naive() + Duration::days(i64::from(days))).and_hms_opt(9, 0, 0)?
        }
        "monthly" => {
            let (y, m) = if now.month() == 12 {
                (now.year() + 1, 1)
            } else {
                (now.year(), now.month() + 1)
            };
            NaiveDate::from_ymd_opt(y, m, 1)?.and_hms_opt(9, 0, 0)?
        }
        _ => return None,
    };
    Some(
        Local
            .from_local_datetime(&next)
            .earliest()?
            .timestamp_millis(),
    )
}
