//! Time-entry models and the activity-to-entry builder.

pub mod builder;
pub mod models;
pub mod query;

pub use builder::{BuiltTimeEntry, EntrySettings, SegmentInput, build_entries, describe};
pub use models::*;
pub use query::{EntryFilter, MAX_QUERY_ROWS, NONE};
