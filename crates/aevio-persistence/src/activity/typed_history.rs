use sea_orm::entity::prelude::*;
use sea_orm::{ActiveValue::Set, ConnectionTrait, DbErr, EntityTrait, QueryFilter};

use super::{ActivityStore, LOCKED};
use crate::database::entities::ai::{classify_jobs, embeddings, suggestions};
use crate::database::entities::entry_query::{entries, events, segments};

/// A typed before/after change for one entry. Unlike the legacy history rows,
/// this snapshot is a SeaORM model and can be applied inside one sync
/// transaction without opening a second rusqlite transaction.
pub(super) struct TypedEdit {
    pub(super) label: &'static str,
    pub(super) changes: Vec<TypedChange>,
}

pub(super) struct TypedChange {
    pub(super) id: String,
    pub(super) before: Option<entries::Model>,
    pub(super) after: entries::Model,
}

pub(super) struct TypedStructural {
    pub(super) label: &'static str,
    pub(super) entries: Vec<StructuralEntryChange>,
    pub(super) segments: Vec<SegmentChange>,
    pub(super) dependents: Vec<DependentChange>,
}

pub(super) struct StructuralEntryChange {
    pub(super) id: String,
    pub(super) before: Option<entries::Model>,
    pub(super) after: Option<entries::Model>,
}

pub(super) struct SegmentChange {
    pub(super) id: i64,
    pub(super) before: Option<segments::Model>,
    pub(super) after: Option<segments::Model>,
}

pub(super) struct DependentChange {
    pub(super) entry_id: String,
    pub(super) before: DependentSnapshot,
    pub(super) after: DependentSnapshot,
}

#[derive(Default)]
pub(super) struct DependentSnapshot {
    pub(super) suggestions: Vec<suggestions::Model>,
    pub(super) jobs: Vec<classify_jobs::Model>,
    pub(super) embeddings: Vec<embeddings::Model>,
    pub(super) events: Vec<events::Model>,
}

pub(super) fn apply(
    store: &ActivityStore,
    edit: &TypedEdit,
    undo: bool,
    now: u64,
) -> Result<(), String> {
    let database = store.database().map_err(|error| error.to_string())?;
    database
        .transaction(|transaction| {
            for change in &edit.changes {
                apply_change(transaction, change, undo, now)?;
            }
            Ok::<_, DbErr>(())
        })
        .map_err(|error| error.to_string())
}

pub(super) fn apply_structural(
    store: &ActivityStore,
    edit: &TypedStructural,
    undo: bool,
    now: u64,
) -> Result<(), String> {
    let database = store.database().map_err(|error| error.to_string())?;
    database
        .transaction(|transaction| {
            for change in &edit.dependents {
                apply_dependent_change(transaction, change, undo)?;
            }
            for change in &edit.entries {
                apply_structural_entry(transaction, change, undo, now)?;
            }
            for change in &edit.segments {
                apply_segment_change(transaction, change, undo)?;
            }
            Ok::<_, DbErr>(())
        })
        .map_err(|error| error.to_string())
}

fn apply_structural_entry<C: ConnectionTrait>(
    database: &C,
    change: &StructuralEntryChange,
    undo: bool,
    now: u64,
) -> Result<(), DbErr> {
    let expected = if undo {
        change.after.as_ref()
    } else {
        change.before.as_ref()
    };
    let target = if undo {
        change.before.as_ref()
    } else {
        change.after.as_ref()
    };
    let current = entries::Entity::find_by_id(change.id.clone()).one(database)?;
    if !entry_matches(current.as_ref(), expected) {
        return Err(DbErr::Custom(
            "These entries have changed since; undo/redo cannot safely overwrite them.".into(),
        ));
    }
    if current.as_ref().is_some_and(|row| row.status == "approved")
        && target.as_ref().is_none_or(|row| row.status != "approved")
    {
        return Err(DbErr::Custom(LOCKED.to_owned()));
    }
    match target {
        Some(row) => {
            let mut model: entries::ActiveModel = row.clone().into();
            model.started_at = Set(row.started_at);
            model.ended_at = Set(row.ended_at);
            model.description = Set(row.description.clone());
            model.category_id = Set(row.category_id.clone());
            model.project_id = Set(row.project_id.clone());
            model.task_id = Set(row.task_id.clone());
            model.status = Set(row.status.clone());
            model.approved_by = Set(row.approved_by.clone());
            model.source = Set(row.source.clone());
            model.billable = Set(row.billable);
            model.invoice_id = Set(row.invoice_id.clone());
            model.created_at = Set(row.created_at);
            model.updated_at = Set(now as i64);
            model.deleted_at = Set(row.deleted_at);
            model.description_origin = Set(row.description_origin.clone());
            if current.is_some() {
                model.update(database)?;
            } else {
                model.insert(database)?;
            }
        }
        None => {
            entries::Entity::delete_by_id(change.id.clone()).exec(database)?;
        }
    }
    Ok(())
}

