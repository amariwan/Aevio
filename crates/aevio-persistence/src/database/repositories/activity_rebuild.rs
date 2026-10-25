use std::collections::{HashMap, HashSet};

use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::database::entities::ai::{classify_jobs, embeddings, suggestions};
use crate::database::entities::entry_query::{entries, events, segments};
use aevio_core::entry_builder::{BuiltTimeEntry, EntrySettings, SegmentInput, build_entries};

pub struct ActivityRebuildRepository;

impl ActivityRebuildRepository {
    pub fn rebuild(
        database: &DatabaseConnection,
        start_ms: i64,
        end_ms: i64,
        now: i64,
    ) -> Result<bool, DbErr> {
        database
            .transaction(|transaction| Self::rebuild_in(transaction, start_ms, end_ms, now))
            .map_err(|error| match error {
                sea_orm::TransactionError::Connection(error)
                | sea_orm::TransactionError::Transaction(error) => error,
            })
    }

    fn rebuild_in<C: ConnectionTrait>(
        db: &C,
        start_ms: i64,
        end_ms: i64,
        now: i64,
    ) -> Result<bool, DbErr> {
        let segment_rows = segments::Entity::find()
            .filter(segments::Column::StartedAt.gte(start_ms))
            .filter(segments::Column::StartedAt.lte(end_ms))
            .order_by_asc(segments::Column::StartedAt)
            .all(db)?;
        let segment_inputs = segment_rows
            .iter()
            .map(|row| SegmentInput {
                id: row.id,
                app: row.app.clone(),
                title: row.title.clone(),
                kind: row.kind.clone(),
                label: row.label.clone(),
                started_at: row.started_at.max(0) as u64,
                ended_at: row.ended_at.map(|value| value.max(0) as u64),
                entry_id: row.entry_id.clone(),
            })
            .collect::<Vec<_>>();
        let existing = entries::Entity::find()
            .filter(entries::Column::EndedAt.gte(start_ms))
            .filter(entries::Column::StartedAt.lte(end_ms))
            .all(db)?;
        let frozen = existing
            .iter()
            .filter(|row| !(row.deleted_at.is_none() && row.status == "building"))
            .map(to_built)
            .collect::<Vec<_>>();
        let frozen_ids = frozen
            .iter()
            .map(|row| row.id.as_str())
            .collect::<HashSet<_>>();
        let built = build_entries(
            &segment_inputs,
            &frozen,
            &EntrySettings::default(),
            now as u64,
        );
        let linked = segment_rows
            .iter()
            .map(|row| (row.id, row.entry_id.as_deref()))
            .collect::<HashMap<_, _>>();
        let existing_by_id = existing
            .iter()
            .map(|row| (row.id.as_str(), row))
            .collect::<HashMap<_, _>>();

        let mut changed = false;
        let mut kept = HashSet::new();
        for entry in built
            .iter()
            .filter(|row| !frozen_ids.contains(row.id.as_str()))
        {
            kept.insert(entry.id.as_str());
            let before = existing_by_id.get(entry.id.as_str());
            upsert_entry(db, entry, before.copied(), now)?;
            for segment_id in &entry.segment_ids {
                if linked.get(segment_id) == Some(&Some(entry.id.as_str())) {
                    continue;
                }
                segments::Entity::update_many()
                    .col_expr(segments::Column::EntryId, Expr::value(entry.id.clone()))
                    .filter(segments::Column::Id.eq(*segment_id))
                    .exec(db)?;
            }
            if before.map(|row| row.status.as_str()) != Some(entry.status.as_str()) {
                changed = true;
                if entry.status == "pending" {
                    enqueue(db, &entry.id, "classify", now)?;
                }
            }
        }

        for stale in existing.iter().filter(|row| {
            row.deleted_at.is_none() && row.status == "building" && !kept.contains(row.id.as_str())
        }) {
            events::Entity::delete_many()
                .filter(events::Column::EntryId.eq(stale.id.clone()))
                .exec(db)?;
            suggestions::Entity::delete_many()
                .filter(suggestions::Column::EntryId.eq(stale.id.clone()))
                .exec(db)?;
            classify_jobs::Entity::delete_many()
                .filter(classify_jobs::Column::EntryId.eq(stale.id.clone()))
                .exec(db)?;
            embeddings::Entity::delete_by_id(stale.id.clone()).exec(db)?;
            segments::Entity::update_many()
                .col_expr(segments::Column::EntryId, Expr::value(None::<String>))
                .filter(segments::Column::EntryId.eq(stale.id.clone()))
                .exec(db)?;
            entries::Entity::delete_by_id(stale.id.clone()).exec(db)?;
            changed = true;
        }

        changed |= enqueue_unclassified(db, start_ms, end_ms, now)? > 0;
        Ok(changed)
    }
}

fn to_built(row: &entries::Model) -> BuiltTimeEntry {
    BuiltTimeEntry {
        id: row.id.clone(),
        started_at: row.started_at.max(0) as u64,
        ended_at: row.ended_at.max(0) as u64,
        description: row.description.clone(),
        category_id: row.category_id.clone(),
        project_id: row.project_id.clone(),
        status: row.status.clone(),
        approved_by: row.approved_by.clone(),
        source: row.source.clone(),
        billable: row.billable != 0,
        invoice_id: row.invoice_id.clone(),
        created_at: row.created_at.max(0) as u64,
        updated_at: row.updated_at.max(0) as u64,
        deleted_at: row.deleted_at.map(|value| value.max(0) as u64),
        segment_ids: Vec::new(),
    }
}

