use std::path::{Path, PathBuf};

use aevio_contracts::plugin::{
    PluginInstallationScope, PluginInstallationSummary, PluginLifecycleState, PluginManifest,
    PluginRelease,
};
use aevio_persistence::database::entities::plugins::{
    installation_history, installations, runtime_health,
};
use aevio_persistence::database::repositories::plugins::PluginRepository;
use sea_orm::{ActiveValue::Set, DatabaseConnection};
use semver::Version;
use uuid::Uuid;

use super::plugins::{PluginServiceError, audit, parse_state, scope_name, state_name, to_summary};
use super::plugins_compatibility::{validate_compatibility, validate_dependencies};
use super::plugins_wasm::{WasmLimits, validate_module_file};

pub use super::plugins_health::health;

pub fn remove_staged_artifact(path: &str, root: &Path) -> Result<(), PluginServiceError> {
    let path = Path::new(path);
    let root = root
        .canonicalize()
        .map_err(|error| PluginServiceError::InvalidState(error.to_string()))?;
    let resolved = path
        .canonicalize()
        .map_err(|error| PluginServiceError::InvalidState(error.to_string()))?;
    if resolved == root || !resolved.starts_with(&root) {
        return Err(PluginServiceError::Permission(
            "plugin artifact path is outside the plugin storage root".into(),
        ));
    }
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(PluginServiceError::InvalidState(error.to_string())),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(PluginServiceError::InvalidState(
            "plugin artifact path is not a directory".into(),
        ));
    }
    std::fs::remove_dir_all(path)
        .map_err(|error| PluginServiceError::InvalidState(error.to_string()))
}

/// Returns the declared HTML entrypoint for an active external plugin.
pub fn ui_entrypoint(
    db: &DatabaseConnection,
    plugin_id: &str,
) -> Result<String, PluginServiceError> {
    let installation = PluginRepository::new(db)
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    if parse_state(&installation.state)? != PluginLifecycleState::Active {
        return Err(PluginServiceError::InvalidState(
            "plugin must be active".into(),
        ));
    }
    let manifest: PluginManifest = serde_json::from_str(&installation.manifest_json)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    manifest
        .entrypoints
        .ui
        .ok_or_else(|| PluginServiceError::NotFound("plugin UI entrypoint".into()))
}

/// Resolves a UI resource without allowing traversal, symlinks, or access to
/// files outside the UI bundle's entrypoint directory.
pub fn ui_asset_path(
    db: &DatabaseConnection,
    plugin_id: &str,
    relative_path: &str,
) -> Result<PathBuf, PluginServiceError> {
    if relative_path.is_empty()
        || relative_path.starts_with('/')
        || relative_path.contains(['\\', '%'])
        || relative_path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(PluginServiceError::Permission(
            "plugin UI path is invalid".into(),
        ));
    }
    let installation = PluginRepository::new(db)
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    if parse_state(&installation.state)? != PluginLifecycleState::Active {
        return Err(PluginServiceError::InvalidState(
            "plugin must be active".into(),
        ));
    }
    let manifest: PluginManifest = serde_json::from_str(&installation.manifest_json)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    let entrypoint = manifest
        .entrypoints
        .ui
        .ok_or_else(|| PluginServiceError::NotFound("plugin UI entrypoint".into()))?;
    let ui_root = Path::new(&entrypoint)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let requested = Path::new(relative_path);
    if !ui_root.as_os_str().is_empty() && !requested.starts_with(ui_root) {
        return Err(PluginServiceError::Permission(
            "plugin UI resource is outside the declared UI bundle".into(),
        ));
    }
    if relative_path == "manifest.json" || relative_path.ends_with(".wasm") {
        return Err(PluginServiceError::Permission(
            "plugin UI cannot expose executable or manifest files".into(),
        ));
    }
    let artifact_root = installation
        .artifact_path
        .as_deref()
        .ok_or_else(|| PluginServiceError::InvalidState("plugin artifact is unavailable".into()))?;
    let root = Path::new(artifact_root)
        .canonicalize()
        .map_err(|error| PluginServiceError::InvalidState(error.to_string()))?;
    let candidate = root.join(relative_path);
    let resolved = candidate
        .canonicalize()
        .map_err(|error| PluginServiceError::NotFound(error.to_string()))?;
    if !resolved.starts_with(&root) || !resolved.is_file() {
        return Err(PluginServiceError::NotFound(relative_path.into()));
    }
    Ok(resolved)
}

