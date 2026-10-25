//! Report reads: filtered entries, pivots, and export bodies.
//!
//! The report domain (grouping, bucketing, export rendering) lives in
//! `aevio_core::reports`. This module supplies the two halves the domain
//! cannot have: reading the rows, and handing them to the domain.

pub mod query;

pub use aevio_core::reports::{
    ExportFormat, ExportResult, GroupBy, MAX_BUCKETS, ReportEntry, ReportSegment, RollupCell,
};
pub use aevio_core::{EntryFilter, MAX_QUERY_ROWS};

pub use query::{
    dominant_apps, export_body, export_body_with_database, export_entries_with_database,
    query_entries, query_entries_with_database, rollup, rollup_with_database,
};