fn upsert_entry<C: ConnectionTrait>(
    db: &C,
    entry: &BuiltTimeEntry,
    before: Option<&entries::Model>,
    now: i64,
) -> Result<(), DbErr> {
    if let Some(row) = before {
        let mut model: entries::ActiveModel = row.clone().into();
        model.started_at = Set(entry.started_at as i64);
        model.ended_at = Set(entry.ended_at as i64);
        model.status = Set(entry.status.clone());
        if row.description_origin == "template" {
            model.description = Set(entry.description.clone());
        }
        model.updated_at = Set(now);
        model.update(db)?;
        return Ok(());
    }
    entries::ActiveModel {
        id: Set(entry.id.clone()),
        started_at: Set(entry.started_at as i64),
        ended_at: Set(entry.ended_at as i64),
        description: Set(entry.description.clone()),
        category_id: Set(None),
        project_id: Set(None),
        task_id: Set(None),
        status: Set(entry.status.clone()),
        approved_by: Set(None),
        source: Set("auto".into()),
        billable: Set(0),
        invoice_id: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        description_origin: Set("template".into()),
    }
    .insert(db)?;
    Ok(())
}

fn enqueue<C: ConnectionTrait>(db: &C, entry_id: &str, kind: &str, now: i64) -> Result<(), DbErr> {
    if let Some(row) = classify_jobs::Entity::find()
        .filter(classify_jobs::Column::EntryId.eq(entry_id.to_owned()))
        .filter(classify_jobs::Column::Kind.eq(kind.to_owned()))
        .one(db)?
    {
        let mut model: classify_jobs::ActiveModel = row.into();
        model.state = Set("queued".into());
        model.attempts = Set(0);
        model.next_at = Set(now);
        model.last_error = Set(None);
        model.created_at = Set(now);
        model.updated_at = Set(now);
        model.update(db)?;
    } else {
        classify_jobs::ActiveModel {
            id: Set(uuid::Uuid::now_v7().to_string()),
            entry_id: Set(entry_id.to_owned()),
            kind: Set(kind.to_owned()),
            state: Set("queued".into()),
            attempts: Set(0),
            next_at: Set(now),
            last_error: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)?;
    }
    entries::Entity::update_many()
        .col_expr(entries::Column::Status, Expr::value("processing"))
        .col_expr(entries::Column::UpdatedAt, Expr::value(now))
        .filter(entries::Column::Id.eq(entry_id.to_owned()))
        .filter(entries::Column::Status.eq("pending"))
        .exec(db)?;
    Ok(())
}

fn enqueue_unclassified<C: ConnectionTrait>(
    db: &C,
    start: i64,
    end: i64,
    now: i64,
) -> Result<usize, DbErr> {
    let candidates = entries::Entity::find()
        .filter(entries::Column::DeletedAt.is_null())
        .filter(entries::Column::Status.eq("pending"))
        .filter(entries::Column::EndedAt.gte(start))
        .filter(entries::Column::StartedAt.lte(end))
        .all(db)?;
    let jobs = classify_jobs::Entity::find()
        .filter(classify_jobs::Column::Kind.eq("classify"))
        .all(db)?;
    let suggestions = suggestions::Entity::find().all(db)?;
    let queued = jobs
        .into_iter()
        .map(|row| row.entry_id)
        .collect::<HashSet<_>>();
    let suggested = suggestions
        .into_iter()
        .map(|row| row.entry_id)
        .collect::<HashSet<_>>();
    let mut count = 0;
    for entry in candidates {
        if (entry.source == "auto"
            || (entry.source == "manual" && entry.description.is_empty() && entry.ended_at <= now))
            && !queued.contains(&entry.id)
            && !suggested.contains(&entry.id)
        {
            enqueue(db, &entry.id, "classify", now)?;
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use crate::activity::ActivityStore;
    use crate::database::connect;
    use crate::database::repositories::activity::{ActivityRepository, NewSegment};

    #[test]
    fn rebuild_uses_typed_transaction_and_queues_closed_entries() {
        let dir = std::env::temp_dir().join(format!("aevio-rebuild-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(crate::activity::DB_FILE);
        let mut sqlite = rusqlite::Connection::open(&path).unwrap();
        crate::bootstrap::migrations::run_migrations(&mut sqlite).unwrap();
        drop(sqlite);
        let database = connect(&path).unwrap();
        let repository = ActivityRepository::new(&database);
        let id = repository
            .open_segment(NewSegment {
                app: "Editor",
                title: "main.rs",
                kind: "activity",
                label: None,
                started_at: 0,
                bundle_id: None,
                url: None,
                domain: None,
            })
            .unwrap();
        repository.close_segment(id, 10 * 60 * 1000).unwrap();

        let mut store = ActivityStore::load(&dir).unwrap();
        assert!(
            store
                .rebuild_range(0, 20 * 60 * 1000, 20 * 60 * 1000)
                .unwrap()
        );
        let entries = store.list_time_entries(0, 20 * 60 * 1000).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, "processing");

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir(dir);
    }
}
