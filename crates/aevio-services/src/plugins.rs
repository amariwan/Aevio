//! Plugin catalog and local installation use cases.
//!
//! The service owns policy and lifecycle orchestration. It never exposes the
//! database connection to a plugin and does not treat a permission request as
//! a grant.

use std::path::Path;

use aevio_application::identity::Role;
use aevio_contracts::plugin::{
    PluginCatalogEntry, PluginContributionSnapshot, PluginContributions, PluginInstallPlan,
    PluginInstallationScope, PluginInstallationSummary, PluginLifecycleState, PluginManifest,
    PluginPlatform, PluginRelease,
};
use aevio_core::plugins::{PluginLifecycleEvent, transition};
use aevio_persistence::database::entities::plugins::installations;
use aevio_persistence::database::repositories::plugins::PluginRepository;
use sea_orm::{ActiveValue::Set, DatabaseConnection, DbErr};
use uuid::Uuid;

#[path = "plugins_builtin.rs"]
mod plugins_builtin;
use plugins_builtin::{builtin_catalog, builtin_manifest};

#[derive(Debug)]
pub enum PluginServiceError {
    Database(DbErr),
    InvalidManifest(String),
    NotFound(String),
    InvalidState(String),
    Permission(String),
    Serialization(String),
}

impl std::fmt::Display for PluginServiceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(error) => write!(formatter, "plugin database error: {error}"),
            Self::InvalidManifest(error) => write!(formatter, "invalid plugin manifest: {error}"),
            Self::NotFound(plugin) => write!(formatter, "plugin not found: {plugin}"),
            Self::InvalidState(error) => write!(formatter, "invalid plugin state: {error}"),
            Self::Permission(error) => write!(formatter, "plugin permission error: {error}"),
            Self::Serialization(error) => write!(formatter, "plugin serialization error: {error}"),
        }
    }
}

impl std::error::Error for PluginServiceError {}

impl From<DbErr> for PluginServiceError {
    fn from(error: DbErr) -> Self {
        Self::Database(error)
    }
}

/// Trusted host context for one plugin authorization decision. Plugin input
/// never supplies these values; local adapters derive them from the
/// installation and host-owned device state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginExecutionContext {
    pub plugin_id: String,
    pub installation_id: String,
    pub release_version: String,
    pub runtime_instance: String,
    pub actor_ref: String,
    pub workspace_ref: String,
    pub device_ref: String,
    pub role: Role,
    pub platform: PluginPlatform,
}

impl PluginExecutionContext {
    pub fn local(
        installation: &installations::Model,
        device_ref: impl Into<String>,
    ) -> Result<Self, PluginServiceError> {
        let workspace_ref = match installation.scope.as_str() {
            "personal" => "personal",
            "workspace" => "default",
            _ => {
                return Err(PluginServiceError::InvalidState(
                    "plugin installation scope is invalid".into(),
                ));
            }
        };
        Ok(Self {
            plugin_id: installation.plugin_id.clone(),
            installation_id: installation.id.clone(),
            release_version: installation.version.clone(),
            runtime_instance: Uuid::now_v7().to_string(),
            actor_ref: "local-user".into(),
            workspace_ref: workspace_ref.into(),
            device_ref: device_ref.into(),
            role: Role::Owner,
            platform: PluginPlatform::Desktop,
        })
    }
}

/// Capability checks are repeated at the privileged host boundary. A grant in
/// the database is data, not authority for arbitrary plugin code.
#[derive(Debug, Clone)]
pub struct CapabilityGateway {
    plugin_id: String,
    grants: std::collections::BTreeSet<String>,
}

impl CapabilityGateway {
    pub fn new(plugin_id: impl Into<String>, grants: impl IntoIterator<Item = String>) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            grants: grants.into_iter().collect(),
        }
    }

    pub fn require(&self, capability: &str) -> Result<(), PluginServiceError> {
        if self.grants.contains(capability) {
            Ok(())
        } else {
            Err(PluginServiceError::Permission(format!(
                "plugin {} is not granted {capability}",
                self.plugin_id
            )))
        }
    }
}

pub fn catalog(db: &DatabaseConnection) -> Result<Vec<PluginCatalogEntry>, PluginServiceError> {
    let installed = PluginRepository::new(db).list_installations()?;
    let now = aevio_core::timers::now_epoch_ms() as i64;
    let mut entries = builtin_catalog();
    for entry in &mut entries {
        entry.installed = installed
            .iter()
            .find(|row| row.plugin_id == entry.manifest.id)
            .map(|row| to_summary_with_effective_permissions(db, row.clone(), now))
            .transpose()?;
    }
    for row in installed {
        if entries
            .iter()
            .any(|entry| entry.manifest.id == row.plugin_id)
        {
            continue;
        }
        let manifest: PluginManifest = serde_json::from_str(&row.manifest_json)
            .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
        entries.push(PluginCatalogEntry {
            releases: vec![PluginRelease {
                version: row.version.clone(),
                artifact_digest: row.artifact_digest.clone(),
                artifact_size: row
                    .artifact_path
                    .as_deref()
                    .and_then(|path| std::fs::metadata(path).ok())
                    .map(|metadata| metadata.len())
                    .unwrap_or_default(),
                signature: None,
                channel: "installed".into(),
                published_at: row.created_at,
                revoked: false,
            }],
            manifest,
            installed: Some(to_summary_with_effective_permissions(db, row, now)?),
        });
    }
    Ok(entries)
}

