//! Persistent plugin update policy. The Store may present choices, but the
//! privileged installer is the authority that enforces them.

use aevio_contracts::plugin::{
    PluginCatalogEntry, PluginRelease, PluginUpdateCandidate, PluginUpdateChannel,
    PluginUpdatePolicy,
};
use aevio_persistence::database::entities::plugins::update_policies;
use aevio_persistence::database::repositories::plugins::PluginRepository;
use sea_orm::{ActiveValue::Set, DatabaseConnection};
use semver::Version;

use super::plugins::{PluginServiceError, audit};

pub fn get(
    db: &DatabaseConnection,
    plugin_id: &str,
) -> Result<PluginUpdatePolicy, PluginServiceError> {
    if plugin_id.trim().is_empty() {
        return Err(PluginServiceError::Permission(
            "plugin id is required".into(),
        ));
    }
    let repository = PluginRepository::new(db);
    Ok(repository
        .update_policy(plugin_id)?
        .map(to_policy)
        .unwrap_or_else(|| PluginUpdatePolicy {
            plugin_id: plugin_id.into(),
            channel: PluginUpdateChannel::Stable,
            pinned_version: None,
            automatic_updates: false,
            updated_at: 0,
        }))
}

pub fn set(
    db: &DatabaseConnection,
    mut policy: PluginUpdatePolicy,
    now: i64,
) -> Result<PluginUpdatePolicy, PluginServiceError> {
    if policy.plugin_id.trim().is_empty() || policy.plugin_id.len() > 160 {
        return Err(PluginServiceError::Permission(
            "plugin id is invalid".into(),
        ));
    }
    if let Some(version) = &policy.pinned_version {
        Version::parse(version).map_err(|_| {
            PluginServiceError::Permission("pinned plugin version is not valid semver".into())
        })?;
    }
    let repository = PluginRepository::new(db);
    repository.upsert_update_policy(
        &policy.plugin_id,
        update_policies::ActiveModel {
            id: Set(policy.plugin_id.clone()),
            channel: Set(policy.channel.as_str().into()),
            pinned_version: Set(policy.pinned_version.clone()),
            automatic_updates: Set(i64::from(policy.automatic_updates)),
            updated_at: Set(now),
        },
    )?;
    policy.updated_at = now;
    if let Some(installation) = repository.installation(&policy.plugin_id)? {
        audit(
            &repository,
            &installation.id,
            "update_policy_changed",
            now,
            "local-user",
        )?;
    }
    Ok(policy)
}

pub fn allows_release(
    db: &DatabaseConnection,
    plugin_id: &str,
    release: &PluginRelease,
) -> Result<(), PluginServiceError> {
    let policy = get(db, plugin_id)?;
    if let Some(pinned) = policy.pinned_version
        && pinned != release.version
    {
        return Err(PluginServiceError::InvalidState(
            "plugin release is outside the pinned version".into(),
        ));
    }
    let allowed = match policy.channel {
        PluginUpdateChannel::Stable => release.channel == "stable",
        PluginUpdateChannel::Beta => matches!(release.channel.as_str(), "stable" | "beta"),
        PluginUpdateChannel::Development => true,
    };
    if allowed {
        Ok(())
    } else {
        Err(PluginServiceError::InvalidState(
            "plugin release is outside the selected update channel".into(),
        ))
    }
}

