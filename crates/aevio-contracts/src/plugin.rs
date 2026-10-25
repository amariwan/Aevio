//! Versioned contracts for the Aevio plugin host.
//!
//! These types are deliberately declarative. A manifest can describe what a
//! plugin wants to contribute, but it cannot grant itself access to a host
//! service. Grants are evaluated by the privileged host boundary.

use std::collections::HashSet;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MANIFEST_VERSION: u32 = 1;
pub const PLUGIN_API_VERSION: &str = "1.0.0";
pub const MAX_MANIFEST_BYTES: usize = 512 * 1024;
pub const MAX_ASSET_BYTES: u64 = 25 * 1024 * 1024;
pub const MIN_PLUGIN_JOB_INTERVAL_MS: u64 = 60_000;
pub const MAX_PLUGIN_JOB_INTERVAL_MS: u64 = 30 * 24 * 60 * 60 * 1_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub manifest_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: PublisherIdentity,
    pub description: String,
    pub category: String,
    pub compatibility: PluginCompatibility,
    pub platforms: Vec<PluginPlatform>,
    pub entrypoints: PluginEntrypoints,
    #[serde(default)]
    pub icons: Vec<PluginAsset>,
    #[serde(default)]
    pub screenshots: Vec<PluginAsset>,
    #[serde(default)]
    pub permissions: Vec<PluginPermissionRequest>,
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
    #[serde(default)]
    pub optional_dependencies: Vec<PluginDependency>,
    #[serde(default)]
    pub contributes: PluginContributions,
    #[serde(default)]
    pub event_subscriptions: Vec<EventSubscription>,
    #[serde(default)]
    pub offline: OfflineSupport,
    #[serde(default)]
    pub data_schema: Option<PluginDataSchema>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PublisherIdentity {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub public_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PluginCompatibility {
    pub aevio: String,
    pub plugin_api: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginPlatform {
    Desktop,
    Web,
    Macos,
    Windows,
    Linux,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PluginEntrypoints {
    pub ui: Option<String>,
    pub component: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PluginAsset {
    pub path: String,
    #[serde(default)]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PluginDependency {
    pub plugin_id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct EventSubscription {
    /// Stable contribution identity owned by the subscribing plugin.
    pub id: String,
    /// Versioned event type supplied by the publisher. When omitted, `id` is
    /// retained as the event type for backwards-compatible manifests.
    #[serde(default)]
    pub event_type: Option<String>,
    /// Optional host workflow action to invoke for each durable delivery.
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PluginPermissionRequest {
    pub id: String,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PluginContributions {
    #[serde(default)]
    pub navigation: Vec<NavigationContribution>,
    #[serde(default)]
    pub pages: Vec<PageContribution>,
    #[serde(default)]
    pub widgets: Vec<WidgetContribution>,
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
    #[serde(default)]
    pub settings: Vec<SettingsContribution>,
    #[serde(default)]
    pub search: Vec<SearchContribution>,
    #[serde(default)]
    pub workflow_actions: Vec<WorkflowContribution>,
    #[serde(default)]
    pub workflow_triggers: Vec<WorkflowContribution>,
    #[serde(default)]
    pub jobs: Vec<JobContribution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct NavigationContribution {
    pub id: String,
    pub label: String,
    pub page: String,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PageContribution {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct WidgetContribution {
    pub id: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CommandContribution {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct SettingsContribution {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct SearchContribution {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowContribution {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct JobContribution {
    pub id: String,
    pub title: String,
    pub min_interval_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct OfflineSupport {
    pub activation: bool,
    pub network_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PluginDataSchema {
    pub version: u32,
    pub namespace: String,
    pub max_bytes: u64,
}

/// Capabilities are intentionally an allow-list, not a free-form privilege
/// string. Adding a capability is an API/security decision.
pub mod capabilities {
    pub const STORAGE_OWN_READ: &str = "storage:own:read";
    pub const STORAGE_OWN_WRITE: &str = "storage:own:write";
    pub const PROJECTS_READ: &str = "projects:read";
    pub const PROJECTS_WRITE: &str = "projects:write";
    pub const TASKS_READ: &str = "tasks:read";
    pub const TASKS_WRITE: &str = "tasks:write";
    pub const CLIENTS_READ: &str = "clients:read";
    pub const CLIENTS_WRITE: &str = "clients:write";
    pub const CALENDAR_READ: &str = "calendar:read";
    pub const CALENDAR_WRITE: &str = "calendar:write";
    pub const NOTIFICATIONS_SHOW: &str = "notifications:show";
    pub const FILES_READ_SELECTED: &str = "files:read-selected";
    pub const FILES_WRITE_SELECTED: &str = "files:write-selected";
    pub const NETWORK_FETCH: &str = "network:fetch";
    pub const SEARCH_CONTRIBUTE: &str = "search:contribute";
    pub const COMMANDS_REGISTER: &str = "commands:register";
    pub const WIDGETS_REGISTER: &str = "widgets:register";
    pub const EVENTS_SUBSCRIBE: &str = "events:subscribe";
    pub const EVENTS_PUBLISH: &str = "events:publish";
    pub const JOBS_SCHEDULE: &str = "jobs:schedule";

    pub const ALL: &[&str] = &[
        STORAGE_OWN_READ,
        STORAGE_OWN_WRITE,
        PROJECTS_READ,
        PROJECTS_WRITE,
        TASKS_READ,
        TASKS_WRITE,
        CLIENTS_READ,
        CLIENTS_WRITE,
        CALENDAR_READ,
        CALENDAR_WRITE,
        NOTIFICATIONS_SHOW,
        FILES_READ_SELECTED,
        FILES_WRITE_SELECTED,
        NETWORK_FETCH,
        SEARCH_CONTRIBUTE,
        COMMANDS_REGISTER,
        WIDGETS_REGISTER,
        EVENTS_SUBSCRIBE,
        EVENTS_PUBLISH,
        JOBS_SCHEDULE,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestValidationError {
    pub field: String,
    pub message: String,
}

impl std::fmt::Display for ManifestValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ManifestValidationError {}

impl PluginManifest {
    pub fn validate(&self) -> Result<(), ManifestValidationError> {
        if self.manifest_version != MANIFEST_VERSION {
            return Err(error("manifestVersion", "unsupported manifest version"));
        }
        validate_id("id", &self.id, false)?;
        if self.name.trim().is_empty() || self.name.chars().count() > 120 {
            return Err(error("name", "must contain 1-120 characters"));
        }
        if self.description.chars().count() > 10_000 {
            return Err(error("description", "is too long"));
        }
        Version::parse(&self.version)
            .map_err(|_| error("version", "must be a valid semantic version"))?;
        VersionReq::parse(&self.compatibility.aevio)
            .map_err(|_| error("compatibility.aevio", "must be a valid version requirement"))?;
        VersionReq::parse(&self.compatibility.plugin_api).map_err(|_| {
            error(
                "compatibility.pluginApi",
                "must be a valid version requirement",
            )
        })?;
        if self.platforms.is_empty() {
            return Err(error("platforms", "must declare at least one platform"));
        }
        validate_entrypoint("entrypoints.ui", self.entrypoints.ui.as_deref(), &["html"])?;
        validate_entrypoint(
            "entrypoints.component",
            self.entrypoints.component.as_deref(),
            &["wasm"],
        )?;
        validate_assets("icons", &self.icons, 8)?;
        validate_assets("screenshots", &self.screenshots, 12)?;
        for permission in &self.permissions {
            validate_capability("permissions", &permission.id)?;
            if permission.scope.as_deref().is_some_and(|scope| {
                scope.is_empty()
                    || scope.len() > 120
                    || scope.chars().any(|character| character.is_control())
            }) {
                return Err(error(
                    "permissions.scope",
                    "must be a bounded non-control string",
                ));
            }
            if permission
                .reason
                .as_deref()
                .is_some_and(|reason| reason.chars().count() > 500)
            {
                return Err(error("permissions.reason", "is too long"));
            }
        }
        for dependency in self.dependencies.iter().chain(&self.optional_dependencies) {
            validate_id("dependencies.pluginId", &dependency.plugin_id, false)?;
            if dependency.plugin_id == self.id {
                return Err(error("dependencies.pluginId", "cannot depend on itself"));
            }
            VersionReq::parse(&dependency.version).map_err(|_| {
                error(
                    "dependencies.version",
                    "must be a valid version requirement",
                )
            })?;
        }
        for event in &self.event_subscriptions {
            validate_id("eventSubscriptions.id", &event.id, true)?;
            if !event.id.starts_with(&format!("{}/", self.id)) {
                return Err(error(
                    "eventSubscriptions.id",
                    "must be namespaced by the plugin id",
                ));
            }
            if let Some(event_type) = &event.event_type {
                validate_event_type(event_type)?;
            }
            if let Some(workflow_id) = &event.workflow_id {
                validate_id("eventSubscriptions.workflowId", workflow_id, true)?;
                if !workflow_id.starts_with(&format!("{}/", self.id)) {
                    return Err(error(
                        "eventSubscriptions.workflowId",
                        "must be namespaced by the plugin id",
                    ));
                }
                if !self
                    .contributes
                    .workflow_actions
                    .iter()
                    .any(|workflow| workflow.id == *workflow_id)
                {
                    return Err(error(
                        "eventSubscriptions.workflowId",
                        "must reference a declared workflow action",
                    ));
                }
            }
            for capability in &event.required_capabilities {
                validate_capability("eventSubscriptions.requiredCapabilities", capability)?;
            }
        }
        if let Some(schema) = &self.data_schema {
            validate_id("dataSchema.namespace", &schema.namespace, true)?;
            if schema.max_bytes > 100 * 1024 * 1024 {
                return Err(error("dataSchema.maxBytes", "exceeds the host limit"));
            }
        }
        let mut ids = HashSet::new();
        for id in contribution_ids(self) {
            validate_id("contributions.id", &id, true)?;
            if !id.starts_with(&format!("{}/", self.id)) {
                return Err(error(
                    "contributions.id",
                    "must be namespaced by the plugin id",
                ));
            }
            if !ids.insert(id) {
                return Err(error("contributions", "contains a duplicate id"));
            }
        }
        for job in &self.contributes.jobs {
            if !(MIN_PLUGIN_JOB_INTERVAL_MS..=MAX_PLUGIN_JOB_INTERVAL_MS)
                .contains(&job.min_interval_ms)
            {
                return Err(error(
                    "contributions.jobs.minIntervalMs",
                    "must be between 1 minute and 30 days",
                ));
            }
        }
        for command in &self.contributes.commands {
            for capability in &command.required_capabilities {
                validate_capability("contributions.commands.requiredCapabilities", capability)?;
            }
        }
        for workflow in self
            .contributes
            .workflow_actions
            .iter()
            .chain(&self.contributes.workflow_triggers)
        {
            for capability in &workflow.required_capabilities {
                validate_capability("contributions.workflow.requiredCapabilities", capability)?;
            }
        }
        Ok(())
    }
}

fn validate_assets(
    field: &str,
    assets: &[PluginAsset],
    maximum: usize,
) -> Result<(), ManifestValidationError> {
    if assets.len() > maximum {
        return Err(error(field, "contains too many assets"));
    }
    for asset in assets {
        validate_package_asset_path(field, &asset.path)?;
        if asset.mime_type.as_deref().is_some_and(|mime| {
            mime.is_empty()
                || mime.len() > 100
                || mime.chars().any(|character| character.is_control())
        }) {
            return Err(error(field, "contains an invalid MIME type"));
        }
    }
    Ok(())
}

fn validate_package_asset_path(field: &str, path: &str) -> Result<(), ManifestValidationError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.split('/').any(|part| part == ".." || part.is_empty())
    {
        return Err(error(field, "contains an unsafe asset path"));
    }
    let extension = path.rsplit('.').next().unwrap_or_default();
    if !["png", "jpg", "jpeg", "webp", "svg"].contains(&extension) {
        return Err(error(field, "contains an unsupported asset format"));
    }
    Ok(())
}

fn validate_capability(field: &str, capability: &str) -> Result<(), ManifestValidationError> {
    if capabilities::ALL.contains(&capability) {
        Ok(())
    } else {
        Err(error(field, "contains an unsupported capability"))
    }
}

fn validate_event_type(event_type: &str) -> Result<(), ManifestValidationError> {
    if event_type.is_empty()
        || event_type.len() > 128
        || !event_type.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '-' | '_' | '/')
        })
    {
        return Err(error(
            "eventSubscriptions.eventType",
            "must be a bounded lowercase versioned event name",
        ));
    }
    Ok(())
}

fn contribution_ids(manifest: &PluginManifest) -> Vec<String> {
    let contributions = &manifest.contributes;
    contributions
        .navigation
        .iter()
        .map(|item| item.id.clone())
        .chain(contributions.pages.iter().map(|item| item.id.clone()))
        .chain(contributions.widgets.iter().map(|item| item.id.clone()))
        .chain(contributions.commands.iter().map(|item| item.id.clone()))
        .chain(contributions.settings.iter().map(|item| item.id.clone()))
        .chain(contributions.search.iter().map(|item| item.id.clone()))
        .chain(
            contributions
                .workflow_actions
                .iter()
                .map(|item| item.id.clone()),
        )
        .chain(
            contributions
                .workflow_triggers
                .iter()
                .map(|item| item.id.clone()),
        )
        .chain(contributions.jobs.iter().map(|item| item.id.clone()))
        .collect()
}

fn validate_entrypoint(
    field: &str,
    path: Option<&str>,
    extensions: &[&str],
) -> Result<(), ManifestValidationError> {
    let Some(path) = path else { return Ok(()) };
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.split('/').any(|part| part == ".." || part.is_empty())
    {
        return Err(error(field, "must be a relative package path"));
    }
    let extension = path.rsplit('.').next().unwrap_or_default();
    if !extensions.contains(&extension) {
        return Err(error(field, "has an unsupported entrypoint format"));
    }
    Ok(())
}

fn validate_id(
    field: &str,
    value: &str,
    allow_namespace: bool,
) -> Result<(), ManifestValidationError> {
    let valid = !value.is_empty()
        && value.len() <= 160
        && value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '-' | '_' | '/')
        })
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
        && (allow_namespace || !value.contains('/'));
    if valid {
        Ok(())
    } else {
        Err(error(field, "contains invalid identifier characters"))
    }
}

fn error(field: &str, message: &str) -> ManifestValidationError {
    ManifestValidationError {
        field: field.into(),
        message: message.into(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginLifecycleState {
    Discovered,
    InstallRequested,
    Resolving,
    Downloading,
    Verifying,
    AwaitingPermissions,
    Staging,
    Installed,
    Activating,
    Active,
    Disabled,
    UpdateAvailable,
    Updating,
    Deactivating,
    Uninstalling,
    Uninstalled,
    Failed,
    Quarantined,
    Incompatible,
    RecoveryRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginInstallationScope {
    Personal,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRelease {
    pub version: String,
    pub artifact_digest: String,
    pub artifact_size: u64,
    pub signature: Option<String>,
    pub channel: String,
    pub published_at: i64,
    #[serde(default)]
    pub revoked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum PluginUpdateChannel {
    #[default]
    Stable,
    Beta,
    Development,
}

impl PluginUpdateChannel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Development => "development",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "stable" => Some(Self::Stable),
            "beta" => Some(Self::Beta),
            "development" => Some(Self::Development),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginUpdatePolicy {
    pub plugin_id: String,
    pub channel: PluginUpdateChannel,
    #[serde(default)]
    pub pinned_version: Option<String>,
    pub automatic_updates: bool,
    pub updated_at: i64,
}

/// Server-authoritative workspace intent. This never represents a local
/// artifact or a device grant; clients reconcile it into their own verified
/// installation state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginWorkspaceDesiredState {
    pub workspace_id: String,
    pub plugin_id: String,
    pub requested_version: Option<String>,
    pub channel: PluginUpdateChannel,
    pub enabled: bool,
    pub requested_permissions: Vec<String>,
    pub revision: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginWorkspaceDesiredStateInput {
    pub requested_version: Option<String>,
    pub channel: PluginUpdateChannel,
    pub enabled: bool,
    pub requested_permissions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCatalogEntry {
    pub manifest: PluginManifest,
    pub releases: Vec<PluginRelease>,
    pub installed: Option<PluginInstallationSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginUpdateCandidate {
    pub plugin_id: String,
    pub current_version: String,
    pub release: PluginRelease,
    pub added_permissions: Vec<String>,
    pub requires_permission_approval: bool,
    pub automatic_updates: bool,
}

/// Host-composed contributions from active installations. The host filters
/// capability-backed contribution kinds before exposing this snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginContributionSnapshot {
    pub plugin_id: String,
    pub plugin_name: String,
    pub version: String,
    pub contributions: PluginContributions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginContributionPreference {
    pub plugin_id: String,
    pub contribution_id: String,
    pub visible: bool,
    pub pinned: bool,
    pub order_index: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHealthSummary {
    pub plugin_id: String,
    pub state: String,
    pub last_success_at: Option<i64>,
    pub last_failure_at: Option<i64>,
    pub crash_count: u64,
    pub permission_denials: u64,
    pub resource_violations: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommandRequest {
    pub plugin_id: String,
    pub command_id: String,
    #[serde(default)]
    pub input: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommandResult {
    pub plugin_id: String,
    pub command_id: String,
    pub kind: String,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginWorkflowRequest {
    pub plugin_id: String,
    pub workflow_id: String,
    pub trigger: String,
    pub idempotency_key: String,
    #[serde(default)]
    pub input: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginWorkflowRunSummary {
    pub id: String,
    pub plugin_id: String,
    pub workflow_id: String,
    pub trigger: String,
    pub idempotency_key: String,
    pub status: String,
    pub attempt_count: u64,
    pub max_attempts: u64,
    pub input: Value,
    pub output: Option<Value>,
    pub last_error: Option<String>,
    pub created_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEventPublicationSummary {
    pub event_id: String,
    pub event_type: String,
    pub delivery_count: usize,
    pub occurred_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEventDeliverySummary {
    pub id: String,
    pub event_id: String,
    pub plugin_id: String,
    pub workflow_id: String,
    pub status: String,
    pub attempt_count: u64,
    pub max_attempts: u64,
    pub last_error: Option<String>,
    pub created_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginNetworkRequest {
    pub url: String,
    #[serde(default = "default_network_method")]
    pub method: String,
    #[serde(default)]
    pub body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginNetworkResponse {
    pub status: u16,
    pub body: String,
}

fn default_network_method() -> String {
    "GET".into()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSearchResult {
    pub plugin_id: String,
    pub contribution_id: String,
    pub title: String,
    pub detail: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSearchFailure {
    pub plugin_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSearchAggregate {
    pub results: Vec<PluginSearchResult>,
    pub failures: Vec<PluginSearchFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginJobScheduleRequest {
    pub plugin_id: String,
    pub job_id: String,
    pub interval_ms: u64,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginJobSummary {
    pub id: String,
    pub plugin_id: String,
    pub job_id: String,
    pub interval_ms: u64,
    pub next_run_at: i64,
    pub last_run_at: Option<i64>,
    pub run_count: u64,
    pub enabled: bool,
    pub state: String,
    pub attempt_count: u64,
    pub max_attempts: u64,
    pub last_error: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallationSummary {
    pub installation_id: String,
    pub plugin_id: String,
    pub version: String,
    pub scope: PluginInstallationScope,
    pub state: PluginLifecycleState,
    pub granted_permissions: Vec<String>,
    pub updated_at: i64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallPlan {
    pub plan_id: String,
    pub plugin_id: String,
    pub release: PluginRelease,
    pub scope: PluginInstallationScope,
    pub new_permissions: Vec<String>,
    pub warnings: Vec<String>,
    pub base_revision: u64,
}
