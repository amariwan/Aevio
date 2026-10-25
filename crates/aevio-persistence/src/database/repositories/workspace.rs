use sea_orm::entity::prelude::*;
use sea_orm::{ActiveValue::Set, DatabaseConnection, DbErr, QueryOrder, TransactionTrait};

use crate::database::entities::workspace::{
    attachments, comments, cycles, documents, initiative_projects, initiatives, mentions, notes,
};

pub struct WorkspaceRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> WorkspaceRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn list_documents(&self) -> Result<Vec<documents::Model>, DbErr> {
        documents::Entity::find()
            .order_by_desc(documents::Column::UpdatedAt)
            .all(self.db)
    }
    pub fn document(&self, id: &str) -> Result<Option<documents::Model>, DbErr> {
        documents::Entity::find_by_id(id.to_owned()).one(self.db)
    }
    pub fn insert_document(&self, model: documents::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }
    pub fn update_document(&self, model: documents::ActiveModel) -> Result<(), DbErr> {
        model.update(self.db).map(|_| ())
    }
    pub fn delete_document(&self, id: &str) -> Result<(), DbErr> {
        documents::Entity::delete_by_id(id.to_owned())
            .exec(self.db)
            .map(|_| ())
    }

    pub fn list_notes(&self) -> Result<Vec<notes::Model>, DbErr> {
        notes::Entity::find()
            .order_by_desc(notes::Column::UpdatedAt)
            .all(self.db)
    }
    pub fn note(&self, id: &str) -> Result<Option<notes::Model>, DbErr> {
        notes::Entity::find_by_id(id.to_owned()).one(self.db)
    }
    pub fn insert_note(&self, model: notes::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }
    pub fn update_note(&self, model: notes::ActiveModel) -> Result<(), DbErr> {
        model.update(self.db).map(|_| ())
    }
    pub fn delete_note(&self, id: &str) -> Result<(), DbErr> {
        notes::Entity::delete_by_id(id.to_owned())
            .exec(self.db)
            .map(|_| ())
    }

    pub fn list_comments(
        &self,
        target_type: &str,
        target_id: &str,
    ) -> Result<Vec<comments::Model>, DbErr> {
        comments::Entity::find()
            .filter(comments::Column::TargetType.eq(target_type.to_owned()))
            .filter(comments::Column::TargetId.eq(target_id.to_owned()))
            .order_by_asc(comments::Column::CreatedAt)
            .all(self.db)
    }
    pub fn comment(&self, id: &str) -> Result<Option<comments::Model>, DbErr> {
        comments::Entity::find_by_id(id.to_owned()).one(self.db)
    }
    pub fn insert_comment(&self, model: comments::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn list_attachments(
        &self,
        target_type: &str,
        target_id: &str,
    ) -> Result<Vec<attachments::Model>, DbErr> {
        attachments::Entity::find()
            .filter(attachments::Column::TargetType.eq(target_type.to_owned()))
            .filter(attachments::Column::TargetId.eq(target_id.to_owned()))
            .order_by_desc(attachments::Column::CreatedAt)
            .all(self.db)
    }
    pub fn insert_attachment(&self, model: attachments::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }
    pub fn attachment(&self, id: &str) -> Result<Option<attachments::Model>, DbErr> {
        attachments::Entity::find_by_id(id.to_owned()).one(self.db)
    }

    pub fn list_mentions(
        &self,
        target_type: &str,
        target_id: &str,
    ) -> Result<Vec<mentions::Model>, DbErr> {
        mentions::Entity::find()
            .filter(mentions::Column::TargetType.eq(target_type.to_owned()))
            .filter(mentions::Column::TargetId.eq(target_id.to_owned()))
            .order_by_asc(mentions::Column::CreatedAt)
            .all(self.db)
    }
    pub fn replace_mentions(
        &self,
        target_type: &str,
        target_id: &str,
        rows: Vec<mentions::ActiveModel>,
    ) -> Result<(), DbErr> {
        let tx = self.db.begin()?;
        mentions::Entity::delete_many()
            .filter(mentions::Column::TargetType.eq(target_type.to_owned()))
            .filter(mentions::Column::TargetId.eq(target_id.to_owned()))
            .exec(&tx)?;
        for row in rows {
            row.insert(&tx)?;
        }
        tx.commit()
    }

    pub fn list_cycles(&self) -> Result<Vec<cycles::Model>, DbErr> {
        cycles::Entity::find()
            .order_by_desc(cycles::Column::StartsAt)
            .all(self.db)
    }
    pub fn cycle(&self, id: &str) -> Result<Option<cycles::Model>, DbErr> {
        cycles::Entity::find_by_id(id.to_owned()).one(self.db)
    }
    pub fn insert_cycle(&self, model: cycles::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn list_initiatives(&self) -> Result<Vec<(initiatives::Model, Vec<String>)>, DbErr> {
        let rows = initiatives::Entity::find()
            .order_by_desc(initiatives::Column::UpdatedAt)
            .all(self.db)?;
        rows.into_iter()
            .map(|initiative| {
                let ids = initiative_projects::Entity::find()
                    .filter(initiative_projects::Column::InitiativeId.eq(initiative.id.clone()))
                    .order_by_asc(initiative_projects::Column::ProjectId)
                    .all(self.db)?
                    .into_iter()
                    .map(|row| row.project_id)
                    .collect();
                Ok((initiative, ids))
            })
            .collect()
    }
    pub fn insert_initiative(
        &self,
        id: &str,
        initiative: initiatives::ActiveModel,
        projects: &[String],
    ) -> Result<(), DbErr> {
        let tx = self.db.begin()?;
        initiative.insert(&tx)?;
        for project_id in projects {
            initiative_projects::ActiveModel {
                initiative_id: Set(id.to_owned()),
                project_id: Set(project_id.clone()),
            }
            .insert(&tx)?;
        }
        tx.commit()
    }
}
