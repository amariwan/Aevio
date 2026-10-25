use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, ExprTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};

use crate::database::entities::ai::time_entries;

pub struct AgentContextRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> AgentContextRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn pending_context(&self) -> Result<(u32, Vec<String>), DbErr> {
        let pending = time_entries::Entity::find()
            .filter(time_entries::Column::Status.eq("pending"))
            .count(self.db)? as u32;
        let preview = time_entries::Entity::find()
            .filter(time_entries::Column::Status.eq("pending"))
            .filter(time_entries::Column::DeletedAt.is_null())
            .order_by_desc(time_entries::Column::StartedAt)
            .limit(8)
            .select_only()
            .column(time_entries::Column::Description)
            .into_tuple::<String>()
            .all(self.db)?
            .into_iter()
            .map(|description| {
                if description.is_empty() {
                    "(ohne Beschreibung)".to_string()
                } else {
                    description
                }
            })
            .collect();
        Ok((pending, preview))
    }

    pub fn matching_entry_ids(&self, reference: &str) -> Result<Vec<String>, DbErr> {
        time_entries::Entity::find()
            .filter(time_entries::Column::DeletedAt.is_null())
            .filter(
                time_entries::Column::Id
                    .eq(reference.to_owned())
                    .or(time_entries::Column::Id.like(format!("%{reference}"))),
            )
            .limit(6)
            .select_only()
            .column(time_entries::Column::Id)
            .into_tuple::<String>()
            .all(self.db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::entities::ai::time_entries;
    use crate::database::sea_orm::{ConnectionTrait, Database, DbBackend, Schema};
    use sea_orm::ActiveValue::Set;

    #[test]
    fn preserves_pending_count_and_preview_null_semantics() {
        let db = Database::connect("sqlite::memory:").unwrap();
        let schema = Schema::new(DbBackend::Sqlite);
        db.execute(&schema.create_table_from_entity(time_entries::Entity))
            .unwrap();
        for (id, description, deleted_at, started_at) in [
            ("pending", "", None, 2_i64),
            ("deleted", "hidden", Some(3_i64), 3),
        ] {
            time_entries::Entity::insert(time_entries::ActiveModel {
                id: Set(id.into()),
                source: Set("manual".into()),
                description: Set(description.into()),
                category_id: Set(None),
                project_id: Set(None),
                status: Set("pending".into()),
                ended_at: Set(4),
                started_at: Set(started_at),
                deleted_at: Set(deleted_at),
                updated_at: Set(4),
            })
            .exec(&db)
            .unwrap();
        }
        let context = AgentContextRepository::new(&db).pending_context().unwrap();
        assert_eq!(context.0, 2);
        assert_eq!(context.1, vec!["(ohne Beschreibung)".to_string()]);
    }
}
