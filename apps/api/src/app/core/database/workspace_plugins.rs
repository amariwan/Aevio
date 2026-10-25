use super::{DatabaseBackend, Repository, postgres, sqlite};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginWorkspaceDesiredStateRecord {
    pub workspace_id: String,
    pub plugin_id: String,
    pub requested_version: Option<String>,
    pub channel: String,
    pub enabled: bool,
    pub requested_permissions_json: String,
    pub revision: i64,
    pub updated_at: i64,
}

impl<'db> Repository<'db> {
    pub async fn workspace_plugin_states(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<PluginWorkspaceDesiredStateRecord>, String> {
        match self.database() {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| {
                    sqlite::Repository::new(&database).workspace_plugin_states(workspace_id)
                })
                .map(|rows| rows.into_iter().map(sqlite_state).collect())
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .workspace_plugin_states(workspace_id)
                .await
                .map(|rows| rows.into_iter().map(postgres_state).collect())
                .map_err(|error| error.to_string()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_workspace_plugin_state(
        &self,
        workspace_id: String,
        plugin_id: String,
        requested_version: Option<String>,
        channel: String,
        enabled: bool,
        requested_permissions_json: String,
        now: i64,
    ) -> Result<PluginWorkspaceDesiredStateRecord, String> {
        match self.database() {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| {
                    sqlite::Repository::new(&database).upsert_workspace_plugin_state(
                        workspace_id,
                        plugin_id,
                        requested_version,
                        channel,
                        enabled,
                        requested_permissions_json,
                        now,
                    )
                })
                .map(sqlite_state)
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .upsert_workspace_plugin_state(
                    workspace_id,
                    plugin_id,
                    requested_version,
                    channel,
                    enabled,
                    requested_permissions_json,
                    now,
                )
                .await
                .map(postgres_state)
                .map_err(|error| error.to_string()),
        }
    }
}

fn sqlite_state(row: sqlite::PluginWorkspaceDesiredStateRow) -> PluginWorkspaceDesiredStateRecord {
    PluginWorkspaceDesiredStateRecord {
        workspace_id: row.workspace_id,
        plugin_id: row.plugin_id,
        requested_version: row.requested_version,
        channel: row.channel,
        enabled: row.enabled,
        requested_permissions_json: row.requested_permissions_json,
        revision: row.revision,
        updated_at: row.updated_at,
    }
}

fn postgres_state(
    row: postgres::PluginWorkspaceDesiredStateRow,
) -> PluginWorkspaceDesiredStateRecord {
    PluginWorkspaceDesiredStateRecord {
        workspace_id: row.workspace_id,
        plugin_id: row.plugin_id,
        requested_version: row.requested_version,
        channel: row.channel,
        enabled: row.enabled,
        requested_permissions_json: row.requested_permissions_json,
        revision: row.revision,
        updated_at: row.updated_at,
    }
}
