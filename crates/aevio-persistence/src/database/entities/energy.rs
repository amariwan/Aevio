use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "energy_samples")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub sampled_at: i64,
    pub duration_ms: i64,
    pub cpu_time_ms: i64,
    pub energy_nj: i64,
    pub power_watts: f64,
    pub impact_level: String,
    pub ai_active: i64,
    pub on_battery: i64,
    pub battery_level: Option<f64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
