use std::path::PathBuf;
use std::sync::Arc;

use aevio_contracts::sync::{SyncChange, SyncPushRequest, SyncPushResponse};

use crate::app::config::DatabaseConfig;

pub use aevio_persistence::server::sqlite;
pub use aevio_persistence_postgres as postgres;

mod registry;
pub use registry::PluginRegistryRecord;
mod workspace_plugins;
pub use workspace_plugins::PluginWorkspaceDesiredStateRecord;

#[derive(Clone)]
pub enum DatabaseBackend {
    Sqlite(PathBuf),
    Postgres(Arc<postgres::DatabaseConnection>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRecord {
    pub id: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    pub account_id: String,
    pub created_at: i64,
    pub revoked_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRecord {
    pub id: String,
    pub name: String,
}

impl DatabaseBackend {
    pub async fn open(config: &DatabaseConfig) -> Result<Self, String> {
        match config {
            DatabaseConfig::Sqlite(path) => {
                let database = sqlite::connect(path).map_err(|error| error.to_string())?;
                sqlite::initialize(&database).map_err(|error| error.to_string())?;
                Ok(Self::Sqlite(path.clone()))
            }
            DatabaseConfig::Postgres(url) => {
                let database = postgres::connect(url)
                    .await
                    .map_err(|error| error.to_string())?;
                postgres::initialize(&database)
                    .await
                    .map_err(|error| error.to_string())?;
                Ok(Self::Postgres(Arc::new(database)))
            }
        }
    }

    pub async fn ping(&self) -> bool {
        match self {
            Self::Sqlite(path) => {
                sqlite::connect(path).is_ok_and(|database| database.ping().is_ok())
            }
            Self::Postgres(database) => database.ping().await.is_ok(),
        }
    }

    pub fn storage_name(&self) -> &'static str {
        match self {
            Self::Sqlite(_) => "sqlite",
            Self::Postgres(_) => "postgres",
        }
    }
}

pub struct Repository<'db> {
    database: &'db DatabaseBackend,
}

impl<'db> Repository<'db> {
    pub fn new(database: &'db DatabaseBackend) -> Self {
        Self { database }
    }

    fn database(&self) -> &DatabaseBackend {
        self.database
    }

