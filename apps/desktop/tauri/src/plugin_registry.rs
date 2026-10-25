//! Remote plugin-registry transport.
//!
//! Credentials stay in the invoking command only. The registry is never
//! allowed to stage an artifact without the same local package verifier used
//! by file-based installs.

use std::time::Duration;

use aevio_contracts::plugin::{
    PluginCatalogEntry, PluginInstallationScope, PluginInstallationSummary, PluginManifest,
    PluginRelease, PluginUpdateCandidate, PluginWorkspaceDesiredState,
    PluginWorkspaceDesiredStateInput, MAX_ASSET_BYTES,
};
use aevio_infrastructure::plugins_package::{stage_package, verify_package};
use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager};
use url::Url;

use crate::commands::{with_database, with_database_reader};
use crate::AppState;

const MAX_CATALOG_BYTES: usize = 4 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteInstallRequest {
    pub registry_url: String,
    pub bearer_token: String,
    pub plugin_id: String,
    pub release: PluginRelease,
    pub scope: Option<PluginInstallationScope>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub enable: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePluginStatesRequest {
    pub registry_url: String,
    pub bearer_token: String,
    pub workspace_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetWorkspacePluginStateRequest {
    pub registry_url: String,
    pub bearer_token: String,
    pub workspace_id: String,
    pub plugin_id: String,
    pub input: PluginWorkspaceDesiredStateInput,
}

#[tauri::command(rename = "plugin_registry_catalog")]
pub async fn catalog(
    registry_url: String,
    bearer_token: String,
) -> Result<Vec<PluginCatalogEntry>, String> {
    let endpoint = endpoint(&registry_url, "api/v1/plugins")?;
    let response = client()?
        .get(endpoint)
        .bearer_auth(required_token(&bearer_token)?)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;
    let bytes = read_bounded(response, MAX_CATALOG_BYTES).await?;
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid plugin catalog: {error}"))
}

#[tauri::command(rename = "plugin_registry_updates")]
pub async fn updates(
    app: AppHandle,
    registry_url: String,
    bearer_token: String,
) -> Result<Vec<PluginUpdateCandidate>, String> {
    let remote = catalog(registry_url, bearer_token).await?;
    with_database_reader(&app, |database| {
        aevio_services::plugins_update::available_updates(database, &remote)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command(rename = "plugin_workspace_plugin_states")]
pub async fn workspace_plugin_states(
    request: WorkspacePluginStatesRequest,
) -> Result<Vec<PluginWorkspaceDesiredState>, String> {
    validate_segment(&request.workspace_id, "workspace id")?;
    let endpoint = endpoint(
        &request.registry_url,
        &format!("api/v1/workspaces/{}/plugins", request.workspace_id),
    )?;
    let response = client()?
        .get(endpoint)
        .bearer_auth(required_token(&request.bearer_token)?)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;
    let bytes = read_bounded(response, MAX_CATALOG_BYTES).await?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid workspace plugin state: {error}"))
}

#[tauri::command(rename = "plugin_set_workspace_plugin_state")]
pub async fn set_workspace_plugin_state(
    request: SetWorkspacePluginStateRequest,
) -> Result<PluginWorkspaceDesiredState, String> {
    validate_segment(&request.workspace_id, "workspace id")?;
    validate_segment(&request.plugin_id, "plugin id")?;
    let endpoint = endpoint(
        &request.registry_url,
        &format!(
            "api/v1/workspaces/{}/plugins/{}",
            request.workspace_id, request.plugin_id
        ),
    )?;
    let response = client()?
        .put(endpoint)
        .bearer_auth(required_token(&request.bearer_token)?)
        .json(&request.input)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;
    let bytes = read_bounded(response, MAX_CATALOG_BYTES).await?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid workspace plugin state: {error}"))
}

#[tauri::command(rename = "plugin_install_remote")]
pub async fn install(
    app: AppHandle,
    request: RemoteInstallRequest,
) -> Result<PluginInstallationSummary, String> {
    let token = required_token(&request.bearer_token)?.to_owned();
    validate_segment(&request.plugin_id, "plugin id")?;
    validate_segment(&request.release.version, "plugin version")?;
    let endpoint = endpoint(
        &request.registry_url,
        &format!(
            "api/v1/plugins/{}/releases/{}/artifact",
            request.plugin_id, request.release.version
        ),
    )?;
    let response = client()?
        .get(endpoint)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;
    let bytes = read_bounded(response, MAX_ASSET_BYTES as usize).await?;
    let verified = verify_package(&bytes, &request.release).map_err(|error| error.to_string())?;
    let manifest: PluginManifest = verified.artifact.manifest.clone();
    if manifest.id != request.plugin_id {
        return Err("plugin artifact identity does not match the requested plugin".into());
    }
    let state = app.state::<AppState>();
    let root = state
        .database_path
        .parent()
        .ok_or_else(|| "Aevio data directory is unavailable".to_string())?
        .join("plugins");
    let staged = stage_package(&verified, &root).map_err(|error| error.to_string())?;
    let mut persisted = false;
    let result = with_database(&app, |database| {
        let mut installation = aevio_services::plugins_artifacts::install_external(
            database,
            manifest,
            request.release,
            request.scope.unwrap_or(PluginInstallationScope::Personal),
            &staged.path,
            aevio_core::timers::now_epoch_ms() as i64,
        )
        .map_err(|error| error.to_string())?;
        persisted = true;
        if !request.permissions.is_empty() {
            installation = aevio_services::plugins::grant_permissions(
                database,
                &installation.plugin_id,
                request.permissions,
                aevio_core::timers::now_epoch_ms() as i64,
            )
            .map_err(|error| error.to_string())?;
        }
        if request.enable {
            installation = aevio_services::plugins::set_enabled(
                database,
                &installation.plugin_id,
                true,
                aevio_core::timers::now_epoch_ms() as i64,
            )
            .map_err(|error| error.to_string())?;
        }
        Ok(installation)
    });
    if result.is_err() && !persisted {
        let _ = std::fs::remove_dir_all(&staged.path);
    } else {
        let _ = app.emit(crate::EVENT_PLUGINS_CHANGED, ());
    }
    result
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|error| error.to_string())
}

fn endpoint(registry_url: &str, suffix: &str) -> Result<Url, String> {
    let mut base = Url::parse(registry_url.trim())
        .map_err(|error| format!("invalid registry URL: {error}"))?;
    if !matches!(base.scheme(), "https" | "http")
        || base.host_str().is_none()
        || base.username() != ""
        || base.password().is_some()
        || base.query().is_some()
        || base.fragment().is_some()
    {
        return Err("registry URL must be an HTTP(S) URL without credentials or query data".into());
    }
    let base_path = base.path().trim_end_matches('/');
    base.set_path(&format!("{base_path}/{suffix}"));
    Ok(base)
}

fn required_token(token: &str) -> Result<&str, String> {
    let token = token.trim();
    if token.is_empty() || token.len() > 4096 || token.chars().any(char::is_control) {
        return Err("a non-empty registry bearer token is required".into());
    }
    Ok(token)
}

fn validate_segment(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 200
        || value.contains('/')
        || value.contains('\\')
        || value.chars().any(char::is_control)
    {
        return Err(format!("invalid {label}"));
    }
    Ok(())
}

async fn read_bounded(mut response: reqwest::Response, limit: usize) -> Result<Vec<u8>, String> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err("remote plugin response exceeds the host size limit".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err("remote plugin response exceeds the host size limit".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
