use sea_orm::entity::prelude::*;
use sea_orm::{
    ActiveValue::Set, Condition, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
};

use super::typed_history::{
    DependentChange, DependentSnapshot, SegmentChange, StructuralEntryChange, TypedStructural,
};
use crate::activity::{KIND_BREAK, LOCKED};
use crate::database::entities::ai::suggestions;
use crate::database::entities::entry_query::{entries, segments};

pub(super) struct SplitResult {
    pub(super) command: TypedStructural,
}

pub(super) struct MergeResult {
    pub(super) command: TypedStructural,
}

struct StructuralSnapshots<'a> {
    before_segments: &'a [segments::Model],
    after_segments: &'a [segments::Model],
    before_first: &'a entries::Model,
    after_first: &'a entries::Model,
    after_second: &'a entries::Model,
    dependents: Vec<DependentChange>,
}

struct MergeEntrySnapshots<'a> {
    primary_before: &'a entries::Model,
    primary_after: &'a entries::Model,
    secondary_before: &'a entries::Model,
}

pub(super) fn split(
    database: &DatabaseConnection,
    id: &str,
    second_id: &str,
    at: u64,
    now: u64,
) -> Result<SplitResult, DbErr> {
    database
        .transaction(|transaction| {
            let original = entries::Entity::find_by_id(id.to_owned())
                .one(transaction)?
                .ok_or_else(|| DbErr::RecordNotFound("time entry".into()))?;
            validate_split(&original, at, now)?;
            let end = original.ended_at.max(if original.status == "building" {
                now as i64
            } else {
                original.ended_at
            });
            let touched_before = segments_for_split(transaction, id, &original)?;
            let original_dependents = super::typed_history::capture_dependents(transaction, id)?;
            if original.status == "building" {
                segments::Entity::update_many()
                    .col_expr(segments::Column::EntryId, Expr::value(id.to_owned()))
                    .filter(segments::Column::EntryId.is_null())
                    .filter(segments::Column::Kind.ne(KIND_BREAK))
                    .filter(segments::Column::StartedAt.gte(original.started_at))
                    .exec(transaction)?;
            }
            let mut inserted_ids = Vec::new();
            let crossing = segments::Entity::find()
                .filter(segments::Column::EntryId.eq(id.to_owned()))
                .filter(segments::Column::StartedAt.lt(at as i64))
                .filter(
                    Condition::any()
                        .add(segments::Column::EndedAt.is_null())
                        .add(segments::Column::EndedAt.gt(at as i64)),
                )
                .all(transaction)?;
            for segment in crossing {
                let mut first: segments::ActiveModel = segment.clone().into();
                first.ended_at = Set(Some(at as i64));
                first.update(transaction)?;
                let piece = segments::ActiveModel {
                    id: Default::default(),
                    entry_id: Set(Some(second_id.to_owned())),
                    app: Set(segment.app),
                    title: Set(segment.title),
                    kind: Set(segment.kind),
                    label: Set(segment.label),
                    reviewed: Set(segment.reviewed),
                    app_id: Set(segment.app_id),
                    bundle_id: Set(segment.bundle_id),
                    url: Set(segment.url),
                    domain: Set(segment.domain),
                    started_at: Set(at as i64),
                    ended_at: Set(segment.ended_at),
                }
                .insert(transaction)?;
                inserted_ids.push(piece.id);
            }
            segments::Entity::update_many()
                .col_expr(segments::Column::EntryId, Expr::value(second_id.to_owned()))
                .filter(segments::Column::EntryId.eq(id.to_owned()))
                .filter(segments::Column::StartedAt.gte(at as i64))
                .exec(transaction)?;

            let second_model = entries::ActiveModel {
                id: Set(second_id.to_owned()),
                started_at: Set(at as i64),
                ended_at: Set(end),
                description: Set(original.description.clone()),
                category_id: Set(original.category_id.clone()),
                project_id: Set(original.project_id.clone()),
                task_id: Set(original.task_id.clone()),
                status: Set(original.status.clone()),
                approved_by: Set(original.approved_by.clone()),
                source: Set(original.source.clone()),
                billable: Set(original.billable),
                invoice_id: Set(original.invoice_id.clone()),
                created_at: Set(now as i64),
                updated_at: Set(now as i64),
                deleted_at: Set(None),
                description_origin: Set(original.description_origin.clone()),
            };
            let second = second_model.insert(transaction)?;
            let mut second_dependents = DependentSnapshot::default();
            for mut suggestion in original_dependents.suggestions {
                suggestion.id = uuid::Uuid::now_v7().to_string();
                suggestion.entry_id = second_id.to_owned();
                suggestions::Entity::insert(super::typed_history::suggestion_model(&suggestion))
                    .exec(transaction)?;
                second_dependents.suggestions.push(suggestion);
            }
            let mut first = original.clone();
            first.ended_at = at as i64;
            if first.status == "building" {
                first.status = "pending".to_owned();
            }
            first.updated_at = now as i64;
            let mut first_model: entries::ActiveModel = original.clone().into();
            set_entry(&mut first_model, &first);
            first_model.update(transaction)?;

            let touched_after = segments_for_ids(transaction, id, second_id, &inserted_ids)?;
            Ok(SplitResult {
                command: structural_command(
                    id,
                    second_id,
                    StructuralSnapshots {
                        before_segments: &touched_before,
                        after_segments: &touched_after,
                        before_first: &original,
                        after_first: &first,
                        after_second: &second,
                        dependents: vec![DependentChange {
                            entry_id: second_id.to_owned(),
                            before: DependentSnapshot::default(),
                            after: second_dependents,
                        }],
                    },
                ),
            })
        })
        .map_err(|error| match error {
            sea_orm::TransactionError::Connection(error)
            | sea_orm::TransactionError::Transaction(error) => error,
        })
}

