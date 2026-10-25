use std::collections::{HashMap, HashSet};

use sea_orm::entity::prelude::*;
use sea_orm::{ActiveValue::Set, Condition, DatabaseConnection, DbErr, QueryOrder, QuerySelect};

use crate::database::entities::entry_query::{entries, events, projects, segments};
use aevio_core::models::{EntryEvent, TimeEntry};
use aevio_core::{EntryFilter, MAX_QUERY_ROWS, NONE};

pub struct EntryQueryRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> EntryQueryRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn list(&self, filter: &EntryFilter, limit: u32) -> Result<Vec<TimeEntry>, DbErr> {
        let needs_post_filter = filter.app.as_deref().is_some_and(|value| !value.is_empty())
            || filter
                .search
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty());
        let mut query = entries::Entity::find()
            .filter(entries::Column::DeletedAt.is_null())
            .filter(entries::Column::StartedAt.gte(filter.start_ms as i64))
            .filter(entries::Column::StartedAt.lt(filter.end_ms as i64))
            .order_by_desc(entries::Column::StartedAt)
            .order_by_asc(entries::Column::Id);
        if !needs_post_filter {
            query = query.limit(limit.clamp(1, MAX_QUERY_ROWS) as u64);
        }

        query = apply_optional_id(
            query,
            entries::Column::CategoryId,
            filter.category_id.as_deref(),
        );
        query = apply_optional_id(
            query,
            entries::Column::ProjectId,
            filter.project_id.as_deref(),
        );
        if let Some(status) = filter.status.as_deref().filter(|value| !value.is_empty()) {
            query = query.filter(match status {
                "approved" => Condition::all().add(entries::Column::Status.eq("approved")),
                "pending" => Condition::all().add(entries::Column::Status.ne("approved")),
                other => return Err(DbErr::Custom(format!("unknown status filter {other}"))),
            });
        }
        if let Some(billable) = filter.billable {
            query = query.filter(entries::Column::Billable.eq(billable as i64));
        }

        let projects = projects::Entity::find().all(self.db)?;
        let client_projects = filter.client_id.as_deref().map(|client_id| {
            projects
                .iter()
                .filter(|project| project.client_id.as_deref() == Some(client_id))
                .map(|project| project.id.clone())
                .collect::<HashSet<_>>()
        });
        if let Some(project_ids) = client_projects {
            query = query.filter(entries::Column::ProjectId.is_in(project_ids));
        }

        let rows = query.all(self.db)?;
        let ids = rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>();
        let related_segments = if ids.is_empty() {
            Vec::new()
        } else {
            segments::Entity::find()
                .filter(segments::Column::EntryId.is_in(ids))
                .all(self.db)?
        };
        let search_terms = filter
            .search
            .as_deref()
            .unwrap_or("")
            .split_whitespace()
            .map(|term| term.replace('%', ""))
            .filter(|term| !term.is_empty())
            .take(8)
            .collect::<Vec<_>>();
        let app = filter.app.as_deref().filter(|value| !value.is_empty());
        Ok(rows
            .into_iter()
            .filter(|row| {
                let row_segments = related_segments
                    .iter()
                    .filter(|segment| segment.entry_id.as_deref() == Some(row.id.as_str()))
                    .collect::<Vec<_>>();
                let app_matches = app.is_none_or(|wanted| {
                    row_segments.iter().any(|segment| {
                        segment.kind != "break"
                            && segment
                                .domain
                                .as_deref()
                                .filter(|domain| !domain.is_empty())
                                .unwrap_or(&segment.app)
                                == wanted
                    })
                });
                let search_matches = search_terms.iter().all(|term| {
                    let term = term.to_lowercase();
                    row.description.to_lowercase().contains(&term)
                        || row_segments
                            .iter()
                            .any(|segment| segment.title.to_lowercase().contains(&term))
                });
                app_matches && search_matches
            })
            .map(to_time_entry)
            .take(limit.clamp(1, MAX_QUERY_ROWS) as usize)
            .collect())
    }

    pub fn dominant_apps(&self, start: i64, end: i64) -> Result<HashMap<String, String>, DbErr> {
        let entries = entries::Entity::find()
            .filter(entries::Column::DeletedAt.is_null())
            .filter(entries::Column::EndedAt.gte(start))
            .filter(entries::Column::StartedAt.lte(end))
            .all(self.db)?;
        let ids = entries
            .iter()
            .map(|entry| entry.id.clone())
            .collect::<Vec<_>>();
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut totals = HashMap::<(String, String), i64>::new();
        for segment in segments::Entity::find()
            .filter(segments::Column::EntryId.is_in(ids))
            .all(self.db)?
        {
            if segment.kind == "break" {
                continue;
            }
            let Some(entry_id) = segment.entry_id else {
                continue;
            };
            let app = segment
                .domain
                .filter(|domain| !domain.is_empty())
                .unwrap_or(segment.app);
            let ms =
                (segment.ended_at.unwrap_or(end).min(end) - segment.started_at.max(start)).max(0);
            *totals.entry((entry_id, app)).or_default() += ms;
        }
        let mut best = HashMap::<String, (String, i64)>::new();
        for ((entry_id, app), ms) in totals {
            let slot = best.entry(entry_id).or_insert_with(|| (app.clone(), ms));
            if ms > slot.1 {
                *slot = (app, ms);
            }
        }
        Ok(best.into_iter().map(|(id, (app, _))| (id, app)).collect())
    }

    pub fn get(&self, id: &str) -> Result<TimeEntry, DbErr> {
        entries::Entity::find_by_id(id.to_owned())
            .one(self.db)?
            .map(to_time_entry)
            .ok_or_else(|| DbErr::RecordNotFound("time entry".into()))
    }

    pub fn detail_segments(
        &self,
        entry_id: &str,
        start: i64,
        end: i64,
    ) -> Result<Vec<segments::Model>, DbErr> {
        segments::Entity::find()
            .filter(
                Condition::any()
                    .add(segments::Column::EntryId.eq(entry_id.to_owned()))
                    .add(
                        Condition::all()
                            .add(segments::Column::EntryId.is_null())
                            .add(segments::Column::StartedAt.gte(start))
                            .add(segments::Column::EndedAt.lte(end))
                            .add(segments::Column::Kind.ne("break")),
                    ),
            )
            .order_by_asc(segments::Column::StartedAt)
            .all(self.db)
    }

    pub fn log_event(
        &self,
        entry_id: &str,
        kind: &str,
        actor: &str,
        payload: Option<&str>,
        at: u64,
    ) -> Result<(), DbErr> {
        events::Entity::insert(events::ActiveModel {
            id: Set(uuid::Uuid::now_v7().to_string()),
            entry_id: Set(entry_id.to_owned()),
            kind: Set(kind.to_owned()),
            actor: Set(actor.to_owned()),
            payload: Set(payload.map(str::to_owned)),
            at: Set(at as i64),
        })
        .exec(self.db)
        .map(|_| ())
    }

    pub fn events(&self, entry_id: &str) -> Result<Vec<EntryEvent>, DbErr> {
        events::Entity::find()
            .filter(events::Column::EntryId.eq(entry_id.to_owned()))
            .order_by_asc(events::Column::At)
            .all(self.db)
            .map(|rows| rows.into_iter().map(to_event).collect())
    }
}

