//! The shared entry filter.
//!
//! The filter itself is domain: which entries a view asks for. Turning it
//! into SQL belongs to the persistence adapters, so this module holds only
//! the vocabulary and the shape of the request.

use serde::{Deserialize, Serialize};

/// The filter value meaning "no category / project / client".
pub const NONE: &str = "none";
/// The Log tab's ceiling. Anything bigger is a range to aggregate, not list.
pub const MAX_QUERY_ROWS: u32 = 5_000;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EntryFilter {
    pub start_ms: u64,
    pub end_ms: u64,
    pub category_id: Option<String>,
    pub project_id: Option<String>,
    pub client_id: Option<String>,
    pub app: Option<String>,
    pub status: Option<String>,
    pub billable: Option<bool>,
    pub search: Option<String>,
}