pub fn install_external(
    db: &DatabaseConnection,
    manifest: PluginManifest,
    release: PluginRelease,
    scope: PluginInstallationScope,
    artifact_path: &Path,
    now: i64,
) -> Result<PluginInstallationSummary, PluginServiceError> {
    manifest
        .validate()
        .map_err(|error| PluginServiceError::InvalidManifest(error.to_string()))?;
    validate_compatibility(&manifest)?;
    if release.revoked || release.version != manifest.version || !artifact_path.is_dir() {
        return Err(PluginServiceError::InvalidManifest(
            "release, manifest and staged artifact do not agree".into(),
        ));
    }
    super::plugins_update::allows_release(db, &manifest.id, &release)?;
    if let Some(ui) = &manifest.entrypoints.ui
        && !artifact_path.join(ui).is_file()
    {
        return Err(PluginServiceError::InvalidManifest(
            "declared HTML UI entrypoint is missing from the staged artifact".into(),
        ));
    }
    if let Some(component) = &manifest.entrypoints.component {
        let component_path = artifact_path.join(component);
        if !component_path.is_file() {
            return Err(PluginServiceError::InvalidManifest(
                "declared WebAssembly component is missing from the staged artifact".into(),
            ));
        }
        validate_module_file(&component_path, WasmLimits::default()).map_err(|error| {
            PluginServiceError::InvalidManifest(format!("invalid WebAssembly component: {error}"))
        })?;
    }
    let repository = PluginRepository::new(db);
    validate_dependencies(&repository, &manifest)?;
    let manifest_json = serde_json::to_string(&manifest)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    let artifact_path = artifact_path.display().to_string();
    if let Some(existing) = repository.installation(&manifest.id)? {
        let was_uninstalled = existing.state == state_name(PluginLifecycleState::Uninstalled);
        let current_version = Version::parse(&existing.version)
            .map_err(|error| PluginServiceError::InvalidState(error.to_string()))?;
        let incoming_version = Version::parse(&manifest.version)
            .map_err(|error| PluginServiceError::InvalidManifest(error.to_string()))?;
        if incoming_version < current_version
            || (incoming_version == current_version && !was_uninstalled)
        {
            return Err(PluginServiceError::InvalidState(
                "plugin updates cannot downgrade or reuse an installed version".into(),
            ));
        }
        if existing.state == state_name(PluginLifecycleState::Active) {
            return Err(PluginServiceError::InvalidState(
                "disable the plugin before updating it".into(),
            ));
        }
        let preserved_permissions = super::plugins::effective_permissions(db, &existing, now)?
            .into_iter()
            .filter(|permission| {
                manifest
                    .permissions
                    .iter()
                    .any(|requested| requested.id == *permission)
            })
            .collect::<Vec<_>>();
        let preserved_permissions_json = serde_json::to_string(&preserved_permissions)
            .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
        if !was_uninstalled {
            repository.insert_history(history_from_installation(&existing, now, "update"))?;
        }
        let mut model: installations::ActiveModel = existing.clone().into();
        model.publisher_id = Set(manifest.publisher.id);
        model.version = Set(manifest.version);
        model.scope = Set(scope_name(scope).into());
        model.state = Set(state_name(PluginLifecycleState::Installed).into());
        model.manifest_json = Set(manifest_json);
        model.granted_permissions_json = Set(preserved_permissions_json);
        model.artifact_digest = Set(release.artifact_digest);
        model.artifact_path = Set(Some(artifact_path));
        model.revision = Set(existing.revision + 1);
        model.updated_at = Set(now);
        model.last_error = Set(None);
        repository.update_installation(model)?;
        record_operation(
            &repository,
            &existing.id,
            if was_uninstalled {
                "reinstall"
            } else {
                "update"
            },
            existing.revision + 1,
            now,
        )?;
        return repository
            .installation(&manifest.id)?
            .map(to_summary)
            .ok_or_else(|| PluginServiceError::NotFound(manifest.id));
    }

    let installation_id = Uuid::now_v7().to_string();
    repository.insert_installation(installations::ActiveModel {
        id: Set(installation_id.clone()),
        plugin_id: Set(manifest.id.clone()),
        publisher_id: Set(manifest.publisher.id),
        version: Set(manifest.version),
        scope: Set(scope_name(scope).into()),
        state: Set(state_name(PluginLifecycleState::Installed).into()),
        manifest_json: Set(manifest_json),
        granted_permissions_json: Set("[]".into()),
        artifact_digest: Set(release.artifact_digest),
        artifact_path: Set(Some(artifact_path)),
        revision: Set(1),
        last_error: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        last_activated_at: Set(None),
    })?;
    record_operation(&repository, &installation_id, "install", 1, now)?;
    audit(
        &repository,
        &installation_id,
        "installed_external",
        now,
        "local-user",
    )?;
    repository
        .installation(&manifest.id)?
        .map(to_summary)
        .ok_or_else(|| PluginServiceError::NotFound(manifest.id))
}