pub(super) fn merge(
    database: &DatabaseConnection,
    primary_id: &str,
    secondary_id: &str,
    now: u64,
) -> Result<MergeResult, DbErr> {
    database
        .transaction(|transaction| {
            if primary_id == secondary_id {
                return Err(DbErr::Custom(
                    "An entry cannot be merged with itself".into(),
                ));
            }
            let primary = entries::Entity::find_by_id(primary_id.to_owned())
                .one(transaction)?
                .ok_or_else(|| DbErr::RecordNotFound("primary entry".into()))?;
            let secondary = entries::Entity::find_by_id(secondary_id.to_owned())
                .one(transaction)?
                .ok_or_else(|| DbErr::RecordNotFound("secondary entry".into()))?;
            validate_merge(&primary, &secondary)?;
            let before_segments = segments_for_merge(transaction, primary_id, secondary_id)?;
            let secondary_dependents =
                super::typed_history::capture_dependents(transaction, secondary_id)?;
            segments::Entity::update_many()
                .col_expr(
                    segments::Column::EntryId,
                    Expr::value(primary_id.to_owned()),
                )
                .filter(segments::Column::EntryId.eq(secondary_id.to_owned()))
                .exec(transaction)?;
            let mut after_primary = primary.clone();
            after_primary.started_at = primary.started_at.min(secondary.started_at);
            after_primary.ended_at = primary.ended_at.max(secondary.ended_at);
            if primary.status == "building" || secondary.status == "building" {
                after_primary.ended_at = after_primary.ended_at.max(now as i64);
            }
            after_primary.status = if primary.status == "building" || secondary.status == "building"
            {
                "pending".to_owned()
            } else {
                primary.status.clone()
            };
            after_primary.updated_at = now as i64;
            let mut primary_model: entries::ActiveModel = primary.clone().into();
            set_entry(&mut primary_model, &after_primary);
            primary_model.update(transaction)?;
            delete_dependents(transaction, secondary_id)?;
            entries::Entity::delete_by_id(secondary_id.to_owned()).exec(transaction)?;
            let after_segments = segments::Entity::find()
                .filter(segments::Column::EntryId.eq(primary_id.to_owned()))
                .order_by_asc(segments::Column::Id)
                .all(transaction)?;
            let command = merge_command(
                primary_id,
                secondary_id,
                &before_segments,
                &after_segments,
                MergeEntrySnapshots {
                    primary_before: &primary,
                    primary_after: &after_primary,
                    secondary_before: &secondary,
                },
                secondary_dependents,
            );
            Ok(MergeResult { command })
        })
        .map_err(|error| match error {
            sea_orm::TransactionError::Connection(error)
            | sea_orm::TransactionError::Transaction(error) => error,
        })
}

