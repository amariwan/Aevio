use sea_orm::entity::prelude::*;
use sea_orm::{
    ActiveValue::{NotSet, Set},
    Condition, DatabaseConnection, DbErr, QueryFilter, QueryOrder,
};

use crate::database::entities::ai::apps;
use aevio_core::models::AppRecord;

pub struct AppRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> AppRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn list(&self) -> Result<Vec<AppRecord>, DbErr> {
        apps::Entity::find()
            .filter(apps::Column::DeletedAt.is_null())
            .order_by_desc(apps::Column::LastSeen)
            .all(self.db)
            .map(|rows| rows.into_iter().map(to_domain).collect())
    }

    pub fn update(
        &self,
        id: &str,
        default_category_id: Option<String>,
        default_project_id: Option<String>,
        excluded: Option<bool>,
        now: u64,
    ) -> Result<AppRecord, DbErr> {
        let row = apps::Entity::find_by_id(id.to_owned())
            .one(self.db)?
            .ok_or_else(|| DbErr::RecordNotFound("app".into()))?;
        let mut model: apps::ActiveModel = row.into();
        if let Some(category_id) = default_category_id {
            model.default_category_id = Set(Some(category_id));
        }
        if let Some(project_id) = default_project_id {
            model.default_project_id = Set(Some(project_id));
        }
        if let Some(excluded) = excluded {
            model.excluded = Set(excluded as i64);
        }
        model.updated_at = Set(now as i64);
        model.update(self.db)?;
        apps::Entity::find_by_id(id.to_owned())
            .one(self.db)?
            .map(to_domain)
            .ok_or_else(|| DbErr::RecordNotFound("app".into()))
    }

    pub fn is_excluded(
        &self,
        identifier: &str,
        bundle_id: Option<&str>,
        display_name: &str,
    ) -> Result<bool, DbErr> {
        let alternate = bundle_id.unwrap_or(identifier);
        Ok(apps::Entity::find()
            .filter(apps::Column::DeletedAt.is_null())
            .filter(apps::Column::Excluded.ne(0))
            .filter(
                Condition::any()
                    .add(apps::Column::Identifier.eq(identifier.to_owned()))
                    .add(apps::Column::Identifier.eq(alternate.to_owned()))
                    .add(apps::Column::Identifier.eq(display_name.to_owned()))
                    .add(apps::Column::DisplayName.eq(display_name.to_owned())),
            )
            .one(self.db)?
            .is_some())
    }

    pub fn observe(
        &self,
        kind: &str,
        identifier: &str,
        display_name: &str,
        now: u64,
    ) -> Result<(), DbErr> {
        if let Some(row) = apps::Entity::find()
            .filter(apps::Column::Identifier.eq(identifier.to_owned()))
            .one(self.db)?
        {
            let mut model: apps::ActiveModel = row.into();
            model.last_seen = Set(now as i64);
            model.updated_at = Set(now as i64);
            model.update(self.db)?;
        } else {
            apps::ActiveModel {
                id: Set(uuid::Uuid::now_v7().to_string()),
                kind: Set(kind.to_owned()),
                identifier: Set(identifier.to_owned()),
                display_name: Set(display_name.to_owned()),
                default_category_id: NotSet,
                default_project_id: NotSet,
                excluded: Set(0),
                first_seen: Set(now as i64),
                last_seen: Set(now as i64),
                created_at: Set(now as i64),
                updated_at: Set(now as i64),
                deleted_at: NotSet,
            }
            .insert(self.db)?;
        }
        Ok(())
    }
}

fn to_domain(row: apps::Model) -> AppRecord {
    AppRecord {
        id: row.id,
        kind: row.kind,
        identifier: row.identifier,
        display_name: row.display_name,
        default_category_id: row.default_category_id,
        default_project_id: row.default_project_id,
        excluded: row.excluded != 0,
        first_seen: row.first_seen.max(0) as u64,
        last_seen: row.last_seen.max(0) as u64,
        created_at: row.created_at.max(0) as u64,
        updated_at: row.updated_at.max(0) as u64,
        deleted_at: row.deleted_at.map(|value| value.max(0) as u64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::migrations::run_migrations;
    use crate::database::connect;
    use rusqlite::Connection;

    #[test]
    fn lists_and_updates_existing_app_rows() {
        let path = std::env::temp_dir().join(format!("aevio-apps-{}.db", uuid::Uuid::now_v7()));
        let mut sqlite = Connection::open(&path).unwrap();
        run_migrations(&mut sqlite).unwrap();
        sqlite.execute_batch("INSERT INTO apps (id,kind,identifier,display_name,excluded,first_seen,last_seen,created_at,updated_at) VALUES ('a','desktop','com.example','Example',0,1,2,1,2);").unwrap();
        drop(sqlite);
        let database = connect(&path).unwrap();
        let repository = AppRepository::new(&database);
        assert_eq!(repository.list().unwrap()[0].id, "a");
        let updated = repository
            .update("a", Some("cat".into()), None, Some(true), 3)
            .unwrap();
        assert_eq!(updated.default_category_id.as_deref(), Some("cat"));
        assert!(
            repository
                .is_excluded("com.example", None, "Example")
                .unwrap()
        );
        drop(database);
        std::fs::remove_file(path).unwrap();
    }
}
