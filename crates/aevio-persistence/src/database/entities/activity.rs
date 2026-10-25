pub mod segments {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "segments")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub app: String,
        pub title: String,
        pub kind: String,
        pub label: Option<String>,
        pub started_at: i64,
        pub ended_at: Option<i64>,
        pub reviewed: i64,
        pub app_id: Option<String>,
        pub bundle_id: Option<String>,
        pub url: Option<String>,
        pub domain: Option<String>,
        pub entry_id: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod daily_rollups {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "daily_rollups")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub day_epoch: i64,
        #[sea_orm(primary_key)]
        pub kind: String,
        pub total_ms: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
