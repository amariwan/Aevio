//! Turning an entry filter into SQL, and reading entries through it.
//!
//! The filter itself lives in the domain (`aevio_core::EntryFilter`); this
//! module owns the only place that knows how its fields map onto columns.

use rusqlite::{Connection, Row, params_from_iter, types::Value};

use aevio_core::models::TimeEntry;
use aevio_core::{EntryFilter, MAX_QUERY_ROWS};

/// How many search words one filter may carry.
const MAX_SEARCH_TERMS: usize = 8;

/// An app or a website: sites are keyed by domain, apps by name.
pub const APP_KEY: &str = "COALESCE(NULLIF(s.domain, ''), s.app)";

/// A SQL fragment and the values it binds.
pub struct Clause {
    pub sql: String,
    pub params: Vec<Value>,
}

/// The filter as a SQL clause over the `time_entries te` alias.
///
/// The clause expects the `projects p` alias to be joined, because a filter
/// on the client of an entry's project needs it.
pub fn clause(filter: &EntryFilter) -> Result<Clause, String> {
    if filter.end_ms > i64::MAX as u64 {
        return Err("range exceeds the supported timestamp".to_string());
    }
    if filter.end_ms <= filter.start_ms {
        return Err("the range must end after it starts".to_string());
    }
    let mut sql = vec![
        "te.deleted_at IS NULL".to_string(),
        "te.started_at >= ?".to_string(),
        "te.started_at < ?".to_string(),
    ];
    let mut values: Vec<Value> = vec![
        (filter.start_ms as i64).into(),
        (filter.end_ms as i64).into(),
    ];

    id_condition("te.category_id", &filter.category_id, &mut sql, &mut values);
    id_condition("te.project_id", &filter.project_id, &mut sql, &mut values);
    id_condition("p.client_id", &filter.client_id, &mut sql, &mut values);

    if let Some(app) = filter.app.as_deref().filter(|app| !app.is_empty()) {
        sql.push(format!(
            "EXISTS (SELECT 1 FROM segments s WHERE s.entry_id = te.id AND s.kind != 'break' AND {APP_KEY} = ?)"
        ));
        values.push(app.to_string().into());
    }
    match filter.status.as_deref() {
        None | Some("") => {}
        Some("approved") => sql.push("te.status = 'approved'".to_string()),
        Some("pending") => sql.push("te.status != 'approved'".to_string()),
        Some(other) => return Err(format!("unknown status filter {other}")),
    }
    if let Some(billable) = filter.billable {
        sql.push("te.billable = ?".to_string());
        values.push((billable as i64).into());
    }
    for term in search_terms(filter.search.as_deref().unwrap_or("")) {
        let pattern = format!("%{term}%");
        sql.push(
            "(te.description LIKE ? OR te.id IN (
                   SELECT s.entry_id FROM segments s
                   WHERE s.entry_id IS NOT NULL
                     AND s.id IN (SELECT rowid FROM segments_fts WHERE title LIKE ?)))"
                .to_string(),
        );
        values.push(pattern.clone().into());
        values.push(pattern.into());
    }
    Ok(Clause {
        sql: sql.join(" AND "),
        params: values,
    })
}

/// An id filter: absent means "any", `none` means "no id at all".
fn id_condition(column: &str, value: &Option<String>, sql: &mut Vec<String>, out: &mut Vec<Value>) {
    match value.as_deref() {
        None | Some("") => {}
        Some(aevio_core::NONE) => sql.push(format!("{column} IS NULL")),
        Some(id) => {
            sql.push(format!("{column} = ?"));
            out.push(id.to_string().into());
        }
    }
}

/// The words a search filter looks for, cleaned of wildcard characters.
fn search_terms(search: &str) -> Vec<String> {
    search
        .split_whitespace()
        .map(|term| term.replace('%', ""))
        .filter(|term| !term.is_empty())
        .take(MAX_SEARCH_TERMS)
        .collect()
}

/// Reads one entry row, in the column order `query_entries` selects.
pub fn time_entry_from_row(row: &Row<'_>) -> rusqlite::Result<TimeEntry> {
    Ok(TimeEntry {
        id: row.get(0)?,
        started_at: row.get::<_, i64>(1)? as u64,
        ended_at: row.get::<_, i64>(2)? as u64,
        description: row.get(3)?,
        category_id: row.get(4)?,
        project_id: row.get(5)?,
        status: row.get(6)?,
        approved_by: row.get(7)?,
        source: row.get(8)?,
        billable: row.get::<_, i64>(9)? != 0,
        invoice_id: row.get(10)?,
        created_at: row.get::<_, i64>(11)? as u64,
        updated_at: row.get::<_, i64>(12)? as u64,
        deleted_at: row.get::<_, Option<i64>>(13)?.map(|value| value as u64),
        description_origin: row.get(14)?,
        ai: None,
        dominant_app: None,
    })
}

/// Entries matching a filter, newest first.
pub fn query_entries(
    conn: &Connection,
    filter: &EntryFilter,
    limit: u32,
) -> Result<Vec<TimeEntry>, String> {
    let clause = clause(filter)?;
    let mut values = clause.params;
    values.push((limit.clamp(1, MAX_QUERY_ROWS) as i64).into());
    let mut stmt = conn
        .prepare(&format!(
            "SELECT te.id, te.started_at, te.ended_at, te.description, te.category_id, te.project_id,
                te.status, te.approved_by, te.source, te.billable, te.invoice_id, te.created_at,
                te.updated_at, te.deleted_at, te.description_origin
         FROM time_entries te LEFT JOIN projects p ON p.id = te.project_id
         WHERE {} ORDER BY te.started_at DESC, te.id ASC LIMIT ?;",
            clause.sql
        ))
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(params_from_iter(values), time_entry_from_row)
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<_>>()
        .map_err(|error| error.to_string())
}
