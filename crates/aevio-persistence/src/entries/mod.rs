//! Entry queries: the filter vocabulary is domain, the SQL is persistence.
//!
//! Every view asks for entries through the same filter, so turning that
//! filter into SQL happens in exactly one place. The legacy rusqlite path
//! and the SeaORM path both read through these helpers, which keeps the
//! two adapters answering the same question the same way.

pub mod query;

pub use query::{APP_KEY, Clause, clause, query_entries, time_entry_from_row};
