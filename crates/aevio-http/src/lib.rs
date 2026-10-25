//! Shared Axum adapter for the Aevio HTTP protocol.
//!
//! Both server binaries expose the same wire contract. Putting the response
//! envelope and the status mapping in one crate keeps them from drifting.
pub mod auth;
pub mod error;
