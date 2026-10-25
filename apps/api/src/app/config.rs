use std::env;
use std::path::PathBuf;

use aevio_services::identity::ServiceScope;

/// Shortest accepted service token. Machine credentials must not be guessable.
const MIN_TOKEN_LENGTH: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DatabaseConfig {
    Sqlite(PathBuf),
    Postgres(String),
}

pub struct ServerConfig {
    pub database: DatabaseConfig,
    pub token: String,
    pub bind: String,
    pub workspace: Option<String>,
    pub allow_registration: bool,
    pub admin_email: Option<String>,
    pub admin_password: Option<String>,
    pub workspace_name: String,
    pub service_scope: ServiceScope,
    pub plugin_artifact_dir: PathBuf,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let admin_email = env::var("AEVIO_SERVER_ADMIN_EMAIL").ok();
        let admin_password = env::var("AEVIO_SERVER_ADMIN_PASSWORD").ok();
        if admin_email.is_some() != admin_password.is_some() {
            return Err("AEVIO_SERVER_ADMIN_EMAIL and AEVIO_SERVER_ADMIN_PASSWORD must be configured together".into());
        }
        let token = env::var("AEVIO_SERVER_TOKEN").map_err(|_| "AEVIO_SERVER_TOKEN must be set")?;
        if token.trim().len() < MIN_TOKEN_LENGTH {
            return Err(format!(
                "AEVIO_SERVER_TOKEN must be at least {MIN_TOKEN_LENGTH} characters"
            )
            .into());
        }
        let workspace = env::var("AEVIO_SERVER_WORKSPACE")
            .ok()
            .filter(|value| !value.is_empty());
        let default_permissions = "read,write".to_owned();
        let service_scope = resolve_service_scope(
            workspace.as_deref(),
            &env::var("AEVIO_SERVER_TOKEN_WORKSPACES").unwrap_or_default(),
            &env::var("AEVIO_SERVER_TOKEN_PERMISSIONS").unwrap_or(default_permissions),
        )?;
        Ok(Self {
            database: resolve_database(
                env::var("AEVIO_SERVER_DATABASE_URL").ok().as_deref(),
                env::var("AEVIO_SERVER_DATABASE").ok().as_deref(),
            )?,
            token: token.trim().to_owned(),
            bind: env::var("AEVIO_SERVER_BIND").unwrap_or_else(|_| "127.0.0.1:47832".into()),
            workspace,
            allow_registration: env::var("AEVIO_SERVER_ALLOW_REGISTRATION")
                .is_ok_and(|value| value == "true"),
            admin_email,
            admin_password,
            workspace_name: env::var("AEVIO_SERVER_WORKSPACE_NAME")
                .unwrap_or_else(|_| "Main workspace".into()),
            service_scope,
            plugin_artifact_dir: PathBuf::from(
                env::var("AEVIO_SERVER_PLUGIN_ARTIFACT_DIR")
                    .unwrap_or_else(|_| "aevio-plugin-artifacts".into()),
            ),
        })
    }
}

fn resolve_database(
    database_url: Option<&str>,
    sqlite_path: Option<&str>,
) -> Result<DatabaseConfig, Box<dyn std::error::Error>> {
    match database_url {
        Some(url) if url.starts_with("postgres://") || url.starts_with("postgresql://") => {
            Ok(DatabaseConfig::Postgres(url.to_owned()))
        }
        Some(_) => Err("AEVIO_SERVER_DATABASE_URL must be a PostgreSQL URL".into()),
        None => Ok(DatabaseConfig::Sqlite(PathBuf::from(
            sqlite_path.unwrap_or("aevio-server.db"),
        ))),
    }
}

/// The deployment token is never global: without an explicit workspace it
/// cannot authenticate anything, and a single-workspace deployment cannot be
/// widened beyond that workspace.
fn resolve_service_scope(
    deployment_workspace: Option<&str>,
    workspaces: &str,
    permissions: &str,
) -> Result<ServiceScope, Box<dyn std::error::Error>> {
    let workspaces = if workspaces.trim().is_empty() {
        deployment_workspace.unwrap_or_default()
    } else {
        workspaces
    };
    let scope = ServiceScope::from_csv(workspaces, permissions);
    if scope.is_empty() {
        return Err(
            "AEVIO_SERVER_TOKEN_WORKSPACES (or AEVIO_SERVER_WORKSPACE) must name at least one workspace and AEVIO_SERVER_TOKEN_PERMISSIONS at least one of read, write, admin"
                .into(),
        );
    }
    if let Some(allowed) = deployment_workspace
        && scope
            .workspaces()
            .iter()
            .any(|workspace| workspace != allowed)
    {
        return Err(format!(
            "AEVIO_SERVER_TOKEN_WORKSPACES must not exceed AEVIO_SERVER_WORKSPACE ({allowed})"
        )
        .into());
    }
    Ok(scope)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{DatabaseConfig, MIN_TOKEN_LENGTH, resolve_database, resolve_service_scope};
    use aevio_services::identity::{Permission, Role};

    #[test]
    fn deployment_workspace_is_the_default_token_scope() {
        let scope = resolve_service_scope(Some("workspace-a"), "", "read,write").expect("scope");
        assert_eq!(scope.workspaces(), ["workspace-a"]);
        assert_eq!(scope.permissions(), [Permission::Read, Permission::Write]);
        const { assert!(MIN_TOKEN_LENGTH >= 16) };
    }

    #[test]
    fn token_permissions_default_to_read_and_write() {
        let scope = resolve_service_scope(None, "workspace-a", "read,write").expect("scope");
        assert!(!scope.allows_permission(Permission::Admin));
        assert!(scope.allows("workspace-a", Permission::Write));
        assert!(!scope.allows("workspace-a", Role::Admin.permission()));
    }

    #[test]
    fn tokens_without_scope_are_rejected() {
        assert!(resolve_service_scope(None, "", "read,write").is_err());
        assert!(resolve_service_scope(Some("workspace-a"), "", "").is_err());
        assert!(resolve_service_scope(None, "workspace-a", "bogus").is_err());
    }

    #[test]
    fn a_single_workspace_deployment_cannot_be_widened() {
        assert!(
            resolve_service_scope(Some("workspace-a"), "workspace-a,workspace-b", "read").is_err()
        );
        let scope =
            resolve_service_scope(Some("workspace-a"), " workspace-a ", "read").expect("scope");
        assert_eq!(scope.workspaces(), ["workspace-a"]);
    }

    #[test]
    fn postgres_url_selects_the_postgres_adapter() {
        assert_eq!(
            resolve_database(Some("postgres://db/aevio"), None).expect("database"),
            DatabaseConfig::Postgres("postgres://db/aevio".into())
        );
    }

    #[test]
    fn sqlite_path_remains_the_offline_server_default() {
        assert_eq!(
            resolve_database(None, None).expect("database"),
            DatabaseConfig::Sqlite(PathBuf::from("aevio-server.db"))
        );
        assert_eq!(
            resolve_database(None, Some("custom.db")).expect("database"),
            DatabaseConfig::Sqlite(PathBuf::from("custom.db"))
        );
    }

    #[test]
    fn only_postgres_urls_are_accepted_as_remote_server_databases() {
        assert!(resolve_database(Some("mysql://db/aevio"), None).is_err());
        assert!(resolve_database(Some(""), None).is_err());
    }
}
