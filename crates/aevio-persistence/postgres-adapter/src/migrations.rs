//! Versioned schema migrations for the PostgreSQL adapter.
//!
//! The schema is applied through an append-only ledger, so a fresh database and
//! an existing deployment follow exactly the same path. Every migration runs in
//! one transaction while a PostgreSQL advisory lock serializes concurrent starts;
//! `CREATE TABLE IF NOT EXISTS` alone is not race safe against a parallel creator.
//!
//! Never edit a released migration. Add the next version instead.

use std::collections::HashSet;

use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement, TransactionError,
    TransactionTrait, Value,
};

/// One append-only schema step.
pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

/// Released migrations, oldest first. The ledger key is `version`.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial_sync_schema",
        sql: INITIAL_SYNC_SCHEMA,
    },
    Migration {
        version: 2,
        name: "plugin_registry",
        sql: PLUGIN_REGISTRY_SCHEMA,
    },
    Migration {
        version: 3,
        name: "plugin_publisher_keys",
        sql: PLUGIN_PUBLISHER_KEYS_SCHEMA,
    },
    Migration {
        version: 4,
        name: "plugin_workspace_desired_state",
        sql: PLUGIN_WORKSPACE_DESIRED_STATE_SCHEMA,
    },
];

/// Schema of the version ledger itself.
const LEDGER: &str = "CREATE TABLE IF NOT EXISTS schema_migrations (
  version BIGINT PRIMARY KEY,
  name TEXT NOT NULL,
  applied_at BIGINT NOT NULL
);";

/// Two instances starting at the same time must serialize schema creation.
const ADVISORY_LOCK_KEY: i64 = 0x0A36_10F0;

/// Applies every pending migration and records it. Re-running is a no-op.
pub async fn apply(database: &DatabaseConnection) -> Result<(), DbErr> {
    database
        .transaction::<_, (), DbErr>(|tx| {
            Box::pin(async move {
                tx.execute_unprepared(&format!(
                    "SELECT pg_advisory_xact_lock({ADVISORY_LOCK_KEY})"
                ))
                .await?;
                tx.execute_unprepared(LEDGER).await?;
                let applied = applied_versions(tx).await?;
                for migration in MIGRATIONS {
                    if applied.contains(&migration.version) {
                        continue;
                    }
                    tx.execute_unprepared(migration.sql).await?;
                    record(tx, migration).await?;
                }
                Ok(())
            })
        })
        .await
        .map_err(|error| match error {
            TransactionError::Connection(error) | TransactionError::Transaction(error) => error,
        })
}

async fn applied_versions<C>(tx: &C) -> Result<HashSet<i64>, DbErr>
where
    C: ConnectionTrait + ?Sized,
{
    let rows = tx
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT version FROM schema_migrations",
        ))
        .await?;
    rows.iter()
        .map(|row| row.try_get::<i64>("", "version"))
        .collect()
}

async fn record<C>(tx: &C, migration: &Migration) -> Result<(), DbErr>
where
    C: ConnectionTrait + ?Sized,
{
    tx.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO schema_migrations (version, name, applied_at)
         VALUES ($1, $2, (EXTRACT(EPOCH FROM now()) * 1000)::BIGINT)",
        [Value::from(migration.version), Value::from(migration.name)],
    ))
    .await?;
    Ok(())
}

/// Migration 1: the original server schema, kept byte-for-byte additive so an
/// existing unversioned deployment is adopted without touching its data.
const INITIAL_SYNC_SCHEMA: &str = "\
        CREATE TABLE IF NOT EXISTS accounts (
          id TEXT PRIMARY KEY, email TEXT NOT NULL UNIQUE, password_hash TEXT NOT NULL,
          status TEXT NOT NULL DEFAULT 'active', created_at BIGINT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS access_tokens (
          token_hash TEXT PRIMARY KEY, account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
          created_at BIGINT NOT NULL, revoked_at BIGINT
        );
        CREATE TABLE IF NOT EXISTS workspaces (
          id TEXT PRIMARY KEY, name TEXT NOT NULL, created_at BIGINT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS workspace_memberships (
          workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
          account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
          role TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'member', 'viewer')),
          created_at BIGINT NOT NULL,
          PRIMARY KEY (workspace_id, account_id)
        );
        CREATE INDEX IF NOT EXISTS idx_memberships_account ON workspace_memberships(account_id);
        CREATE TABLE IF NOT EXISTS sync_operations (
          operation_id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, device_id TEXT NOT NULL,
          entity_type TEXT NOT NULL, entity_id TEXT NOT NULL, operation TEXT NOT NULL,
          payload TEXT NOT NULL, base_revision BIGINT, revision BIGSERIAL UNIQUE NOT NULL,
          created_at BIGINT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_sync_operations_workspace_revision ON sync_operations(workspace_id, revision);
        CREATE INDEX IF NOT EXISTS idx_sync_operations_entity ON sync_operations(workspace_id, entity_type, entity_id, revision);";

const PLUGIN_REGISTRY_SCHEMA: &str = "\
        CREATE TABLE IF NOT EXISTS plugin_registry (
          plugin_id TEXT PRIMARY KEY, publisher_id TEXT NOT NULL, manifest_json TEXT NOT NULL,
          created_at BIGINT NOT NULL, updated_at BIGINT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS plugin_registry_releases (
          plugin_id TEXT NOT NULL REFERENCES plugin_registry(plugin_id) ON DELETE CASCADE,
          version TEXT NOT NULL, artifact_digest TEXT NOT NULL, artifact_size BIGINT NOT NULL,
          signature TEXT NOT NULL, channel TEXT NOT NULL, published_at BIGINT NOT NULL,
          revoked BOOLEAN NOT NULL DEFAULT FALSE,
          PRIMARY KEY (plugin_id, version)
        );
        CREATE INDEX IF NOT EXISTS idx_plugin_registry_releases_digest
          ON plugin_registry_releases(artifact_digest);";

const PLUGIN_PUBLISHER_KEYS_SCHEMA: &str = "\
        CREATE TABLE IF NOT EXISTS plugin_publisher_keys (
          publisher_id TEXT NOT NULL, key_id TEXT NOT NULL, public_key TEXT NOT NULL,
          created_at BIGINT NOT NULL, expires_at BIGINT, revoked_at BIGINT,
          PRIMARY KEY (publisher_id, key_id)
        );
        CREATE INDEX IF NOT EXISTS idx_plugin_publisher_keys_active
          ON plugin_publisher_keys(publisher_id, revoked_at, expires_at);";

const PLUGIN_WORKSPACE_DESIRED_STATE_SCHEMA: &str = "\
        CREATE TABLE IF NOT EXISTS plugin_workspace_desired_state (
          workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
          plugin_id TEXT NOT NULL,
          requested_version TEXT,
          channel TEXT NOT NULL DEFAULT 'stable',
          enabled BOOLEAN NOT NULL DEFAULT TRUE,
          requested_permissions_json TEXT NOT NULL DEFAULT '[]',
          revision BIGINT NOT NULL DEFAULT 1,
          updated_at BIGINT NOT NULL,
          PRIMARY KEY (workspace_id, plugin_id)
        );
        CREATE INDEX IF NOT EXISTS idx_plugin_workspace_desired_state_workspace
          ON plugin_workspace_desired_state(workspace_id, updated_at);";
