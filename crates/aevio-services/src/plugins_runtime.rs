//! Safe host execution for declarative plugin contributions.
//!
//! This module executes host-owned first-party routes and bounded external
//! WebAssembly components. Native package code and unknown handlers fail closed
//! instead of becoming an accidental code execution path.

use aevio_contracts::plugin::{
    PluginCommandRequest, PluginCommandResult, PluginLifecycleState, PluginSearchAggregate,
    PluginSearchFailure, PluginSearchResult, PluginWorkflowRequest, capabilities,
};
use aevio_persistence::database::repositories::{
    catalog::CatalogRepository, music_tracks::MusicTrackRepository, plugins::PluginRepository,
};
use sea_orm::DatabaseConnection;
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;

use super::plugins::{CapabilityGateway, PluginServiceError, parse_state};
use super::plugins_wasm::{WasmLimits, execute as execute_wasm};

const MAX_SEARCH_RESULTS: usize = 50;

pub fn execute_command(
    db: &DatabaseConnection,
    request: PluginCommandRequest,
) -> Result<PluginCommandResult, PluginServiceError> {
    execute_command_with_effects(db, request).map(|(result, _)| result)
}

pub fn execute_command_with_effects(
    db: &DatabaseConnection,
    request: PluginCommandRequest,
) -> Result<
    (
        PluginCommandResult,
        Vec<super::plugins_host::PluginRuntimeEffect>,
    ),
    PluginServiceError,
> {
    execute_command_with_effects_for_device(db, request, "legacy-device")
}

pub fn execute_command_with_effects_for_device(
    db: &DatabaseConnection,
    request: PluginCommandRequest,
    device_ref: &str,
) -> Result<
    (
        PluginCommandResult,
        Vec<super::plugins_host::PluginRuntimeEffect>,
    ),
    PluginServiceError,
> {
    let device_ref = device_ref.to_owned();
    let (manifest, grants) = active_manifest(db, &request.plugin_id)?;
    let gateway = CapabilityGateway::new(request.plugin_id.clone(), grants);
    gateway.require(capabilities::COMMANDS_REGISTER)?;
    let command = manifest
        .contributes
        .commands
        .iter()
        .find(|item| item.id == request.command_id)
        .ok_or_else(|| PluginServiceError::NotFound(request.command_id.clone()))?;
    for capability in &command.required_capabilities {
        gateway.require(capability)?;
    }
    if !request.input.is_null() && !request.input.is_object() {
        return Err(PluginServiceError::InvalidState(
            "command input must be a JSON object".into(),
        ));
    }
    if let Some(target) = host_target(&manifest.id, &request.command_id) {
        return Ok((
            PluginCommandResult {
                plugin_id: manifest.id,
                command_id: command.id.clone(),
                kind: "open_host_route".into(),
                payload: json!({ "target": target }),
            },
            Vec::new(),
        ));
    }
    let component = manifest.entrypoints.component.as_deref().ok_or_else(|| {
        PluginServiceError::InvalidState("plugin command has no supported runtime".into())
    })?;
    let installation = PluginRepository::new(db)
        .installation(&manifest.id)?
        .ok_or_else(|| PluginServiceError::NotFound(manifest.id.clone()))?;
    let artifact_path = installation.artifact_path.ok_or_else(|| {
        PluginServiceError::InvalidState("plugin component artifact is unavailable".into())
    })?;
    let component_path = std::path::Path::new(&artifact_path).join(component);
    let bytes = std::fs::read(&component_path).map_err(|error| {
        PluginServiceError::InvalidState(format!("plugin component cannot be read: {error}"))
    })?;
    let plugin_id = manifest.id.clone();
    let runtime_database = db.clone();
    let runtime_plugin_id = plugin_id.clone();
    let effects = Rc::new(RefCell::new(Vec::new()));
    let runtime_effects = Rc::clone(&effects);
    let payload = execute_wasm(
        &bytes,
        &json!({ "commandId": request.command_id, "input": request.input }),
        WasmLimits::default(),
        move |method, payload| {
            super::plugins_host::wasm_host_call_for_device(
                &runtime_database,
                &runtime_plugin_id,
                method,
                payload,
                &mut runtime_effects.borrow_mut(),
                &device_ref,
            )
        },
    )
    .map_err(|error| PluginServiceError::InvalidState(error.to_string()))?;
    Ok((
        PluginCommandResult {
            plugin_id: manifest.id,
            command_id: command.id.clone(),
            kind: "wasm_command".into(),
            payload,
        },
        effects.borrow().clone(),
    ))
}

pub fn execute_workflow_action_with_effects_for_device(
    db: &DatabaseConnection,
    request: &PluginWorkflowRequest,
    device_ref: &str,
) -> Result<
    (
        serde_json::Value,
        Vec<super::plugins_host::PluginRuntimeEffect>,
    ),
    PluginServiceError,
