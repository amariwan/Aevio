//! The report domain: what a pivot groups by, what a cell holds, and how a
//! filtered range is rendered into an export.
//!
//! Totals and pivots are computed over the entries themselves, so a year of
//! entries never crosses IPC just to be summed. Day, week, and month
//! boundaries are local-time concepts: the caller passes bucket boundaries as
//! epoch milliseconds instead of Rust guessing a timezone, so DST days come
//! out as 23 or 25 hours on their own.
//!
//! This module holds no database or filesystem code: the persistence adapters
//! read the rows, and this module turns them into cells and files' worth of
//! text.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Local};
use serde::Serialize;

/// Enough for a year by day, plus the closing boundary.
pub const MAX_BUCKETS: usize = 400;

/// What a pivot row is keyed by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupBy {
    Project,
    Client,
    Category,
    App,
    Status,
    None,
}

impl GroupBy {
    pub fn parse(value: &str) -> Result<Self, String> {
        Ok(match value {
            "project" => Self::Project,
            "client" => Self::Client,
            "category" => Self::Category,
            "app" => Self::App,
            "status" => Self::Status,
            "none" => Self::None,
            other => return Err(format!("unknown grouping {other}")),
        })
    }
}

/// One cell of a pivot: a group key in one bucket.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollupCell {
    /// The group's id (or app/status name); `None` is "No project",
    /// "Uncategoaeviod", "No client", or the single group of `none`.
    pub key: Option<String>,
    /// Index into the buckets the caller passed.
    pub bucket: u32,
    pub ms: u64,
    pub entries: u32,
    pub approved_ms: u64,
    /// Entries waiting for review (status `pending`).
    pub pending: u32,
    pub billable_ms: u64,
}

/// One captured segment of an entry, as a report needs it: which app or
/// site it belonged to and how long it lasted.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportSegment {
    pub app: String,
    /// The site the segment was on, when it was a browser.
    pub domain: Option<String>,
    /// The segment kind; `break` segments are excluded from every total.
    pub kind: String,
    pub started_at: u64,
    pub ended_at: Option<u64>,
}

/// One entry with the context a report needs: the entry itself, the names
/// the export prints, and the segments the app pivot sums over.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportEntry {
    pub started_at: u64,
    pub ended_at: u64,
    pub description: String,
    pub category_id: Option<String>,
    pub project_id: Option<String>,
    /// The client of the entry's project, when it has one.
    pub project_client_id: Option<String>,
    pub status: String,
    pub billable: bool,
    pub category_name: Option<String>,
    pub project_name: Option<String>,
    pub client_name: Option<String>,
    pub segments: Vec<ReportSegment>,
}

/// Sums the entries per group per bucket. `boundaries` holds n + 1 ascending
/// epoch-ms values for n buckets; an entry lands in the bucket its start
/// falls in. Grouping by app sums segment time instead, so one entry can
/// contribute to several apps.
pub fn rollup(
    entries: &[ReportEntry],
    boundaries: &[u64],
    group_by: GroupBy,
) -> Result<Vec<RollupCell>, String> {
    validate_boundaries(boundaries)?;

    let mut cells = HashMap::<(u32, Option<String>), RollupCell>::new();
    for entry in entries {
        let Some(bucket) = bucket_of(entry, boundaries) else {
            continue;
        };
        if group_by == GroupBy::App {
            let mut seen = HashSet::new();
            for segment in entry
                .segments
                .iter()
                .filter(|segment| segment.kind != "break")
            {
                let key = segment
                    .domain
                    .as_deref()
                    .filter(|domain| !domain.is_empty())
                    .unwrap_or(&segment.app)
                    .to_string();
                let ms = segment_ms(segment, entry);
                let count_entry = seen.insert(key.clone());
                add_rollup(&mut cells, bucket, Some(key), entry, ms, count_entry);
            }
        } else {
            let key = group_key(group_by, entry);
            let ms = entry.ended_at.saturating_sub(entry.started_at);
            add_rollup(&mut cells, bucket, key, entry, ms, true);
        }
    }

    let mut cells = cells.into_values().collect::<Vec<_>>();
    cells.sort_by(|left, right| {
        left.bucket
            .cmp(&right.bucket)
            .then_with(|| left.key.cmp(&right.key))
    });
    Ok(cells)
}

/// The bucket an entry falls into, by its start time.
fn bucket_of(entry: &ReportEntry, boundaries: &[u64]) -> Option<u32> {
    boundaries
        .windows(2)
        .position(|pair| entry.started_at >= pair[0] && entry.started_at < pair[1])
        .map(|bucket| bucket as u32)
}

/// The group an entry belongs to, for every grouping but `app`.
fn group_key(group_by: GroupBy, entry: &ReportEntry) -> Option<String> {
    match group_by {
        GroupBy::Project => entry.project_id.clone(),
        GroupBy::Client => entry.project_client_id.clone(),
        GroupBy::Category => entry.category_id.clone(),
        GroupBy::Status => Some(if entry.status == "approved" {
            "approved".into()
        } else {
            "pending".into()
        }),
        GroupBy::None => None,
        GroupBy::App => unreachable!("app grouping is handled per segment"),
    }
}

