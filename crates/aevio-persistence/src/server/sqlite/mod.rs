use std::path::Path;

use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbErr};

pub mod entities;
pub mod repository;

pub use repository::{PluginRegistryRow, PluginWorkspaceDesiredStateRow, Repository};

pub fn connect(path: &Path) -> Result<DatabaseConnection, DbErr> {
    let mut options = ConnectOptions::new(format!("sqlite://{}?mode=rwc", path.display()));
    options
        .max_connections(1)
        .min_connections(1)
        .sqlx_logging(false);
    let database = Database::connect(options)?;
    // Required PRAGMAs for the documented single-instance SQLite server.
    database.execute_unprepared("PRAGMA foreign_keys = ON")?;
    database.execute_unprepared("PRAGMA busy_timeout = 5000")?;
    database.execute_unprepared("PRAGMA journal_mode = WAL")?;
    database.execute_unprepared("PRAGMA synchronous = NORMAL")?;
    Ok(database)
}

pub fn initialize(database: &DatabaseConnection) -> Result<(), DbErr> {
    migrate(database)
}

/// Historical server schema bootstrap. It is intentionally raw SQL and must not
/// be used by request handlers or repositories.
fn migrate(database: &DatabaseConnection) -> Result<(), DbErr> {
    database.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS accounts (
          id TEXT PRIMARY KEY, email TEXT NOT NULL UNIQUE, password_hash TEXT NOT NULL,
          status TEXT NOT NULL DEFAULT 'active', created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS access_tokens (
          token_hash TEXT PRIMARY KEY, account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
          created_at INTEGER NOT NULL, revoked_at INTEGER
        );
        CREATE TABLE IF NOT EXISTS workspaces (
          id TEXT PRIMARY KEY, name TEXT NOT NULL, created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS workspace_memberships (
          workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
          account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
          role TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'member', 'viewer')),
          created_at INTEGER NOT NULL,
          PRIMARY KEY (workspace_id, account_id)
        );
        CREATE INDEX IF NOT EXISTS idx_memberships_account ON workspace_memberships(account_id);
        CREATE TABLE IF NOT EXISTS sync_operations (
          operation_id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, device_id TEXT NOT NULL,
          entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, operation TEXT NOT NULL,
          payload TEXT NOT NULL, base_revision INTEGER, revision INTEGER NOT NULL UNIQUE,
          created_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_sync_operations_workspace_revision ON sync_operations(workspace_id, revision);
        CREATE INDEX IF NOT EXISTS idx_sync_operations_entity ON sync_operations(workspace_id, entity_type, entity_id, revision);
        CREATE TABLE IF NOT EXISTS plugin_registry (
          plugin_id TEXT PRIMARY KEY, publisher_id TEXT NOT NULL, manifest_json TEXT NOT NULL,
          created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS plugin_registry_releases (
          plugin_id TEXT NOT NULL REFERENCES plugin_registry(plugin_id) ON DELETE CASCADE,
          version TEXT NOT NULL, artifact_digest TEXT NOT NULL, artifact_size INTEGER NOT NULL,
          signature TEXT NOT NULL, channel TEXT NOT NULL, published_at INTEGER NOT NULL,
          revoked INTEGER NOT NULL DEFAULT 0,
          PRIMARY KEY (plugin_id, version)
        );
        CREATE INDEX IF NOT EXISTS idx_plugin_registry_releases_digest
          ON plugin_registry_releases(artifact_digest);
        CREATE TABLE IF NOT EXISTS plugin_publisher_keys (
          publisher_id TEXT NOT NULL, key_id TEXT NOT NULL, public_key TEXT NOT NULL,
          created_at INTEGER NOT NULL, expires_at INTEGER, revoked_at INTEGER,
          PRIMARY KEY (publisher_id, key_id)
        );
        CREATE INDEX IF NOT EXISTS idx_plugin_publisher_keys_active
          ON plugin_publisher_keys(publisher_id, revoked_at, expires_at);
        CREATE TABLE IF NOT EXISTS plugin_workspace_desired_state (
          workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
          plugin_id TEXT NOT NULL,
          requested_version TEXT,
          channel TEXT NOT NULL DEFAULT 'stable',
          enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
          requested_permissions_json TEXT NOT NULL DEFAULT '[]',
          revision INTEGER NOT NULL DEFAULT 1,
          updated_at INTEGER NOT NULL,
          PRIMARY KEY (workspace_id, plugin_id)
        );
        CREATE INDEX IF NOT EXISTS idx_plugin_workspace_desired_state_workspace
          ON plugin_workspace_desired_state(workspace_id, updated_at);",
    ).map(|_| ())
}