> {
    let (manifest, grants) = active_manifest(db, &request.plugin_id)?;
    let gateway = CapabilityGateway::new(request.plugin_id.clone(), grants);
    let action = manifest
        .contributes
        .workflow_actions
        .iter()
        .find(|item| item.id == request.workflow_id)
        .ok_or_else(|| PluginServiceError::NotFound(request.workflow_id.clone()))?;
    for capability in &action.required_capabilities {
        gateway.require(capability)?;
    }
    if !request.input.is_null() && !request.input.is_object() {
        return Err(PluginServiceError::InvalidState(
            "workflow input must be a JSON object".into(),
        ));
    }
    let component = manifest.entrypoints.component.as_deref().ok_or_else(|| {
        PluginServiceError::InvalidState("workflow action has no supported runtime".into())
    })?;
    let installation = PluginRepository::new(db)
        .installation(&manifest.id)?
        .ok_or_else(|| PluginServiceError::NotFound(manifest.id.clone()))?;
    let artifact_path = installation.artifact_path.ok_or_else(|| {
        PluginServiceError::InvalidState("plugin component artifact is unavailable".into())
    })?;
    let bytes = std::fs::read(std::path::Path::new(&artifact_path).join(component))
        .map_err(|error| PluginServiceError::InvalidState(error.to_string()))?;
    let runtime_database = db.clone();
    let runtime_plugin_id = manifest.id.clone();
    let device_ref = device_ref.to_owned();
    let effects = Rc::new(RefCell::new(Vec::new()));
    let runtime_effects = Rc::clone(&effects);
    let output = execute_wasm(
        &bytes,
        &json!({
            "workflowId": request.workflow_id,
            "trigger": request.trigger,
            "input": request.input,
        }),
        WasmLimits::default(),
        move |method, payload| {
            super::plugins_host::wasm_host_call_for_device(
                &runtime_database,
                &runtime_plugin_id,
                method,
                payload,
                &mut runtime_effects.borrow_mut(),
                &device_ref,
            )
        },
    )
    .map_err(|error| PluginServiceError::InvalidState(error.to_string()))?;
    Ok((output, effects.borrow().clone()))
}