/// Selects the highest policy-compatible release newer than the installed
/// version. The registry is untrusted input, so invalid or revoked releases
/// are ignored and the installer remains the final authority.
pub fn available_updates(
    db: &DatabaseConnection,
    catalog: &[PluginCatalogEntry],
) -> Result<Vec<PluginUpdateCandidate>, PluginServiceError> {
    let repository = PluginRepository::new(db);
    let installations = repository.list_installations()?;
    let mut candidates = Vec::new();
    for entry in catalog {
        let Some(installation) = installations
            .iter()
            .find(|row| row.plugin_id == entry.manifest.id)
        else {
            continue;
        };
        if installation.state
            == super::plugins::state_name(
                aevio_contracts::plugin::PluginLifecycleState::Uninstalled,
            )
        {
            continue;
        }
        let current_version = Version::parse(&installation.version)
            .map_err(|error| PluginServiceError::InvalidState(error.to_string()))?;
        let selected = entry
            .releases
            .iter()
            .filter(|release| !release.revoked)
            .filter_map(|release| {
                let version = Version::parse(&release.version).ok()?;
                (version > current_version
                    && allows_release(db, &entry.manifest.id, release).is_ok())
                .then_some((version, release))
            })
            .max_by(|left, right| left.0.cmp(&right.0));
        let Some((_, release)) = selected else {
            continue;
        };
        let previous_manifest = serde_json::from_str::<aevio_contracts::plugin::PluginManifest>(
            &installation.manifest_json,
        )
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
        let previous_permissions = previous_manifest
            .permissions
            .iter()
            .map(|permission| permission.id.as_str())
            .collect::<std::collections::HashSet<_>>();
        let added_permissions = entry
            .manifest
            .permissions
            .iter()
            .map(|permission| permission.id.clone())
            .filter(|permission| !previous_permissions.contains(permission.as_str()))
            .collect::<Vec<_>>();
        let policy = get(db, &entry.manifest.id)?;
        candidates.push(PluginUpdateCandidate {
            plugin_id: entry.manifest.id.clone(),
            current_version: installation.version.clone(),
            release: release.clone(),
            requires_permission_approval: !added_permissions.is_empty(),
            added_permissions,
            automatic_updates: policy.automatic_updates,
        });
    }
    candidates.sort_by(|left, right| left.plugin_id.cmp(&right.plugin_id));
    Ok(candidates)
}

fn to_policy(row: update_policies::Model) -> PluginUpdatePolicy {
    PluginUpdatePolicy {
        plugin_id: row.id,
        channel: PluginUpdateChannel::parse(&row.channel).unwrap_or_default(),
        pinned_version: row.pinned_version,
        automatic_updates: row.automatic_updates != 0,
        updated_at: row.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::{allows_release, available_updates, get, set};
    use crate::plugins;
    use aevio_contracts::plugin::{
        PluginPermissionRequest, PluginRelease, PluginUpdateChannel, PluginUpdatePolicy,
    };
    use aevio_persistence::bootstrap::migrations::run_migrations;
    use aevio_persistence::database::connect;
    use rusqlite::Connection;
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
    fn policy_selects_channels_and_pins_exact_versions() {
        let (_directory, db) = database();
        assert_eq!(
            get(&db, "dev.example").unwrap().channel,
            PluginUpdateChannel::Stable
        );
        set(
            &db,
            PluginUpdatePolicy {
                plugin_id: "dev.example".into(),
                channel: PluginUpdateChannel::Beta,
                pinned_version: Some("2.0.0".into()),
                automatic_updates: true,
                updated_at: 0,
            },
            10,
        )
        .unwrap();
        let release = |version: &str, channel: &str| PluginRelease {
            version: version.into(),
            artifact_digest: "digest".into(),
            artifact_size: 1,
            signature: None,
            channel: channel.into(),
            published_at: 1,
            revoked: false,
        };
        assert!(allows_release(&db, "dev.example", &release("2.0.0", "beta")).is_ok());
        assert!(allows_release(&db, "dev.example", &release("2.0.1", "beta")).is_err());
        assert!(allows_release(&db, "dev.example", &release("2.0.0", "development")).is_err());
    }

    #[test]
    fn update_discovery_selects_newest_allowed_release_and_discloses_permissions() {
        let (_directory, db) = database();
        let mut entry = plugins::public_catalog()
            .into_iter()
            .find(|entry| entry.manifest.id == "dev.aevio.project-planning")
            .unwrap();
        plugins::install_builtin(
            &db,
            &entry.manifest.id,
            aevio_contracts::plugin::PluginInstallationScope::Personal,
            1,
        )
        .unwrap();
        entry.manifest.version = "1.2.0".into();
        entry.manifest.permissions.push(PluginPermissionRequest {
            id: "events:publish".into(),
            scope: None,
            reason: None,
        });
        entry.releases = vec![
            PluginRelease {
                version: "1.1.0".into(),
                artifact_digest: "one".into(),
                artifact_size: 1,
                signature: None,
                channel: "stable".into(),
                published_at: 1,
                revoked: false,
            },
            PluginRelease {
                version: "1.2.0".into(),
                artifact_digest: "two".into(),
                artifact_size: 1,
                signature: None,
                channel: "stable".into(),
                published_at: 2,
                revoked: false,
            },
        ];
        let updates = available_updates(&db, &[entry]).unwrap();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].release.version, "1.2.0");
        assert_eq!(updates[0].added_permissions, vec!["events:publish"]);
        assert!(updates[0].requires_permission_approval);
    }
}
