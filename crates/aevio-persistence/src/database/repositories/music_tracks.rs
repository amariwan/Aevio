use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};
use uuid::Uuid;

use crate::database::entities::music_tracks as track;

pub struct MusicTrackRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> MusicTrackRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn list(&self) -> Result<Vec<track::Model>, sea_orm::DbErr> {
        track::Entity::find()
            .order_by_asc(track::Column::SortOrder)
            .all(self.db)
    }

    pub fn seed_missing(
        &self,
        seeds: &[(&str, &str, &str, &str, i32)],
        created_at: i64,
    ) -> Result<(), sea_orm::DbErr> {
        for (id, title, description, kind, sort_order) in seeds {
            if track::Entity::find_by_id((*id).to_owned())
                .one(self.db)?
                .is_none()
            {
                track::ActiveModel {
                    id: Set((*id).to_owned()),
                    title: Set((*title).to_owned()),
                    description: Set((*description).to_owned()),
                    kind: Set((*kind).to_owned()),
                    sort_order: Set(*sort_order),
                    created_at: Set(created_at),
                    file_path: Set(None),
                }
                .insert(self.db)?;
            }
        }
        Ok(())
    }

    pub fn insert_local(
        &self,
        title: String,
        file_path: String,
        created_at: i64,
    ) -> Result<track::Model, sea_orm::DbErr> {
        let model = track::ActiveModel {
            id: Set(Uuid::now_v7().to_string()),
            title: Set(title),
            description: Set("Imported local audio".into()),
            kind: Set("local".into()),
            sort_order: Set(10_000),
            created_at: Set(created_at),
            file_path: Set(Some(file_path)),
        };
        model.insert(self.db)
    }

    pub fn by_id(&self, id: &str) -> Result<Option<track::Model>, sea_orm::DbErr> {
        track::Entity::find()
            .filter(track::Column::Id.eq(id.to_owned()))
            .one(self.db)
    }

    pub fn delete(&self, id: &str) -> Result<(), sea_orm::DbErr> {
        track::Entity::delete_by_id(id.to_owned())
            .exec(self.db)
            .map(|_| ())
    }
}
