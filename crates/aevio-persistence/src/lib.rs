//! Persistence adapters for Aevio.
//!
//! SeaORM-based SQLite/PostgreSQL adapters live here, together with the
//! legacy rusqlite adapters that still back a few read paths. The crate owns
//! every database concern: schemas, migrations, repositories and the query
//! helpers that turn rows into domain models. `aevio-core` stays free of
//! persistence so the domain can be reasoned about without a database.

pub mod activity;
pub mod ai;
pub mod bootstrap;
pub mod coach;
pub mod coach_adaptive;
pub mod database;
pub mod entries;
pub mod projects;
pub mod reports;
pub mod server;

/// Canonical file name of the SQLite database inside the app data directory.
pub const DATABASE_FILE: &str = "activity.db";

/// Flatten a rusqlite error into the crate-wide `String` error type.
pub(crate) fn err(error: rusqlite::Error) -> String {
    error.to_string()
}