pub fn rollback(
    db: &DatabaseConnection,
    plugin_id: &str,
    now: i64,
) -> Result<PluginInstallationSummary, PluginServiceError> {
    let repository = PluginRepository::new(db);
    let current = repository
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    if current.state == state_name(PluginLifecycleState::Active) {
        return Err(PluginServiceError::InvalidState(
            "disable the plugin before rollback".into(),
        ));
    }
    let previous = repository.latest_history(plugin_id)?.ok_or_else(|| {
        PluginServiceError::InvalidState("no rollback version is available".into())
    })?;
    if let Some(path) = &previous.artifact_path
        && !Path::new(path).is_dir()
    {
        return Err(PluginServiceError::InvalidState(
            "rollback artifact is missing; plugin needs recovery".into(),
        ));
    }
    let mut model: installations::ActiveModel = current.clone().into();
    model.publisher_id = Set(previous.publisher_id.clone());
    model.version = Set(previous.version.clone());
    model.scope = Set(previous.scope.clone());
    model.state = Set(
        if previous.state == state_name(PluginLifecycleState::Active) {
            state_name(PluginLifecycleState::Disabled).into()
        } else {
            previous.state.clone()
        },
    );
    model.manifest_json = Set(previous.manifest_json.clone());
    model.granted_permissions_json = Set("[]".into());
    model.artifact_digest = Set(previous.artifact_digest.clone());
    model.artifact_path = Set(previous.artifact_path.clone());
    model.revision = Set(current.revision + 1);
    model.updated_at = Set(now);
    model.last_error = Set(None);
    repository.update_installation(model)?;
    for grant in repository.list_grants(&current.id)? {
        repository.revoke_grant(&current.id, &grant.capability, now)?;
    }
    repository.delete_history(&previous.id)?;
    record_operation(
        &repository,
        &current.id,
        "rollback",
        current.revision + 1,
        now,
    )?;
    audit(&repository, &current.id, "rolled_back", now, "local-user")?;
    repository
        .installation(plugin_id)?
        .map(to_summary)
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))
}

pub fn record_runtime_failure(
    db: &DatabaseConnection,
    plugin_id: &str,
    error: &str,
    now: i64,
) -> Result<PluginInstallationSummary, PluginServiceError> {
    let repository = PluginRepository::new(db);
    let installation = repository
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    let previous = repository.runtime_health(&installation.id)?;
    let crashes = previous.as_ref().map_or(1, |health| health.crash_count + 1);
    let quarantined = crashes >= 3;
    repository.upsert_runtime_health(
        &installation.id,
        runtime_health::ActiveModel {
            installation_id: Set(installation.id.clone()),
            state: Set(if quarantined { "quarantined" } else { "failed" }.into()),
            last_success_at: Set(previous.and_then(|health| health.last_success_at)),
            last_failure_at: Set(Some(now)),
            crash_count: Set(crashes),
            permission_denials: Set(0),
            resource_violations: Set(0),
            last_error: Set(Some(error.into())),
        },
    )?;
    if quarantined {
        let mut model: installations::ActiveModel = installation.clone().into();
        model.state = Set(state_name(PluginLifecycleState::Quarantined).into());
        model.last_error = Set(Some(error.into()));
        model.revision = Set(installation.revision + 1);
        model.updated_at = Set(now);
        repository.update_installation(model)?;
        audit(&repository, &installation.id, "quarantined", now, "runtime")?;
        let can_roll_back = repository
            .latest_history(plugin_id)?
            .and_then(|history| history.artifact_path)
            .is_some_and(|path| Path::new(&path).is_dir());
        if can_roll_back && let Err(rollback_error) = rollback(db, plugin_id, now) {
            audit(
                &repository,
                &installation.id,
                "automatic_rollback_failed",
                now,
                &rollback_error.to_string(),
            )?;
        }
    }
    repository
        .installation(plugin_id)?
        .map(to_summary)
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))
}

