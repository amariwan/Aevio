//! Reading the rows a report needs.
//!
//! Two adapters answer the same questions here. The legacy rusqlite path
//! still backs the writer-side store; the SeaORM path backs the read-only
//! connections the app and the CLI open on their own. Both hand their rows
//! to `aevio_core::reports`, which owns the grouping and rendering.

use std::collections::HashMap;

use rusqlite::{Connection, Row, params, params_from_iter, types::Value};

use aevio_core::EntryFilter;
use aevio_core::models::TimeEntry;
use aevio_core::reports::{ExportFormat, GroupBy, ReportEntry, ReportSegment, RollupCell};

use crate::database::DatabaseConnection;
use crate::database::repositories::ai::AiRepository;
use crate::database::repositories::entry_query::EntryQueryRepository;
use crate::database::repositories::reports::{ReportRepository, ReportRow};
use crate::entries::query::APP_KEY;
use crate::err;

/// A segment's share of its entry, clamped to the entry's bounds so app
/// totals add up to the same time as every other grouping (the open segment
/// ends with its entry).
const SEGMENT_MS: &str = "MAX(0, MIN(COALESCE(s.ended_at, te.ended_at), te.ended_at) - MAX(s.started_at, te.started_at))";

// --- Legacy rusqlite reads ----------------------------------------------------

/// Entries matching a filter, newest first, with the AI summary and the
/// dominant app the list views show.
pub fn query_entries(
    conn: &Connection,
    filter: &EntryFilter,
    limit: u32,
) -> Result<Vec<TimeEntry>, String> {
    let mut entries = crate::entries::query::query_entries(conn, filter, limit)?;

    let mut ai = crate::ai::store::summaries(conn, filter.start_ms, filter.end_ms)?;
    let mut apps = dominant_apps(conn, filter.start_ms, filter.end_ms)?;
    for entry in &mut entries {
        entry.ai = ai.remove(&entry.id);
        entry.dominant_app = apps.remove(&entry.id);
    }
    Ok(entries)
}

/// Entry id -> the app or site with most of its active time, for every entry
/// overlapping the range.
pub fn dominant_apps(
    conn: &Connection,
    start_ms: u64,
    end_ms: u64,
) -> Result<HashMap<String, String>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT s.entry_id, {APP_KEY} AS app, SUM({SEGMENT_MS}) AS ms
             FROM segments s JOIN time_entries te ON te.id = s.entry_id
             WHERE te.deleted_at IS NULL AND te.ended_at >= ?1 AND te.started_at <= ?2
               AND s.kind != 'break'
             GROUP BY s.entry_id, app;"
        ))
        .map_err(err)?;
    let rows = stmt
        .query_map(params![start_ms as i64, end_ms as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(err)?;
    let mut best: HashMap<String, (String, i64)> = HashMap::new();
    for row in rows {
        let (entry, app, ms) = row.map_err(err)?;
        let slot = best.entry(entry).or_insert_with(|| (app.clone(), ms));
        if ms > slot.1 {
            *slot = (app, ms);
        }
    }
    Ok(best.into_iter().map(|(id, (app, _))| (id, app)).collect())
}

/// Sums the filtered entries per group per bucket, in SQL.
pub fn rollup(
    conn: &Connection,
    filter: &EntryFilter,
    boundaries: &[u64],
    group_by: GroupBy,
) -> Result<Vec<RollupCell>, String> {
    aevio_core::reports::validate_boundaries(boundaries)?;

    let clause = crate::entries::query::clause(filter)?;
    let buckets = serde_json::to_string(boundaries).map_err(|error| error.to_string())?;
    let mut values: Vec<Value> = vec![buckets.into()];
    values.extend(clause.params);

    let buckets_cte = "WITH b AS (
        SELECT CAST(key AS INTEGER) AS i,
               CAST(value AS INTEGER) AS lo,
               CAST(LEAD(value) OVER (ORDER BY key) AS INTEGER) AS hi
        FROM json_each(?)
    )";
    let key = key_sql(group_by);
    let sql = if group_by == GroupBy::App {
        format!(
            "{buckets_cte}
             SELECT {key} AS k, b.i,
                    SUM({SEGMENT_MS}),
                    COUNT(DISTINCT te.id),
                    SUM(CASE WHEN te.status = 'approved' THEN {SEGMENT_MS} ELSE 0 END),
                    COUNT(DISTINCT CASE WHEN te.status = 'pending' THEN te.id END),
                    SUM(CASE WHEN te.billable = 1 THEN {SEGMENT_MS} ELSE 0 END)
             FROM segments s
             JOIN time_entries te ON te.id = s.entry_id
             LEFT JOIN projects p ON p.id = te.project_id
             JOIN b ON te.started_at >= b.lo AND te.started_at < b.hi
             WHERE s.kind != 'break' AND {}
             GROUP BY k, b.i
             ORDER BY b.i;",
            clause.sql
        )
    } else {
        format!(
            "{buckets_cte}
             SELECT {key} AS k, b.i,
                    SUM(te.ended_at - te.started_at),
                    COUNT(*),
                    SUM(CASE WHEN te.status = 'approved' THEN te.ended_at - te.started_at ELSE 0 END),
                    SUM(CASE WHEN te.status = 'pending' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN te.billable = 1 THEN te.ended_at - te.started_at ELSE 0 END)
             FROM time_entries te
             LEFT JOIN projects p ON p.id = te.project_id
             JOIN b ON te.started_at >= b.lo AND te.started_at < b.hi
             WHERE {}
             GROUP BY k, b.i
             ORDER BY b.i;",
            clause.sql
        )
    };

    let mut stmt = conn.prepare(&sql).map_err(err)?;
    let rows = stmt
        .query_map(params_from_iter(values), |row| {
            Ok(RollupCell {
                key: row.get(0)?,
                bucket: row.get::<_, i64>(1)? as u32,
                ms: row.get::<_, i64>(2)?.max(0) as u64,
                entries: row.get::<_, i64>(3)? as u32,
                approved_ms: row.get::<_, i64>(4)?.max(0) as u64,
                pending: row.get::<_, i64>(5)? as u32,
                billable_ms: row.get::<_, i64>(6)?.max(0) as u64,
            })
        })
        .map_err(err)?;
    rows.collect::<rusqlite::Result<_>>().map_err(err)
}