pub fn search(
    db: &DatabaseConnection,
    plugin_id: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<PluginSearchResult>, PluginServiceError> {
    let (manifest, grants) = active_manifest(db, plugin_id)?;
    CapabilityGateway::new(plugin_id, grants).require(capabilities::SEARCH_CONTRIBUTE)?;
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let limit = limit.min(MAX_SEARCH_RESULTS);
    let mut results = match plugin_id {
        "dev.aevio.project-planning" => CatalogRepository::new(db)
            .projects()?
            .into_iter()
            .filter(|project| project.name.to_ascii_lowercase().contains(&query))
            .map(|project| PluginSearchResult {
                plugin_id: plugin_id.into(),
                contribution_id: format!("{plugin_id}/project-search"),
                title: project.name,
                detail: project.description.unwrap_or_default(),
                target: format!("projects/{}", project.id),
            })
            .collect(),
        "dev.aevio.music" => MusicTrackRepository::new(db)
            .list()?
            .into_iter()
            .filter(|track| {
                track.title.to_ascii_lowercase().contains(&query)
                    || track.description.to_ascii_lowercase().contains(&query)
            })
            .map(|track| PluginSearchResult {
                plugin_id: plugin_id.into(),
                contribution_id: format!("{plugin_id}/track-search"),
                title: track.title,
                detail: track.description,
                target: format!("timers/music/{}", track.id),
            })
            .collect(),
        _ => Vec::new(),
    };
    results.truncate(limit);
    let declared = manifest
        .contributes
        .search
        .iter()
        .map(|item| item.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    results.retain(|result| declared.contains(result.contribution_id.as_str()));
    Ok(results)
}

pub fn search_all(
    db: &DatabaseConnection,
    query: &str,
    limit: usize,
) -> Result<PluginSearchAggregate, PluginServiceError> {
    let limit = limit.min(MAX_SEARCH_RESULTS);
    if query.trim().is_empty() || limit == 0 {
        return Ok(PluginSearchAggregate {
            results: Vec::new(),
            failures: Vec::new(),
        });
    }
    let installations = PluginRepository::new(db).list_installations()?;
    let mut aggregate = PluginSearchAggregate {
        results: Vec::new(),
        failures: Vec::new(),
    };
    for installation in installations.into_iter().filter(|installation| {
        parse_state(&installation.state).ok() == Some(PluginLifecycleState::Active)
    }) {
        let manifest = match serde_json::from_str::<aevio_contracts::plugin::PluginManifest>(
            &installation.manifest_json,
        ) {
            Ok(manifest) => manifest,
            Err(_) => {
                aggregate.failures.push(PluginSearchFailure {
                    plugin_id: installation.plugin_id,
                    reason: "invalid_manifest".into(),
                });
                continue;
            }
        };
        if manifest.contributes.search.is_empty() {
            continue;
        }
        let plugin_id = manifest.id;
        match search(db, &plugin_id, query, limit) {
            Ok(mut results) => aggregate.results.append(&mut results),
            Err(_) => aggregate.failures.push(PluginSearchFailure {
                plugin_id,
                reason: "provider_unavailable".into(),
            }),
        }
        if aggregate.results.len() >= limit {
            break;
        }
    }
    aggregate.results.truncate(limit);
    Ok(aggregate)
}

pub fn publish_event(
    db: &DatabaseConnection,
    plugin_id: &str,
    event: &str,
    payload: &serde_json::Value,
) -> Result<String, PluginServiceError> {
    Ok(super::plugins_events::publish(
        db,
        plugin_id,
        event,
        payload,
        aevio_core::timers::now_epoch_ms() as i64,
    )?
    .event_name)
}

pub fn authorize_subscription(
    db: &DatabaseConnection,
    plugin_id: &str,
    event: &str,
) -> Result<(), PluginServiceError> {
    super::plugins_events::authorize_subscription(db, plugin_id, event)
}

pub fn validate_event_name(event: &str) -> Result<(), PluginServiceError> {
    super::plugins_events::validate_event_type(event)
}

fn active_manifest(
    db: &DatabaseConnection,
    plugin_id: &str,
) -> Result<(aevio_contracts::plugin::PluginManifest, Vec<String>), PluginServiceError> {
    let row = PluginRepository::new(db)
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    if parse_state(&row.state)? != PluginLifecycleState::Active {
        return Err(PluginServiceError::InvalidState(
            "plugin must be active".into(),
        ));
    }
    let manifest = serde_json::from_str(&row.manifest_json)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    Ok((
        manifest,
        super::plugins::effective_permissions(db, &row, aevio_core::timers::now_epoch_ms() as i64)?,
    ))
}

fn host_target(plugin_id: &str, command_id: &str) -> Option<&'static str> {
    if !command_id.ends_with("/open") {
        return None;
    }
    match plugin_id {
        "dev.aevio.project-planning" => Some("projects"),
        "dev.aevio.music" => Some("timers"),
        "dev.aevio.notes" => Some("notes"),
        "dev.aevio.calendar" => Some("calendar"),
        "dev.aevio.crm" => Some("clients"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{execute_command, host_target, search_all, validate_event_name};
    use aevio_contracts::plugin::{
        PluginCommandRequest, PluginInstallationScope, PluginLifecycleState, capabilities,
    };
    use aevio_persistence::bootstrap::migrations::run_migrations;
    use aevio_persistence::database::connect;
    use rusqlite::Connection;
    use serde_json::json;
    use tempfile::tempdir;

    fn database() -> (tempfile::TempDir, sea_orm::DatabaseConnection) {
        let directory = tempdir().unwrap();
        let path = directory.path().join("activity.db");
        let mut connection = Connection::open(&path).unwrap();
        run_migrations(&mut connection).unwrap();
        drop(connection);
        (directory, connect(&path).unwrap())
    }

    #[test]
    fn first_party_commands_resolve_only_to_host_routes() {
        assert_eq!(
            host_target("dev.aevio.music", "dev.aevio.music/open"),
            Some("timers")
        );
        assert_eq!(host_target("dev.example", "dev.example/open"), None);
        assert_eq!(host_target("dev.aevio.music", "dev.aevio.music/run"), None);
    }

    #[test]
    fn event_names_are_bounded_and_lowercase() {
        assert!(validate_event_name("entry.created").is_ok());
        assert!(validate_event_name("Entry.Created").is_err());
        assert!(validate_event_name("bad event").is_err());
    }

    #[test]
    fn active_first_party_command_executes_through_the_host_route() {
        let (_directory, database) = database();
        let plugin_id = "dev.aevio.music";
        super::super::plugins::install_builtin(
            &database,
            plugin_id,
            PluginInstallationScope::Personal,
            1,
        )
        .unwrap();
        let permissions = super::super::plugins::catalog(&database)
            .unwrap()
            .into_iter()
            .find(|entry| entry.manifest.id == plugin_id)
            .unwrap()
            .manifest
            .permissions
            .into_iter()
            .map(|permission| permission.id)
            .collect();
        super::super::plugins::grant_permissions(&database, plugin_id, permissions, 2).unwrap();
        super::super::plugins::set_enabled(&database, plugin_id, true, 3).unwrap();
        let result = execute_command(
            &database,
            PluginCommandRequest {
                plugin_id: plugin_id.into(),
                command_id: format!("{plugin_id}/open"),
                input: json!({}),
            },
        )
        .unwrap();
        assert_eq!(result.kind, "open_host_route");
        assert_eq!(result.payload["target"], "timers");
        let revoked = super::super::plugins::revoke_permissions(
            &database,
            plugin_id,
            vec![capabilities::COMMANDS_REGISTER.into()],
            4,
        )
        .unwrap();
        assert_eq!(revoked.state, PluginLifecycleState::Disabled);
        assert!(
            execute_command(
                &database,
                PluginCommandRequest {
                    plugin_id: plugin_id.into(),
                    command_id: format!("{plugin_id}/open"),
                    input: json!({}),
                },
            )
            .is_err()
        );
    }

    #[test]
    fn aggregate_search_is_empty_and_safe_without_active_providers() {
        let (_directory, database) = database();
        let result = search_all(&database, "project", 20).unwrap();
        assert!(result.results.is_empty());
        assert!(result.failures.is_empty());
    }
}
