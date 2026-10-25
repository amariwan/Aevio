use sea_orm::entity::prelude::*;
use sea_orm::{DatabaseConnection, DbErr, QueryOrder, QuerySelect};

use crate::database::entities::{project_discovery, project_rules, settings};
use aevio_core::entries::models::ActivitySegment;

pub struct ProjectDiscoveryRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> ProjectDiscoveryRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn project_names(&self) -> Result<Vec<String>, DbErr> {
        project_rules::projects::Entity::find()
            .filter(project_rules::projects::Column::DeletedAt.is_null())
            .select_only()
            .column(project_rules::projects::Column::Name)
            .into_tuple::<String>()
            .all(self.db)
    }

    pub fn rule_patterns(&self) -> Result<Vec<String>, DbErr> {
        project_rules::rules::Entity::find()
            .filter(project_rules::rules::Column::DeletedAt.is_null())
            .filter(project_rules::rules::Column::ProjectId.is_not_null())
            .select_only()
            .column(project_rules::rules::Column::Pattern)
            .into_tuple::<String>()
            .all(self.db)
    }

    pub fn dismissed_value(&self, key: &str) -> Result<Option<String>, DbErr> {
        settings::Entity::find_by_id(key.to_owned())
            .one(self.db)
            .map(|row| row.map(|row| row.value))
    }

    pub fn segments_since(&self, since_ms: i64) -> Result<Vec<ActivitySegment>, DbErr> {
        project_discovery::Entity::find()
            .filter(project_discovery::Column::StartedAt.gte(since_ms))
            .filter(project_discovery::Column::Kind.ne("break"))
            .order_by_asc(project_discovery::Column::StartedAt)
            .all(self.db)
            .map(|rows| {
                rows.into_iter()
                    .map(|row| ActivitySegment {
                        id: row.id,
                        app: row.app,
                        title: row.title,
                        kind: row.kind,
                        label: row.label,
                        started_at: row.started_at.max(0) as u64,
                        ended_at: row.ended_at.map(|value| value.max(0) as u64),
                        reviewed: row.reviewed != 0,
                        app_id: row.app_id,
                        bundle_id: row.bundle_id,
                        url: row.url,
                        domain: row.domain,
                        entry_id: row.entry_id,
                    })
                    .collect()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::sea_orm::{ConnectionTrait, Database, DbBackend, Schema};
    use sea_orm::ActiveValue::Set;

    #[test]
    fn segments_preserve_order_and_exclude_breaks() {
        let db = Database::connect("sqlite::memory:").unwrap();
        let schema = Schema::new(DbBackend::Sqlite);
        db.execute(&schema.create_table_from_entity(project_discovery::Entity))
            .unwrap();
        for (id, kind, started_at) in [
            (1_i64, "capture", 20_i64),
            (2, "break", 10),
            (3, "capture", 30),
        ] {
            project_discovery::Entity::insert(project_discovery::ActiveModel {
                id: Set(id),
                app: Set("Code".into()),
                title: Set("Aevio".into()),
                kind: Set(kind.into()),
                label: Set(None),
                started_at: Set(started_at),
                ended_at: Set(Some(started_at + 1)),
                reviewed: Set(0),
                app_id: Set(None),
                bundle_id: Set(None),
                url: Set(None),
                domain: Set(None),
                entry_id: Set(None),
            })
            .exec(&db)
            .unwrap();
        }
        let rows = ProjectDiscoveryRepository::new(&db)
            .segments_since(15)
            .unwrap();
        assert_eq!(
            rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            vec![1, 3]
        );
    }
}