fn apply_segment_change<C: ConnectionTrait>(
    database: &C,
    change: &SegmentChange,
    undo: bool,
) -> Result<(), DbErr> {
    let expected = if undo {
        change.after.as_ref()
    } else {
        change.before.as_ref()
    };
    let target = if undo {
        change.before.as_ref()
    } else {
        change.after.as_ref()
    };
    let current = segments::Entity::find_by_id(change.id).one(database)?;
    if current.as_ref() != expected {
        return Err(DbErr::Custom(
            "Segments have changed since; undo/redo cannot safely overwrite them.".into(),
        ));
    }
    segments::Entity::delete_by_id(change.id).exec(database)?;
    if let Some(row) = target {
        segments::Entity::insert(segment_model(row)).exec(database)?;
    }
    Ok(())
}

fn entry_matches(current: Option<&entries::Model>, expected: Option<&entries::Model>) -> bool {
    match (current, expected) {
        (None, None) => true,
        (Some(current), Some(expected)) => same_except_updated(current, expected),
        _ => false,
    }
}

fn segment_model(row: &segments::Model) -> segments::ActiveModel {
    segments::ActiveModel {
        id: Set(row.id),
        entry_id: Set(row.entry_id.clone()),
        app: Set(row.app.clone()),
        title: Set(row.title.clone()),
        kind: Set(row.kind.clone()),
        label: Set(row.label.clone()),
        reviewed: Set(row.reviewed),
        app_id: Set(row.app_id.clone()),
        bundle_id: Set(row.bundle_id.clone()),
        url: Set(row.url.clone()),
        domain: Set(row.domain.clone()),
        started_at: Set(row.started_at),
        ended_at: Set(row.ended_at),
    }
}

fn apply_dependent_change<C: ConnectionTrait>(
    database: &C,
    change: &DependentChange,
    undo: bool,
) -> Result<(), DbErr> {
    let expected = if undo { &change.after } else { &change.before };
    let target = if undo { &change.before } else { &change.after };
    let current = capture_dependents(database, &change.entry_id)?;
    if !dependent_equal(&current, expected) {
        return Err(DbErr::Custom(
            "Entry AI metadata has changed since; undo/redo cannot safely overwrite it.".into(),
        ));
    }
    suggestions::Entity::delete_many()
        .filter(suggestions::Column::EntryId.eq(change.entry_id.clone()))
        .exec(database)?;
    classify_jobs::Entity::delete_many()
        .filter(classify_jobs::Column::EntryId.eq(change.entry_id.clone()))
        .exec(database)?;
    embeddings::Entity::delete_many()
        .filter(embeddings::Column::EntryId.eq(change.entry_id.clone()))
        .exec(database)?;
    events::Entity::delete_many()
        .filter(events::Column::EntryId.eq(change.entry_id.clone()))
        .exec(database)?;
    for row in &target.suggestions {
        suggestions::Entity::insert(suggestion_model(row)).exec(database)?;
    }
    for row in &target.jobs {
        classify_jobs::Entity::insert(job_model(row)).exec(database)?;
    }
    for row in &target.embeddings {
        embeddings::Entity::insert(embedding_model(row)).exec(database)?;
    }
    for row in &target.events {
        events::Entity::insert(event_model(row)).exec(database)?;
    }
    Ok(())
}

