macro_rules! plugin_entity {
    ($module:ident, $table:literal, { $( $field:ident : $ty:ty ),* $(,)? }) => {
        pub mod $module {
            use sea_orm::entity::prelude::*;
            #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
            #[sea_orm(table_name = $table)]
            pub struct Model {
                #[sea_orm(primary_key, auto_increment = false)]
                pub id: String,
                $(pub $field: $ty,)*
            }
            #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
            pub enum Relation {}
            impl ActiveModelBehavior for ActiveModel {}
        }
    };
}

plugin_entity!(installations, "plugin_installations", {
    plugin_id: String,
    publisher_id: String,
    version: String,
    scope: String,
    state: String,
    manifest_json: String,
    granted_permissions_json: String,
    artifact_digest: String,
    artifact_path: Option<String>,
    revision: i64,
    last_error: Option<String>,
    created_at: i64,
    updated_at: i64,
    last_activated_at: Option<i64>
});

plugin_entity!(operations, "plugin_operations", {
    installation_id: String,
    kind: String,
    state: String,
    revision: i64,
    error: Option<String>,
    created_at: i64,
    updated_at: i64
});

plugin_entity!(grants, "plugin_grants", {
    installation_id: String,
    capability: String,
    scope: Option<String>,
    expires_at: Option<i64>,
    revoked_at: Option<i64>,
    created_at: i64
});

plugin_entity!(audit_events, "plugin_audit_events", {
    installation_id: Option<String>,
    event_type: String,
    actor: String,
    payload_json: String,
    created_at: i64
});

plugin_entity!(storage_entries, "plugin_storage_entries", {
    installation_id: String,
    user_ref: String,
    workspace_ref: String,
    device_ref: String,
    key: String,
    value_json: String,
    schema_version: i64,
    size_bytes: i64,
    updated_at: i64
});

plugin_entity!(installation_history, "plugin_installation_history", {
    plugin_id: String,
    publisher_id: String,
    version: String,
    scope: String,
    state: String,
    manifest_json: String,
    artifact_digest: String,
    artifact_path: Option<String>,
    recorded_at: i64,
    reason: String
});

plugin_entity!(jobs, "plugin_jobs", {
    installation_id: String,
    job_id: String,
    interval_ms: i64,
    next_run_at: i64,
    last_run_at: Option<i64>,
    run_count: i64,
    enabled: i64,
    payload_json: String,
    attempt_count: i64,
    max_attempts: i64,
    lease_until: Option<i64>,
    last_error: Option<String>,
    state: String,
    created_at: i64,
    updated_at: i64
});

plugin_entity!(workflow_runs, "plugin_workflow_runs", {
    installation_id: String,
    workflow_id: String,
    trigger: String,
    idempotency_key: String,
    status: String,
    attempt_count: i64,
    max_attempts: i64,
    input_json: String,
    output_json: Option<String>,
    last_error: Option<String>,
    created_at: i64,
    updated_at: i64,
    completed_at: Option<i64>
});

plugin_entity!(events, "plugin_events", {
    publisher_installation_id: String,
    publisher_plugin_id: String,
    event_type: String,
    payload_json: String,
    occurred_at: i64
});

plugin_entity!(event_deliveries, "plugin_event_deliveries", {
    event_id: String,
    subscriber_installation_id: String,
    workflow_id: String,
    status: String,
    attempt_count: i64,
    max_attempts: i64,
    lease_until: Option<i64>,
    last_error: Option<String>,
    created_at: i64,
    updated_at: i64,
    completed_at: Option<i64>
});

plugin_entity!(update_policies, "plugin_update_policies", {
    channel: String,
    pinned_version: Option<String>,
    automatic_updates: i64,
    updated_at: i64
});

plugin_entity!(layout_preferences, "plugin_layout_preferences", {
    installation_id: String,
    contribution_id: String,
    visible: i64,
    pinned: i64,
    order_index: i64,
    updated_at: i64
});

pub mod runtime_health {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "plugin_runtime_health")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub installation_id: String,
        pub state: String,
        pub last_success_at: Option<i64>,
        pub last_failure_at: Option<i64>,
        pub crash_count: i64,
        pub permission_denials: i64,
        pub resource_violations: i64,
        pub last_error: Option<String>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
