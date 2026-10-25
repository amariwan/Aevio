use std::collections::{BTreeMap, BTreeSet};

use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use super::{ActivityStore, LOCKED, history::Command};
use crate::database::entities::ai::apps;
use crate::database::entities::catalog::{categories, projects};
use crate::database::entities::entry_query::{entries, segments};
use crate::database::entities::project_rules::rules;
use aevio_core::models::ActivitySegment;

const CONFLICT: &str =
    "The deleted items have changed since this operation; undo/redo cannot safely overwrite them.";
const APPROVED: &str = "This deletion affects approved entries. Unapprove those entries before deleting their category or project.";
const INVOICED: &str = "This deletion affects invoiced time. Remove its draft invoice first; finalized time cannot be deleted.";

#[derive(Clone, PartialEq)]
struct Links {
    category: Option<String>,
    project: Option<String>,
    deleted: Option<i64>,
}

struct EntryChange {
    id: String,
    before: Links,
    after: Links,
}

struct CategoryChange {
    id: String,
    archived: i64,
}

#[derive(Default)]
pub(super) struct TypedDeletion {
    at: i64,
    entries: Vec<EntryChange>,
    categories: Vec<CategoryChange>,
    projects: Vec<String>,
    rules: Vec<String>,
    apps: Vec<(String, Option<String>, Option<String>)>,
    segments: Vec<ActivitySegment>,
}

impl TypedDeletion {
    pub(super) fn capture<C: ConnectionTrait>(
        db: &C,
        ids: &[String],
        category: Option<&str>,
        project: Option<&str>,
        now: u64,
    ) -> Result<Self, String> {
        let mut result = Self {
            at: now as i64,
            ..Self::default()
        };
        let entry_ids = ids.iter().cloned().collect::<BTreeSet<_>>();
        let category_ids = category
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        let project_ids = project
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();

        for id in &entry_ids {
            let row = entries::Entity::find_by_id(id.clone())
                .one(db)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| CONFLICT.to_owned())?;
            if row.deleted_at.is_some() {
                continue;
            }
            if row.status == "approved" {
                return Err(LOCKED.to_owned());
            }
            let rows = segments::Entity::find()
                .filter(segments::Column::EntryId.eq(id.clone()))
                .all(db)
                .map_err(|e| e.to_string())?;
            for row in rows {
                result.segments.push(ActivitySegment {
                    id: row.id,
                    app: row.app,
                    title: row.title,
                    kind: row.kind,
                    label: row.label,
                    started_at: row.started_at.max(0) as u64,
                    ended_at: Some(row.ended_at.unwrap_or(now as i64).max(0) as u64),
                    reviewed: row.reviewed != 0,
                    app_id: row.app_id,
                    bundle_id: row.bundle_id,
                    url: row.url,
                    domain: row.domain,
                    entry_id: row.entry_id,
                });
            }
        }

        for id in &category_ids {
            if let Some(row) = categories::Entity::find_by_id(id.clone())
                .one(db)
                .map_err(|e| e.to_string())?
                && row.deleted_at.is_none()
            {
                result.categories.push(CategoryChange {
                    id: id.clone(),
                    archived: row.archived,
                });
            }
        }
        for id in &project_ids {
            if projects::Entity::find_by_id(id.clone())
                .one(db)
                .map_err(|e| e.to_string())?
                .is_some_and(|row| row.deleted_at.is_none())
            {
                result.projects.push(id.clone());
            }
        }

        let mut affected = BTreeMap::new();
        for row in entries::Entity::find()
            .filter(entries::Column::DeletedAt.is_null())
            .all(db)
            .map_err(|e| e.to_string())?
        {
            let affects_entry = entry_ids.contains(&row.id);
            let affects_category = row
                .category_id
                .as_ref()
                .is_some_and(|id| category_ids.contains(id));
            let affects_project = row
                .project_id
                .as_ref()
                .is_some_and(|id| project_ids.contains(id));
            if !(affects_entry || affects_category || affects_project) {
                continue;
            }
            if row.status == "approved" {
                return Err(APPROVED.into());
            }
            if row.invoice_id.is_some() {
                return Err(INVOICED.into());
            }
            affected.insert(
                row.id,
                Links {
                    category: row.category_id,
                    project: row.project_id,
                    deleted: row.deleted_at,
                },
            );
        }
        for (id, before) in affected {
            let mut after = before.clone();
            if entry_ids.contains(&id) {
                after.deleted = Some(result.at);
            }
            if before
                .category
                .as_ref()
                .is_some_and(|value| category_ids.contains(value))
            {
                after.category = None;
            }
            if before
                .project
                .as_ref()
                .is_some_and(|value| project_ids.contains(value))
            {
                after.project = None;
            }
            result.entries.push(EntryChange { id, before, after });
        }

