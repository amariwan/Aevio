//! Runtime and operating-system adapters for Aevio.

pub mod decision_settings;
pub mod ipc;
pub mod paths;
pub mod plugins;
pub mod plugins_package;
pub mod settings_storage;
pub mod state;
pub mod timers_store;

pub use ipc::{protocol, rpc};