pub(super) fn capture_dependents<C: ConnectionTrait>(
    database: &C,
    entry_id: &str,
) -> Result<DependentSnapshot, DbErr> {
    Ok(DependentSnapshot {
        suggestions: suggestions::Entity::find()
            .filter(suggestions::Column::EntryId.eq(entry_id.to_owned()))
            .all(database)?,
        jobs: classify_jobs::Entity::find()
            .filter(classify_jobs::Column::EntryId.eq(entry_id.to_owned()))
            .all(database)?,
        embeddings: embeddings::Entity::find()
            .filter(embeddings::Column::EntryId.eq(entry_id.to_owned()))
            .all(database)?,
        events: events::Entity::find()
            .filter(events::Column::EntryId.eq(entry_id.to_owned()))
            .all(database)?,
    })
}

fn dependent_equal(left: &DependentSnapshot, right: &DependentSnapshot) -> bool {
    left.suggestions == right.suggestions
        && left.jobs == right.jobs
        && left.embeddings == right.embeddings
        && left.events == right.events
}

pub(super) fn suggestion_model(row: &suggestions::Model) -> suggestions::ActiveModel {
    suggestions::ActiveModel {
        id: Set(row.id.clone()),
        entry_id: Set(row.entry_id.clone()),
        field: Set(row.field.clone()),
        value_id: Set(row.value_id.clone()),
        confidence: Set(row.confidence),
        signals: Set(row.signals.clone()),
        rationale: Set(row.rationale.clone()),
        alternatives: Set(row.alternatives.clone()),
        engine: Set(row.engine.clone()),
        model_version: Set(row.model_version.clone()),
        outcome: Set(row.outcome.clone()),
        created_at: Set(row.created_at),
        updated_at: Set(row.updated_at),
        raw_confidence: Set(row.raw_confidence),
        tier: Set(row.tier.clone()),
        deleted_at: Set(row.deleted_at),
    }
}

fn job_model(row: &classify_jobs::Model) -> classify_jobs::ActiveModel {
    classify_jobs::ActiveModel {
        id: Set(row.id.clone()),
        entry_id: Set(row.entry_id.clone()),
        kind: Set(row.kind.clone()),
        state: Set(row.state.clone()),
        attempts: Set(row.attempts),
        next_at: Set(row.next_at),
        last_error: Set(row.last_error.clone()),
        created_at: Set(row.created_at),
        updated_at: Set(row.updated_at),
    }
}

fn embedding_model(row: &embeddings::Model) -> embeddings::ActiveModel {
    embeddings::ActiveModel {
        entry_id: Set(row.entry_id.clone()),
        vec: Set(row.vec.clone()),
        text_hash: Set(row.text_hash.clone()),
        features: Set(row.features.clone()),
        model: Set(row.model.clone()),
        created_at: Set(row.created_at),
    }
}

fn event_model(row: &events::Model) -> events::ActiveModel {
    events::ActiveModel {
        id: Set(row.id.clone()),
        entry_id: Set(row.entry_id.clone()),
        kind: Set(row.kind.clone()),
        actor: Set(row.actor.clone()),
        payload: Set(row.payload.clone()),
        at: Set(row.at),
    }
}

fn apply_change<C: ConnectionTrait>(
    database: &C,
    change: &TypedChange,
    undo: bool,
    now: u64,
) -> Result<(), DbErr> {
    let Some(before) = &change.before else {
        return if undo {
            remove_created(database, &change.id)
        } else {
            entries::Entity::insert(model_for(&change.after, now)).exec(database)?;
            Ok(())
        };
    };

    let target = if undo { before } else { &change.after };
    let expected = if undo { &change.after } else { before };
    let current = entries::Entity::find_by_id(change.id.clone())
        .one(database)?
        .ok_or_else(|| {
            DbErr::Custom(
                "These entries have changed since; undo/redo cannot safely overwrite them.".into(),
            )
        })?;
    if current.status == "approved" && target.status != "approved" {
        return Err(DbErr::Custom(LOCKED.to_owned()));
    }
    if !same_except_updated(&current, expected) {
        return Err(DbErr::Custom(
            "These entries have changed since; undo/redo cannot safely overwrite them.".into(),
        ));
    }
    let mut model: entries::ActiveModel = target.clone().into();
    model.started_at = Set(target.started_at);
    model.ended_at = Set(target.ended_at);
    model.description = Set(target.description.clone());
    model.category_id = Set(target.category_id.clone());
    model.project_id = Set(target.project_id.clone());
    model.task_id = Set(target.task_id.clone());
    model.status = Set(target.status.clone());
    model.approved_by = Set(target.approved_by.clone());
    model.source = Set(target.source.clone());
    model.billable = Set(target.billable);
    model.invoice_id = Set(target.invoice_id.clone());
    model.created_at = Set(target.created_at);
    model.deleted_at = Set(target.deleted_at);
    model.description_origin = Set(target.description_origin.clone());
    model.updated_at = Set(now as i64);
    model.update(database)?;
    Ok(())
}