pub fn installed(
    db: &DatabaseConnection,
) -> Result<Vec<PluginInstallationSummary>, PluginServiceError> {
    let now = aevio_core::timers::now_epoch_ms() as i64;
    PluginRepository::new(db)
        .list_installations()?
        .into_iter()
        .map(|row| to_summary_with_effective_permissions(db, row, now))
        .collect()
}

/// Authorizes a network request against an origin explicitly declared by the
/// plugin permission. The request URL is supplied by the privileged adapter;
/// plugin input never grants itself an origin.
pub fn authorize_network_origin(
    db: &DatabaseConnection,
    plugin_id: &str,
    request_url: &str,
) -> Result<(), PluginServiceError> {
    let row = PluginRepository::new(db)
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    if parse_state(&row.state)? != PluginLifecycleState::Active {
        return Err(PluginServiceError::InvalidState(
            "plugin must be active".into(),
        ));
    }
    let granted = effective_permissions(db, &row, aevio_core::timers::now_epoch_ms() as i64)?;
    CapabilityGateway::new(plugin_id, granted.clone())
        .require(aevio_contracts::plugin::capabilities::NETWORK_FETCH)?;
    let target = url::Url::parse(request_url)
        .map_err(|_| PluginServiceError::Permission("network URL is invalid".into()))?;
    if target.username() != "" || target.password().is_some() {
        return Err(PluginServiceError::Permission(
            "network URLs cannot contain credentials".into(),
        ));
    }
    let manifest: PluginManifest = serde_json::from_str(&row.manifest_json)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    let allowed = manifest.permissions.iter().filter(|permission| {
        permission.id == aevio_contracts::plugin::capabilities::NETWORK_FETCH
            && permission
                .scope
                .as_deref()
                .is_some_and(|scope| origin_matches(scope, &target))
    });
    if allowed.count() == 0 {
        return Err(PluginServiceError::Permission(
            "network origin was not approved by the plugin manifest".into(),
        ));
    }
    Ok(())
}

fn origin_matches(scope: &str, target: &url::Url) -> bool {
    let Ok(allowed) = url::Url::parse(scope) else {
        return false;
    };
    allowed.scheme() == target.scheme()
        && allowed.host_str() == target.host_str()
        && allowed.port_or_known_default() == target.port_or_known_default()
        && allowed.username().is_empty()
        && allowed.password().is_none()
}

/// Compose only active, locally installed contributions. Capability-backed
/// contribution kinds are filtered at this privileged boundary so manifest
/// metadata alone can never register a command, search provider, widget, or
/// workflow.
pub fn active_contributions(
    db: &DatabaseConnection,
) -> Result<Vec<PluginContributionSnapshot>, PluginServiceError> {
    let repository = PluginRepository::new(db);
    let preferences = super::plugins_layout::list(db)?;
    repository
        .list_installations()?
        .into_iter()
        .filter(|row| row.state == state_name(PluginLifecycleState::Active))
        .map(|row| {
            let manifest: PluginManifest = serde_json::from_str(&row.manifest_json)
                .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
            let granted =
                effective_permissions(db, &row, aevio_core::timers::now_epoch_ms() as i64)?;
            let mut contributions = filter_contributions(manifest.contributes, &granted);
            super::plugins_layout::apply(
                &mut contributions,
                &preferences
                    .iter()
                    .filter(|preference| preference.plugin_id == manifest.id)
                    .cloned()
                    .collect::<Vec<_>>(),
            );
            Ok(PluginContributionSnapshot {
                plugin_id: manifest.id.clone(),
                plugin_name: manifest.name.clone(),
                version: manifest.version.clone(),
                contributions,
            })
        })
        .collect()
}

fn filter_contributions(
    mut contributions: PluginContributions,
    granted: &[String],
) -> PluginContributions {
    let has = |capability: &str| granted.iter().any(|item| item == capability);
    if !has(aevio_contracts::plugin::capabilities::COMMANDS_REGISTER) {
        contributions.commands.clear();
    } else {
        contributions.commands.retain(|command| {
            command
                .required_capabilities
                .iter()
                .all(|capability| has(capability))
        });
    }
    if !has(aevio_contracts::plugin::capabilities::SEARCH_CONTRIBUTE) {
        contributions.search.clear();
    }
    if !has(aevio_contracts::plugin::capabilities::WIDGETS_REGISTER) {
        contributions.widgets.clear();
    }
    contributions.workflow_actions.retain(|workflow| {
        workflow
            .required_capabilities
            .iter()
            .all(|capability| has(capability))
    });
    contributions.workflow_triggers.retain(|workflow| {
        workflow
            .required_capabilities
            .iter()
            .all(|capability| has(capability))
    });
    if !has(aevio_contracts::plugin::capabilities::JOBS_SCHEDULE) {
        contributions.jobs.clear();
    }
    contributions
}

