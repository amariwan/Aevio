pub mod tasks {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "coach_tasks")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub title: String,
        pub notes: Option<String>,
        pub priority: i64,
        pub status: String,
        pub due_at: Option<i64>,
        pub reminder_at: Option<i64>,
        pub estimate_ms: Option<i64>,
        pub created_at: i64,
        pub updated_at: i64,
        pub completed_at: Option<i64>,
        pub notified_at: Option<i64>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod preferences {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "coach_preferences")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub value: String,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod interventions {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "coach_interventions")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub day: String,
        pub kind: String,
        pub at: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod memory {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "coach_memory")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub content: String,
        pub kind: String,
        pub updated_at: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod task_time_links {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "task_time_links")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub entry_id: String,
        pub daily_task_id: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod daily_tasks {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "daily_tasks")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub day: String,
        pub personal_task_id: Option<String>,
        pub project_task_id: Option<String>,
        pub estimate_minutes: i64,
        pub important: i64,
        pub position: i64,
        pub postponements: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod task_steps {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "task_steps")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub daily_task_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub position: i64,
        pub title: String,
        pub done: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod daily_plans {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "daily_plans")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub day: String,
        pub time_zone: String,
        pub goal: String,
        pub notes: String,
        pub available_minutes: i64,
        pub energy: Option<i64>,
        pub state: String,
        pub revision: i64,
        pub created_at: i64,
        pub updated_at: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod daily_reviews {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "daily_reviews")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub day: String,
        pub outcome: String,
        pub worked: String,
        pub obstacles: String,
        pub tomorrow: String,
        pub revision: i64,
        pub updated_at: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod focus {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "coach_focus")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub daily_task_id: String,
        pub segment_id: i64,
        pub goal: String,
        pub next_step: String,
        pub planned_minutes: i64,
        pub body_doubling: i64,
        pub state: String,
        pub started_at: i64,
        pub ended_at: Option<i64>,
        pub reminded_at: Option<i64>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod brain_entries {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "brain_dump_entries")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub content: String,
        pub category: String,
        pub converted_task_id: Option<String>,
        pub updated_at: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod routines {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "coach_routines")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub title: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod routine_items {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "coach_routine_items")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub routine_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub position: i32,
        pub title: String,
        pub estimate_minutes: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod routine_runs {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "coach_routine_runs")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub routine_id: String,
        #[sea_orm(primary_key, auto_increment = false)]
        pub day: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod focus_interruptions {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "focus_interruptions")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub focus_id: String,
        pub at: i64,
        pub reason: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
