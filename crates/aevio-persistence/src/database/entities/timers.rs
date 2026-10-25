use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "timers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub label: String,
    pub accumulated_ms: i64,
    pub started_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::timer_segments::Entity")]
    Segments,
}

impl Related<super::timer_segments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Segments.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