/// Returns the registry entries that are safe to expose before local state is loaded.
pub fn public_catalog() -> Vec<PluginCatalogEntry> {
    builtin_catalog()
}

pub fn plan_install(
    db: &DatabaseConnection,
    plugin_id: &str,
    scope: PluginInstallationScope,
) -> Result<PluginInstallPlan, PluginServiceError> {
    let entry = builtin_catalog()
        .into_iter()
        .find(|entry| entry.manifest.id == plugin_id)
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    entry
        .manifest
        .validate()
        .map_err(|error| PluginServiceError::InvalidManifest(error.to_string()))?;
    let release = entry
        .releases
        .into_iter()
        .find(|release| !release.revoked)
        .ok_or_else(|| PluginServiceError::NotFound(format!("release for {plugin_id}")))?;
    let existing = PluginRepository::new(db).installation(plugin_id)?;
    let granted = existing
        .as_ref()
        .map(|row| effective_permissions(db, row, aevio_core::timers::now_epoch_ms() as i64))
        .transpose()?
        .unwrap_or_default();
    let new_permissions = entry
        .manifest
        .permissions
        .iter()
        .map(|permission| permission.id.clone())
        .filter(|permission| !granted.contains(permission))
        .collect();
    Ok(PluginInstallPlan {
        plan_id: Uuid::now_v7().to_string(),
        plugin_id: plugin_id.into(),
        release,
        scope,
        new_permissions,
        warnings: vec!["This first-party module uses host-owned declarative contributions; it does not load arbitrary native code.".into()],
        base_revision: existing.map(|row| row.revision.max(0) as u64).unwrap_or(0),
    })
}

pub fn install_builtin(
    db: &DatabaseConnection,
    plugin_id: &str,
    scope: PluginInstallationScope,
    now: i64,
) -> Result<PluginInstallationSummary, PluginServiceError> {
    let plan = plan_install(db, plugin_id, scope)?;
    install_builtin_plan(db, plan, now)
}

/// Executes a previously returned plan only when its release and base revision
/// still match current state. The plan is revalidated instead of trusting its
/// serialized contents from an untrusted UI client.
pub fn install_builtin_from_plan(
    db: &DatabaseConnection,
    plan: PluginInstallPlan,
    now: i64,
) -> Result<PluginInstallationSummary, PluginServiceError> {
    let current_revision = PluginRepository::new(db)
        .installation(&plan.plugin_id)?
        .map(|row| row.revision.max(0) as u64)
        .unwrap_or(0);
    if current_revision != plan.base_revision {
        return Err(PluginServiceError::InvalidState(
            "plugin installation plan is stale".into(),
        ));
    }
    let fresh = plan_install(db, &plan.plugin_id, plan.scope)?;
    if fresh.release != plan.release || fresh.scope != plan.scope {
        return Err(PluginServiceError::InvalidState(
            "plugin installation plan no longer matches the catalog".into(),
        ));
    }
    install_builtin_plan(db, fresh, now)
}

fn install_builtin_plan(
    db: &DatabaseConnection,
    plan: PluginInstallPlan,
    now: i64,
) -> Result<PluginInstallationSummary, PluginServiceError> {
    let repository = PluginRepository::new(db);
    if let Some(existing) = repository.installation(&plan.plugin_id)? {
        if existing.state == state_name(PluginLifecycleState::Active) {
            return Ok(to_summary(existing));
        }
        let mut model: installations::ActiveModel = existing.clone().into();
        model.scope = Set(scope_name(plan.scope).into());
        model.state = Set(state_name(PluginLifecycleState::Installed).into());
        model.revision = Set(existing.revision + 1);
        model.updated_at = Set(now);
        model.last_error = Set(None);
        repository.update_installation(model)?;
        record_operation(
            &repository,
            &existing.id,
            "install",
            existing.revision + 1,
            now,
        )?;
        return repository
            .installation(&plan.plugin_id)?
            .map(to_summary)
            .ok_or_else(|| PluginServiceError::NotFound(plan.plugin_id));
    }

    let installation_id = Uuid::now_v7().to_string();
    let manifest = builtin_manifest(&plan.plugin_id)
        .ok_or_else(|| PluginServiceError::NotFound(plan.plugin_id.clone()))?;
    let manifest_json = serde_json::to_string(&manifest)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    repository.insert_installation(installations::ActiveModel {
        id: Set(installation_id.clone()),
        plugin_id: Set(plan.plugin_id.clone()),
        publisher_id: Set(manifest.publisher.id),
        version: Set(plan.release.version),
        scope: Set(scope_name(plan.scope).into()),
        state: Set(state_name(PluginLifecycleState::Installed).into()),
        manifest_json: Set(manifest_json),
        granted_permissions_json: Set("[]".into()),
        artifact_digest: Set(plan.release.artifact_digest),
        artifact_path: Set(None),
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
        "installed",
        now,
        "local-user",
    )?;
    repository
        .installation(&plan.plugin_id)?
        .map(to_summary)
        .ok_or_else(|| PluginServiceError::NotFound(plan.plugin_id))
}

pub fn grant_permissions(
    db: &DatabaseConnection,
    plugin_id: &str,
    requested: Vec<String>,
    now: i64,
) -> Result<PluginInstallationSummary, PluginServiceError> {
    let repository = PluginRepository::new(db);
    let row = repository
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    let manifest: PluginManifest = serde_json::from_str(&row.manifest_json)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    let allowed: std::collections::HashSet<&str> = manifest
        .permissions
        .iter()
        .map(|permission| permission.id.as_str())
        .collect();
    if let Some(permission) = requested
        .iter()
        .find(|permission| !allowed.contains(permission.as_str()))
    {
        return Err(PluginServiceError::Permission(format!(
            "{permission} was not requested by the manifest"
        )));
    }
    let mut granted = effective_permissions(db, &row, now)?
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let previous_count = granted.len();
    granted.extend(requested.iter().cloned());
    let granted = granted.into_iter().collect::<Vec<_>>();
    let encoded = serde_json::to_string(&granted)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    let mut model: installations::ActiveModel = row.clone().into();
    model.granted_permissions_json = Set(encoded);
    model.revision = Set(row.revision + 1);
    model.updated_at = Set(now);
    if granted.len() != previous_count {
        repository.update_installation(model)?;
    }
    for permission in &granted {
        repository.upsert_persistent_grant(&row.id, permission, now)?;
    }
    audit(
        &repository,
        &row.id,
        "permissions_granted",
        now,
        "local-user",
    )?;
    repository
        .installation(plugin_id)?
        .map(to_summary)
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))
}

