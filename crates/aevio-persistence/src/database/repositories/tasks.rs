use sea_orm::entity::prelude::*;
use sea_orm::sea_query::Order;
use sea_orm::{
    ActiveValue::{NotSet, Set},
    DatabaseConnection, DatabaseTransaction, DbErr, EntityTrait, QueryFilter, QueryOrder,
};

use crate::database::entities::tasks::{labels, task_labels, tasks};
use aevio_core::entries::models::{Label, NewLabel, NewTask, Task, UpdateLabel, UpdateTask};

pub struct TaskRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> TaskRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn list(&self, project_id: Option<&str>) -> Result<Vec<Task>, DbErr> {
        let mut query = tasks::Entity::find().filter(tasks::Column::DeletedAt.is_null());
        if let Some(project_id) = project_id {
            query = query.filter(tasks::Column::ProjectId.eq(project_id.to_owned()));
        }
        let rows = query
            .order_by(
                Expr::cust("CASE status WHEN 'in_progress' THEN 0 WHEN 'todo' THEN 1 ELSE 2 END"),
                Order::Asc,
            )
            .order_by(Expr::cust("due_date IS NULL"), Order::Asc)
            .order_by_asc(tasks::Column::DueDate)
            .order_by(Expr::cust("title COLLATE NOCASE"), Order::Asc)
            .all(self.db)?;
        rows.into_iter().map(|row| self.to_domain(row)).collect()
    }

    pub fn create(&self, input: NewTask, now: u64) -> Result<Task, DbErr> {
        self.ensure_project_exists(&input.project_id)?;
        let id = uuid::Uuid::now_v7().to_string();
        let status = input.status.unwrap_or_else(|| "todo".to_owned());
        let keywords = normalize_keywords(input.keywords.unwrap_or_default());
        let task = tasks::ActiveModel {
            id: Set(id.clone()),
            project_id: Set(input.project_id),
            title: Set(input.title.trim().to_owned()),
            description: Set(input.description),
            status: Set(status.clone()),
            priority: Set(input.priority.unwrap_or_else(|| "normal".to_owned())),
            due_date: Set(input.due_date.map(|value| value as i64)),
            keywords_json: Set(serde_json::to_string(&keywords)
                .map_err(|error| DbErr::Custom(error.to_string()))?),
            ai_instructions: Set(input.ai_instructions),
            created_at: Set(now as i64),
            updated_at: Set(now as i64),
            completed_at: Set((status == "done").then_some(now as i64)),
            deleted_at: NotSet,
        };
        let label_ids = input.label_ids.unwrap_or_default();
        self.db
            .transaction(|txn| -> Result<(), DbErr> {
                tasks::Entity::insert(task).exec(txn)?;
                self.replace_labels(txn, &id, &label_ids)?;
                Ok(())
            })
            .map_err(|error| DbErr::Custom(error.to_string()))?;
        self.get(&id)
    }

    pub fn update(&self, id: &str, patch: UpdateTask, now: u64) -> Result<Task, DbErr> {
        let current = self.get(id)?;
        let project_id = patch.project_id.unwrap_or(current.project_id);
        self.ensure_project_exists(&project_id)?;
        let status = patch.status.unwrap_or(current.status);
        let model = tasks::ActiveModel {
            id: Set(id.to_owned()),
            project_id: Set(project_id),
            title: Set(patch
                .title
                .map(|value| value.trim().to_owned())
                .unwrap_or(current.title)),
            description: Set(patch.description.unwrap_or(current.description)),
            status: Set(status.clone()),
            priority: Set(patch.priority.unwrap_or(current.priority)),
            due_date: Set(patch
                .due_date
                .unwrap_or(current.due_date)
                .map(|value| value as i64)),
            keywords_json: Set(serde_json::to_string(&normalize_keywords(
                patch.keywords.unwrap_or(current.keywords),
            ))
            .map_err(|error| DbErr::Custom(error.to_string()))?),
            ai_instructions: Set(patch.ai_instructions.unwrap_or(current.ai_instructions)),
            created_at: NotSet,
            updated_at: Set(now as i64),
            completed_at: Set(
                (status == "done").then_some(current.completed_at.unwrap_or(now) as i64)
            ),
            deleted_at: NotSet,
        };
        self.db
            .transaction(|txn| -> Result<(), DbErr> {
                model.update(txn)?;
                if let Some(label_ids) = patch.label_ids {
                    self.replace_labels(txn, id, &label_ids)?;
                }
                Ok(())
            })
            .map_err(|error| DbErr::Custom(error.to_string()))?;
        self.get(id)
    }

    pub fn delete(&self, id: &str, now: u64) -> Result<(), DbErr> {
        tasks::Entity::update_many()
            .col_expr(tasks::Column::DeletedAt, Expr::value(now as i64))
            .col_expr(tasks::Column::UpdatedAt, Expr::value(now as i64))
            .filter(tasks::Column::Id.eq(id.to_owned()))
            .filter(tasks::Column::DeletedAt.is_null())
            .exec(self.db)
            .map(|_| ())
    }

    pub fn list_labels(&self) -> Result<Vec<Label>, DbErr> {
        labels::Entity::find()
            .filter(labels::Column::DeletedAt.is_null())
            .order_by(Expr::cust("name COLLATE NOCASE"), Order::Asc)
            .all(self.db)
            .map(|rows| rows.into_iter().map(label_from_model).collect())
    }

    pub fn create_label(&self, input: NewLabel, now: u64) -> Result<Label, DbErr> {
        let id = uuid::Uuid::now_v7().to_string();
        labels::Entity::insert(labels::ActiveModel {
            id: Set(id.clone()),
            name: Set(input.name.trim().to_owned()),
            color: Set(input.color),
            created_at: Set(now as i64),
            updated_at: Set(now as i64),
            deleted_at: NotSet,
        })
        .exec(self.db)?;
        self.get_label(&id)
    }

    pub fn update_label(&self, id: &str, patch: UpdateLabel, now: u64) -> Result<Label, DbErr> {
        let current = self.get_label(id)?;
        labels::ActiveModel {
            id: Set(id.to_owned()),
            name: Set(patch
                .name
                .map(|value| value.trim().to_owned())
                .unwrap_or(current.name)),
            color: Set(patch.color.unwrap_or(current.color)),
            created_at: NotSet,
            updated_at: Set(now as i64),
            deleted_at: NotSet,
        }
        .update(self.db)?;
        self.get_label(id)
    }

    pub fn delete_label(&self, id: &str, now: u64) -> Result<(), DbErr> {
        labels::Entity::update_many()
            .col_expr(labels::Column::DeletedAt, Expr::value(now as i64))
            .col_expr(labels::Column::UpdatedAt, Expr::value(now as i64))
            .filter(labels::Column::Id.eq(id.to_owned()))
            .exec(self.db)
            .map(|_| ())
    }

    fn get(&self, id: &str) -> Result<Task, DbErr> {
        tasks::Entity::find_by_id(id.to_owned())
            .one(self.db)?
            .ok_or_else(|| DbErr::RecordNotFound("task".into()))
            .and_then(|row| self.to_domain(row))
    }

    fn to_domain(&self, row: tasks::Model) -> Result<Task, DbErr> {
        let labels = task_labels::Entity::find()
            .filter(task_labels::Column::TaskId.eq(row.id.clone()))
            .find_also_related(labels::Entity)
            .all(self.db)?
            .into_iter()
            .filter_map(|(_, label)| label)
            .filter(|label| label.deleted_at.is_none())
            .map(label_from_model)
            .collect();
        Ok(Task {
            id: row.id,
            project_id: row.project_id,
            title: row.title,
            description: row.description,
            status: row.status,
            priority: row.priority,
            due_date: row.due_date.map(|value| value as u64),
            keywords: serde_json::from_str(&row.keywords_json).unwrap_or_default(),
            ai_instructions: row.ai_instructions,
            created_at: row.created_at as u64,
            updated_at: row.updated_at as u64,
            completed_at: row.completed_at.map(|value| value as u64),
            deleted_at: row.deleted_at.map(|value| value as u64),
            labels,
        })
    }

    fn get_label(&self, id: &str) -> Result<Label, DbErr> {
        labels::Entity::find_by_id(id.to_owned())
            .one(self.db)?
            .map(label_from_model)
            .ok_or_else(|| DbErr::RecordNotFound("label".into()))
    }

    fn ensure_project_exists(&self, id: &str) -> Result<(), DbErr> {
        let exists =
            crate::database::entities::catalog::projects::Entity::find_by_id(id.to_owned())
                .filter(crate::database::entities::catalog::projects::Column::DeletedAt.is_null())
                .one(self.db)?
                .is_some();
        exists.then_some(()).ok_or_else(|| {
            DbErr::Custom("The selected project does not exist or is archived.".into())
        })
    }

    fn replace_labels(
        &self,
        txn: &DatabaseTransaction,
        task_id: &str,
        label_ids: &[String],
    ) -> Result<(), DbErr> {
        task_labels::Entity::delete_many()
            .filter(task_labels::Column::TaskId.eq(task_id.to_owned()))
            .exec(txn)?;
        for label_id in label_ids {
            if labels::Entity::find_by_id(label_id.to_owned())
                .filter(labels::Column::DeletedAt.is_null())
                .one(txn)?
                .is_some()
            {
                task_labels::Entity::insert(task_labels::ActiveModel {
                    task_id: Set(task_id.to_owned()),
                    label_id: Set(label_id.clone()),
                })
                .exec(txn)?;
            }
        }
        Ok(())
    }
}