        for row in rules::Entity::find()
            .filter(rules::Column::DeletedAt.is_null())
            .all(db)
            .map_err(|e| e.to_string())?
        {
            if row
                .category_id
                .as_ref()
                .is_some_and(|id| category_ids.contains(id))
                || row
                    .project_id
                    .as_ref()
                    .is_some_and(|id| project_ids.contains(id))
            {
                result.rules.push(row.id);
            }
        }
        for row in apps::Entity::find().all(db).map_err(|e| e.to_string())? {
            if row
                .default_category_id
                .as_ref()
                .is_some_and(|id| category_ids.contains(id))
                || row
                    .default_project_id
                    .as_ref()
                    .is_some_and(|id| project_ids.contains(id))
            {
                result
                    .apps
                    .push((row.id, row.default_category_id, row.default_project_id));
            }
        }
        Ok(result)
    }

    pub(super) fn apply<C: ConnectionTrait>(
        &self,
        db: &C,
        undo: bool,
        now: u64,
    ) -> Result<(), String> {
        if !undo {
            let active = entries::Entity::find()
                .filter(entries::Column::DeletedAt.is_null())
                .all(db)
                .map_err(|e| e.to_string())?;
            for row in active {
                let affected = row
                    .category_id
                    .as_ref()
                    .is_some_and(|id| self.categories.iter().any(|item| &item.id == id))
                    || row
                        .project_id
                        .as_ref()
                        .is_some_and(|id| self.projects.contains(id));
                if affected && row.status == "approved" {
                    return Err(LOCKED.to_owned());
                }
            }
        }
        for change in &self.entries {
            let row = entries::Entity::find_by_id(change.id.clone())
                .one(db)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| CONFLICT.to_owned())?;
            let (from, to) = if undo {
                (&change.after, &change.before)
            } else {
                (&change.before, &change.after)
            };
            if row.category_id != from.category
                || row.project_id != from.project
                || row.deleted_at != from.deleted
                || row.invoice_id.is_some()
            {
                return Err(CONFLICT.into());
            }
            if row.status == "approved" && from != to {
                return Err(LOCKED.into());
            }
            let mut model: entries::ActiveModel = row.into();
            model.category_id = Set(to.category.clone());
            model.project_id = Set(to.project.clone());
            model.deleted_at = Set(to.deleted);
            model.updated_at = Set(now as i64);
            model.update(db).map_err(|e| e.to_string())?;
        }
        for change in &self.categories {
            let row = categories::Entity::find_by_id(change.id.clone())
                .one(db)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| CONFLICT.to_owned())?;
            let from = if undo { Some(self.at) } else { None };
            let to = if undo { None } else { Some(self.at) };
            if row.deleted_at != from {
                return Err(CONFLICT.into());
            }
            let mut model: categories::ActiveModel = row.into();
            model.deleted_at = Set(to);
            model.archived = Set(if undo { change.archived } else { 1 });
            model.updated_at = Set(now as i64);
            model.update(db).map_err(|e| e.to_string())?;
        }
        for id in &self.projects {
            let row = projects::Entity::find_by_id(id.clone())
                .one(db)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| CONFLICT.to_owned())?;
            let from = if undo { Some(self.at) } else { None };
            let to = if undo { None } else { Some(self.at) };
            if row.deleted_at != from {
                return Err(CONFLICT.into());
            }
            let mut model: projects::ActiveModel = row.into();
            model.deleted_at = Set(to);
            model.updated_at = Set(now as i64);
            model.update(db).map_err(|e| e.to_string())?;
        }
        for id in &self.rules {
            let row = rules::Entity::find_by_id(id.clone())
                .one(db)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| CONFLICT.to_owned())?;
            let from = if undo { Some(self.at) } else { None };
            let to = if undo { None } else { Some(self.at) };
            if row.deleted_at != from {
                return Err(CONFLICT.into());
            }
            let mut model: rules::ActiveModel = row.into();
            model.deleted_at = Set(to);
            model.updated_at = Set(now as i64);
            model.update(db).map_err(|e| e.to_string())?;
        }
        for (id, category, project) in &self.apps {
            let row = apps::Entity::find_by_id(id.clone())
                .one(db)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| CONFLICT.to_owned())?;
            let after_category = category
                .as_ref()
                .filter(|value| !self.categories.iter().any(|item| &item.id == *value));
            let after_project = project
                .as_ref()
                .filter(|value| !self.projects.contains(value));
            let (from_category, from_project, to_category, to_project) = if undo {
                (
                    after_category,
                    after_project,
                    category.as_ref(),
                    project.as_ref(),
                )
            } else {
                (
                    category.as_ref(),
                    project.as_ref(),
                    after_category,
                    after_project,
                )
            };
            if row.default_category_id.as_ref() != from_category
                || row.default_project_id.as_ref() != from_project
            {
                return Err(CONFLICT.into());
            }
            let mut model: apps::ActiveModel = row.into();
            model.default_category_id = Set(to_category.cloned());
            model.default_project_id = Set(to_project.cloned());
            model.updated_at = Set(now as i64);
            model.update(db).map_err(|e| e.to_string())?;
        }
        for segment in &self.segments {
            if undo {
                segments::ActiveModel {
                    id: Set(segment.id),
                    app: Set(segment.app.clone()),
                    title: Set(segment.title.clone()),
                    kind: Set(segment.kind.clone()),
                    label: Set(segment.label.clone()),
                    reviewed: Set(i64::from(segment.reviewed)),
                    app_id: Set(segment.app_id.clone()),
                    bundle_id: Set(segment.bundle_id.clone()),
                    url: Set(segment.url.clone()),
                    domain: Set(segment.domain.clone()),
                    started_at: Set(segment.started_at as i64),
                    ended_at: Set(segment.ended_at.map(|value| value as i64)),
                    entry_id: Set(segment.entry_id.clone()),
                }
                .insert(db)
                .map_err(|e| e.to_string())?;
            } else {
                segments::Entity::delete_many()
                    .filter(segments::Column::Id.eq(segment.id))
                    .filter(segments::Column::EntryId.eq(segment.entry_id.clone()))
                    .exec(db)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}

impl ActivityStore {
    pub(super) fn delete_typed(
        &mut self,
        ids: &[String],
        category: Option<&str>,
        project: Option<&str>,
        now: u64,
    ) -> Result<(), String> {
        let path = self
            .database_path
            .clone()
            .ok_or_else(|| "database path unavailable for deletion".to_owned())?;
        let database = crate::database::connect(&path).map_err(|e| e.to_string())?;
        let deletion = database
            .transaction(|tx| {
                let deletion = TypedDeletion::capture(tx, ids, category, project, now)
                    .map_err(sea_orm::DbErr::Custom)?;
                if deletion.entries.is_empty()
                    && deletion.categories.is_empty()
                    && deletion.projects.is_empty()
                {
                    return Err(sea_orm::DbErr::Custom(
                        "Those items no longer exist.".into(),
                    ));
                }
                deletion
                    .apply(tx, false, now)
                    .map_err(sea_orm::DbErr::Custom)?;
                Ok(deletion)
            })
            .map_err(|e| match e {
                sea_orm::TransactionError::Connection(e)
                | sea_orm::TransactionError::Transaction(e) => e.to_string(),
            })?;
        self.forget_typed_deleted_segment(&deletion);
        self.record_undo(Command::TypedDeletion(deletion));
        Ok(())
    }

    fn forget_typed_deleted_segment(&mut self, deletion: &TypedDeletion) {
        if self.current.as_ref().is_some_and(|current| {
            deletion
                .segments
                .iter()
                .any(|segment| segment.id == current.id)
        }) {
            self.current = None;
            self.pending_switch = None;
        }
    }
}

#[cfg(not(test))]
impl ActivityStore {
    pub(super) fn delete_undoable(
        &mut self,
        ids: &[String],
        category: Option<&str>,
        project: Option<&str>,
        now: u64,
    ) -> Result<(), String> {
        self.delete_typed(ids, category, project, now)
    }
}

pub(super) fn apply_typed(
    store: &mut ActivityStore,
    deletion: &mut TypedDeletion,
    undo: bool,
    now: u64,
) -> Result<(), String> {
    let path = store
        .database_path
        .clone()
        .ok_or_else(|| "database path unavailable for deletion".to_owned())?;
    let database = crate::database::connect(&path).map_err(|e| e.to_string())?;
    database
        .transaction(|tx| {
            deletion
                .apply(tx, undo, now)
                .map_err(sea_orm::DbErr::Custom)
        })
        .map_err(|e| match e {
            sea_orm::TransactionError::Connection(e)
            | sea_orm::TransactionError::Transaction(e) => e.to_string(),
        })?;
    if !undo {
        store.forget_typed_deleted_segment(deletion);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_entry_deletion_round_trips_through_undo_and_redo() {
        let dir = std::env::temp_dir().join(format!("aevio-typed-delete-{}", uuid::Uuid::now_v7()));
        let mut store = ActivityStore::load(&dir).unwrap();
        store
            .conn_mut()
            .execute(
                "INSERT INTO time_entries (id, started_at, ended_at, description, status, source, billable, created_at, updated_at, description_origin) VALUES (?1, 0, 100, 'work', 'pending', 'manual', 0, 0, 0, 'user')",
                rusqlite::params!["entry-1"],
            )
            .unwrap();

        store
            .delete_typed(&["entry-1".into()], None, None, 200)
            .unwrap();
        assert_eq!(store.time_entry("entry-1").unwrap().deleted_at, Some(200));
        assert_eq!(store.undo(300).unwrap(), Some("Delete"));
        assert_eq!(store.time_entry("entry-1").unwrap().deleted_at, None);
        assert_eq!(store.redo(400).unwrap(), Some("Delete"));
        assert_eq!(store.time_entry("entry-1").unwrap().deleted_at, Some(200));

        let _ = std::fs::remove_file(dir.join(crate::activity::DB_FILE));
        let _ = std::fs::remove_dir(dir);
    }
}
