use aevio_contracts::plugin::*;

fn manifest() -> PluginManifest {
    PluginManifest {
        manifest_version: MANIFEST_VERSION,
        id: "dev.aevio.example".into(),
        name: "Example".into(),
        version: "1.0.0".into(),
        publisher: PublisherIdentity {
            id: "aevio".into(),
            display_name: "Aevio".into(),
            verified: true,
            public_key: None,
        },
        description: "Example plugin".into(),
        category: "productivity".into(),
        compatibility: PluginCompatibility {
            aevio: ">=0.9.0, <2.0.0".into(),
            plugin_api: "^1.0.0".into(),
        },
        platforms: vec![PluginPlatform::Desktop],
        entrypoints: PluginEntrypoints::default(),
        icons: Vec::new(),
        screenshots: Vec::new(),
        permissions: vec![PluginPermissionRequest {
            id: capabilities::STORAGE_OWN_READ.into(),
            scope: None,
            reason: None,
        }],
        contributes: PluginContributions {
            navigation: vec![NavigationContribution {
                id: "dev.aevio.example/main".into(),
                label: "Example".into(),
                page: "dev.aevio.example/page".into(),
                icon: None,
            }],
            ..Default::default()
        },
        dependencies: Vec::new(),
        optional_dependencies: Vec::new(),
        event_subscriptions: Vec::new(),
        offline: OfflineSupport {
            activation: true,
            network_required: false,
        },
        data_schema: None,
    }
}

#[test]
fn validates_a_namespaced_manifest() {
    assert_eq!(manifest().validate(), Ok(()));
}

#[test]
fn rejects_unsupported_permissions_and_traversal() {
    let mut invalid = manifest();
    invalid.permissions[0].id = "database:all".into();
    assert!(invalid.validate().is_err());
    invalid = manifest();
    invalid.entrypoints.ui = Some("../index.html".into());
    assert!(invalid.validate().is_err());
}

#[test]
fn rejects_duplicate_contributions() {
    let mut invalid = manifest();
    invalid.contributes.pages = vec![
        PageContribution {
            id: "dev.aevio.example/main".into(),
            title: "One".into(),
        },
        PageContribution {
            id: "dev.aevio.example/main".into(),
            title: "Two".into(),
        },
    ];
    assert!(invalid.validate().is_err());
}

#[test]
fn rejects_unknown_manifest_fields_during_deserialization() {
    let mut value = serde_json::to_value(manifest()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("isAdmin".into(), serde_json::Value::Bool(true));
    assert!(serde_json::from_value::<PluginManifest>(value).is_err());
}

#[test]
fn validates_assets_dependencies_and_event_capabilities() {
    let mut valid = manifest();
    valid.icons = vec![PluginAsset {
        path: "assets/icon.png".into(),
        mime_type: Some("image/png".into()),
    }];
    valid.dependencies = vec![PluginDependency {
        plugin_id: "dev.aevio.other".into(),
        version: "^1.0.0".into(),
    }];
    valid.event_subscriptions = vec![EventSubscription {
        id: "dev.aevio.example/tasks-completed".into(),
        event_type: Some("tasks.completed.v1".into()),
        workflow_id: None,
        required_capabilities: vec![capabilities::EVENTS_SUBSCRIBE.into()],
    }];
    assert!(valid.validate().is_ok());
    valid.icons[0].path = "../escape.png".into();
    assert!(valid.validate().is_err());
}