pub fn recover(
    db: &DatabaseConnection,
    plugin_id: &str,
    now: i64,
) -> Result<PluginInstallationSummary, PluginServiceError> {
    let repository = PluginRepository::new(db);
    let installation = repository
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    let state = parse_state(&installation.state)?;
    if !matches!(
        state,
        PluginLifecycleState::Failed
            | PluginLifecycleState::Quarantined
            | PluginLifecycleState::RecoveryRequired
    ) {
        return Err(PluginServiceError::InvalidState(
            "plugin does not require recovery".into(),
        ));
    }
    if installation
        .artifact_path
        .as_deref()
        .is_some_and(|path| !Path::new(path).is_dir())
    {
        return Err(PluginServiceError::InvalidState(
            "staged artifact is missing".into(),
        ));
    }
    let mut model: installations::ActiveModel = installation.clone().into();
    model.state = Set(state_name(PluginLifecycleState::Installed).into());
    model.last_error = Set(None);
    model.revision = Set(installation.revision + 1);
    model.updated_at = Set(now);
    repository.update_installation(model)?;
    audit(
        &repository,
        &installation.id,
        "recovered",
        now,
        "local-user",
    )?;
    repository
        .installation(plugin_id)?
        .map(to_summary)
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))
}

fn history_from_installation(
    installation: &installations::Model,
    now: i64,
    reason: &str,
) -> installation_history::ActiveModel {
    installation_history::ActiveModel {
        id: Set(Uuid::now_v7().to_string()),
        plugin_id: Set(installation.plugin_id.clone()),
        publisher_id: Set(installation.publisher_id.clone()),
        version: Set(installation.version.clone()),
        scope: Set(installation.scope.clone()),
        state: Set(installation.state.clone()),
        manifest_json: Set(installation.manifest_json.clone()),
        artifact_digest: Set(installation.artifact_digest.clone()),
        artifact_path: Set(installation.artifact_path.clone()),
        recorded_at: Set(now),
        reason: Set(reason.into()),
    }
}

