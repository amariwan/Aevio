//! Everything about Aevio's data that needs no Tauri: the stores, their
//! schema, entry queries and reports, settings, where the app keeps its
//! files, and the protocol and local endpoint `aevio` uses. The app links it
//! for its windows and tracker; `aevio` links it to read and edit the same data
//! while the app is closed.

pub mod ai;
pub mod breaks;
pub mod capture;
pub mod categories;
pub mod clients;
pub mod coach;
pub mod decision;
pub mod energy;
pub mod entries;
pub mod integrations;
pub mod invoices;
pub mod music;
pub mod platform;
pub mod plugins;
pub mod projects;
pub mod reports;
pub mod settings;
pub mod timers;

// Compatibility facades for the public API used by the desktop app and CLI.
// New code should import the responsibility-based modules above directly.
pub use entries::query::{EntryFilter, MAX_QUERY_ROWS, NONE};
pub use entries::{builder as entry_builder, models};

/// Canonical installed app identity, also used by independent clients.
pub const APP_IDENTIFIER: &str = "com.offlinestudios.aevio";
pub const DATABASE_FILE: &str = "activity.db";
