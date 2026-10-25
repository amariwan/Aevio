//! Durable workspace composition preferences for plugin contributions.

use std::cmp::Ordering;
use std::collections::HashMap;

use aevio_contracts::plugin::{
    PluginContributionPreference, PluginContributions, PluginLifecycleState,
};
use aevio_persistence::database::{
    entities::plugins::layout_preferences, repositories::plugins::PluginRepository,
};
use sea_orm::{ActiveValue::Set, DatabaseConnection};
use uuid::Uuid;

use super::plugins::{PluginServiceError, parse_state};

const MAX_ORDER_INDEX: i32 = 1_000_000;

pub fn list(
    db: &DatabaseConnection,
) -> Result<Vec<PluginContributionPreference>, PluginServiceError> {
    let repository = PluginRepository::new(db);
    let mut result = Vec::new();
    for installation in repository.list_installations()? {
        if parse_state(&installation.state)? != PluginLifecycleState::Active {
            continue;
        }
        let manifest: aevio_contracts::plugin::PluginManifest =
            serde_json::from_str(&installation.manifest_json)
                .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
        let persisted = repository
            .layout_preferences()?
            .into_iter()
            .filter(|preference| preference.installation_id == installation.id)
            .map(|preference| {
                (
                    preference.contribution_id,
                    PluginContributionPreference {
                        plugin_id: manifest.id.clone(),
                        contribution_id: String::new(),
                        visible: preference.visible != 0,
                        pinned: preference.pinned != 0,
                        order_index: preference.order_index as i32,
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        for (index, contribution_id) in contribution_ids(&manifest.contributes)
            .into_iter()
            .enumerate()
        {
            let mut preference =
                persisted
                    .get(&contribution_id)
                    .cloned()
                    .unwrap_or(PluginContributionPreference {
                        plugin_id: manifest.id.clone(),
                        contribution_id: contribution_id.clone(),
                        visible: true,
                        pinned: false,
                        order_index: index.min(MAX_ORDER_INDEX as usize) as i32,
                    });
            preference.contribution_id = contribution_id;
            result.push(preference);
        }
    }
    result.sort_by(preference_order);
    Ok(result)
}

pub fn set(
    db: &DatabaseConnection,
    mut preference: PluginContributionPreference,
    now: i64,
) -> Result<PluginContributionPreference, PluginServiceError> {
    if preference.order_index < 0 || preference.order_index > MAX_ORDER_INDEX {
        return Err(PluginServiceError::InvalidState(
            "plugin contribution order is out of bounds".into(),
        ));
    }
    preference.pinned &= preference.visible;
    let repository = PluginRepository::new(db);
    let installation = repository
        .installation(&preference.plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(preference.plugin_id.clone()))?;
    if parse_state(&installation.state)? != PluginLifecycleState::Active {
        return Err(PluginServiceError::InvalidState(
            "plugin must be active to change its layout".into(),
        ));
    }
    let manifest: aevio_contracts::plugin::PluginManifest =
        serde_json::from_str(&installation.manifest_json)
            .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    if !contribution_ids(&manifest.contributes)
        .iter()
        .any(|id| id == &preference.contribution_id)
    {
        return Err(PluginServiceError::NotFound(
            preference.contribution_id.clone(),
        ));
    }
    if let Some(existing) =
        repository.layout_preference(&installation.id, &preference.contribution_id)?
    {
        let mut model: layout_preferences::ActiveModel = existing.into();
        model.visible = Set(i64::from(preference.visible));
        model.pinned = Set(i64::from(preference.pinned));
        model.order_index = Set(i64::from(preference.order_index));
        model.updated_at = Set(now);
        repository.update_layout_preference(model)?;
    } else {
        repository.insert_layout_preference(layout_preferences::ActiveModel {
            id: Set(Uuid::now_v7().to_string()),
            installation_id: Set(installation.id),
            contribution_id: Set(preference.contribution_id.clone()),
            visible: Set(i64::from(preference.visible)),
            pinned: Set(i64::from(preference.pinned)),
            order_index: Set(i64::from(preference.order_index)),
            updated_at: Set(now),
        })?;
    }
    Ok(preference)
}

pub fn restore(db: &DatabaseConnection) -> Result<(), PluginServiceError> {
    PluginRepository::new(db).delete_layout_preferences()?;
    Ok(())
}

pub fn apply(
    contributions: &mut PluginContributions,
    preferences: &[PluginContributionPreference],
) {
    let by_id = preferences
        .iter()
        .map(|preference| (preference.contribution_id.as_str(), preference))
        .collect::<HashMap<_, _>>();
    sort_visible(&mut contributions.navigation, &by_id, |item| &item.id);
    sort_visible(&mut contributions.pages, &by_id, |item| &item.id);
    sort_visible(&mut contributions.widgets, &by_id, |item| &item.id);
    sort_visible(&mut contributions.settings, &by_id, |item| &item.id);
}

fn sort_visible<T>(
    values: &mut Vec<T>,
    preferences: &HashMap<&str, &PluginContributionPreference>,
    id: impl Fn(&T) -> &String,
) {
    values.retain(|value| {
        preferences
            .get(id(value).as_str())
            .is_none_or(|preference| preference.visible)
    });
    values.sort_by(|left, right| {
        let left_preference = preferences.get(id(left).as_str()).copied();
        let right_preference = preferences.get(id(right).as_str()).copied();
        preference_values(left_preference).cmp(&preference_values(right_preference))
    });
}

fn preference_order(
    left: &PluginContributionPreference,
    right: &PluginContributionPreference,
) -> Ordering {
    preference_values(Some(left)).cmp(&preference_values(Some(right)))
}

fn preference_values(preference: Option<&PluginContributionPreference>) -> (bool, i32, &str) {
    preference
        .map(|preference| {
            (
                !preference.pinned,
                preference.order_index,
                preference.contribution_id.as_str(),
            )
        })
        .unwrap_or((false, MAX_ORDER_INDEX, ""))
}

fn contribution_ids(contributions: &PluginContributions) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::{apply, list, restore, set};
    use aevio_contracts::plugin::{
        NavigationContribution, PluginContributionPreference, PluginContributions,
        PluginInstallationScope,
    };
    use aevio_persistence::bootstrap::migrations::run_migrations;
    use aevio_persistence::database::connect;
    use rusqlite::Connection;
    use tempfile::tempdir;

    #[test]
    fn hidden_navigation_is_removed_and_pinned_navigation_sorts_first() {
        let mut contributions = PluginContributions {
            navigation: vec![
                NavigationContribution {
                    id: "dev.test/second".into(),
                    label: "Second".into(),
                    page: "second".into(),
                    icon: None,
                },
                NavigationContribution {
                    id: "dev.test/first".into(),
                    label: "First".into(),
                    page: "first".into(),
                    icon: None,
                },
                NavigationContribution {
                    id: "dev.test/hidden".into(),
                    label: "Hidden".into(),
                    page: "hidden".into(),
                    icon: None,
                },
            ],
            ..Default::default()
        };
        let preferences = vec![
            PluginContributionPreference {
                plugin_id: "dev.test".into(),
                contribution_id: "dev.test/second".into(),
                visible: true,
                pinned: false,
                order_index: 0,
            },
            PluginContributionPreference {
                plugin_id: "dev.test".into(),
                contribution_id: "dev.test/first".into(),
                visible: true,
                pinned: true,
                order_index: 9,
            },
            PluginContributionPreference {
                plugin_id: "dev.test".into(),
                contribution_id: "dev.test/hidden".into(),
                visible: false,
                pinned: false,
                order_index: 1,
            },
        ];
        apply(&mut contributions, &preferences);
        assert_eq!(
            contributions
                .navigation
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["dev.test/first", "dev.test/second"]
        );
    }

    #[test]
    fn layout_preferences_round_trip_through_active_contributions() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("activity.db");
        let mut connection = Connection::open(&path).unwrap();
        run_migrations(&mut connection).unwrap();
        drop(connection);
        let database = connect(&path).unwrap();
        let plugin_id = "dev.aevio.project-planning";
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

        let navigation_id = format!("{plugin_id}/main");
        set(
            &database,
            PluginContributionPreference {
                plugin_id: plugin_id.into(),
                contribution_id: navigation_id.clone(),
                visible: false,
                pinned: true,
                order_index: 2,
            },
            4,
        )
        .unwrap();
        assert!(list(&database).unwrap().iter().any(|preference| {
            preference.contribution_id == navigation_id && !preference.visible && !preference.pinned
        }));
        assert!(
            !super::super::plugins::active_contributions(&database)
                .unwrap()
                .into_iter()
                .find(|snapshot| snapshot.plugin_id == plugin_id)
                .unwrap()
                .contributions
                .navigation
                .iter()
                .any(|navigation| navigation.id == navigation_id)
        );
        restore(&database).unwrap();
        assert!(
            super::super::plugins::active_contributions(&database)
                .unwrap()
                .into_iter()
                .find(|snapshot| snapshot.plugin_id == plugin_id)
                .unwrap()
                .contributions
                .navigation
                .iter()
                .any(|navigation| navigation.id == navigation_id)
        );
    }
}