fn record_operation(
    repository: &PluginRepository<'_>,
    installation_id: &str,
    kind: &str,
    revision: i64,
    now: i64,
) -> Result<(), PluginServiceError> {
    use aevio_persistence::database::entities::plugins::operations;
    repository.insert_operation(operations::ActiveModel {
        id: Set(Uuid::now_v7().to_string()),
        installation_id: Set(installation_id.into()),
        kind: Set(kind.into()),
        state: Set("completed".into()),
        revision: Set(revision),
        error: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aevio_contracts::plugin::{
        MANIFEST_VERSION, PluginCompatibility, PluginEntrypoints, PluginPlatform, PublisherIdentity,
    };
    use aevio_persistence::bootstrap::migrations::run_migrations;
    use rusqlite::Connection;
    use tempfile::tempdir;

    fn manifest(version: &str) -> PluginManifest {
        PluginManifest {
            manifest_version: MANIFEST_VERSION,
            id: "dev.aevio.rollback-test".into(),
            name: "Rollback Test".into(),
            version: version.into(),
            publisher: PublisherIdentity {
                id: "test".into(),
                display_name: "Test".into(),
                verified: true,
                public_key: None,
            },
            description: "test".into(),
            category: "test".into(),
            compatibility: PluginCompatibility {
                aevio: ">=0.9.0, <1.0.0".into(),
                plugin_api: "^1.0.0".into(),
            },
            platforms: vec![PluginPlatform::Desktop],
            entrypoints: PluginEntrypoints::default(),
            icons: Vec::new(),
            screenshots: Vec::new(),
            permissions: Vec::new(),
            dependencies: Vec::new(),
            optional_dependencies: Vec::new(),
            contributes: Default::default(),
            event_subscriptions: Vec::new(),
            offline: Default::default(),
            data_schema: None,
        }
    }

    fn release(version: &str) -> PluginRelease {
        PluginRelease {
            version: version.into(),
            artifact_digest: format!("digest-{version}"),
            artifact_size: 1,
            signature: None,
            channel: "stable".into(),
            published_at: 0,
            revoked: false,
        }
    }

    fn database() -> (tempfile::TempDir, DatabaseConnection) {
        let directory = tempdir().unwrap();
        let path = directory.path().join("activity.db");
        let mut connection = Connection::open(&path).unwrap();
        run_migrations(&mut connection).unwrap();
        drop(connection);
        (
            directory,
            aevio_persistence::database::connect(&path).unwrap(),
        )
    }

    #[test]
    fn external_update_rolls_back_to_the_previous_staged_version() {
        let (directory, database) = database();
        let first_path = directory.path().join("v1");
        let second_path = directory.path().join("v2");
        std::fs::create_dir_all(&first_path).unwrap();
        std::fs::create_dir_all(&second_path).unwrap();
        install_external(
            &database,
            manifest("1.0.0"),
            release("1.0.0"),
            PluginInstallationScope::Personal,
            &first_path,
            1,
        )
        .unwrap();
        let updated = install_external(
            &database,
            manifest("2.0.0"),
            release("2.0.0"),
            PluginInstallationScope::Personal,
            &second_path,
            2,
        )
        .unwrap();
        assert_eq!(updated.version, "2.0.0");
        let rolled_back = rollback(&database, &updated.plugin_id, 3).unwrap();
        assert_eq!(rolled_back.version, "1.0.0");
    }

    #[test]
    fn three_runtime_failures_quarantine_and_recovery_reinstates_installed_state() {
        let (directory, database) = database();
        let artifact_path = directory.path().join("v1");
        std::fs::create_dir_all(&artifact_path).unwrap();
        let installed = install_external(
            &database,
            manifest("1.0.0"),
            release("1.0.0"),
            PluginInstallationScope::Personal,
            &artifact_path,
            1,
        )
        .unwrap();
        for now in 2..=4 {
            record_runtime_failure(&database, &installed.plugin_id, "crashed", now).unwrap();
        }
        let recovered = recover(&database, &installed.plugin_id, 5).unwrap();
        assert_eq!(recovered.state, PluginLifecycleState::Installed);
    }

    #[test]
    fn external_wasm_entrypoints_require_the_bounded_host_abi() {
        let (directory, database) = database();
        let artifact_path = directory.path().join("component");
        std::fs::create_dir_all(&artifact_path).unwrap();
        let mut plugin = manifest("1.0.0");
        plugin.entrypoints.component = Some("runtime.wasm".into());
        std::fs::write(
            artifact_path.join("runtime.wasm"),
            wat::parse_str(
                r#"(module
                    (memory (export "memory") 1)
                    (global $heap (mut i32) (i32.const 1024))
                    (func (export "aevio_alloc") (param i32) (result i32)
                        global.get $heap)
                    (func (export "aevio_handle")
                        (param i32 i32 i32 i32) (result i32)
                        i32.const 0))"#,
            )
            .unwrap(),
        )
        .unwrap();
        let result = install_external(
            &database,
            plugin,
            release("1.0.0"),
            PluginInstallationScope::Personal,
            &artifact_path,
            1,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn external_ui_resources_are_confined_to_the_declared_bundle() {
        let (directory, database) = database();
        let artifact_path = directory.path().join("ui-component");
        std::fs::create_dir_all(artifact_path.join("ui")).unwrap();
        std::fs::write(
            artifact_path.join("ui/index.html"),
            "<script src=app.js></script>",
        )
        .unwrap();
        std::fs::write(artifact_path.join("ui/app.js"), "console.log('ok')").unwrap();
        std::fs::write(artifact_path.join("secret.txt"), "private").unwrap();
        let mut plugin = manifest("1.0.0");
        plugin.entrypoints.ui = Some("ui/index.html".into());
        let installed = install_external(
            &database,
            plugin,
            release("1.0.0"),
            PluginInstallationScope::Personal,
            &artifact_path,
            1,
        )
        .unwrap();
        super::super::plugins::set_enabled(&database, &installed.plugin_id, true, 2).unwrap();
        assert!(ui_asset_path(&database, &installed.plugin_id, "ui/app.js").is_ok());
        assert!(ui_asset_path(&database, &installed.plugin_id, "secret.txt").is_err());
        assert!(ui_asset_path(&database, &installed.plugin_id, "ui/../secret.txt").is_err());
        assert!(ui_asset_path(&database, &installed.plugin_id, "manifest.json").is_err());
    }
}