pub fn revoke_permissions(
    db: &DatabaseConnection,
    plugin_id: &str,
    revoked: Vec<String>,
    now: i64,
) -> Result<PluginInstallationSummary, PluginServiceError> {
    let repository = PluginRepository::new(db);
    let row = repository
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    let manifest: PluginManifest = serde_json::from_str(&row.manifest_json)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    let requested: std::collections::HashSet<&str> = manifest
        .permissions
        .iter()
        .map(|permission| permission.id.as_str())
        .collect();
    if let Some(permission) = revoked
        .iter()
        .find(|permission| !requested.contains(permission.as_str()))
    {
        return Err(PluginServiceError::Permission(format!(
            "{permission} was not requested by the manifest"
        )));
    }
    let revoked = revoked
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    let granted = effective_permissions(db, &row, now)?
        .into_iter()
        .filter(|permission| !revoked.contains(permission))
        .collect::<Vec<_>>();
    let active = parse_state(&row.state)? == PluginLifecycleState::Active;
    let missing_required = manifest
        .permissions
        .iter()
        .any(|permission| !granted.iter().any(|item| item == &permission.id));
    let next_state = if active && missing_required {
        Some(state_name(PluginLifecycleState::Disabled))
    } else {
        None
    };
    let mut model: installations::ActiveModel = row.clone().into();
    model.granted_permissions_json = Set(serde_json::to_string(&granted)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?);
    if let Some(state) = next_state {
        model.state = Set(state.into());
    }
    model.revision = Set(row.revision + 1);
    model.updated_at = Set(now);
    repository.update_installation(model)?;
    for permission in &revoked {
        repository.revoke_grant(&row.id, permission, now)?;
    }
    record_operation(
        &repository,
        &row.id,
        "permissions_revoked",
        row.revision + 1,
        now,
    )?;
    audit(
        &repository,
        &row.id,
        if next_state.is_some() {
            "permissions_revoked_and_disabled"
        } else {
            "permissions_revoked"
        },
        now,
        "local-user",
    )?;
    repository
        .installation(plugin_id)?
        .map(to_summary)
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))
}

pub fn set_enabled(
    db: &DatabaseConnection,
    plugin_id: &str,
    enabled: bool,
    now: i64,
) -> Result<PluginInstallationSummary, PluginServiceError> {
    let repository = PluginRepository::new(db);
    let row = repository
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    let manifest: PluginManifest = serde_json::from_str(&row.manifest_json)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    let current = parse_state(&row.state)?;
    if enabled && current == PluginLifecycleState::Active {
        return Ok(to_summary(row));
    }
    if !enabled && current == PluginLifecycleState::Disabled {
        return Ok(to_summary(row));
    }
    let next = if enabled {
        let granted = effective_permissions(db, &row, aevio_core::timers::now_epoch_ms() as i64)?;
        let gateway = CapabilityGateway::new(plugin_id, granted);
        for permission in &manifest.permissions {
            gateway.require(&permission.id)?;
        }
        transition(current, PluginLifecycleEvent::Activate)
            .and_then(|state| transition(state, PluginLifecycleEvent::Enable))
            .map_err(|error| PluginServiceError::InvalidState(error.to_string()))?
    } else {
        transition(current, PluginLifecycleEvent::Deactivate)
            .and_then(|state| transition(state, PluginLifecycleEvent::Disable))
            .map_err(|error| PluginServiceError::InvalidState(error.to_string()))?
    };
    let mut model: installations::ActiveModel = row.clone().into();
    model.state = Set(state_name(next).into());
    model.revision = Set(row.revision + 1);
    model.updated_at = Set(now);
    model.last_activated_at = Set(if enabled {
        Some(now)
    } else {
        row.last_activated_at
    });
    repository.update_installation(model)?;
    record_operation(
        &repository,
        &row.id,
        if enabled { "enable" } else { "disable" },
        row.revision + 1,
        now,
    )?;
    audit(
        &repository,
        &row.id,
        if enabled { "enabled" } else { "disabled" },
        now,
        "local-user",
    )?;
    repository
        .installation(plugin_id)?
        .map(to_summary)
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))
}

