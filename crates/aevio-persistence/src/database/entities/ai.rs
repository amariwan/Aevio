pub mod classify_jobs {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "classify_jobs")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub entry_id: String,
        pub kind: String,
        pub state: String,
        pub attempts: i64,
        pub next_at: i64,
        pub last_error: Option<String>,
        pub created_at: i64,
        pub updated_at: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod suggestions {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "suggestions")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub entry_id: String,
        pub field: String,
        pub value_id: Option<String>,
        pub confidence: f64,
        pub signals: Option<String>,
        pub rationale: Option<String>,
        pub alternatives: Option<String>,
        pub engine: String,
        pub model_version: Option<String>,
        pub outcome: Option<String>,
        pub created_at: i64,
        pub updated_at: i64,
        pub raw_confidence: Option<f64>,
        pub tier: Option<String>,
        pub deleted_at: Option<i64>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod time_entries {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "time_entries")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub source: String,
        pub description: String,
        pub category_id: Option<String>,
        pub project_id: Option<String>,
        pub status: String,
        pub ended_at: i64,
        pub started_at: i64,
        pub deleted_at: Option<i64>,
        pub updated_at: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod embeddings {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "entry_embeddings")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub entry_id: String,
        pub vec: Vec<u8>,
        pub text_hash: String,
        pub features: String,
        pub model: String,
        pub created_at: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod rules {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "rules")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub match_kind: String,
        pub pattern: String,
        pub category_id: Option<String>,
        pub project_id: Option<String>,
        pub priority: i64,
        pub enabled: i64,
        pub origin: String,
        pub created_at: i64,
        pub updated_at: i64,
        pub deleted_at: Option<i64>,
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
        pub status: String,
        pub deleted_at: Option<i64>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod apps {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "apps")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub kind: String,
        pub identifier: String,
        pub display_name: String,
        pub default_category_id: Option<String>,
        pub default_project_id: Option<String>,
        pub excluded: i64,
        pub first_seen: i64,
        pub last_seen: i64,
        pub created_at: i64,
        pub updated_at: i64,
        pub deleted_at: Option<i64>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod model_artifacts {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "model_artifacts")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub kind: String,
        pub path: String,
        pub trained_at: i64,
        pub n_examples: i32,
        pub holdout_acc: Option<f64>,
        pub calibration: Option<String>,
        pub active: i32,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