fn validate_merge(primary: &entries::Model, secondary: &entries::Model) -> Result<(), DbErr> {
    if primary.deleted_at.is_some() || secondary.deleted_at.is_some() {
        return Err(DbErr::Custom(
            "Entries to merge must not be deleted.".into(),
        ));
    }
    if primary.invoice_id.is_some() || secondary.invoice_id.is_some() {
        return Err(DbErr::Custom("Invoiced entries cannot be merged.".into()));
    }
    if primary.status == "approved" || secondary.status == "approved" {
        return Err(DbErr::Custom(LOCKED.to_owned()));
    }
    if primary.ended_at != secondary.started_at && secondary.ended_at != primary.started_at {
        return Err(DbErr::Custom(
            "Time entries must start and stop at the same border to be mergeable".into(),
        ));
    }
    Ok(())
}

fn segments_for_merge<C: ConnectionTrait>(
    database: &C,
    primary: &str,
    secondary: &str,
) -> Result<Vec<segments::Model>, DbErr> {
    segments::Entity::find()
        .filter(
            Condition::any()
                .add(segments::Column::EntryId.eq(primary.to_owned()))
                .add(segments::Column::EntryId.eq(secondary.to_owned())),
        )
        .order_by_asc(segments::Column::Id)
        .all(database)
}

fn delete_dependents<C: ConnectionTrait>(database: &C, entry_id: &str) -> Result<(), DbErr> {
    crate::database::entities::ai::suggestions::Entity::delete_many()
        .filter(crate::database::entities::ai::suggestions::Column::EntryId.eq(entry_id.to_owned()))
        .exec(database)?;
    crate::database::entities::ai::classify_jobs::Entity::delete_many()
        .filter(
            crate::database::entities::ai::classify_jobs::Column::EntryId.eq(entry_id.to_owned()),
        )
        .exec(database)?;
    crate::database::entities::ai::embeddings::Entity::delete_many()
        .filter(crate::database::entities::ai::embeddings::Column::EntryId.eq(entry_id.to_owned()))
        .exec(database)?;
    crate::database::entities::entry_query::events::Entity::delete_many()
        .filter(
            crate::database::entities::entry_query::events::Column::EntryId.eq(entry_id.to_owned()),
        )
        .exec(database)?;
    Ok(())
}