fn label_from_model(row: labels::Model) -> Label {
    Label {
        id: row.id,
        name: row.name,
        color: row.color,
        created_at: row.created_at as u64,
        updated_at: row.updated_at as u64,
        deleted_at: row.deleted_at.map(|value| value as u64),
    }
}

fn normalize_keywords(keywords: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for keyword in keywords {
        let value = keyword.trim().to_lowercase();
        if !value.is_empty() && !normalized.contains(&value) {
            normalized.push(value);
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::migrations::run_migrations;
    use crate::database::connect;
    use rusqlite::Connection;

    #[test]
    fn task_and_label_crud_preserves_nullable_fields_and_soft_delete() {
        let path = std::env::temp_dir().join(format!("aevio-tasks-{}.db", uuid::Uuid::now_v7()));
        let mut sqlite = Connection::open(&path).unwrap();
        run_migrations(&mut sqlite).unwrap();
        sqlite.execute_batch(
            "INSERT INTO projects (id, name, color, status, created_at, updated_at) VALUES ('project', 'Project', 'blue', 'active', 1, 1);",
        ).unwrap();
        drop(sqlite);

        let database = connect(&path).unwrap();
        let repository = TaskRepository::new(&database);
        let label = repository
            .create_label(
                NewLabel {
                    name: "Work".into(),
                    color: "blue".into(),
                },
                10,
            )
            .unwrap();
        let task = repository
            .create(
                NewTask {
                    project_id: "project".into(),
                    title: "Task".into(),
                    description: None,
                    status: None,
                    priority: None,
                    due_date: None,
                    keywords: Some(vec![" Rust ".into(), "rust".into()]),
                    ai_instructions: None,
                    label_ids: Some(vec![label.id.clone()]),
                },
                20,
            )
            .unwrap();
        assert_eq!(task.keywords, vec!["rust"]);
        assert_eq!(task.labels, vec![label]);

        let updated = repository
            .update(
                &task.id,
                UpdateTask {
                    description: Some(Some("Details".into())),
                    ..Default::default()
                },
                30,
            )
            .unwrap();
        assert_eq!(updated.description.as_deref(), Some("Details"));
        repository.delete(&task.id, 40).unwrap();
        assert!(repository.list(None).unwrap().is_empty());
        drop(database);
        std::fs::remove_file(path).unwrap();
    }
}
