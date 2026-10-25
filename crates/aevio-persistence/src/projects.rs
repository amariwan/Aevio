//! Database-backed project reads used by the IPC layer.

use aevio_core::entries::models::ActivitySegment;
pub use aevio_core::projects::{
    HintPreview, ImportSummary, ProjectRule, ProjectStats, ProjectSuggestion,
};

use crate::database::DatabaseConnection;
use crate::database::repositories::finance::FinanceRepository;
use crate::database::repositories::project_discovery::ProjectDiscoveryRepository;
use crate::database::repositories::project_rules::ProjectRulesRepository;
use crate::database::repositories::settings::SettingsRepository;

#[cfg(test)]
use rusqlite::{Connection, params};

pub fn project_stats_with_database(
    database: &DatabaseConnection,
    range_start: u64,
    range_end: u64,
    month_start: u64,
) -> Result<Vec<ProjectStats>, String> {
    FinanceRepository::new(database)
        .project_stats(range_start as i64, range_end as i64, month_start as i64)
        .map(|rows| {
            rows.into_iter()
                .map(|row| ProjectStats {
                    project_id: row.project_id,
                    entries: row.entries,
                    total_ms: row.total_ms,
                    range_ms: row.range_ms,
                    month_ms: row.month_ms,
                    billable_ms: row.billable_ms,
                    billable_month_ms: row.billable_month_ms,
                    unbilled_ms: row.unbilled_ms,
                    unbilled_entries: row.unbilled_entries,
                    last_activity: row.last_activity,
                })
                .collect()
        })
        .map_err(|error| error.to_string())
}

pub fn project_rules_with_database(
    database: &DatabaseConnection,
    project_id: &str,
) -> Result<Vec<ProjectRule>, String> {
    ProjectRulesRepository::new(database)
        .list_for_project(project_id)
        .map(|rows| {
            rows.into_iter()
                .map(|(id, match_kind, pattern, origin)| ProjectRule {
                    id,
                    match_kind,
                    pattern,
                    origin,
                })
                .collect()
        })
        .map_err(|error| error.to_string())
}

pub fn preview_hints_with_database(
    database: &DatabaseConnection,
    hints: &str,
    since_ms: u64,
) -> Result<HintPreview, String> {
    let segments = ProjectDiscoveryRepository::new(database)
        .segments_since(since_ms as i64)
        .map_err(|error| error.to_string())?;
    Ok(aevio_core::projects::preview_hints_over(&segments, hints))
}

pub fn discover_with_database(
    database: &DatabaseConnection,
    since_ms: u64,
) -> Result<Vec<ProjectSuggestion>, String> {
    let repository = ProjectDiscoveryRepository::new(database);
    let segments: Vec<ActivitySegment> = repository
        .segments_since(since_ms as i64)
        .map_err(|error| error.to_string())?;
    let project_names = repository
        .project_names()
        .map_err(|error| error.to_string())?;
    let rule_patterns = repository
        .rule_patterns()
        .map_err(|error| error.to_string())?;
    let dismissed = repository
        .dismissed_value("project_suggestions_dismissed")
        .map_err(|error| error.to_string())?
        .unwrap_or_default();
    let skipped = dismissed
        .split('\n')
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect();
    Ok(aevio_core::projects::suggest_projects(
        &segments,
        &aevio_core::projects::known_names(&project_names, &rule_patterns),
        &skipped,
    ))
}