pub fn uninstall(
    db: &DatabaseConnection,
    plugin_id: &str,
    now: i64,
    artifact_root: Option<&Path>,
) -> Result<(), PluginServiceError> {
    let repository = PluginRepository::new(db);
    let row = repository
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    if parse_state(&row.state)? == PluginLifecycleState::Active {
        return Err(PluginServiceError::InvalidState(
            "disable the plugin before uninstalling".into(),
        ));
    }
    if parse_state(&row.state)? == PluginLifecycleState::Uninstalled {
        return Ok(());
    }
    let next = transition(row_state(&row)?, PluginLifecycleEvent::StartUninstall)
        .and_then(|state| transition(state, PluginLifecycleEvent::CompleteUninstall))
        .map_err(|error| PluginServiceError::InvalidState(error.to_string()))?;
    let mut artifact_paths = repository
        .history_for_plugin(plugin_id)?
        .into_iter()
        .filter_map(|history| history.artifact_path)
        .collect::<Vec<_>>();
    if let Some(path) = row.artifact_path.as_deref() {
        artifact_paths.push(path.to_owned());
    }
    if !artifact_paths.is_empty() {
        let root = artifact_root.ok_or_else(|| {
            PluginServiceError::InvalidState("plugin storage root is unavailable".into())
        })?;
        for path in artifact_paths {
            super::plugins_artifacts::remove_staged_artifact(&path, root)?;
        }
    }
    for grant in repository.list_grants(&row.id)? {
        repository.revoke_grant(&row.id, &grant.capability, now)?;
    }
    let mut model: installations::ActiveModel = row.clone().into();
    model.state = Set(state_name(next).into());
    model.granted_permissions_json = Set("[]".into());
    model.artifact_path = Set(None);
    model.last_error = Set(None);
    model.revision = Set(row.revision + 1);
    model.updated_at = Set(now);
    repository.update_installation(model)?;
    repository.delete_history_for_plugin(plugin_id)?;
    record_operation(&repository, &row.id, "uninstall", row.revision + 1, now)?;
    audit(&repository, &row.id, "uninstalled", now, "local-user")?;
    Ok(())
}

pub fn delete_data(
    db: &DatabaseConnection,
    plugin_id: &str,
    now: i64,
) -> Result<(), PluginServiceError> {
    let repository = PluginRepository::new(db);
    let row = repository
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    if parse_state(&row.state)? == PluginLifecycleState::Active {
        return Err(PluginServiceError::InvalidState(
            "disable the plugin before deleting its data".into(),
        ));
    }
    repository.delete_plugin_data(&row.id, plugin_id)?;
    audit(&repository, &row.id, "data_deleted", now, "local-user")?;
    Ok(())
}

pub(super) fn to_summary(row: installations::Model) -> PluginInstallationSummary {
    PluginInstallationSummary {
        installation_id: row.id,
        plugin_id: row.plugin_id,
        version: row.version,
        scope: parse_scope(&row.scope),
        state: parse_state_lossy(&row.state),
        granted_permissions: parse_permissions_lossy(&row.granted_permissions_json),
        updated_at: row.updated_at,
        last_error: row.last_error,
    }
}

fn to_summary_with_effective_permissions(
    db: &DatabaseConnection,
    row: installations::Model,
    now: i64,
) -> Result<PluginInstallationSummary, PluginServiceError> {
    let mut summary = to_summary(row.clone());
    summary.granted_permissions = effective_permissions(db, &row, now)?;
    Ok(summary)
}

fn record_operation(
    repository: &PluginRepository<'_>,
    installation_id: &str,
    kind: &str,
    revision: i64,
    now: i64,
) -> Result<(), PluginServiceError> {
    use aevio_persistence::database::entities::plugins::operations;
    repository
        .insert_operation(operations::ActiveModel {
            id: Set(Uuid::now_v7().to_string()),
            installation_id: Set(installation_id.into()),
            kind: Set(kind.into()),
            state: Set("completed".into()),
            revision: Set(revision),
            error: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        })
        .map_err(Into::into)
}

pub(crate) fn audit(
    repository: &PluginRepository<'_>,
    installation_id: &str,
    event_type: &str,
    now: i64,
    actor: &str,
) -> Result<(), PluginServiceError> {
    use aevio_persistence::database::entities::plugins::audit_events;
    repository
        .insert_audit_event(audit_events::ActiveModel {
            id: Set(Uuid::now_v7().to_string()),
            installation_id: Set(Some(installation_id.into())),
            event_type: Set(event_type.into()),
            actor: Set(actor.into()),
            payload_json: Set("{}".into()),
            created_at: Set(now),
        })
        .map_err(Into::into)
}

