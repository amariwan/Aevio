use std::sync::Arc;

use crate::app::shared::ApiError;
use aevio_services::identity::ServiceScope;

use crate::app::config::ServerConfig;
use crate::app::core::database::DatabaseBackend;

/// Per-process state. One SQLite connection is opened per request because the
/// runtime is synchronous and the server holds no connection pool.
#[derive(Clone)]
pub struct AppState {
    pub database: Arc<DatabaseBackend>,
    pub token: Arc<String>,
    pub workspace: Option<String>,
    pub allow_registration: bool,
    pub service_scope: Arc<ServiceScope>,
    pub plugin_artifact_dir: Arc<std::path::PathBuf>,
}

impl AppState {
    pub fn new(config: &ServerConfig, database: DatabaseBackend) -> Self {
        Self {
            database: Arc::new(database),
            token: Arc::new(config.token.clone()),
            workspace: config.workspace.clone(),
            allow_registration: config.allow_registration,
            service_scope: Arc::new(config.service_scope.clone()),
            plugin_artifact_dir: Arc::new(config.plugin_artifact_dir.clone()),
        }
    }

    pub async fn connect(&self) -> Result<&DatabaseBackend, ApiError> {
        Ok(self.database.as_ref())
    }

    pub fn service_scope(&self) -> ServiceScope {
        (*self.service_scope).clone()
    }
}