fn model_for(row: &entries::Model, now: u64) -> entries::ActiveModel {
    let mut model: entries::ActiveModel = row.clone().into();
    model.updated_at = Set(now as i64);
    model
}

fn remove_created<C: ConnectionTrait>(database: &C, id: &str) -> Result<(), DbErr> {
    let current = entries::Entity::find_by_id(id.to_owned()).one(database)?;
    if let Some(row) = current
        && row.status == "approved"
    {
        return Err(DbErr::Custom(LOCKED.to_owned()));
    }
    segments::Entity::update_many()
        .col_expr(segments::Column::EntryId, Expr::value(None::<String>))
        .filter(segments::Column::EntryId.eq(id.to_owned()))
        .exec(database)?;
    suggestions::Entity::delete_many()
        .filter(suggestions::Column::EntryId.eq(id.to_owned()))
        .exec(database)?;
    classify_jobs::Entity::delete_many()
        .filter(classify_jobs::Column::EntryId.eq(id.to_owned()))
        .exec(database)?;
    embeddings::Entity::delete_many()
        .filter(embeddings::Column::EntryId.eq(id.to_owned()))
        .exec(database)?;
    events::Entity::delete_many()
        .filter(events::Column::EntryId.eq(id.to_owned()))
        .exec(database)?;
    entries::Entity::delete_by_id(id.to_owned()).exec(database)?;
    Ok(())
}

fn same_except_updated(left: &entries::Model, right: &entries::Model) -> bool {
    left.id == right.id
        && left.started_at == right.started_at
        && left.ended_at == right.ended_at
        && left.description == right.description
        && left.category_id == right.category_id
        && left.project_id == right.project_id
        && left.task_id == right.task_id
        && left.status == right.status
        && left.approved_by == right.approved_by
        && left.source == right.source
        && left.billable == right.billable
        && left.invoice_id == right.invoice_id
        && left.created_at == right.created_at
        && left.deleted_at == right.deleted_at
        && left.description_origin == right.description_origin
}

#[cfg(test)]
mod tests {
    use super::*;
    use aevio_core::models::{NewTimeEntry, UpdateTimeEntry};

    #[test]
    fn typed_entry_edit_round_trips_through_activity_store_history() {
        let directory =
            std::env::temp_dir().join(format!("aevio-typed-history-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&directory).unwrap();
        let mut store = ActivityStore::load(&directory).unwrap();
        let entry = store
            .create_manual_entry(
                NewTimeEntry {
                    started_at: 100,
                    ended_at: 200,
                    description: "Original".into(),
                    category_id: Some("category".into()),
                    project_id: None,
                    billable: Some(true),
                    review: true,
                },
                250,
            )
            .unwrap();
        store
            .update_time_entry(
                &entry.id,
                UpdateTimeEntry {
                    description: Some("Changed".into()),
                    ..Default::default()
                },
                300,
            )
            .unwrap();
        assert_eq!(store.time_entry(&entry.id).unwrap().description, "Changed");
        assert_eq!(store.undo(400).unwrap(), Some("Edit"));
        assert_eq!(store.time_entry(&entry.id).unwrap().description, "Original");
        assert_eq!(store.redo(500).unwrap(), Some("Edit"));
        assert_eq!(store.time_entry(&entry.id).unwrap().description, "Changed");
        drop(store);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
