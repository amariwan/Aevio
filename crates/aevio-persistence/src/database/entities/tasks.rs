#[allow(clippy::module_inception)]
pub mod tasks {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "tasks")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub project_id: String,
        pub title: String,
        pub description: Option<String>,
        pub status: String,
        pub priority: String,
        pub due_date: Option<i64>,
        pub keywords_json: String,
        pub ai_instructions: Option<String>,
        pub created_at: i64,
        pub updated_at: i64,
        pub completed_at: Option<i64>,
        pub deleted_at: Option<i64>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::task_labels::Entity")]
        TaskLabels,
    }

    impl Related<super::task_labels::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::TaskLabels.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod labels {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "labels")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub name: String,
        pub color: String,
        pub created_at: i64,
        pub updated_at: i64,
        pub deleted_at: Option<i64>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::task_labels::Entity")]
        TaskLabels,
    }

    impl Related<super::task_labels::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::TaskLabels.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod task_labels {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "task_labels")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub task_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub label_id: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::tasks::Entity",
            from = "Column::TaskId",
            to = "super::tasks::Column::Id"
        )]
        Task,
        #[sea_orm(
            belongs_to = "super::labels::Entity",
            from = "Column::LabelId",
            to = "super::labels::Column::Id"
        )]
        Label,
    }

    impl Related<super::tasks::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Task.def()
        }
    }

    impl Related<super::labels::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Label.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}
