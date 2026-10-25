use sea_orm::entity::prelude::*;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::database::entities::catalog::{categories, clients, projects};
use crate::database::entities::entry_query::entries;
use crate::database::entities::invoices::records as invoices;

pub struct CatalogRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> CatalogRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn categories(&self) -> Result<Vec<categories::Model>, DbErr> {
        categories::Entity::find()
            .filter(categories::Column::DeletedAt.is_null())
            .order_by_asc(categories::Column::Sort)
            .order_by_asc(categories::Column::Name)
            .all(self.db)
    }

    pub fn category(&self, id: &str) -> Result<Option<categories::Model>, DbErr> {
        categories::Entity::find_by_id(id.to_owned()).one(self.db)
    }

    pub fn create_category(
        &self,
        model: categories::ActiveModel,
    ) -> Result<categories::Model, DbErr> {
        let id = match model.id.clone() {
            Set(id) => id,
            _ => return Err(DbErr::Custom("category id is required".into())),
        };
        categories::Entity::insert(model).exec(self.db)?;
        categories::Entity::find_by_id(id)
            .one(self.db)
            .map(|row| row.expect("inserted category must be readable"))
    }

    pub fn update_category(
        &self,
        model: categories::ActiveModel,
    ) -> Result<categories::Model, DbErr> {
        let id = match model.id.clone() {
            Set(id) => id,
            _ => return Err(DbErr::Custom("category id is required".into())),
        };
        model.update(self.db)?;
        categories::Entity::find_by_id(id)
            .one(self.db)?
            .ok_or_else(|| DbErr::RecordNotFound("category".into()))
    }

    pub fn projects(&self) -> Result<Vec<projects::Model>, DbErr> {
        projects::Entity::find()
            .filter(projects::Column::DeletedAt.is_null())
            .order_by_asc(projects::Column::Name)
            .all(self.db)
    }

    pub fn project(&self, id: &str) -> Result<Option<projects::Model>, DbErr> {
        projects::Entity::find_by_id(id.to_owned()).one(self.db)
    }

    pub fn create_project(&self, model: projects::ActiveModel) -> Result<projects::Model, DbErr> {
        let id = match model.id.clone() {
            Set(id) => id,
            _ => return Err(DbErr::Custom("project id is required".into())),
        };
        projects::Entity::insert(model).exec(self.db)?;
        projects::Entity::find_by_id(id)
            .one(self.db)
            .map(|row| row.expect("inserted project must be readable"))
    }

    pub fn update_project(&self, model: projects::ActiveModel) -> Result<projects::Model, DbErr> {
        let id = match model.id.clone() {
            Set(id) => id,
            _ => return Err(DbErr::Custom("project id is required".into())),
        };
        model.update(self.db)?;
        projects::Entity::find_by_id(id)
            .one(self.db)?
            .ok_or_else(|| DbErr::RecordNotFound("project".into()))
    }

    pub fn project_has_invoiced_time(&self, project_id: &str) -> Result<bool, DbErr> {
        entries::Entity::find()
            .filter(entries::Column::ProjectId.eq(project_id.to_owned()))
            .filter(entries::Column::DeletedAt.is_null())
            .filter(entries::Column::InvoiceId.is_not_null())
            .one(self.db)
            .map(|row| row.is_some())
    }

    pub fn clients(&self) -> Result<Vec<clients::Model>, DbErr> {
        clients::Entity::find()
            .filter(clients::Column::DeletedAt.is_null())
            .order_by_asc(clients::Column::Name)
            .all(self.db)
    }

    pub fn client(&self, id: &str) -> Result<Option<clients::Model>, DbErr> {
        clients::Entity::find_by_id(id.to_owned()).one(self.db)
    }

    pub fn create_client(&self, model: clients::ActiveModel) -> Result<clients::Model, DbErr> {
        let id = match model.id.clone() {
            Set(id) => id,
            _ => return Err(DbErr::Custom("client id is required".into())),
        };
        clients::Entity::insert(model).exec(self.db)?;
        clients::Entity::find_by_id(id)
            .one(self.db)
            .map(|row| row.expect("inserted client must be readable"))
    }

    pub fn update_client(&self, model: clients::ActiveModel) -> Result<clients::Model, DbErr> {
        let id = match model.id.clone() {
            Set(id) => id,
            _ => return Err(DbErr::Custom("client id is required".into())),
        };
        model.update(self.db)?;
        clients::Entity::find_by_id(id)
            .one(self.db)?
            .ok_or_else(|| DbErr::RecordNotFound("client".into()))
    }

    pub fn delete_client(&self, id: &str, now: i64) -> Result<String, DbErr> {
        self.db
            .transaction(|txn| {
                let client = clients::Entity::find_by_id(id.to_owned())
                    .one(txn)?
                    .ok_or_else(|| DbErr::RecordNotFound("client".into()))?;
                if invoices::Entity::find()
                    .filter(invoices::Column::ClientId.eq(id.to_owned()))
                    .one(txn)?
                    .is_some()
                {
                    return Err(DbErr::Custom(format!(
                        "{} has invoices, so it can't be deleted. Archive it instead.",
                        client.name
                    )));
                }
                let project_rows = projects::Entity::find()
                    .filter(projects::Column::ClientId.eq(id.to_owned()))
                    .all(txn)?;
                for project in project_rows {
                    let mut model: projects::ActiveModel = project.into();
                    model.client_id = Set(None);
                    model.updated_at = Set(now);
                    model.update(txn)?;
                }
                clients::Entity::update_many()
                    .col_expr(clients::Column::DeletedAt, Expr::value(now))
                    .col_expr(clients::Column::UpdatedAt, Expr::value(now))
                    .filter(clients::Column::Id.eq(id.to_owned()))
                    .exec(txn)?;
                Ok(client.name)
            })
            .map_err(|error| DbErr::Custom(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::migrations::run_migrations;
    use crate::database::connect;
    use rusqlite::Connection;

    #[test]
    fn reads_catalog_models_from_an_existing_database() {
        let path = std::env::temp_dir().join(format!("aevio-catalog-{}.db", uuid::Uuid::now_v7()));
        let mut sqlite = Connection::open(&path).unwrap();
        run_migrations(&mut sqlite).unwrap();
        sqlite
            .execute_batch(
                "INSERT INTO categories (id, name, color, sort, created_at, updated_at) VALUES ('cat', 'Work', 'blue', 1, 1, 1);
                 INSERT INTO clients (id, name, currency, created_at, updated_at) VALUES ('cli', 'Client', 'USD', 1, 1);
                 INSERT INTO projects (id, client_id, name, color, status, created_at, updated_at) VALUES ('pro', 'cli', 'Project', 'blue', 'active', 1, 1);",
            )
            .unwrap();
        drop(sqlite);

        let database = connect(&path).unwrap();
        let repository = CatalogRepository::new(&database);
        assert!(
            repository
                .categories()
                .unwrap()
                .iter()
                .any(|row| row.id == "cat")
        );
        assert!(
            repository
                .clients()
                .unwrap()
                .iter()
                .any(|row| row.id == "cli")
        );
        assert!(
            repository
                .projects()
                .unwrap()
                .iter()
                .any(|row| row.id == "pro")
        );
        drop(database);
        std::fs::remove_file(path).unwrap();
    }
}
