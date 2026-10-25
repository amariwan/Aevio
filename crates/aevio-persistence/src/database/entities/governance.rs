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

entity!(workspaces, "workspaces", {
    name: String, currency: String, timezone: String, defaults_json: String,
    created_at: i64, updated_at: i64
});
entity!(teams, "teams", {
    workspace_id: String, name: String, currency: String, timezone: String,
    rate_strategy: String, auto_approve: i64, created_at: i64, updated_at: i64
});
entity!(departments, "departments", {
    workspace_id: String, name: String, created_at: i64, updated_at: i64
});
entity!(members, "members", {
    workspace_id: String, email: String, display_name: String, status: String,
    org_role: String, job_title: Option<String>, manager_id: Option<String>,
    default_team_id: Option<String>, department_id: Option<String>,
    bill_rate_cents: Option<i64>, cost_rate_cents: Option<i64>, billable_default: i64,
    effective_defaults_json: String, created_at: i64, updated_at: i64
});

pub mod team_members {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "team_members")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub team_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub member_id: String,
        pub team_role: String,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod member_permissions {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "member_permissions")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub member_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub permission: String,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod audit {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "governance_audit")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub workspace_id: String,
        pub actor_id: String,
        pub action: String,
        pub entity_type: String,
        pub entity_id: String,
        pub before_json: Option<String>,
        pub after_json: Option<String>,
        pub at: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
