use sea_orm::entity::prelude::*;
use sea_orm::sea_query::Condition;
use sea_orm::{ActiveValue::Set, DatabaseConnection, DbErr, QueryOrder, QuerySelect};

use crate::database::entities::work_intelligence::{
    projects, tasks, time_entries, work_relationships,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Neighbor {
    pub from_type: String,
    pub from_id: String,
    pub relation: String,
    pub to_type: String,
    pub to_id: String,
}

pub struct NewRelationship<'a> {
    pub from_type: &'a str,
    pub from_id: &'a str,
    pub relation: &'a str,
    pub to_type: &'a str,
    pub to_id: &'a str,
    pub provenance: &'a str,
    pub created_at: i64,
}

pub struct WorkIntelligenceRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> WorkIntelligenceRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn relationships(&self) -> Result<Vec<work_relationships::Model>, DbErr> {
        work_relationships::Entity::find()
            .filter(work_relationships::Column::DeletedAt.is_null())
            .order_by_desc(work_relationships::Column::CreatedAt)
            .all(self.db)
    }

    pub fn pending_entries(&self) -> Result<u32, DbErr> {
        time_entries::Entity::find()
            .filter(time_entries::Column::DeletedAt.is_null())
            .filter(time_entries::Column::Status.eq("pending"))
            .count(self.db)
            .map(|count| count as u32)
    }

    pub fn entity_exists(&self, kind: &str, id: &str) -> Result<bool, DbErr> {
        match kind {
            "project" => projects::Entity::find_by_id(id.to_owned())
                .filter(projects::Column::DeletedAt.is_null())
                .one(self.db)
                .map(|row| row.is_some()),
            "task" => tasks::Entity::find_by_id(id.to_owned())
                .filter(tasks::Column::DeletedAt.is_null())
                .one(self.db)
                .map(|row| row.is_some()),
            "time_entry" => time_entries::Entity::find_by_id(id.to_owned())
                .filter(time_entries::Column::DeletedAt.is_null())
                .one(self.db)
                .map(|row| row.is_some()),
            _ => Ok(false),
        }
    }

    pub fn add_relationship(
        &self,
        input: NewRelationship<'_>,
    ) -> Result<work_relationships::Model, DbErr> {
        if let Some(existing) = work_relationships::Entity::find()
            .filter(work_relationships::Column::FromType.eq(input.from_type.to_owned()))
            .filter(work_relationships::Column::FromId.eq(input.from_id.to_owned()))
            .filter(work_relationships::Column::Relation.eq(input.relation.to_owned()))
            .filter(work_relationships::Column::ToType.eq(input.to_type.to_owned()))
            .filter(work_relationships::Column::ToId.eq(input.to_id.to_owned()))
            .filter(work_relationships::Column::DeletedAt.is_null())
            .one(self.db)?
        {
            return Ok(existing);
        }
        let model = work_relationships::ActiveModel {
            id: Set(uuid::Uuid::now_v7().to_string()),
            from_type: Set(input.from_type.to_owned()),
            from_id: Set(input.from_id.to_owned()),
            relation: Set(input.relation.to_owned()),
            to_type: Set(input.to_type.to_owned()),
            to_id: Set(input.to_id.to_owned()),
            provenance: Set(input.provenance.to_owned()),
            created_at: Set(input.created_at),
            deleted_at: Set(None),
        };
        model.insert(self.db)
    }

    pub fn label(&self, kind: &str, id: &str) -> Result<String, DbErr> {
        let label = match kind {
            "project" => projects::Entity::find_by_id(id.to_owned())
                .one(self.db)?
                .map(|row| row.name),
            "task" => tasks::Entity::find_by_id(id.to_owned())
                .one(self.db)?
                .map(|row| row.title),
            "time_entry" => time_entries::Entity::find_by_id(id.to_owned())
                .one(self.db)?
                .map(|row| row.description),
            _ => None,
        };
        Ok(label.unwrap_or_else(|| id.to_owned()))
    }

    pub fn neighbors(&self, kind: &str, id: &str) -> Result<Vec<Neighbor>, DbErr> {
        work_relationships::Entity::find()
            .filter(work_relationships::Column::DeletedAt.is_null())
            .filter(
                Condition::any()
                    .add(
                        Condition::all()
                            .add(work_relationships::Column::FromType.eq(kind.to_owned()))
                            .add(work_relationships::Column::FromId.eq(id.to_owned())),
                    )
                    .add(
                        Condition::all()
                            .add(work_relationships::Column::ToType.eq(kind.to_owned()))
                            .add(work_relationships::Column::ToId.eq(id.to_owned())),
                    ),
            )
            .all(self.db)
            .map(|rows| {
                rows.into_iter()
                    .map(|row| Neighbor {
                        from_type: row.from_type,
                        from_id: row.from_id,
                        relation: row.relation,
                        to_type: row.to_type,
                        to_id: row.to_id,
                    })
                    .collect()
            })
    }

    pub fn project_children(&self, id: &str) -> Result<Vec<(String, String)>, DbErr> {
        let mut children = Vec::new();
        children.extend(
            tasks::Entity::find()
                .filter(tasks::Column::ProjectId.eq(id.to_owned()))
                .filter(tasks::Column::DeletedAt.is_null())
                .select_only()
                .column(tasks::Column::Id)
                .into_tuple::<String>()
                .all(self.db)?
                .into_iter()
                .map(|id| ("task".to_owned(), id)),
        );
        children.extend(
            time_entries::Entity::find()
                .filter(time_entries::Column::ProjectId.eq(id.to_owned()))
                .filter(time_entries::Column::DeletedAt.is_null())
                .select_only()
                .column(time_entries::Column::Id)
                .into_tuple::<String>()
                .all(self.db)?
                .into_iter()
                .map(|id| ("time_entry".to_owned(), id)),
        );
        Ok(children)
    }

    pub fn parent(&self, kind: &str, id: &str) -> Result<Option<(String, String)>, DbErr> {
        match kind {
            "task" => tasks::Entity::find_by_id(id.to_owned())
                .filter(tasks::Column::DeletedAt.is_null())
                .one(self.db)
                .map(|row| row.map(|row| ("project".to_owned(), row.project_id))),
            "time_entry" => time_entries::Entity::find_by_id(id.to_owned())
                .filter(time_entries::Column::DeletedAt.is_null())
                .one(self.db)
                .map(|row| row.and_then(|row| row.project_id.map(|id| ("project".to_owned(), id)))),
            _ => Ok(None),
        }
    }
}
