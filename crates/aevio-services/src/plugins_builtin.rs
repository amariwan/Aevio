use aevio_contracts::plugin::{
    CommandContribution, JobContribution, NavigationContribution, OfflineSupport, PageContribution,
    PluginCatalogEntry, PluginCompatibility, PluginContributions, PluginEntrypoints,
    PluginManifest, PluginPermissionRequest, PluginPlatform, PluginRelease, PublisherIdentity,
    SearchContribution, capabilities,
};

pub(crate) fn builtin_catalog() -> Vec<PluginCatalogEntry> {
    [
        "dev.aevio.project-planning",
        "dev.aevio.music",
        "dev.aevio.notes",
        "dev.aevio.crm",
    ]
    .into_iter()
    .filter_map(|id| {
        builtin_manifest(id).map(|manifest| PluginCatalogEntry {
            releases: vec![release(&manifest)],
            manifest,
            installed: None,
        })
    })
    .collect()
}

pub(crate) fn builtin_manifest(id: &str) -> Option<PluginManifest> {
    let (name, category, description, page, permissions) = match id {
        "dev.aevio.project-planning" => (
            "Project Planning",
            "productivity",
            "Projects, tasks, kanban and milestones backed by Aevio's existing domain.",
            "projects",
            vec![
                capabilities::PROJECTS_READ,
                capabilities::PROJECTS_WRITE,
                capabilities::TASKS_READ,
                capabilities::TASKS_WRITE,
                capabilities::COMMANDS_REGISTER,
                capabilities::SEARCH_CONTRIBUTE,
                capabilities::JOBS_SCHEDULE,
            ],
        ),
        "dev.aevio.music" => (
            "Music",
            "focus",
            "Offline focus music using the existing local catalog and mediated playback controls.",
            "timers",
            vec![
                capabilities::FILES_READ_SELECTED,
                capabilities::COMMANDS_REGISTER,
                capabilities::SEARCH_CONTRIBUTE,
            ],
        ),
        "dev.aevio.notes" => (
            "Notes",
            "productivity",
            "Notes and documents using Aevio's existing workspace content services.",
            "notes",
            vec![
                capabilities::STORAGE_OWN_READ,
                capabilities::STORAGE_OWN_WRITE,
                capabilities::COMMANDS_REGISTER,
            ],
        ),
        "dev.aevio.crm" => (
            "CRM",
            "business",
            "Clients and contacts backed by the existing Aevio client domain.",
            "clients",
            vec![
                capabilities::CLIENTS_READ,
                capabilities::CLIENTS_WRITE,
                capabilities::COMMANDS_REGISTER,
            ],
        ),
        _ => return None,
    };
    Some(PluginManifest {
        manifest_version: aevio_contracts::plugin::MANIFEST_VERSION,
        id: id.into(),
        name: name.into(),
        version: "1.0.0".into(),
        publisher: PublisherIdentity {
            id: "aevio.first-party".into(),
            display_name: "Aevio".into(),
            verified: true,
            public_key: None,
        },
        description: description.into(),
        category: category.into(),
        compatibility: PluginCompatibility {
            aevio: ">=0.9.39, <1.0.0".into(),
            plugin_api: "^1.0.0".into(),
        },
        platforms: vec![PluginPlatform::Desktop],
        entrypoints: PluginEntrypoints::default(),
        icons: Vec::new(),
        screenshots: Vec::new(),
        permissions: permissions
            .into_iter()
            .map(|id| PluginPermissionRequest {
                id: id.into(),
                scope: Some("workspace".into()),
                reason: Some("Required by the declared first-party contribution".into()),
            })
            .collect(),
        dependencies: Vec::new(),
        optional_dependencies: Vec::new(),
        contributes: PluginContributions {
            navigation: {
                let mut entries = vec![NavigationContribution {
                    id: format!("{id}/main"),
                    label: name.into(),
                    page: format!("{id}/{page}"),
                    icon: None,
                }];
                if id == "dev.aevio.project-planning" {
                    entries.push(NavigationContribution {
                        id: format!("{id}/tasks"),
                        label: "Tasks".into(),
                        page: format!("{id}/tasks"),
                        icon: None,
                    });
                }
                entries
            },
            pages: vec![PageContribution {
                id: format!("{id}/{page}"),
                title: name.into(),
            }],
            commands: vec![CommandContribution {
                id: format!("{id}/open"),
                title: format!("Open {name}"),
                description: None,
                required_capabilities: Vec::new(),
            }],
            search: match id {
                "dev.aevio.project-planning" => vec![SearchContribution {
                    id: format!("{id}/project-search"),
                    label: "Projects".into(),
                    scopes: vec!["projects".into()],
                }],
                "dev.aevio.music" => vec![SearchContribution {
                    id: format!("{id}/track-search"),
                    label: "Music".into(),
                    scopes: vec!["music".into()],
                }],
                _ => Vec::new(),
            },
            jobs: if id == "dev.aevio.project-planning" {
                vec![JobContribution {
                    id: format!("{id}/refresh"),
                    title: "Refresh project suggestions".into(),
                    min_interval_ms: 15 * 60 * 1_000,
                }]
            } else {
                Vec::new()
            },
            ..Default::default()
        },
        event_subscriptions: Vec::new(),
        offline: OfflineSupport {
            activation: true,
            network_required: false,
        },
        data_schema: None,
    })
}

fn release(manifest: &PluginManifest) -> PluginRelease {
    PluginRelease {
        version: manifest.version.clone(),
        artifact_digest: format!("builtin:{}@{}", manifest.id, manifest.version),
        artifact_size: 0,
        signature: None,
        channel: "stable".into(),
        published_at: 0,
        revoked: false,
    }
}