pub fn dismiss_suggestion_with_database(
    database: &DatabaseConnection,
    key: &str,
) -> Result<(), String> {
    let settings = SettingsRepository::new(database);
    let current = settings
        .value("project_suggestions_dismissed")
        .map_err(|error| error.to_string())?
        .unwrap_or_default();
    let mut values: Vec<String> = current
        .split('\n')
        .filter(|value| !value.is_empty() && *value != key)
        .map(str::to_owned)
        .collect();
    values.push(key.to_owned());
    settings
        .set("project_suggestions_dismissed", &values.join("\n"))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
pub fn project_stats(
    conn: &Connection,
    range_start: u64,
    range_end: u64,
    month_start: u64,
) -> Result<Vec<ProjectStats>, String> {
    let mut statement = conn
        .prepare(
            "SELECT project_id, started_at, ended_at, billable, status, invoice_id
             FROM time_entries
             WHERE deleted_at IS NULL AND project_id IS NOT NULL",
        )
        .map_err(crate::err)?;
    let mut stats = std::collections::HashMap::<String, ProjectStats>::new();
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .map_err(crate::err)?;
    for row in rows {
        let (project_id, started_at, ended_at, billable, status, invoice_id) =
            row.map_err(crate::err)?;
        let started_at = started_at.max(0) as u64;
        let ended_at = ended_at.max(0) as u64;
        let duration = ended_at.saturating_sub(started_at);
        let entry = stats.entry(project_id.clone()).or_insert(ProjectStats {
            project_id,
            entries: 0,
            total_ms: 0,
            range_ms: 0,
            month_ms: 0,
            billable_ms: 0,
            billable_month_ms: 0,
            unbilled_ms: 0,
            unbilled_entries: 0,
            last_activity: None,
        });
        entry.entries += 1;
        entry.total_ms += duration;
        if started_at >= range_start && started_at < range_end {
            entry.range_ms += duration;
        }
        if started_at >= month_start {
            entry.month_ms += duration;
        }
        if billable != 0 {
            entry.billable_ms += duration;
            if started_at >= month_start {
                entry.billable_month_ms += duration;
            }
        }
        entry.last_activity = Some(entry.last_activity.unwrap_or(0).max(ended_at));
        if status == "approved" && billable != 0 && invoice_id.is_none() {
            entry.unbilled_ms += duration;
            entry.unbilled_entries += 1;
        }
    }
    let mut result = stats.into_values().collect::<Vec<_>>();
    result.sort_by(|left, right| left.project_id.cmp(&right.project_id));
    Ok(result)
}

#[cfg(test)]
pub fn sync_hint_rules(
    conn: &Connection,
    project_id: &str,
    hints: Option<&str>,
    now: u64,
) -> Result<(), String> {
    let transaction = conn.unchecked_transaction().map_err(crate::err)?;
    transaction
        .execute(
            "UPDATE rules SET deleted_at=?1, updated_at=?1
             WHERE project_id=?2 AND origin='hint' AND deleted_at IS NULL",
            params![now as i64, project_id],
        )
        .map_err(crate::err)?;
    for hint in aevio_core::projects::parse_hints(hints.unwrap_or_default()) {
        let Some((match_kind, pattern)) = hint.rule else {
            continue;
        };
        transaction
            .execute(
                "INSERT INTO rules(id,match_kind,pattern,project_id,priority,enabled,origin,created_at,updated_at)
                 VALUES(?1,?2,?3,?4,0,1,'hint',?5,?5)",
                params![uuid::Uuid::now_v7().to_string(), match_kind, pattern, project_id, now as i64],
            )
            .map_err(crate::err)?;
    }
    transaction.commit().map_err(crate::err)
}

pub fn import_csv(
    store: &mut crate::activity::ActivityStore,
    text: &str,
    now: u64,
) -> Result<ImportSummary, String> {
    let rows = aevio_core::projects::parse_csv(text);
    if rows.is_empty() {
        return Ok(ImportSummary {
            created: 0,
            clients_created: 0,
            skipped: Vec::new(),
        });
    }
    let mut created = 0;
    let mut skipped = Vec::new();
    for (index, row) in rows.into_iter().skip(1).enumerate() {
        let name = row.first().map(String::as_str).unwrap_or("").trim();
        if name.is_empty() {
            skipped.push(format!("Row {}: missing project name", index + 2));
            continue;
        }
        if store
            .list_projects()?
            .iter()
            .any(|project| project.name.eq_ignore_ascii_case(name))
        {
            skipped.push(format!("Row {}: {name} already exists", index + 2));
            continue;
        }
        store.create_project(
            aevio_core::entries::models::NewProject {
                client_id: None,
                name: name.to_owned(),
                color: aevio_core::projects::PALETTE
                    [created as usize % aevio_core::projects::PALETTE.len()]
                .to_owned(),
                description: row.get(1).filter(|value| !value.trim().is_empty()).cloned(),
                ai_hints: row.get(2).filter(|value| !value.trim().is_empty()).cloned(),
                status: Some("active".to_owned()),
                due_date: None,
                budget_kind: Some("none".to_owned()),
                budget_value: None,
                budget_period: Some("total".to_owned()),
                billable_default: Some(false),
                hourly_rate: None,
            },
            now,
        )?;
        created += 1;
    }
    Ok(ImportSummary {
        created,
        clients_created: 0,
        skipped,
    })
}
