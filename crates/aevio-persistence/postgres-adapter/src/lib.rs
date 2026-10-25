use sea_orm::{Database, DbErr};

pub use sea_orm::DatabaseConnection;

pub mod entities;
pub mod migrations;
pub mod repository;

pub use repository::{PluginRegistryRow, PluginWorkspaceDesiredStateRow, Repository};

pub async fn connect(url: &str) -> Result<DatabaseConnection, DbErr> {
    Database::connect(url).await
}

/// Applies the versioned schema migrations (see `migrations`). Request handlers
/// never execute this SQL.
pub async fn initialize(database: &DatabaseConnection) -> Result<(), DbErr> {
    migrations::apply(database).await
}