fn apply_optional_id(
    query: Select<entries::Entity>,
    column: entries::Column,
    value: Option<&str>,
) -> Select<entries::Entity> {
    match value {
        None | Some("") => query,
        Some(NONE) => query.filter(column.is_null()),
        Some(value) => query.filter(column.eq(value.to_owned())),
    }
}

fn to_time_entry(row: entries::Model) -> TimeEntry {
    TimeEntry {
        id: row.id,
        started_at: row.started_at.max(0) as u64,
        ended_at: row.ended_at.max(0) as u64,
        description: row.description,
        category_id: row.category_id,
        project_id: row.project_id,
        status: row.status,
        approved_by: row.approved_by,
        source: row.source,
        billable: row.billable != 0,
        invoice_id: row.invoice_id,
        created_at: row.created_at.max(0) as u64,
        updated_at: row.updated_at.max(0) as u64,
        deleted_at: row.deleted_at.map(|value| value.max(0) as u64),
        description_origin: row.description_origin,
        ai: None,
        dominant_app: None,
    }
}

fn to_event(row: events::Model) -> EntryEvent {
    EntryEvent {
        id: row.id,
        entry_id: row.entry_id,
        kind: row.kind,
        actor: row.actor,
        payload: row.payload,
        at: row.at.max(0) as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::migrations::run_migrations;
    use crate::database::connect;
    use rusqlite::Connection;

    #[test]
    fn preserves_null_filters_search_and_app_domain_fallback() {
        let path =
            std::env::temp_dir().join(format!("aevio-entry-query-{}.db", uuid::Uuid::now_v7()));
        let mut sqlite = Connection::open(&path).unwrap();
        run_migrations(&mut sqlite).unwrap();
        sqlite
            .execute_batch(
                "INSERT INTO clients (id, name, created_at, updated_at) VALUES ('c', 'Client', 1, 1);
                 INSERT INTO projects (id, client_id, name, color, status, created_at, updated_at) VALUES ('p', 'c', 'Project', '#000000', 'active', 1, 1);
                 INSERT INTO time_entries (id, started_at, ended_at, description, category_id, project_id, status, source, billable, created_at, updated_at, description_origin) VALUES ('e1', 10, 20, 'Build report', NULL, 'p', 'pending', 'auto', 1, 1, 1, 'template');
                 INSERT INTO time_entries (id, started_at, ended_at, description, status, source, billable, created_at, updated_at, description_origin) VALUES ('e2', 30, 40, 'Other', 'pending', 'auto', 0, 1, 1, 'template');
                 INSERT INTO segments (app, title, kind, started_at, ended_at, domain, entry_id) VALUES ('Browser', 'Build report', 'activity', 10, 20, 'example.com', 'e1');",
            )
            .unwrap();
        drop(sqlite);

        let database = connect(&path).unwrap();
        let filter = EntryFilter {
            start_ms: 0,
            end_ms: 100,
            client_id: Some("c".into()),
            app: Some("example.com".into()),
            search: Some("Build".into()),
            ..EntryFilter::default()
        };
        let rows = EntryQueryRepository::new(&database)
            .list(&filter, 10)
            .unwrap();
        assert_eq!(
            rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
            ["e1"]
        );
        drop(database);
        std::fs::remove_file(path).unwrap();
    }
}
