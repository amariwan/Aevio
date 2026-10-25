pub mod accounts {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "accounts")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub email: String,
        pub password_hash: String,
        pub status: String,
        pub created_at: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod tokens {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "access_tokens")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub token_hash: String,
        pub account_id: String,
        pub created_at: i64,
        pub revoked_at: Option<i64>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod workspaces {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "workspaces")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub name: String,
        pub created_at: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod memberships {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "workspace_memberships")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub workspace_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub account_id: String,
        pub role: String,
        pub created_at: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod operations {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "sync_operations")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub operation_id: String,
        pub workspace_id: String,
        pub device_id: String,
        pub entity_type: String,
        pub entity_id: String,
        pub operation: String,
        pub payload: String,
        pub base_revision: Option<i64>,
        pub revision: i64,
        pub created_at: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod plugin_registry {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "plugin_registry")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub plugin_id: String,
        pub publisher_id: String,
        pub manifest_json: String,
        pub created_at: i64,
        pub updated_at: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod plugin_registry_releases {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "plugin_registry_releases")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub plugin_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub version: String,
        pub artifact_digest: String,
        pub artifact_size: i64,
        pub signature: String,
        pub channel: String,
        pub published_at: i64,
        pub revoked: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod plugin_publisher_keys {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "plugin_publisher_keys")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub publisher_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub key_id: String,
        pub public_key: String,
        pub created_at: i64,
        pub expires_at: Option<i64>,
        pub revoked_at: Option<i64>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod plugin_workspace_desired_state {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "plugin_workspace_desired_state")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub workspace_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub plugin_id: String,
        pub requested_version: Option<String>,
        pub channel: String,
        pub enabled: i64,
        pub requested_permissions_json: String,
        pub revision: i64,
        pub updated_at: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
