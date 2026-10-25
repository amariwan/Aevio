//! Reusable application-level building blocks shared by every adapter.
//!
//! Identity, workspace access, session and membership policy plus auth
//! primitives live here so that HTTP, IPC and CLI adapters delegate to a
//! single implementation. Domain services re-export these for backwards
//! compatibility.

pub mod auth;
pub mod identity;
pub mod replay;