pub(super) fn state_name(state: PluginLifecycleState) -> &'static str {
    match state {
        PluginLifecycleState::Discovered => "discovered",
        PluginLifecycleState::InstallRequested => "install_requested",
        PluginLifecycleState::Resolving => "resolving",
        PluginLifecycleState::Downloading => "downloading",
        PluginLifecycleState::Verifying => "verifying",
        PluginLifecycleState::AwaitingPermissions => "awaiting_permissions",
        PluginLifecycleState::Staging => "staging",
        PluginLifecycleState::Installed => "installed",
        PluginLifecycleState::Activating => "activating",
        PluginLifecycleState::Active => "active",
        PluginLifecycleState::Disabled => "disabled",
        PluginLifecycleState::UpdateAvailable => "update_available",
        PluginLifecycleState::Updating => "updating",
        PluginLifecycleState::Deactivating => "deactivating",
        PluginLifecycleState::Uninstalling => "uninstalling",
        PluginLifecycleState::Uninstalled => "uninstalled",
        PluginLifecycleState::Failed => "failed",
        PluginLifecycleState::Quarantined => "quarantined",
        PluginLifecycleState::Incompatible => "incompatible",
        PluginLifecycleState::RecoveryRequired => "recovery_required",
    }
}

fn row_state(row: &installations::Model) -> Result<PluginLifecycleState, PluginServiceError> {
    parse_state(&row.state)
}
pub(super) fn scope_name(scope: PluginInstallationScope) -> &'static str {
    match scope {
        PluginInstallationScope::Personal => "personal",
        PluginInstallationScope::Workspace => "workspace",
    }
}
pub(crate) fn parse_state(value: &str) -> Result<PluginLifecycleState, PluginServiceError> {
    serde_json::from_str(&format!("\"{value}\""))
        .map_err(|_| PluginServiceError::InvalidState(value.into()))
}
fn parse_state_lossy(value: &str) -> PluginLifecycleState {
    parse_state(value).unwrap_or(PluginLifecycleState::Failed)
}
fn parse_scope(value: &str) -> PluginInstallationScope {
    if value == "workspace" {
        PluginInstallationScope::Workspace
    } else {
        PluginInstallationScope::Personal
    }
}
pub(crate) fn parse_permissions(value: &str) -> Result<Vec<String>, PluginServiceError> {
    serde_json::from_str(value)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))
}

/// Reads the normalized grant rows and keeps the installation JSON as a
/// compatibility fallback for databases created before grant rows were
/// populated. Expired and revoked rows never reach a capability gateway.
pub(crate) fn effective_permissions(
    db: &DatabaseConnection,
    installation: &installations::Model,
    now: i64,
) -> Result<Vec<String>, PluginServiceError> {
    let context = PluginExecutionContext::local(installation, "legacy-device")?;
    effective_permissions_for_context(db, installation, &context, now)
}

