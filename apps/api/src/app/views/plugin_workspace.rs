use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use serde_json::from_str;

use aevio_contracts::plugin::{
    PluginUpdateChannel, PluginWorkspaceDesiredState, PluginWorkspaceDesiredStateInput,
};
use aevio_services::identity::Role;

use crate::app::authz::{authenticate, require_workspace};
use crate::app::core::database::{DatabaseBackend, PluginWorkspaceDesiredStateRecord, Repository};
use crate::app::services::auth::unix_now;
use crate::app::shared::ApiError;
use crate::app::state::AppState;

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<String>,
) -> Result<Json<Vec<PluginWorkspaceDesiredState>>, ApiError> {
    let db: &DatabaseBackend = state.connect().await?;
    let actor = authenticate(&headers, &state, db).await?;
    require_workspace(db, &state, &actor, &workspace_id, Role::Viewer).await?;
    let records = Repository::new(db)
        .workspace_plugin_states(&workspace_id)
        .await
        .map_err(ApiError::internal)?;
    records
        .into_iter()
        .map(to_contract)
        .collect::<Result<Vec<_>, _>>()
        .map(Json)
}

pub async fn put(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, plugin_id)): Path<(String, String)>,
    Json(input): Json<PluginWorkspaceDesiredStateInput>,
) -> Result<Json<PluginWorkspaceDesiredState>, ApiError> {
    validate_segment(&workspace_id, "workspace id")?;
    validate_segment(&plugin_id, "plugin id")?;
    validate_input(&input)?;
    let db: &DatabaseBackend = state.connect().await?;
    let actor = authenticate(&headers, &state, db).await?;
    require_workspace(db, &state, &actor, &workspace_id, Role::Admin).await?;
    if !known_plugin(db, &plugin_id).await? {
        return Err(ApiError::not_found("plugin does not exist"));
    }
    let permissions =
        serde_json::to_string(&input.requested_permissions).map_err(ApiError::internal)?;
    let record = Repository::new(db)
        .upsert_workspace_plugin_state(
            workspace_id,
            plugin_id,
            input.requested_version,
            input.channel.as_str().into(),
            input.enabled,
            permissions,
            unix_now(),
        )
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(to_contract(record)?))
}

async fn known_plugin(db: &DatabaseBackend, plugin_id: &str) -> Result<bool, ApiError> {
    if aevio_services::plugins::public_catalog()
        .iter()
        .any(|entry| entry.manifest.id == plugin_id)
    {
        return Ok(true);
    }
    Ok(Repository::new(db)
        .plugin_registry_rows()
        .await
        .map_err(ApiError::internal)?
        .iter()
        .any(|entry| entry.plugin_id == plugin_id))
}

fn to_contract(
    record: PluginWorkspaceDesiredStateRecord,
) -> Result<PluginWorkspaceDesiredState, ApiError> {
    let channel = PluginUpdateChannel::parse(&record.channel)
        .ok_or_else(|| ApiError::internal("stored plugin update channel is invalid"))?;
    let requested_permissions =
        from_str(&record.requested_permissions_json).map_err(ApiError::internal)?;
    Ok(PluginWorkspaceDesiredState {
        workspace_id: record.workspace_id,
        plugin_id: record.plugin_id,
        requested_version: record.requested_version,
        channel,
        enabled: record.enabled,
        requested_permissions,
        revision: record.revision,
        updated_at: record.updated_at,
    })
}

fn validate_input(input: &PluginWorkspaceDesiredStateInput) -> Result<(), ApiError> {
    if let Some(version) = input.requested_version.as_deref() {
        semver::Version::parse(version)
            .map_err(|_| ApiError::invalid_request("requested plugin version is invalid"))?;
    }
    if input.requested_permissions.len() > 64
        || input.requested_permissions.iter().any(|permission| {
            permission.is_empty()
                || permission.len() > 160
                || permission.chars().any(char::is_control)
        })
    {
        return Err(ApiError::invalid_request(
            "requested plugin permissions are invalid",
        ));
    }
    let mut unique = input.requested_permissions.clone();
    unique.sort_unstable();
    unique.dedup();
    if unique.len() != input.requested_permissions.len() {
        return Err(ApiError::invalid_request(
            "requested plugin permissions must be unique",
        ));
    }
    Ok(())
}

fn validate_segment(value: &str, label: &str) -> Result<(), ApiError> {
    if value.is_empty() || value.len() > 160 || value.chars().any(char::is_control) {
        return Err(ApiError::invalid_request(format!("invalid {label}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_input, validate_segment};
    use aevio_contracts::plugin::{PluginUpdateChannel, PluginWorkspaceDesiredStateInput};

    #[test]
    fn desired_state_input_rejects_duplicate_permissions() {
        let input = PluginWorkspaceDesiredStateInput {
            requested_version: Some("1.2.3".into()),
            channel: PluginUpdateChannel::Stable,
            enabled: true,
            requested_permissions: vec!["tasks:read".into(), "tasks:read".into()],
        };
        assert!(validate_input(&input).is_err());
        assert!(validate_segment("workspace-a", "workspace id").is_ok());
        assert!(validate_segment("bad\nworkspace", "workspace id").is_err());
    }
}
