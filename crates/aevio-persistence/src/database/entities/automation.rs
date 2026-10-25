macro_rules! entity {
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

entity!(agent_routines, "agent_routines", {
    name: String, prompt: String, scope: String, cadence: String,
    send_at: Option<String>, timezone: String, delivery_json: String,
    active: i64, next_run_at: Option<i64>, created_at: i64, updated_at: i64
});
entity!(routine_runs, "routine_runs", {
    routine_id: String, status: String, period_start: Option<i64>,
    period_end: Option<i64>, output_json: String, delivery_json: String,
    created_at: i64
});
entity!(report_definitions, "report_definitions", {
    name: String, scope: String, format: String, filter_json: String,
    prompt: Option<String>, dashboard_id: Option<String>, cadence: String,
    send_at: Option<String>, timezone: String, delivery_json: String,
    active: i64, next_run_at: Option<i64>, created_at: i64, updated_at: i64
});
entity!(report_runs, "report_runs", {
    report_id: String, status: String, period_start: i64, period_end: i64,
    output_json: String, delivery_json: String, created_at: i64
});
entity!(saved_dashboards, "saved_dashboards", {
    name: String, filter_json: String, pinned: i64, created_at: i64,
    updated_at: i64
});

// Read-only projection used by the daily workflow. The table has additional
// columns owned by the activity module; this entity deliberately selects only
// the columns needed by the automation boundary.
pub mod time_entries {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "time_entries")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub started_at: i64,
        pub ended_at: i64,
        pub status: String,
        pub billable: i64,
        pub invoice_id: Option<String>,
        pub deleted_at: Option<i64>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod entry_events {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "entry_events")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: i64,
        pub kind: String,
        pub actor: String,
        pub at: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