pub(crate) fn effective_permissions_for_context(
    db: &DatabaseConnection,
    installation: &installations::Model,
    context: &PluginExecutionContext,
    now: i64,
) -> Result<Vec<String>, PluginServiceError> {
    if context.plugin_id != installation.plugin_id
        || context.installation_id != installation.id
        || context.release_version != installation.version
    {
        return Err(PluginServiceError::Permission(
            "plugin execution context does not match the installation".into(),
        ));
    }
    let manifest = serde_json::from_str::<PluginManifest>(&installation.manifest_json)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    if !manifest.platforms.contains(&context.platform) {
        return Err(PluginServiceError::InvalidState(
            "plugin is not available on this platform".into(),
        ));
    }
    let declared = manifest
        .permissions
        .iter()
        .map(|permission| permission.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let repository = PluginRepository::new(db);
    let rows = repository.list_grants(&installation.id)?;
    if rows.is_empty() {
        return parse_permissions(&installation.granted_permissions_json).map(|permissions| {
            permissions
                .into_iter()
                .filter(|permission| {
                    declared.contains(permission.as_str())
                        && context.role.allows(minimum_role(permission))
                })
                .collect()
        });
    }
    Ok(rows
        .into_iter()
        .filter(|grant| {
            grant.revoked_at.is_none()
                && grant.expires_at.is_none_or(|expires_at| expires_at > now)
                && declared.contains(grant.capability.as_str())
                && grant_scope_matches(grant.scope.as_deref(), context)
                && context.role.allows(minimum_role(&grant.capability))
        })
        .map(|grant| grant.capability)
        .collect())
}

fn minimum_role(capability: &str) -> Role {
    if capability.ends_with(":read") || capability == "search:contribute" {
        Role::Viewer
    } else {
        Role::Member
    }
}

fn grant_scope_matches(scope: Option<&str>, context: &PluginExecutionContext) -> bool {
    let Some(scope) = scope else {
        return true;
    };
    scope == context.actor_ref
        || scope == context.workspace_ref
        || scope == context.device_ref
        || scope == format!("actor:{}", context.actor_ref)
        || scope == format!("workspace:{}", context.workspace_ref)
        || scope == format!("device:{}", context.device_ref)
}
fn parse_permissions_lossy(value: &str) -> Vec<String> {
    parse_permissions(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use aevio_contracts::plugin::{CommandContribution, SearchContribution, WorkflowContribution};
    use aevio_persistence::bootstrap::migrations::run_migrations;
    use aevio_persistence::database::entities::plugins::{grants, installations, storage_entries};
    use aevio_persistence::database::repositories::plugins::PluginRepository;
    use rusqlite::Connection;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set};
    use tempfile::tempdir;

    use super::{CapabilityGateway, filter_contributions, origin_matches};

    fn database() -> (tempfile::TempDir, sea_orm::DatabaseConnection) {
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
    fn gateway_rejects_capabilities_not_granted() {
        let gateway = CapabilityGateway::new("dev.example.test", ["projects:read".into()]);
        assert!(gateway.require("projects:read").is_ok());
        assert!(gateway.require("projects:write").is_err());
    }

    #[test]
    fn contribution_registry_filters_capability_backed_entries() {
        let contributions = aevio_contracts::plugin::PluginContributions {
            commands: vec![CommandContribution {
                id: "dev.example.test/run".into(),
                title: "Run".into(),
                description: None,
                required_capabilities: vec![],
            }],
            search: vec![SearchContribution {
                id: "dev.example.test/search".into(),
                label: "Search".into(),
                scopes: vec![],
            }],
            ..Default::default()
        };
        let filtered = filter_contributions(contributions, &[]);
        assert!(filtered.commands.is_empty());
        assert!(filtered.search.is_empty());
    }

    #[test]
    fn workflow_contributions_require_each_declared_capability() {
        let contributions = aevio_contracts::plugin::PluginContributions {
            workflow_actions: vec![
                WorkflowContribution {
                    id: "dev.example.test/read".into(),
                    title: "Read".into(),
                    required_capabilities: vec!["projects:read".into()],
                },
                WorkflowContribution {
                    id: "dev.example.test/write".into(),
                    title: "Write".into(),
                    required_capabilities: vec!["projects:write".into()],
                },
            ],
            workflow_triggers: vec![WorkflowContribution {
                id: "dev.example.test/trigger".into(),
                title: "Trigger".into(),
                required_capabilities: vec!["projects:read".into(), "tasks:read".into()],
            }],
            ..Default::default()
        };
        let filtered = filter_contributions(contributions, &["projects:read".into()]);
        assert_eq!(filtered.workflow_actions.len(), 1);
        assert_eq!(filtered.workflow_actions[0].id, "dev.example.test/read");
        assert!(filtered.workflow_triggers.is_empty());
    }

    #[test]
    fn network_origin_matching_is_exact_and_ignores_paths() {
        let target = url::Url::parse("https://api.example.test/v1/items").unwrap();
        assert!(origin_matches("https://api.example.test", &target));
        assert!(!origin_matches("http://api.example.test", &target));
        assert!(!origin_matches("https://other.example.test", &target));
        assert!(!origin_matches("https://api.example.test:8444", &target));
    }

    #[test]
    fn install_rejects_a_plan_after_the_installation_revision_changes() {
        let (_directory, database) = database();
        let plan = super::plan_install(
            &database,
            "dev.aevio.project-planning",
            aevio_contracts::plugin::PluginInstallationScope::Personal,
        )
        .unwrap();
        super::install_builtin_from_plan(&database, plan.clone(), 1).unwrap();
        let error = super::install_builtin_from_plan(&database, plan, 2).unwrap_err();
        assert!(error.to_string().contains("plan is stale"));
    }

    #[test]
    fn uninstall_retains_plugin_data_until_explicit_deletion() {
        let (_directory, database) = database();
        let plugin_id = "dev.aevio.project-planning";
        super::install_builtin(
            &database,
            plugin_id,
            aevio_contracts::plugin::PluginInstallationScope::Personal,
            1,
        )
        .unwrap();
        let installation = PluginRepository::new(&database)
            .installation(plugin_id)
            .unwrap()
            .unwrap();
        let permission = super::catalog(&database)
            .unwrap()
            .into_iter()
            .find(|entry| entry.manifest.id == plugin_id)
            .unwrap()
            .manifest
            .permissions[0]
            .id
            .clone();
        super::grant_permissions(&database, plugin_id, vec![permission], 2).unwrap();
        PluginRepository::new(&database)
            .insert_storage_entry(storage_entries::ActiveModel {
                id: Set("plugin-data".into()),
                installation_id: Set(installation.id.clone()),
                user_ref: Set("local-user".into()),
                workspace_ref: Set("personal".into()),
                device_ref: Set("test-device".into()),
                key: Set("settings".into()),
                value_json: Set("{\"enabled\":true}".into()),
                schema_version: Set(1),
                size_bytes: Set(16),
                updated_at: Set(2),
            })
            .unwrap();

        super::uninstall(&database, plugin_id, 3, None).unwrap();

        let uninstalled = PluginRepository::new(&database)
            .installation(plugin_id)
            .unwrap()
            .unwrap();
        assert_eq!(uninstalled.state, "uninstalled");
        assert!(
            PluginRepository::new(&database)
                .storage_entry(
                    &uninstalled.id,
                    "local-user",
                    "personal",
                    "test-device",
                    "settings",
                )
                .unwrap()
                .is_some()
        );
        assert_eq!(
            super::catalog(&database)
                .unwrap()
                .into_iter()
                .find(|entry| entry.manifest.id == plugin_id)
                .unwrap()
                .installed
                .unwrap()
                .state,
            aevio_contracts::plugin::PluginLifecycleState::Uninstalled
        );

        super::delete_data(&database, plugin_id, 4).unwrap();
        assert!(
            PluginRepository::new(&database)
                .storage_entry(
                    &uninstalled.id,
                    "local-user",
                    "personal",
                    "test-device",
                    "settings",
                )
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn granting_permissions_merges_without_dropping_existing_grants() {
        let (_directory, database) = database();
        let plugin_id = "dev.aevio.project-planning";
        super::install_builtin(
            &database,
            plugin_id,
            aevio_contracts::plugin::PluginInstallationScope::Personal,
            1,
        )
        .unwrap();
        let permissions = super::catalog(&database)
            .unwrap()
            .into_iter()
            .find(|entry| entry.manifest.id == plugin_id)
            .unwrap()
            .manifest
            .permissions;
        let first = permissions[0].id.clone();
        let second = permissions[1].id.clone();
        super::grant_permissions(&database, plugin_id, vec![first.clone()], 2).unwrap();
        let merged =
            super::grant_permissions(&database, plugin_id, vec![second.clone()], 3).unwrap();
        assert_eq!(merged.granted_permissions, vec![first, second]);
    }

    #[test]
    fn expired_and_revoked_grants_are_not_effective() {
        let (_directory, database) = database();
        let plugin_id = "dev.aevio.project-planning";
        super::install_builtin(
            &database,
            plugin_id,
            aevio_contracts::plugin::PluginInstallationScope::Personal,
            1,
        )
        .unwrap();
        let permission = super::catalog(&database)
            .unwrap()
            .into_iter()
            .find(|entry| entry.manifest.id == plugin_id)
            .unwrap()
            .manifest
            .permissions[0]
            .id
            .clone();
        super::grant_permissions(&database, plugin_id, vec![permission.clone()], 2).unwrap();
        let installation = PluginRepository::new(&database)
            .installation(plugin_id)
            .unwrap()
            .unwrap();
        let grant = PluginRepository::new(&database)
            .list_grants(&installation.id)
            .unwrap()
            .into_iter()
            .find(|grant| grant.capability == permission)
            .unwrap();
        let mut expired: grants::ActiveModel = grant.into();
        expired.expires_at = Set(Some(3));
        expired.update(&database).unwrap();
        assert!(
            super::effective_permissions(&database, &installation, 3)
                .unwrap()
                .is_empty()
        );
        let listed = super::installed(&database)
            .unwrap()
            .into_iter()
            .find(|summary| summary.plugin_id == plugin_id)
            .unwrap();
        assert!(listed.granted_permissions.is_empty());
        super::grant_permissions(&database, plugin_id, vec![permission.clone()], 4).unwrap();
        super::revoke_permissions(&database, plugin_id, vec![permission], 5).unwrap();
        let installation = PluginRepository::new(&database)
            .installation(plugin_id)
            .unwrap()
            .unwrap();
        assert!(
            super::effective_permissions(&database, &installation, 6)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn removing_a_permission_from_the_manifest_invalidates_old_grants() {
        let (_directory, database) = database();
        let plugin_id = "dev.aevio.project-planning";
        super::install_builtin(
            &database,
            plugin_id,
            aevio_contracts::plugin::PluginInstallationScope::Personal,
            1,
        )
        .unwrap();
        let permission = super::catalog(&database)
            .unwrap()
            .into_iter()
            .find(|entry| entry.manifest.id == plugin_id)
            .unwrap()
            .manifest
            .permissions[0]
            .id
            .clone();
        super::grant_permissions(&database, plugin_id, vec![permission.clone()], 2).unwrap();
        let installation = PluginRepository::new(&database)
            .installation(plugin_id)
            .unwrap()
            .unwrap();
        let mut manifest: aevio_contracts::plugin::PluginManifest =
            serde_json::from_str(&installation.manifest_json).unwrap();
        manifest.permissions.retain(|item| item.id != permission);
        let mut changed: installations::ActiveModel = installation.clone().into();
        changed.manifest_json = Set(serde_json::to_string(&manifest).unwrap());
        changed.update(&database).unwrap();
        let updated = PluginRepository::new(&database)
            .installation(plugin_id)
            .unwrap()
            .unwrap();
        assert!(
            super::effective_permissions(&database, &updated, 3)
                .unwrap()
                .is_empty()
        );
    }
}