/// What a pivot row is keyed by, as a SQL expression.
fn key_sql(group_by: GroupBy) -> &'static str {
    match group_by {
        GroupBy::Project => "te.project_id",
        GroupBy::Client => "p.client_id",
        GroupBy::Category => "te.category_id",
        GroupBy::App => APP_KEY,
        GroupBy::Status => "CASE WHEN te.status = 'approved' THEN 'approved' ELSE 'pending' END",
        GroupBy::None => "NULL",
    }
}

/// The export file's contents and how many entries it holds.
pub fn export_body(
    conn: &Connection,
    filter: &EntryFilter,
    format: ExportFormat,
) -> Result<(String, u32), String> {
    let entries = export_rows(conn, filter)?;
    aevio_core::reports::export_body(&entries, format)
}

/// The filtered entries as report rows, oldest first.
fn export_rows(conn: &Connection, filter: &EntryFilter) -> Result<Vec<ReportEntry>, String> {
    let clause = crate::entries::query::clause(filter)?;
    let mut stmt = conn
        .prepare(&format!(
            "SELECT te.started_at, te.ended_at, te.description, te.category_id, te.project_id,
                    c.name, p.name, cl.name, p.client_id, te.status, te.billable
             FROM time_entries te
             LEFT JOIN projects p ON p.id = te.project_id
             LEFT JOIN categories c ON c.id = te.category_id
             LEFT JOIN clients cl ON cl.id = p.client_id
             WHERE {}
             ORDER BY te.started_at ASC;",
            clause.sql
        ))
        .map_err(err)?;
    let rows = stmt
        .query_map(params_from_iter(clause.params), |row| {
            Ok(ReportEntry {
                started_at: row.get::<_, i64>(0)? as u64,
                ended_at: row.get::<_, i64>(1)? as u64,
                description: row.get(2)?,
                category_id: row.get(3)?,
                project_id: row.get(4)?,
                project_client_id: row.get(8)?,
                status: row.get(9)?,
                billable: row.get::<_, i64>(10)? != 0,
                category_name: row.get(5)?,
                project_name: row.get(6)?,
                client_name: row.get(7)?,
                segments: Vec::new(),
            })
        })
        .map_err(err)?;
    rows.collect::<rusqlite::Result<_>>().map_err(err)
}

// --- SeaORM reads -------------------------------------------------------------

