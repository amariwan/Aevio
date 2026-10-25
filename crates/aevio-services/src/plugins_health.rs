use aevio_contracts::plugin::{PluginHealthSummary, PluginLifecycleState};
use aevio_persistence::database::repositories::plugins::PluginRepository;
use sea_orm::DatabaseConnection;

use super::plugins::{PluginServiceError, state_name};

pub fn health(db: &DatabaseConnection) -> Result<Vec<PluginHealthSummary>, PluginServiceError> {
    let repository = PluginRepository::new(db);
    let health = repository
        .all_runtime_health()?
        .into_iter()
        .map(|item| (item.installation_id.clone(), item))
        .collect::<std::collections::HashMap<_, _>>();
    repository
        .list_installations()?
        .into_iter()
        .map(|installation| {
            let state = health
                .get(&installation.id)
                .map(|item| item.state.clone())
                .unwrap_or_else(|| {
                    if installation.state == state_name(PluginLifecycleState::Active) {
                        "healthy".into()
                    } else {
                        "inactive".into()
                    }
                });
            let item = health.get(&installation.id);
            Ok(PluginHealthSummary {
                plugin_id: installation.plugin_id,
                state,
                last_success_at: item.and_then(|value| value.last_success_at),
                last_failure_at: item.and_then(|value| value.last_failure_at),
                crash_count: item.map_or(0, |value| value.crash_count.max(0) as u64),
                permission_denials: item.map_or(0, |value| value.permission_denials.max(0) as u64),
                resource_violations: item
                    .map_or(0, |value| value.resource_violations.max(0) as u64),
                last_error: item.and_then(|value| value.last_error.clone()),
            })
        })
        .collect()
}
