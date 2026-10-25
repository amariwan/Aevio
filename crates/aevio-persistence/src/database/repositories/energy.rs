use sea_orm::entity::prelude::*;
use sea_orm::{ActiveValue::Set, DatabaseConnection, DbErr, QueryFilter, QueryOrder, QuerySelect};

use crate::database::entities::energy;

pub struct EnergyRepository<'db> {
    db: &'db DatabaseConnection,
}

pub struct NewSample {
    pub sampled_at: i64,
    pub duration_ms: i64,
    pub cpu_time_ms: i64,
    pub energy_nj: i64,
    pub power_watts: f64,
    pub impact_level: String,
    pub ai_active: bool,
    pub on_battery: bool,
    pub battery_level: Option<f64>,
}

impl<'db> EnergyRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn insert(&self, row: energy::ActiveModel) -> Result<(), DbErr> {
        row.insert(self.db).map(|_| ())
    }

    pub fn since(&self, sampled_at: i64) -> Result<Vec<energy::Model>, DbErr> {
        energy::Entity::find()
            .filter(energy::Column::SampledAt.gte(sampled_at))
            .order_by_asc(energy::Column::SampledAt)
            .all(self.db)
    }

    pub fn history(&self, sampled_at: i64, limit: u64) -> Result<Vec<energy::Model>, DbErr> {
        energy::Entity::find()
            .filter(energy::Column::SampledAt.gte(sampled_at))
            .order_by_desc(energy::Column::SampledAt)
            .limit(limit)
            .all(self.db)
    }

    pub fn purge_before(&self, sampled_at: i64) -> Result<u64, DbErr> {
        energy::Entity::delete_many()
            .filter(energy::Column::SampledAt.lt(sampled_at))
            .exec(self.db)
            .map(|result| result.rows_affected)
    }

    pub fn reset(&self) -> Result<(), DbErr> {
        energy::Entity::delete_many().exec(self.db).map(|_| ())
    }
}

pub fn sample(input: NewSample) -> energy::ActiveModel {
    energy::ActiveModel {
        sampled_at: Set(input.sampled_at),
        duration_ms: Set(input.duration_ms),
        cpu_time_ms: Set(input.cpu_time_ms),
        energy_nj: Set(input.energy_nj),
        power_watts: Set(input.power_watts),
        impact_level: Set(input.impact_level),
        ai_active: Set(i64::from(input.ai_active)),
        on_battery: Set(i64::from(input.on_battery)),
        battery_level: Set(input.battery_level),
        ..Default::default()
    }
}
