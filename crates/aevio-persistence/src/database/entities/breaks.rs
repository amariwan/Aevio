use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "breaks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub source: String,
    pub schedule_id: Option<String>,
    pub due_at: Option<i64>,
    pub planned_ms: i64,
    pub status: String,
    pub snoozes: i64,
    pub started_at: Option<i64>,
    pub ended_at: Option<i64>,
    pub segment_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

pub mod segments {
    use sea_orm::entity::prelude::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "segments")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub kind: String,
        pub label: Option<String>,
        pub started_at: i64,
        pub ended_at: Option<i64>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
