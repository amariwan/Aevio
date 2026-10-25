use crate::database::entities::ai::apps;
use crate::database::entities::project_rules::{projects, rules};
use sea_orm::entity::prelude::*;
use sea_orm::{ActiveValue::Set, DatabaseConnection, DbErr, EntityTrait, QueryFilter};

pub struct ProjectRulesRepository<'db> {
    db: &'db DatabaseConnection,
}
impl<'db> ProjectRulesRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }
    pub fn attach_manual_rule(&self, key: &str, project_id: &str, now: i64) -> Result<(), DbErr> {
        let project = projects::Entity::find_by_id(project_id.to_owned())
            .filter(projects::Column::DeletedAt.is_null())
            .one(self.db)?;
        if project.is_none() {
            return Err(DbErr::RecordNotFound("project not found".into()));
        }
        rules::ActiveModel {
            id: Set(uuid::Uuid::now_v7().to_string()),
            match_kind: Set("name".into()),
            pattern: Set(key.into()),
            category_id: Set(None),
            project_id: Set(Some(project_id.into())),
            priority: Set(0),
            enabled: Set(1),
            origin: Set("manual".into()),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
        }
        .insert(self.db)
        .map(|_| ())
    }

    pub fn archive_hint_rules(&self, project_id: &str, now: i64) -> Result<(), DbErr> {
        rules::Entity::update_many()
            .col_expr(rules::Column::DeletedAt, Expr::value(now))
            .col_expr(rules::Column::UpdatedAt, Expr::value(now))
            .filter(rules::Column::ProjectId.eq(project_id.to_owned()))
            .filter(rules::Column::Origin.eq("hint"))
            .filter(rules::Column::DeletedAt.is_null())
            .exec(self.db)
            .map(|_| ())
    }

    pub fn create_hint_rule(
        &self,
        match_kind: &str,
        pattern: &str,
        project_id: &str,
        now: i64,
    ) -> Result<(), DbErr> {
        rules::ActiveModel {
            id: Set(uuid::Uuid::now_v7().to_string()),
            match_kind: Set(match_kind.to_owned()),
            pattern: Set(pattern.to_owned()),
            category_id: Set(None),
            project_id: Set(Some(project_id.to_owned())),
            priority: Set(0),
            enabled: Set(1),
            origin: Set("hint".to_owned()),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
        }
        .insert(self.db)
        .map(|_| ())
    }

    pub fn list_for_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<(String, String, String, String)>, DbErr> {
        let mut result = rules::Entity::find()
            .filter(rules::Column::ProjectId.eq(project_id.to_owned()))
            .filter(rules::Column::DeletedAt.is_null())
            .filter(rules::Column::Enabled.eq(1))
            .all(self.db)?
            .into_iter()
            .map(|row| (row.id, row.match_kind, row.pattern, row.origin))
            .collect::<Vec<_>>();
        result.extend(
            apps::Entity::find()
                .filter(apps::Column::DefaultProjectId.eq(project_id.to_owned()))
                .filter(apps::Column::DeletedAt.is_null())
                .all(self.db)?
                .into_iter()
                .map(|row| {
                    (
                        format!("app:{}", row.id),
                        if row.kind == "site" { "domain" } else { "app" }.to_owned(),
                        row.display_name,
                        "app".to_owned(),
                    )
                }),
        );
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::sea_orm::{ConnectionTrait, Database, DbBackend, Schema};
    use sea_orm::ActiveValue::Set;

    #[test]
    fn attaches_rule_only_to_a_live_project() {
        let db = Database::connect("sqlite::memory:").unwrap();
        let schema = Schema::new(DbBackend::Sqlite);
        db.execute(&schema.create_table_from_entity(projects::Entity))
            .unwrap();
        db.execute(&schema.create_table_from_entity(rules::Entity))
            .unwrap();
        projects::Entity::insert(projects::ActiveModel {
            id: Set("project".into()),
            name: Set("Project".into()),
            deleted_at: Set(None),
        })
        .exec(&db)
        .unwrap();
        ProjectRulesRepository::new(&db)
            .attach_manual_rule("invoice", "project", 1)
            .unwrap();
        assert_eq!(rules::Entity::find().all(&db).unwrap().len(), 1);
        projects::Entity::update_many()
            .col_expr(projects::Column::DeletedAt, Expr::value(2_i64))
            .exec(&db)
            .unwrap();
        assert!(
            ProjectRulesRepository::new(&db)
                .attach_manual_rule("other", "project", 3)
                .is_err()
        );
    }
}
