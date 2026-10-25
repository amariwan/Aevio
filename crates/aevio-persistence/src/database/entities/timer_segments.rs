use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "timer_segments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub timer_id: String,
    pub duration_ms: i64,
    pub recorded_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::timers::Entity",
        from = "Column::TimerId",
        to = "super::timers::Column::Id"
    )]
    Timer,
}

impl Related<super::timers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Timer.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