fn merge_command(
    primary_id: &str,
    secondary_id: &str,
    before_segments: &[segments::Model],
    after_segments: &[segments::Model],
    entries: MergeEntrySnapshots<'_>,
    secondary_dependents: DependentSnapshot,
) -> TypedStructural {
    let before_by_id = before_segments
        .iter()
        .map(|row| (row.id, row.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let after_by_id = after_segments
        .iter()
        .map(|row| (row.id, row.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut ids = before_by_id
        .keys()
        .chain(after_by_id.keys())
        .copied()
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    TypedStructural {
        label: "Merge",
        entries: vec![
            StructuralEntryChange {
                id: primary_id.to_owned(),
                before: Some(entries.primary_before.clone()),
                after: Some(entries.primary_after.clone()),
            },
            StructuralEntryChange {
                id: secondary_id.to_owned(),
                before: Some(entries.secondary_before.clone()),
                after: None,
            },
        ],
        segments: ids
            .into_iter()
            .map(|id| SegmentChange {
                id,
                before: before_by_id.get(&id).cloned(),
                after: after_by_id.get(&id).cloned(),
            })
            .collect(),
        dependents: vec![DependentChange {
            entry_id: secondary_id.to_owned(),
            before: secondary_dependents,
            after: DependentSnapshot::default(),
        }],
    }
}

fn validate_split(row: &entries::Model, at: u64, now: u64) -> Result<(), DbErr> {
    if row.deleted_at.is_some() {
        return Err(DbErr::Custom("That entry was deleted.".into()));
    }
    if row.invoice_id.is_some() {
        return Err(DbErr::Custom("Invoiced entries cannot be split.".into()));
    }
    if row.status == "approved" {
        return Err(DbErr::Custom(LOCKED.to_owned()));
    }
    let end = row.ended_at.max(if row.status == "building" {
        now as i64
    } else {
        row.ended_at
    });
    if at <= row.started_at as u64 || at >= end as u64 {
        return Err(DbErr::Custom(
            "Split point must be strictly inside the entry duration".into(),
        ));
    }
    Ok(())
}

fn segments_for_split<C: ConnectionTrait>(
    database: &C,
    id: &str,
    row: &entries::Model,
) -> Result<Vec<segments::Model>, DbErr> {
    let mut rows = segments::Entity::find()
        .filter(segments::Column::EntryId.eq(id.to_owned()))
        .all(database)?;
    let extra = segments::Entity::find()
        .filter(segments::Column::EntryId.is_null())
        .filter(segments::Column::StartedAt.gte(row.started_at))
        .filter(segments::Column::Kind.ne(KIND_BREAK))
        .all(database)?;
    rows.extend(extra);
    rows.sort_by_key(|segment| segment.id);
    rows.dedup_by_key(|segment| segment.id);
    Ok(rows)
}

fn segments_for_ids<C: ConnectionTrait>(
    database: &C,
    first: &str,
    second: &str,
    inserted: &[i64],
) -> Result<Vec<segments::Model>, DbErr> {
    let mut rows = segments::Entity::find()
        .filter(
            Condition::any()
                .add(segments::Column::EntryId.eq(first.to_owned()))
                .add(segments::Column::EntryId.eq(second.to_owned()))
                .add(segments::Column::Id.is_in(inserted.to_vec())),
        )
        .order_by_asc(segments::Column::Id)
        .all(database)?;
    rows.sort_by_key(|segment| segment.id);
    rows.dedup_by_key(|segment| segment.id);
    Ok(rows)
}

fn structural_command(
    first: &str,
    second: &str,
    snapshots: StructuralSnapshots<'_>,
) -> TypedStructural {
    let before_by_id = snapshots
        .before_segments
        .iter()
        .map(|row| (row.id, row.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let after_by_id = snapshots
        .after_segments
        .iter()
        .map(|row| (row.id, row.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut ids = before_by_id
        .keys()
        .chain(after_by_id.keys())
        .copied()
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    TypedStructural {
        label: "Split",
        entries: vec![
            StructuralEntryChange {
                id: first.to_owned(),
                before: Some(snapshots.before_first.clone()),
                after: Some(snapshots.after_first.clone()),
            },
            StructuralEntryChange {
                id: second.to_owned(),
                before: None,
                after: Some(snapshots.after_second.clone()),
            },
        ],
        segments: ids
            .into_iter()
            .map(|id| SegmentChange {
                id,
                before: before_by_id.get(&id).cloned(),
                after: after_by_id.get(&id).cloned(),
            })
            .collect(),
        dependents: snapshots.dependents,
    }
}

fn set_entry(model: &mut entries::ActiveModel, row: &entries::Model) {
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
    model.updated_at = Set(row.updated_at);
    model.deleted_at = Set(row.deleted_at);
    model.description_origin = Set(row.description_origin.clone());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::ActivityStore;
    use crate::database::entities::entry_query::segments;
    use crate::database::repositories::activity::{ActivityRepository, NewSegment};
    use aevio_core::models::NewTimeEntry;

    #[test]
    fn typed_split_round_trips_entry_and_segment_ownership() {
        let directory =
            std::env::temp_dir().join(format!("aevio-typed-split-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&directory).unwrap();
        let mut store = ActivityStore::load(&directory).unwrap();
        let entry = store
            .create_manual_entry(
                NewTimeEntry {
                    started_at: 100,
                    ended_at: 300,
                    description: "Work".into(),
                    category_id: None,
                    project_id: None,
                    billable: None,
                    review: true,
                },
                400,
            )
            .unwrap();
        let database = store.database().unwrap();
        let segment_id = ActivityRepository::new(&database)
            .open_segment(NewSegment {
                app: "Editor",
                title: "main.rs",
                kind: "activity",
                label: None,
                started_at: 120,
                bundle_id: None,
                url: None,
                domain: None,
            })
            .unwrap();
        segments::Entity::update_many()
            .col_expr(segments::Column::EntryId, Expr::value(entry.id.clone()))
            .filter(segments::Column::Id.eq(segment_id))
            .exec(&database)
            .unwrap();
        drop(database);

        let (first, second) = store.split_time_entry(&entry.id, 200, 500).unwrap();
        assert_eq!((first.started_at, first.ended_at), (100, 200));
        assert_eq!((second.started_at, second.ended_at), (200, 300));
        assert_eq!(store.undo(600).unwrap(), Some("Split"));
        assert!(store.time_entry(&second.id).is_err());
        assert_eq!(store.time_entry(&entry.id).unwrap().ended_at, 300);
        assert_eq!(store.redo(700).unwrap(), Some("Split"));
        assert_eq!(store.time_entry(&second.id).unwrap().started_at, 200);
        drop(store);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn typed_merge_round_trips_secondary_and_segments() {
        let directory =
            std::env::temp_dir().join(format!("aevio-typed-merge-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&directory).unwrap();
        let mut store = ActivityStore::load(&directory).unwrap();
        let first = store
            .create_manual_entry(
                NewTimeEntry {
                    started_at: 100,
                    ended_at: 200,
                    description: "First".into(),
                    category_id: None,
                    project_id: None,
                    billable: None,
                    review: true,
                },
                300,
            )
            .unwrap();
        let second = store
            .create_manual_entry(
                NewTimeEntry {
                    started_at: 200,
                    ended_at: 300,
                    description: "Second".into(),
                    category_id: None,
                    project_id: None,
                    billable: None,
                    review: true,
                },
                300,
            )
            .unwrap();
        let database = store.database().unwrap();
        let segment_id = ActivityRepository::new(&database)
            .open_segment(NewSegment {
                app: "Editor",
                title: "main.rs",
                kind: "activity",
                label: None,
                started_at: 220,
                bundle_id: None,
                url: None,
                domain: None,
            })
            .unwrap();
        segments::Entity::update_many()
            .col_expr(segments::Column::EntryId, Expr::value(second.id.clone()))
            .filter(segments::Column::Id.eq(segment_id))
            .exec(&database)
            .unwrap();
        drop(database);

        let merged = store
            .merge_time_entries(&first.id, &second.id, 400)
            .unwrap();
        assert_eq!((merged.started_at, merged.ended_at), (100, 300));
        assert!(store.time_entry(&second.id).is_err());
        assert_eq!(store.undo(500).unwrap(), Some("Merge"));
        assert_eq!(store.time_entry(&second.id).unwrap().started_at, 200);
        assert_eq!(store.redo(600).unwrap(), Some("Merge"));
        assert!(store.time_entry(&second.id).is_err());
        drop(store);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
