use sea_orm::entity::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue::Set, DatabaseConnection, DbErr};

use crate::database::entities::settings;

pub struct SettingsRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> SettingsRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn value(&self, key: &str) -> Result<Option<String>, DbErr> {
        settings::Entity::find_by_id(key.to_owned())
            .one(self.db)
            .map(|row| row.map(|row| row.value))
    }

    pub fn set(&self, key: &str, value: &str) -> Result<(), DbErr> {
        settings::Entity::insert(settings::ActiveModel {
            key: Set(key.to_owned()),
            value: Set(value.to_owned()),
        })
        .on_conflict(
            OnConflict::column(settings::Column::Key)
                .update_column(settings::Column::Value)
                .to_owned(),
        )
        .exec(self.db)
        .map(|_| ())
    }
}