/// Entries matching a filter, newest first, with the AI summary and the
/// dominant app the list views show.
pub fn query_entries_with_database(
    database: &DatabaseConnection,
    filter: &EntryFilter,
    limit: u32,
) -> Result<Vec<TimeEntry>, String> {
    let repository = EntryQueryRepository::new(database);
    let mut entries = repository.list(filter, limit).map_err(db_error)?;
    let mut ai = AiRepository::new(database)
        .summaries(filter.start_ms as i64, filter.end_ms as i64)
        .map_err(db_error)?;
    let mut apps = repository
        .dominant_apps(filter.start_ms as i64, filter.end_ms as i64)
        .map_err(db_error)?;
    for entry in &mut entries {
        entry.ai = ai.remove(&entry.id);
        entry.dominant_app = apps.remove(&entry.id);
    }
    Ok(entries)
}

/// Sums the filtered entries per group per bucket.
pub fn rollup_with_database(
    database: &DatabaseConnection,
    filter: &EntryFilter,
    boundaries: &[u64],
    group_by: GroupBy,
) -> Result<Vec<RollupCell>, String> {
    let rows = ReportRepository::new(database)
        .rows(filter)
        .map_err(db_error)?
        .iter()
        .map(report_entry)
        .collect::<Vec<_>>();
    aevio_core::reports::rollup(&rows, boundaries, group_by)
}

/// The export file's contents and how many entries it holds.
pub fn export_body_with_database(
    database: &DatabaseConnection,
    filter: &EntryFilter,
    format: ExportFormat,
) -> Result<(String, u32), String> {
    let rows = ReportRepository::new(database)
        .rows(filter)
        .map_err(db_error)?
        .iter()
        .map(report_entry)
        .collect::<Vec<_>>();
    aevio_core::reports::export_body(&rows, format)
}

pub fn export_entries_with_database(
    database: &DatabaseConnection,
    filter: &EntryFilter,
    format: ExportFormat,
    directory: &std::path::Path,
) -> Result<aevio_core::reports::ExportResult, String> {
    let (body, count) = export_body_with_database(database, filter, format)?;
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let path = directory.join(format!(
        "aevio-time-{}.{}",
        aevio_core::timers::now_epoch_ms(),
        format.extension()
    ));
    std::fs::write(&path, body).map_err(|error| error.to_string())?;
    Ok(aevio_core::reports::ExportResult {
        path: path.display().to_string(),
        count,
    })
}

/// One stored row, as the report domain needs it.
fn report_entry(row: &ReportRow) -> ReportEntry {
    ReportEntry {
        started_at: row.entry.started_at.max(0) as u64,
        ended_at: row.entry.ended_at.max(0) as u64,
        description: row.entry.description.clone(),
        category_id: row.entry.category_id.clone(),
        project_id: row.entry.project_id.clone(),
        project_client_id: row
            .project
            .as_ref()
            .and_then(|project| project.client_id.clone()),
        status: row.entry.status.clone(),
        billable: row.entry.billable != 0,
        category_name: row.category_name.clone(),
        project_name: row.project.as_ref().map(|project| project.name.clone()),
        client_name: row.client_name.clone(),
        segments: row
            .segments
            .iter()
            .map(|segment| ReportSegment {
                app: segment.app.clone(),
                domain: segment.domain.clone(),
                kind: segment.kind.clone(),
                started_at: segment.started_at.max(0) as u64,
                ended_at: segment.ended_at.map(|value| value.max(0) as u64),
            })
            .collect(),
    }
}

/// Flatten a SeaORM error into the crate-wide `String` error type.
fn db_error(error: sea_orm::DbErr) -> String {
    match error {
        sea_orm::DbErr::Custom(message) => message,
        other => other.to_string(),
    }
}

/// Reads one entry row, in the column order the legacy export selects.
#[allow(dead_code)]
fn export_row(row: &Row<'_>) -> rusqlite::Result<ReportEntry> {
    Ok(ReportEntry {
        started_at: row.get::<_, i64>(0)? as u64,
        ended_at: row.get::<_, i64>(1)? as u64,
        description: row.get(2)?,
        category_id: None,
        project_id: None,
        project_client_id: None,
        status: row.get(9)?,
        billable: row.get::<_, i64>(10)? != 0,
        category_name: row.get(5)?,
        project_name: row.get(6)?,
        client_name: row.get(7)?,
        segments: Vec::new(),
    })
}
