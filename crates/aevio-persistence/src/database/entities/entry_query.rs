pub mod entries {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "time_entries")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub started_at: i64,
        pub ended_at: i64,
        pub description: String,
        pub category_id: Option<String>,
        pub project_id: Option<String>,
        pub task_id: Option<String>,
        pub status: String,
        pub approved_by: Option<String>,
        pub source: String,
        pub billable: i64,
        pub invoice_id: Option<String>,
        pub created_at: i64,
        pub updated_at: i64,
        pub deleted_at: Option<i64>,
        pub description_origin: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod projects {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "projects")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub client_id: Option<String>,
        pub name: String,
        pub deleted_at: Option<i64>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod segments {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "segments")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub entry_id: Option<String>,
        pub app: String,
        pub title: String,
        pub kind: String,
        pub label: Option<String>,
        pub reviewed: i64,
        pub app_id: Option<String>,
        pub bundle_id: Option<String>,
        pub url: Option<String>,
        pub domain: Option<String>,
        pub started_at: i64,
        pub ended_at: Option<i64>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod events {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "entry_events")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub entry_id: String,
        pub kind: String,
        pub actor: String,
        pub payload: Option<String>,
        pub at: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