/// A segment's share of its entry, clamped to the entry's bounds so app
/// totals add up to the same time as every other grouping (the open segment
/// ends with its entry).
fn segment_ms(segment: &ReportSegment, entry: &ReportEntry) -> u64 {
    segment
        .ended_at
        .unwrap_or(entry.ended_at)
        .min(entry.ended_at)
        .saturating_sub(segment.started_at.max(entry.started_at))
}

fn add_rollup(
    cells: &mut HashMap<(u32, Option<String>), RollupCell>,
    bucket: u32,
    key: Option<String>,
    entry: &ReportEntry,
    ms: u64,
    count_entry: bool,
) {
    let cell = cells.entry((bucket, key.clone())).or_insert(RollupCell {
        key,
        bucket,
        ms: 0,
        entries: 0,
        approved_ms: 0,
        pending: 0,
        billable_ms: 0,
    });
    cell.ms = cell.ms.saturating_add(ms);
    if count_entry {
        cell.entries = cell.entries.saturating_add(1);
        if entry.status == "approved" {
            // Approved app rows are summed per segment, while the entry count
            // and pending count are distinct per app/entry pair.
            cell.approved_ms = cell.approved_ms.saturating_add(ms);
        } else {
            cell.pending = cell.pending.saturating_add(1);
        }
    } else if entry.status == "approved" {
        cell.approved_ms = cell.approved_ms.saturating_add(ms);
    }
    if entry.billable {
        cell.billable_ms = cell.billable_ms.saturating_add(ms);
    }
}

/// Checks that the bucket boundaries describe a usable set of buckets.
pub fn validate_boundaries(boundaries: &[u64]) -> Result<(), String> {
    if boundaries.len() < 2 || boundaries.len() > MAX_BUCKETS + 1 {
        return Err(format!(
            "expected 2 to {} bucket boundaries, got {}",
            MAX_BUCKETS + 1,
            boundaries.len()
        ));
    }
    if boundaries.windows(2).any(|pair| pair[1] <= pair[0]) {
        return Err("bucket boundaries must ascend".to_string());
    }
    Ok(())
}

// --- Export -----------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Json,
}

impl ExportFormat {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "csv" => Ok(Self::Csv),
            "json" => Ok(Self::Json),
            other => Err(format!("unknown export format {other}")),
        }
    }

    /// The file extension the export is written with.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
        }
    }
}

/// One exported row. Window titles and URLs are never included: an export
/// leaves the machine, and titles are the private part of the capture.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportRow {
    date: String,
    start: String,
    end: String,
    hours: f64,
    description: String,
    category: String,
    project: String,
    client: String,
    status: String,
    billable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    pub count: u32,
}

/// The timestamp of an epoch-millisecond instant in the machine's timezone.
pub fn local(ms: u64) -> DateTime<Local> {
    DateTime::from_timestamp_millis(ms as i64)
        .unwrap_or_default()
        .with_timezone(&Local)
}

/// The export file's contents and how many entries it holds.
pub fn export_body(entries: &[ReportEntry], format: ExportFormat) -> Result<(String, u32), String> {
    let rows: Vec<ExportRow> = entries.iter().map(export_row).collect();
    Ok((render(&rows, format)?, rows.len() as u32))
}

/// One entry as an export row.
fn export_row(entry: &ReportEntry) -> ExportRow {
    let started = local(entry.started_at);
    let ended = local(entry.ended_at);
    ExportRow {
        date: started.format("%Y-%m-%d").to_string(),
        start: started.format("%H:%M").to_string(),
        end: ended.format("%H:%M").to_string(),
        hours: (entry.ended_at.saturating_sub(entry.started_at) as f64 / 3_600_000.0 * 100.0)
            .round()
            / 100.0,
        description: entry.description.clone(),
        category: entry.category_name.clone().unwrap_or_default(),
        project: entry.project_name.clone().unwrap_or_default(),
        client: entry.client_name.clone().unwrap_or_default(),
        status: if entry.status == "approved" {
            "approved".to_string()
        } else {
            "pending".to_string()
        },
        billable: entry.billable,
    }
}

/// Quotes a CSV field when it needs it, and defuses spreadsheet formulas: a
/// description starting with `=` would otherwise run when the file opens.
fn csv_field(value: &str) -> String {
    let guarded = if value.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{value}")
    } else {
        value.to_string()
    };
    if guarded.contains([',', '"', '\n', '\r']) || guarded != guarded.trim() {
        format!("\"{}\"", guarded.replace('"', "\"\""))
    } else {
        guarded
    }
}

fn render(rows: &[ExportRow], format: ExportFormat) -> Result<String, String> {
    match format {
        ExportFormat::Json => serde_json::to_string_pretty(rows).map_err(|e| e.to_string()),
        ExportFormat::Csv => {
            let mut out = String::from(
                "date,start,end,hours,description,category,project,client,status,billable\n",
            );
            for row in rows {
                let fields = [
                    row.date.clone(),
                    row.start.clone(),
                    row.end.clone(),
                    format!("{:.2}", row.hours),
                    row.description.clone(),
                    row.category.clone(),
                    row.project.clone(),
                    row.client.clone(),
                    row.status.clone(),
                    row.billable.to_string(),
                ];
                let line: Vec<String> = fields.iter().map(|field| csv_field(field)).collect();
                out.push_str(&line.join(","));
                out.push('\n');
            }
            Ok(out)
        }
    }
}