    pub async fn active_account_by_email(
        &self,
        email: &str,
    ) -> Result<Option<AccountRecord>, String> {
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| {
                    sqlite::Repository::new(&database).active_account_by_email(email)
                })
                .map(|account| account.map(sqlite_account))
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .active_account_by_email(email)
                .await
                .map(|account| account.map(postgres_account))
                .map_err(|error| error.to_string()),
        }
    }

    pub async fn session_for_token(
        &self,
        token_hash: &str,
    ) -> Result<Option<SessionRecord>, String> {
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| {
                    sqlite::Repository::new(&database).session_for_token(token_hash)
                })
                .map(|session| session.map(sqlite_session))
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .session_for_token(token_hash)
                .await
                .map(|session| session.map(postgres_session))
                .map_err(|error| error.to_string()),
        }
    }

    pub async fn insert_account(
        &self,
        id: String,
        email: String,
        password_hash: String,
        now: i64,
    ) -> Result<(), String> {
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| {
                    sqlite::Repository::new(&database).insert_account(id, email, password_hash, now)
                })
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .insert_account(id, email, password_hash, now)
                .await
                .map_err(|error| error.to_string()),
        }
    }

    pub async fn insert_account_if_unused(
        &self,
        id: String,
        email: String,
        password_hash: String,
        now: i64,
    ) -> Result<bool, String> {
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| {
                    sqlite::Repository::new(&database).insert_account_if_unused(
                        id,
                        email,
                        password_hash,
                        now,
                    )
                })
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .insert_account_if_unused(id, email, password_hash, now)
                .await
                .map_err(|error| error.to_string()),
        }
    }

    pub async fn insert_token(
        &self,
        token_hash: String,
        account_id: String,
        now: i64,
    ) -> Result<(), String> {
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| {
                    sqlite::Repository::new(&database).insert_token(token_hash, account_id, now)
                })
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .insert_token(token_hash, account_id, now)
                .await
                .map_err(|error| error.to_string()),
        }
    }

    pub async fn ensure_workspace(&self, id: String, name: String, now: i64) -> Result<(), String> {
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| {
                    sqlite::Repository::new(&database).ensure_workspace(id, name, now)
                })
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .ensure_workspace(id, name, now)
                .await
                .map_err(|error| error.to_string()),
        }
    }

    pub async fn workspace_rows(
        &self,
        account_id: &str,
    ) -> Result<Vec<(WorkspaceRecord, String)>, String> {
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| sqlite::Repository::new(&database).workspace_rows(account_id))
                .map(|rows| {
                    rows.into_iter()
                        .map(|(workspace, role)| (sqlite_workspace(workspace), role))
                        .collect()
                })
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .workspace_rows(account_id)
                .await
                .map(|rows| {
                    rows.into_iter()
                        .map(|(workspace, role)| (postgres_workspace(workspace), role))
                        .collect()
                })
                .map_err(|error| error.to_string()),
        }
    }

    pub async fn membership_role(
        &self,
        account_id: &str,
        workspace_id: &str,
    ) -> Result<Option<String>, String> {
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| {
                    sqlite::Repository::new(&database).membership_role(account_id, workspace_id)
                })
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .membership_role(account_id, workspace_id)
                .await
                .map_err(|error| error.to_string()),
        }
    }

    pub async fn owner_count(&self, workspace_id: &str) -> Result<u64, String> {
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| sqlite::Repository::new(&database).owner_count(workspace_id))
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .owner_count(workspace_id)
                .await
                .map_err(|error| error.to_string()),
        }
    }

    pub async fn upsert_membership(
        &self,
        workspace_id: String,
        account_id: String,
        role: String,
        now: i64,
    ) -> Result<(), String> {
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| {
                    sqlite::Repository::new(&database).upsert_membership(
                        workspace_id,
                        account_id,
                        role,
                        now,
                    )
                })
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .upsert_membership(workspace_id, account_id, role, now)
                .await
                .map_err(|error| error.to_string()),
        }
    }

    pub async fn push(&self, request: &SyncPushRequest) -> Result<SyncPushResponse, String> {
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| sqlite::Repository::new(&database).push(request))
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .push(request)
                .await
                .map_err(|error| error.to_string()),
        }
    }

    pub async fn pull(
        &self,
        workspace_id: &str,
        cursor: u64,
        limit: u64,
    ) -> Result<Vec<SyncChange>, String> {
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| {
                    sqlite::Repository::new(&database).pull(workspace_id, cursor, limit)
                })
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .pull(workspace_id, cursor, limit)
                .await
                .map_err(|error| error.to_string()),
        }
    }
}

fn sqlite_account(account: sqlite::entities::accounts::Model) -> AccountRecord {
    AccountRecord {
        id: account.id,
        password_hash: account.password_hash,
    }
}

fn postgres_account(account: postgres::entities::accounts::Model) -> AccountRecord {
    AccountRecord {
        id: account.id,
        password_hash: account.password_hash,
    }
}

fn sqlite_session(session: sqlite::repository::SessionRecord) -> SessionRecord {
    SessionRecord {
        account_id: session.account_id,
        created_at: session.created_at,
        revoked_at: session.revoked_at,
    }
}

fn postgres_session(session: postgres::repository::SessionRecord) -> SessionRecord {
    SessionRecord {
        account_id: session.account_id,
        created_at: session.created_at,
        revoked_at: session.revoked_at,
    }
}

fn sqlite_workspace(workspace: sqlite::entities::workspaces::Model) -> WorkspaceRecord {
    WorkspaceRecord {
        id: workspace.id,
        name: workspace.name,
    }
}

fn postgres_workspace(workspace: postgres::entities::workspaces::Model) -> WorkspaceRecord {
    WorkspaceRecord {
        id: workspace.id,
        name: workspace.name,
    }
}
