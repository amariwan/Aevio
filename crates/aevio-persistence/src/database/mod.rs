mod connection;
pub mod entities;
pub mod repositories;

pub use connection::connect;
pub use sea_orm::{self, DatabaseConnection};

#[cfg(test)]
mod query_plan_tests;
