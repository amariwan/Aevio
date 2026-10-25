use std::collections::HashMap;

use sea_orm::entity::prelude::*;
use sea_orm::{Condition, DatabaseConnection, DbErr, QueryFilter, QueryOrder};

use crate::database::entities::entry_query::{entries, projects, segments};
use crate::database::entities::report::{categories, clients};
use aevio_core::{EntryFilter, NONE};

#[derive(Clone, Debug)]
pub struct ReportRow {
    pub entry: entries::Model,
    pub project: Option<projects::Model>,
    pub category_name: Option<String>,
    pub client_name: Option<String>,
    pub segments: Vec<segments::Model>,
}

pub struct ReportRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> ReportRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn rows(&self, filter: &EntryFilter) -> Result<Vec<ReportRow>, DbErr> {
        validate_range(filter)?;
        let mut query = entries::Entity::find()
            .filter(entries::Column::DeletedAt.is_null())
            .filter(entries::Column::StartedAt.gte(filter.start_ms as i64))
            .filter(entries::Column::StartedAt.lt(filter.end_ms as i64))
            .order_by_asc(entries::Column::StartedAt)
            .order_by_asc(entries::Column::Id);
        query = optional_id(
            query,
            entries::Column::CategoryId,
            filter.category_id.as_deref(),
        );
        query = optional_id(
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

        let all_projects = projects::Entity::find().all(self.db)?;
        let project_by_id = all_projects
            .iter()
            .map(|project| (project.id.as_str(), project))
            .collect::<HashMap<_, _>>();
        if let Some(client) = filter
            .client_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            query = query.filter(match client {
                NONE => {
                    let no_client_projects = all_projects
                        .iter()
                        .filter(|project| project.client_id.is_none())
                        .map(|project| project.id.clone())
                        .collect::<Vec<_>>();
                    Condition::any()
                        .add(entries::Column::ProjectId.is_null())
                        .add(entries::Column::ProjectId.is_in(no_client_projects))
                }
                value => entries::Column::ProjectId
                    .is_in(
                        all_projects
                            .iter()
                            .filter(|project| project.client_id.as_deref() == Some(value))
                            .map(|project| project.id.clone())
                            .collect::<Vec<_>>(),
                    )
                    .into(),
            });
        }

        let rows = query.all(self.db)?;
        let ids = rows
            .iter()
            .map(|entry| entry.id.clone())
            .collect::<Vec<_>>();
        let related_segments = if ids.is_empty() {
            Vec::new()
        } else {
            segments::Entity::find()
                .filter(segments::Column::EntryId.is_in(ids))
                .all(self.db)?
        };
        let categories = categories::Entity::find().all(self.db)?;
        let category_names = categories
            .into_iter()
            .map(|category| (category.id, category.name))
            .collect::<HashMap<_, _>>();
        let client_names = clients::Entity::find()
            .all(self.db)?
            .into_iter()
            .map(|client| (client.id, client.name))
            .collect::<HashMap<_, _>>();
        let search_terms = search_terms(filter.search.as_deref().unwrap_or(""));
        let app = filter.app.as_deref().filter(|value| !value.is_empty());

        Ok(rows
            .into_iter()
            .filter_map(|entry| {
                let project = entry
                    .project_id
                    .as_deref()
                    .and_then(|id| project_by_id.get(id).copied())
                    .cloned();
                let entry_segments = related_segments
                    .iter()
                    .filter(|segment| segment.entry_id.as_deref() == Some(entry.id.as_str()))
                    .cloned()
                    .collect::<Vec<_>>();
                let app_matches = app.is_none_or(|wanted| {
                    entry_segments.iter().any(|segment| {
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
                    entry.description.to_lowercase().contains(&term)
                        || entry_segments
                            .iter()
                            .any(|segment| segment.title.to_lowercase().contains(&term))
                });
                if !app_matches || !search_matches {
                    return None;
                }
                let client_name = project
                    .as_ref()
                    .and_then(|project| project.client_id.as_ref())
                    .and_then(|id| client_names.get(id))
                    .cloned();
                Some(ReportRow {
                    category_name: entry
                        .category_id
                        .as_ref()
                        .and_then(|id| category_names.get(id))
                        .cloned(),
                    client_name,
                    entry,
                    project,
                    segments: entry_segments,
                })
            })
            .collect())
    }
}

fn validate_range(filter: &EntryFilter) -> Result<(), DbErr> {
    if filter.end_ms > i64::MAX as u64 {
        return Err(DbErr::Custom(
            "range exceeds the supported timestamp".into(),
        ));
    }
    if filter.end_ms <= filter.start_ms {
        return Err(DbErr::Custom("the range must end after it starts".into()));
    }
    Ok(())
}

fn optional_id(
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

fn search_terms(search: &str) -> Vec<String> {
    search
        .split_whitespace()
        .map(|term| term.replace('%', ""))
        .filter(|term| !term.is_empty())
        .take(8)
        .collect()
}
