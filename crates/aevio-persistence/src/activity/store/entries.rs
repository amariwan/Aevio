use super::*;
#[cfg(test)]
use crate::entries::time_entry_from_row;
use sea_orm::EntityTrait;

fn persistence_error(error: sea_orm::DbErr) -> String {
    match error {
        sea_orm::DbErr::Custom(message) => message,
        other => other.to_string(),
    }
}

fn segment_from_model(
    row: crate::database::entities::entry_query::segments::Model,
) -> ActivitySegment {
    ActivitySegment {
        id: row.id,
        app: row.app,
        title: row.title,
        kind: row.kind,
        label: row.label,
        started_at: row.started_at.max(0) as u64,
        ended_at: row.ended_at.map(|value| value.max(0) as u64),
        reviewed: row.reviewed != 0,
        app_id: row.app_id,
        bundle_id: row.bundle_id,
        url: row.url,
        domain: row.domain,
        entry_id: row.entry_id,
    }
}

impl ActivityStore {
    pub fn list_time_entries(&self, start_ms: u64, end_ms: u64) -> Result<Vec<TimeEntry>, String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let filter = aevio_core::entries::EntryFilter {
                start_ms,
                end_ms,
                ..Default::default()
            };
            let repository =
                crate::database::repositories::entry_query::EntryQueryRepository::new(&database);
            let mut list = repository
                .list(&filter, aevio_core::entries::MAX_QUERY_ROWS)
                .map_err(|error| error.to_string())?;
            let ai = crate::database::repositories::ai::AiRepository::new(&database)
                .summaries(start_ms as i64, end_ms as i64)
                .map_err(|error| error.to_string())?;
            let apps = repository
                .dominant_apps(start_ms as i64, end_ms as i64)
                .map_err(|error| error.to_string())?;
            for entry in &mut list {
                entry.ai = ai.get(&entry.id).cloned();
                entry.dominant_app = apps.get(&entry.id).cloned();
            }
            return Ok(list);
        }
        #[cfg(not(test))]
        return Err("database path unavailable for entry read".into());
        #[cfg(test)]
        {
            let mut stmt = self
            .conn
            .prepare(
                "SELECT id, started_at, ended_at, description, category_id, project_id, status, approved_by, source, billable, invoice_id, created_at, updated_at, deleted_at, description_origin
                 FROM time_entries
                 WHERE deleted_at IS NULL AND ended_at >= ?1 AND started_at <= ?2
                 ORDER BY started_at ASC;",
            )
            .map_err(|e| e.to_string())?;

            let rows = stmt
                .query_map(params![start_ms as i64, end_ms as i64], time_entry_from_row)
                .map_err(|e| e.to_string())?;

            let mut ai = crate::ai::store::summaries(&self.conn, start_ms, end_ms)?;
            let mut apps = crate::reports::dominant_apps(&self.conn, start_ms, end_ms)?;
            let mut list = Vec::new();
            for entry in rows {
                let mut entry = entry.map_err(|e| e.to_string())?;
                entry.ai = ai.remove(&entry.id);
                entry.dominant_app = apps.remove(&entry.id);
                list.push(entry);
            }
            Ok(list)
        }
    }

    /// The entries with these ids, in the order given (missing ids skipped).
    pub fn time_entries(&self, ids: &[String]) -> Result<Vec<TimeEntry>, String> {
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Ok(entry) = self.time_entry(id)
                && entry.deleted_at.is_none()
            {
                out.push(entry);
            }
        }
        Ok(out)
    }

    pub(crate) fn time_entry(&self, id: &str) -> Result<TimeEntry, String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            return crate::database::repositories::entry_query::EntryQueryRepository::new(
                &database,
            )
            .get(id)
            .map_err(|error| error.to_string());
        }
        #[cfg(not(test))]
        return Err("database path unavailable for entry read".into());
        #[cfg(test)]
        self.conn
            .query_row(
                "SELECT id, started_at, ended_at, description, category_id, project_id, status, approved_by, source, billable, invoice_id, created_at, updated_at, deleted_at, description_origin
                 FROM time_entries WHERE id = ?1;",
                params![id],
                time_entry_from_row,
            )
            .map_err(|e| e.to_string())
    }

    pub(crate) fn log_event(
        &self,
        entry_id: &str,
        kind: &str,
        actor: &str,
        payload: Option<&str>,
        now: u64,
    ) {
        if let Some(path) = self.database_path.as_ref() {
            if let Ok(database) = crate::database::connect(path) {
                let _ = crate::database::repositories::entry_query::EntryQueryRepository::new(
                    &database,
                )
                .log_event(entry_id, kind, actor, payload, now);
            }
            return;
        }
        #[cfg(not(test))]
        return;
        #[cfg(test)]
        let event_id = uuid::Uuid::now_v7().to_string();
        #[cfg(test)]
        let _ = self.conn.execute(
            "INSERT INTO entry_events (id, entry_id, kind, actor, payload, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
            params![event_id, entry_id, kind, actor, payload, now as i64],
        );
    }

    pub fn get_entry_detail(&self, id: &str) -> Result<EntryDetail, String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            return self.get_entry_detail_with_database(&database, id);
        }
        #[cfg(not(test))]
        return Err("database path unavailable for entry detail".into());
        #[cfg(test)]
        self.get_entry_detail_legacy(id)
    }

    #[cfg(test)]
    fn get_entry_detail_legacy(&self, id: &str) -> Result<EntryDetail, String> {
        let entry = self.conn.query_row(
            "SELECT id, started_at, ended_at, description, category_id, project_id, status, approved_by, source, billable, invoice_id, created_at, updated_at, deleted_at, description_origin
             FROM time_entries WHERE id = ?1;",
            params![id],
            time_entry_from_row,
        ).map_err(|e| e.to_string())?;

        let mut stmt = self.conn.prepare(
            "SELECT id, app, title, kind, label, started_at, ended_at, reviewed, app_id, bundle_id, url, domain, entry_id
             FROM segments
             WHERE entry_id = ?1 OR (entry_id IS NULL AND started_at >= ?2 AND ended_at <= ?3 AND kind != 'break')
             ORDER BY started_at ASC;",
        ).map_err(|e| e.to_string())?;

        let segment_rows = stmt
            .query_map(
                params![id, entry.started_at as i64, entry.ended_at as i64],
                segment_from_row,
            )
            .map_err(|e| e.to_string())?;

        let mut segments = Vec::new();
        let mut app_duration: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();
        let mut title_duration: std::collections::HashMap<(String, String), (u64, u64)> =
            std::collections::HashMap::new();
        let mut total_work_ms = 0u64;

        for s in segment_rows {
            let seg = s.map_err(|e| e.to_string())?;
            if seg.kind != "break" {
                let dur = seg
                    .ended_at
                    .unwrap_or(entry.ended_at)
                    .saturating_sub(seg.started_at);
                *app_duration.entry(seg.app.clone()).or_insert(0) += dur;
                total_work_ms += dur;

                let key = (seg.title.clone(), seg.app.clone());
                let entry_td = title_duration.entry(key).or_insert((seg.started_at, 0));
                entry_td.1 += dur;
            }
            segments.push(seg);
        }

        let mut apps = Vec::new();
        for (app, dur) in app_duration {
            let percentage = if total_work_ms > 0 {
                (dur as f64 / total_work_ms as f64) * 100.0
            } else {
                0.0
            };
            apps.push(AppContribution {
                app,
                duration_ms: dur,
                percentage,
            });
        }
        apps.sort_by_key(|a| std::cmp::Reverse(a.duration_ms));

        let mut titles = Vec::new();
        for ((title, app), (started_at, dur)) in title_duration {
            titles.push(TitleItem {
                title,
                app,
                started_at,
                duration_ms: dur,
            });
        }
        titles.sort_by_key(|a| std::cmp::Reverse(a.duration_ms));

        let mut evt_stmt = self
            .conn
            .prepare(
                "SELECT id, entry_id, kind, actor, payload, at
                 FROM entry_events
                 WHERE entry_id = ?1
                 ORDER BY at ASC;",
            )
            .map_err(|e| e.to_string())?;

        let event_rows = evt_stmt
            .query_map(params![id], |row| {
                Ok(EntryEvent {
                    id: row.get(0)?,
                    entry_id: row.get(1)?,
                    kind: row.get(2)?,
                    actor: row.get(3)?,
                    payload: row.get(4)?,
                    at: row.get::<_, i64>(5)? as u64,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut events = Vec::new();
        for evt in event_rows {
            events.push(evt.map_err(|e| e.to_string())?);
        }

        let (suggestions, job) = if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let repository = crate::database::repositories::ai::AiRepository::new(&database);
            (
                repository
                    .latest_suggestions(id)
                    .map_err(|error| error.to_string())?,
                repository.job_for(id).map_err(|error| error.to_string())?,
            )
        } else {
            (
                crate::ai::store::latest_suggestions(&self.conn, id)?,
                crate::ai::store::job_for(&self.conn, id)?,
            )
        };
        let rule_suggestion = crate::ai::store::rule_suggestion(
            &self.conn,
            id,
            entry.category_id.as_deref(),
            entry.project_id.as_deref(),
            &suggestions,
        )?;
        Ok(EntryDetail {
            entry,
            segments,
            apps,
            titles,
            events,
            suggestions,
            rule_suggestion,
            job,
        })
    }

    fn get_entry_detail_with_database(
        &self,
        database: &crate::database::DatabaseConnection,
        id: &str,
    ) -> Result<EntryDetail, String> {
        let query = crate::database::repositories::entry_query::EntryQueryRepository::new(database);
        let entry = query.get(id).map_err(|error| error.to_string())?;
        let models = query
            .detail_segments(id, entry.started_at as i64, entry.ended_at as i64)
            .map_err(|error| error.to_string())?;
        let segments = models
            .into_iter()
            .map(segment_from_model)
            .collect::<Vec<_>>();
        let mut apps = std::collections::HashMap::<String, u64>::new();
        let mut titles = std::collections::HashMap::<(String, String), (u64, u64)>::new();
        let mut total_work_ms = 0;
        for segment in &segments {
            if segment.kind == "break" {
                continue;
            }
            let duration = segment
                .ended_at
                .unwrap_or(entry.ended_at)
                .saturating_sub(segment.started_at);
            *apps.entry(segment.app.clone()).or_default() += duration;
            total_work_ms += duration;
            let slot = titles
                .entry((segment.title.clone(), segment.app.clone()))
                .or_insert((segment.started_at, 0));
            slot.1 += duration;
        }
        let mut app_totals = apps
            .into_iter()
            .map(|(app, duration_ms)| AppContribution {
                percentage: if total_work_ms == 0 {
                    0.0
                } else {
                    duration_ms as f64 / total_work_ms as f64 * 100.0
                },
                app,
                duration_ms,
            })
            .collect::<Vec<_>>();
        app_totals.sort_by_key(|item| std::cmp::Reverse(item.duration_ms));
        let mut title_totals = titles
            .into_iter()
            .map(|((title, app), (started_at, duration_ms))| TitleItem {
                title,
                app,
                started_at,
                duration_ms,
            })
            .collect::<Vec<_>>();
        title_totals.sort_by_key(|item| std::cmp::Reverse(item.duration_ms));
        let events = query.events(id).map_err(|error| error.to_string())?;
        let ai = crate::database::repositories::ai::AiRepository::new(database);
        let suggestions = ai
            .latest_suggestions(id)
            .map_err(|error| error.to_string())?;
        let rule_suggestion = ai
            .rule_suggestion(
                id,
                entry.category_id.as_deref(),
                entry.project_id.as_deref(),
                &suggestions,
            )
            .map_err(|error| error.to_string())?;
        let job = ai.job_for(id).map_err(|error| error.to_string())?;
        Ok(EntryDetail {
            entry,
            segments,
            apps: app_totals,
            titles: title_totals,
            events,
            suggestions,
            rule_suggestion,
            job,
        })
    }

    pub fn update_time_entry(
        &mut self,
        id: &str,
        patch: UpdateTimeEntry,
        now: u64,
    ) -> Result<TimeEntry, String> {
        let mut updated = self.update_time_entries(&[id.to_string()], patch, now)?;
        Ok(updated.remove(0))
    }

    /// One patch applied to many entries, as one undoable change.
    pub fn update_time_entries(
        &mut self,
        ids: &[String],
        patch: UpdateTimeEntry,
        now: u64,
    ) -> Result<Vec<TimeEntry>, String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let repository =
                crate::database::repositories::entry_mutations::EntryMutationRepository::new(
                    &database,
                );
            let before = ids
                .iter()
                .map(|id| {
                    crate::database::entities::entry_query::entries::Entity::find_by_id(id.clone())
                        .one(&database)
                        .map_err(|error| error.to_string())?
                        .ok_or_else(|| "time entry not found".to_owned())
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mut result = Vec::with_capacity(ids.len());
            let mut after = Vec::with_capacity(ids.len());
            for id in ids {
                repository
                    .update(id, patch.clone(), now)
                    .map_err(|error| error.to_string())?;
                self.log_event(id, "edited", "user", None, now);
                after.push(
                    crate::database::entities::entry_query::entries::Entity::find_by_id(id.clone())
                        .one(&database)
                        .map_err(|error| error.to_string())?
                        .ok_or_else(|| "time entry not found".to_owned())?,
                );
                result.push(
                    crate::database::repositories::entry_query::EntryQueryRepository::new(
                        &database,
                    )
                    .get(id)
                    .map_err(|error| error.to_string())?,
                );
            }
            let changes: Vec<super::typed_history::TypedChange> = ids
                .iter()
                .zip(before)
                .zip(after)
                .filter_map(|((id, before), after)| {
                    (before != after).then_some(super::typed_history::TypedChange {
                        id: id.clone(),
                        before: Some(before),
                        after,
                    })
                })
                .collect();
            if !changes.is_empty() {
                self.record_undo(super::history::Command::TypedEdit(
                    super::typed_history::TypedEdit {
                        label: "Edit",
                        changes,
                    },
                ));
            }
            return Ok(result);
        }
        #[cfg(test)]
        return self.edit("Edit", ids, |store| {
            ids.iter()
                .map(|id| store.patch_time_entry(id, patch.clone(), now))
                .collect()
        });
        #[cfg(not(test))]
        Err("database path unavailable for entry mutation".into())
    }

    #[cfg(test)]
    fn patch_time_entry(
        &mut self,
        id: &str,
        patch: UpdateTimeEntry,
        now: u64,
    ) -> Result<TimeEntry, String> {
        let mut entry = self.conn.query_row(
            "SELECT id, started_at, ended_at, description, category_id, project_id, status, approved_by, source, billable, invoice_id, created_at, updated_at, deleted_at, description_origin
             FROM time_entries WHERE id = ?1;",
            params![id],
            time_entry_from_row,
        ).map_err(|e| e.to_string())?;
        if entry.status == "approved" {
            return Err(LOCKED.to_string());
        }

        if let Some(desc) = patch.description {
            if desc != entry.description {
                entry.description_origin = "user".to_string();
            }
            entry.description = desc;
        }
        if let Some(cat) = patch.category_id {
            entry.category_id = if cat.is_empty() { None } else { Some(cat) };
        }
        if let Some(proj) = patch.project_id {
            entry.project_id = if proj.is_empty() { None } else { Some(proj) };
        }
        if let Some(s) = patch.started_at {
            entry.started_at = s;
        }
        if let Some(e) = patch.ended_at {
            entry.ended_at = e;
        }
        if let Some(st) = patch.status {
            entry.status = st;
        }
        if let Some(b) = patch.billable {
            entry.billable = b;
        }
        let was_live = entry.status == "building";
        // Setting the live session's times says when it really ran: it
        // closes there, and capture carries on in a new session.
        if was_live && (patch.started_at.is_some() || patch.ended_at.is_some()) {
            entry.status = "pending".to_string();
        }
        if entry.ended_at <= entry.started_at {
            return Err("Entry end must be after its start".to_string());
        }
        entry.updated_at = now;

        self.conn.execute(
            "UPDATE time_entries SET started_at = ?1, ended_at = ?2, description = ?3, category_id = ?4, project_id = ?5, status = ?6, billable = ?7, updated_at = ?8, description_origin = ?9
             WHERE id = ?10;",
            params![
                entry.started_at as i64,
                entry.ended_at as i64,
                entry.description,
                entry.category_id,
                entry.project_id,
                entry.status,
                entry.billable as i64,
                now as i64,
                entry.description_origin,
                id,
            ],
        ).map_err(|e| e.to_string())?;

        // Re-categorizing an entry the AI already had approved (or the user
        // accepted) is a correction: the suggestion becomes `changed`.
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            crate::database::repositories::ai::AiRepository::new(&database)
                .record_edit(
                    id,
                    entry.category_id.as_deref(),
                    entry.project_id.as_deref(),
                    now as i64,
                )
                .map_err(|error| error.to_string())?;
        } else {
            crate::ai::store::record_edit(
                &self.conn,
                id,
                entry.category_id.as_deref(),
                entry.project_id.as_deref(),
                now,
            )?;
        }
        if was_live && entry.status != "building" {
            if let Some(path) = self.database_path.as_ref() {
                let database = crate::database::connect(path).map_err(|error| error.to_string())?;
                crate::database::repositories::activity::ActivityRepository::new(&database)
                    .describe_closed(id)
                    .map_err(|error| error.to_string())?;
            } else {
                self.describe_closed(id)?;
            }
        }

        self.log_event(id, "edited", "user", None, now);

        self.time_entry(id)
    }

    pub fn approve_time_entries(
        &mut self,
        ids: &[String],
        approved_by: &str,
        now: u64,
    ) -> Result<(), String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let before = ids
                .iter()
                .map(|id| {
                    crate::database::entities::entry_query::entries::Entity::find_by_id(id.clone())
                        .one(&database)
                        .map_err(|error| error.to_string())?
                        .ok_or_else(|| "time entry not found".to_owned())
                })
                .collect::<Result<Vec<_>, _>>()?;
            let rows =
                crate::database::repositories::entry_mutations::EntryMutationRepository::new(
                    &database,
                )
                .approve(ids, approved_by, now)
                .map_err(|error| error.to_string())?;
            for row in rows {
                self.log_event(&row.id, "accepted", approved_by, None, now);
            }
            let after = ids
                .iter()
                .map(|id| {
                    crate::database::entities::entry_query::entries::Entity::find_by_id(id.clone())
                        .one(&database)
                        .map_err(|error| error.to_string())?
                        .ok_or_else(|| "time entry not found".to_owned())
                })
                .collect::<Result<Vec<_>, _>>()?;
            let changes: Vec<super::typed_history::TypedChange> = ids
                .iter()
                .zip(before)
                .zip(after)
                .filter_map(|((id, before), after)| {
                    (before != after).then_some(super::typed_history::TypedChange {
                        id: id.clone(),
                        before: Some(before),
                        after,
                    })
                })
                .collect();
            if !changes.is_empty() {
                self.record_undo(super::history::Command::TypedEdit(
                    super::typed_history::TypedEdit {
                        label: "Accept",
                        changes,
                    },
                ));
            }
            return Ok(());
        }
        #[cfg(test)]
        return self.edit("Accept", ids, |store| store.approve(ids, approved_by, now));
        #[cfg(not(test))]
        Err("database path unavailable for entry mutation".into())
    }

    #[cfg(test)]
    fn approve(&mut self, ids: &[String], approved_by: &str, now: u64) -> Result<(), String> {
        for id in ids {
            let entry = self.time_entry(id)?;
            if entry.status == "approved" {
                continue;
            }
            // Approval is final: the live session closes now, and capture
            // carries on in a new one.
            let ended_at = if entry.status == "building" {
                entry.ended_at.max(now)
            } else {
                entry.ended_at
            };
            self.conn.execute(
                "UPDATE time_entries SET status = 'approved', approved_by = ?1, ended_at = ?2, updated_at = ?3 WHERE id = ?4;",
                params![approved_by, ended_at as i64, now as i64, id],
            ).map_err(|e| e.to_string())?;
            if entry.status == "building" {
                self.describe_closed(id)?;
            }

            // Feedback loop: the suggestion's outcome (accepted or changed)
            // feeds calibration and rule suggestions, and the entry joins the
            // kNN store as soon as it has a vector.
            let has_vector = if let Some(path) = self.database_path.as_ref() {
                let database = crate::database::connect(path).map_err(|error| error.to_string())?;
                let repository = crate::database::repositories::ai::AiRepository::new(&database);
                repository
                    .record_approval(
                        id,
                        entry.category_id.as_deref(),
                        entry.project_id.as_deref(),
                        now as i64,
                    )
                    .map_err(|error| error.to_string())?;
                repository
                    .has_embedding(id)
                    .map_err(|error| error.to_string())?
            } else {
                crate::ai::store::record_approval(
                    &self.conn,
                    id,
                    entry.category_id.as_deref(),
                    entry.project_id.as_deref(),
                    now,
                )?;
                self.conn
                    .query_row(
                        "SELECT EXISTS (SELECT 1 FROM entry_embeddings WHERE entry_id = ?1);",
                        params![id],
                        |row| row.get(0),
                    )
                    .map_err(|e| e.to_string())?
            };
            if !has_vector {
                crate::ai::store::enqueue(&self.conn, id, crate::ai::store::JOB_EMBED, now)?;
            }

            self.log_event(id, "accepted", approved_by, None, now);
        }
        Ok(())
    }

    /// Reject clears the suggestion and leaves the entry pending with no
    /// category, recording negative feedback. A project the AI pre-filled is
    /// cleared too; one the user picked stays.
    pub fn reject_time_entry(&mut self, id: &str, now: u64) -> Result<(), String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let before =
                crate::database::entities::entry_query::entries::Entity::find_by_id(id.to_owned())
                    .one(&database)
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| "time entry not found".to_owned())?;
            crate::database::repositories::entry_mutations::EntryMutationRepository::new(&database)
                .reject(id, now)
                .map_err(|error| error.to_string())?;
            self.log_event(id, "rejected", "user", None, now);
            let after =
                crate::database::entities::entry_query::entries::Entity::find_by_id(id.to_owned())
                    .one(&database)
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| "time entry not found".to_owned())?;
            if before != after {
                self.record_undo(super::history::Command::TypedEdit(
                    super::typed_history::TypedEdit {
                        label: "Reject",
                        changes: vec![super::typed_history::TypedChange {
                            id: id.to_owned(),
                            before: Some(before),
                            after,
                        }],
                    },
                ));
            }
            return Ok(());
        }
        #[cfg(test)]
        return self.edit("Reject", &[id.to_string()], |store| store.reject(id, now));
        #[cfg(not(test))]
        Err("database path unavailable for entry mutation".into())
    }

    /// The live session keeps recording: only its suggestion goes.
    #[cfg(test)]
    fn reject(&mut self, id: &str, now: u64) -> Result<(), String> {
        let entry = self.time_entry(id)?;
        if entry.status == "approved" {
            return Err(LOCKED.to_string());
        }
        let rejected = if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            crate::database::repositories::ai::AiRepository::new(&database)
                .record_rejection(id, now as i64)
                .map_err(|error| error.to_string())?
        } else {
            crate::ai::store::record_rejection(&self.conn, id, now)?
        };
        let prefilled_project = rejected.iter().any(|s| {
            s.field == aevio_core::ai::FIELD_PROJECT
                && s.value_id.is_some()
                && s.value_id == entry.project_id
        });
        self.conn
            .execute(
                "UPDATE time_entries SET category_id = NULL,
                   project_id = CASE WHEN ?3 THEN NULL ELSE project_id END,
                   status = CASE WHEN status = 'building' THEN 'building' ELSE 'pending' END,
                   approved_by = NULL, updated_at = ?1
                 WHERE id = ?2;",
                params![now as i64, id, prefilled_project],
            )
            .map_err(|e| e.to_string())?;

        self.log_event(id, "rejected", "user", None, now);
        Ok(())
    }

    /// Returns approved entries to pending, keeping their category and
    /// project so they can be edited and approved again. Invoiced time stays
    /// frozen. Does not re-run AI classification.
    pub fn unapprove_time_entries(&mut self, ids: &[String], now: u64) -> Result<(), String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let approved_ids = ids
                .iter()
                .filter_map(|id| {
                    self.time_entry(id)
                        .ok()
                        .filter(|entry| entry.status == "approved")
                        .map(|_| id.clone())
                })
                .collect::<std::collections::HashSet<_>>();
            let before = ids
                .iter()
                .map(|id| {
                    crate::database::entities::entry_query::entries::Entity::find_by_id(id.clone())
                        .one(&database)
                        .map_err(|error| error.to_string())?
                        .ok_or_else(|| "time entry not found".to_owned())
                })
                .collect::<Result<Vec<_>, _>>()?;
            let rows =
                crate::database::repositories::entry_mutations::EntryMutationRepository::new(
                    &database,
                )
                .unapprove(ids, now)
                .map_err(|error| error.to_string())?;
            for row in rows {
                if approved_ids.contains(&row.id) {
                    self.log_event(&row.id, "unapproved", "user", None, now);
                }
            }
            let after = ids
                .iter()
                .map(|id| {
                    crate::database::entities::entry_query::entries::Entity::find_by_id(id.clone())
                        .one(&database)
                        .map_err(|error| error.to_string())?
                        .ok_or_else(|| "time entry not found".to_owned())
                })
                .collect::<Result<Vec<_>, _>>()?;
            let changes: Vec<super::typed_history::TypedChange> = ids
                .iter()
                .zip(before)
                .zip(after)
                .filter_map(|((id, before), after)| {
                    (before != after).then_some(super::typed_history::TypedChange {
                        id: id.clone(),
                        before: Some(before),
                        after,
                    })
                })
                .collect();
            if !changes.is_empty() {
                self.record_undo(super::history::Command::TypedEdit(
                    super::typed_history::TypedEdit {
                        label: "Unapprove",
                        changes,
                    },
                ));
            }
            return Ok(());
        }
        #[cfg(test)]
        return self.edit("Unapprove", ids, |store| store.unapprove(ids, now));
        #[cfg(not(test))]
        Err("database path unavailable for entry mutation".into())
    }

    #[cfg(test)]
    fn unapprove(&mut self, ids: &[String], now: u64) -> Result<(), String> {
        for id in ids {
            let entry = self.time_entry(id)?;
            if entry.invoice_id.is_some() {
                return Err("Invoiced entries cannot be unapproved".to_string());
            }
            if entry.status != "approved" {
                continue;
            }
            self.conn
                .execute(
                    "UPDATE time_entries SET status = 'pending', approved_by = NULL, updated_at = ?1 WHERE id = ?2;",
                    params![now as i64, id],
                )
                .map_err(|e| e.to_string())?;
            self.log_event(id, "unapproved", "user", None, now);
        }
        Ok(())
    }

    /// Queues an entry for (re)classification, e.g. "Couldn't categoaevio ·
    /// Retry".
    pub fn queue_classification(&mut self, id: &str, now: u64) -> Result<(), String> {
        let entry = self.time_entry(id)?;
        if entry.status == "approved" {
            return Err(LOCKED.to_string());
        }
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            crate::database::repositories::ai::AiRepository::new(&database)
                .enqueue(
                    id,
                    crate::database::repositories::ai::JOB_CLASSIFY,
                    now as i64,
                )
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
        #[cfg(not(test))]
        return Err("database path unavailable for classification queue".into());
        #[cfg(test)]
        crate::ai::store::enqueue(&self.conn, id, crate::ai::store::JOB_CLASSIFY, now)
    }

    /// Accepts or dismisses an inline rule suggestion. Accepting creates a
    /// `suggested` rule; dismissing stores a disabled one so the same prompt
    /// never comes back.
    pub fn resolve_rule_suggestion(
        &mut self,
        suggestion: &aevio_core::models::RuleSuggestion,
        accept: bool,
        now: u64,
    ) -> Result<(), String> {
        let field = aevio_core::ai::Field::parse(&suggestion.field)?;
        if !matches!(suggestion.match_kind.as_str(), "app" | "domain") {
            return Err(format!("unsupported rule kind {}", suggestion.match_kind));
        }
        let value = suggestion.value_id.as_deref();
        let (category, project) = match field {
            aevio_core::ai::Field::Category => (value, None),
            aevio_core::ai::Field::Project => (None, value),
        };
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            crate::database::repositories::ai::AiRepository::new(&database)
                .create_rule(crate::database::repositories::ai::RuleInput {
                    match_kind: &suggestion.match_kind,
                    pattern: &suggestion.pattern,
                    category_id: category,
                    project_id: project,
                    origin: if accept { "suggested" } else { "dismissed" },
                    enabled: accept,
                    now: now as i64,
                })
                .map_err(|error| error.to_string())?;
        } else {
            #[cfg(not(test))]
            return Err("database path unavailable for rule persistence".into());
            #[cfg(test)]
            crate::ai::store::create_rule(
                &self.conn,
                &suggestion.match_kind,
                &suggestion.pattern,
                category,
                project,
                if accept { "suggested" } else { "dismissed" },
                accept,
                now,
            )?;
        }
        Ok(())
    }

    pub fn delete_time_entry(&mut self, id: &str, now: u64) -> Result<(), String> {
        self.delete_time_entries(&[id.to_string()], now)
    }

    pub fn delete_time_entries(&mut self, ids: &[String], now: u64) -> Result<(), String> {
        self.delete_undoable(ids, None, None, now)
    }

    pub fn create_manual_entry(
        &mut self,
        new_entry: NewTimeEntry,
        now: u64,
    ) -> Result<TimeEntry, String> {
        let id = uuid::Uuid::now_v7().to_string();
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let after =
                crate::database::repositories::entry_mutations::EntryMutationRepository::new(
                    &database,
                )
                .create(&id, &new_entry, now)
                .map_err(persistence_error)?;
            let entry =
                crate::database::repositories::entry_query::EntryQueryRepository::new(&database)
                    .get(&id)
                    .map_err(|error| error.to_string())?;
            self.log_event(&id, "created", "user", None, now);
            self.record_undo(super::history::Command::TypedEdit(
                super::typed_history::TypedEdit {
                    label: "Add",
                    changes: vec![super::typed_history::TypedChange {
                        id: id.clone(),
                        before: None,
                        after,
                    }],
                },
            ));
            return Ok(entry);
        }
        #[cfg(test)]
        return self.edit("Add", std::slice::from_ref(&id), |store| {
            store.insert_manual_entry(&id, new_entry, now)
        });
        #[cfg(not(test))]
        Err("database path unavailable for entry mutation".into())
    }

    #[cfg(test)]
    fn insert_manual_entry(
        &mut self,
        id: &str,
        new_entry: NewTimeEntry,
        now: u64,
    ) -> Result<TimeEntry, String> {
        if new_entry.ended_at <= new_entry.started_at {
            return Err("Entry end must be after its start".to_string());
        }
        let unclassified = new_entry.description.trim().is_empty()
            && new_entry.category_id.is_none()
            && new_entry.project_id.is_none();
        if new_entry.description.trim().is_empty() && !unclassified {
            return Err("Entry description cannot be empty".to_string());
        }
        ensure_no_approved_overlap(&self.conn, new_entry.started_at, new_entry.ended_at)?;
        let billable = new_entry.billable.unwrap_or(false) as i64;
        let pending = unclassified || new_entry.review;
        let status = if pending { "pending" } else { "approved" };
        let approved_by = if pending { None } else { Some("user") };

        self.conn.execute(
            "INSERT INTO time_entries (id, started_at, ended_at, description, category_id, project_id, status, approved_by, source, billable, created_at, updated_at, description_origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'manual', ?9, ?10, ?10, 'user');",
            params![
                id,
                new_entry.started_at as i64,
                new_entry.ended_at as i64,
                new_entry.description,
                new_entry.category_id,
                new_entry.project_id,
                status,
                approved_by,
                billable,
                now as i64,
            ],
        ).map_err(|e| e.to_string())?;

        self.log_event(id, "created", "user", None, now);
        if unclassified && new_entry.ended_at <= now {
            crate::ai::store::enqueue(&self.conn, id, crate::ai::store::JOB_CLASSIFY, now)?;
        } else if !pending {
            // A labeled hand-made entry contributes to kNN; one left for
            // review is embedded once it is approved.
            crate::ai::store::enqueue(&self.conn, id, crate::ai::store::JOB_EMBED, now)?;
        }

        Ok(TimeEntry {
            id: id.to_string(),
            started_at: new_entry.started_at,
            ended_at: new_entry.ended_at,
            description: new_entry.description,
            category_id: new_entry.category_id,
            project_id: new_entry.project_id,
            status: if unclassified && new_entry.ended_at <= now {
                "processing"
            } else {
                status
            }
            .to_string(),
            approved_by: approved_by.map(str::to_string),
            source: "manual".to_string(),
            billable: billable != 0,
            invoice_id: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            description_origin: "user".to_string(),
            ai: None,
            dominant_app: None,
        })
    }

    pub fn rebuild_time_entries_in_range(
        &mut self,
        start_ms: u64,
        end_ms: u64,
        now: u64,
    ) -> Result<Vec<TimeEntry>, String> {
        self.rebuild_range(start_ms, end_ms, now)?;
        self.list_time_entries(start_ms, end_ms)
    }

    /// Re-runs the entry builder over the range and saves the result.
    ///
    /// Only the open (`building`) entry is re-segmented; every closed entry is
    /// frozen, so its id, suggestion, and edits survive. An entry that has
    /// just closed is queued for classification. Returns whether anything a
    /// view shows changed (an entry closed, appeared, or went away), as
    /// opposed to the open entry merely growing.
    pub fn rebuild_range(&mut self, start_ms: u64, end_ms: u64, now: u64) -> Result<bool, String> {
        if let Some(path) = self.database_path.clone() {
            self.release_live_tail(now)?;
            let database = crate::database::connect(&path).map_err(|error| error.to_string())?;
            return crate::database::repositories::activity_rebuild::ActivityRebuildRepository::rebuild(
                &database,
                start_ms as i64,
                end_ms as i64,
                now as i64,
            )
            .map_err(|error| error.to_string());
        }
        #[cfg(not(test))]
        return Err("database path unavailable for activity rebuild".into());
        #[cfg(test)]
        {
            use aevio_core::entry_builder::{
                BuiltTimeEntry, EntrySettings, SegmentInput, build_entries,
            };

            self.release_live_tail(now)?;
            let tx = self.conn.transaction().map_err(|e| e.to_string())?;

            let segments: Vec<SegmentInput> = {
                let mut stmt = tx
                    .prepare(
                        "SELECT id, app, title, kind, label, started_at, ended_at, entry_id
                     FROM segments
                     WHERE started_at >= ?1 AND started_at <= ?2
                     ORDER BY started_at ASC;",
                    )
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map(params![start_ms as i64, end_ms as i64], |row| {
                        Ok(SegmentInput {
                            id: row.get(0)?,
                            app: row.get(1)?,
                            title: row.get(2)?,
                            kind: row.get(3)?,
                            label: row.get(4)?,
                            started_at: row.get::<_, i64>(5)? as u64,
                            ended_at: row.get::<_, Option<i64>>(6)?.map(|v| v as u64),
                            entry_id: row.get(7)?,
                        })
                    })
                    .map_err(|e| e.to_string())?;
                rows.collect::<rusqlite::Result<_>>()
                    .map_err(|e| e.to_string())?
            };

            // Every entry touching the range, deleted ones included: a deleted
            // entry's time stays unassigned instead of being rebuilt.
            let existing: Vec<TimeEntry> = {
                let mut stmt = tx
                .prepare(
                    "SELECT id, started_at, ended_at, description, category_id, project_id, status, approved_by, source, billable, invoice_id, created_at, updated_at, deleted_at, description_origin
                     FROM time_entries
                     WHERE ended_at >= ?1 AND started_at <= ?2;",
                )
                .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map(params![start_ms as i64, end_ms as i64], time_entry_from_row)
                    .map_err(|e| e.to_string())?;
                rows.collect::<rusqlite::Result<_>>()
                    .map_err(|e| e.to_string())?
            };
            let is_open = |e: &TimeEntry| e.deleted_at.is_none() && e.status == "building";
            let frozen: Vec<BuiltTimeEntry> = existing
                .iter()
                .filter(|e| !is_open(e))
                .map(|e| BuiltTimeEntry {
                    id: e.id.clone(),
                    started_at: e.started_at,
                    ended_at: e.ended_at,
                    description: e.description.clone(),
                    category_id: e.category_id.clone(),
                    project_id: e.project_id.clone(),
                    status: e.status.clone(),
                    approved_by: e.approved_by.clone(),
                    source: e.source.clone(),
                    billable: e.billable,
                    invoice_id: e.invoice_id.clone(),
                    created_at: e.created_at,
                    updated_at: e.updated_at,
                    deleted_at: e.deleted_at,
                    segment_ids: Vec::new(),
                })
                .collect();
            let frozen_ids: std::collections::HashSet<&str> =
                frozen.iter().map(|e| e.id.as_str()).collect();

            let built = build_entries(&segments, &frozen, &EntrySettings::default(), now);
            let linked: std::collections::HashMap<i64, Option<&str>> = segments
                .iter()
                .map(|s| (s.id, s.entry_id.as_deref()))
                .collect();

            let mut changed = false;
            let mut kept = std::collections::HashSet::new();
            for entry in built.iter().filter(|e| !frozen_ids.contains(e.id.as_str())) {
                kept.insert(entry.id.clone());
                let before = existing.iter().find(|e| e.id == entry.id);
                tx.execute(
                "INSERT INTO time_entries (id, started_at, ended_at, description, category_id, project_id, status, source, billable, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5, 'auto', 0, ?6, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                   started_at = excluded.started_at,
                   ended_at = excluded.ended_at,
                   status = excluded.status,
                   description = CASE WHEN time_entries.description_origin = 'template'
                                      THEN excluded.description ELSE time_entries.description END,
                   updated_at = excluded.updated_at;",
                params![
                    entry.id,
                    entry.started_at as i64,
                    entry.ended_at as i64,
                    entry.description,
                    entry.status,
                    now as i64,
                ],
            )
            .map_err(|e| e.to_string())?;
                // Only newly linked segments: a long session is rebuilt often.
                for segment_id in &entry.segment_ids {
                    if linked.get(segment_id) == Some(&Some(entry.id.as_str())) {
                        continue;
                    }
                    tx.execute(
                        "UPDATE segments SET entry_id = ?1 WHERE id = ?2;",
                        params![entry.id, segment_id],
                    )
                    .map_err(|e| e.to_string())?;
                }

                let status_changed =
                    before.map(|e| e.status.as_str()) != Some(entry.status.as_str());
                changed |= status_changed;
                if status_changed && entry.status == "pending" {
                    crate::ai::store::enqueue(&tx, &entry.id, crate::ai::store::JOB_CLASSIFY, now)?;
                }
            }

            // An open entry whose segments all moved elsewhere no longer exists.
            // It goes for good: marking it deleted would leave a tombstone that
            // keeps its span empty, and that span is still tracked time.
            for stale in existing
                .iter()
                .filter(|e| is_open(e) && !kept.contains(&e.id))
            {
                history::Stash::take(&tx, &stale.id)?;
                changed = true;
            }

            // Entries from before the AI pipeline existed (or whose job was lost)
            // get classified the first time their day is rebuilt.
            changed |= crate::ai::store::enqueue_unclassified(&tx, start_ms, end_ms, now)? > 0;

            tx.commit().map_err(|e| e.to_string())?;
            Ok(changed)
        }
    }

    /// Capture may still be writing a segment of a session the user closed
    /// (approved it, set its end, merged it into a finished entry). What it
    /// records past that entry's end belongs to the next session, so the
    /// segment is cut there rather than lost inside a frozen entry.
    fn release_live_tail(&mut self, now: u64) -> Result<(), String> {
        let Some(current) = self.current.as_ref() else {
            return Ok(());
        };
        if let Some(path) = self.database_path.clone() {
            let database = crate::database::connect(&path).map_err(|error| error.to_string())?;
            let released =
                crate::database::repositories::activity::ActivityRepository::new(&database)
                    .release_live_tail(current.id, now as i64)
                    .map_err(|error| error.to_string())?;
            if let Some((id, started_at)) = released
                && let Some(current) = self.current.as_mut()
            {
                current.id = id;
                current.started_at = started_at.max(0) as u64;
            }
            return Ok(());
        }
        #[cfg(not(test))]
        return Err("database path unavailable for activity tail release".into());
        #[cfg(test)]
        {
            let owner: Option<(String, i64)> = self
            .conn
            .query_row(
                "SELECT e.id, e.ended_at FROM segments s JOIN time_entries e ON e.id = s.entry_id
                 WHERE s.id = ?1 AND s.ended_at IS NULL
                   AND e.status != 'building' AND e.deleted_at IS NULL AND e.ended_at < ?2;",
                params![current.id, now as i64],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
            let Some((entry_id, ended_at)) = owner else {
                return Ok(());
            };
            if current.started_at >= ended_at as u64 {
                self.conn
                    .execute(
                        "UPDATE segments SET entry_id = NULL WHERE id = ?1;",
                        params![current.id],
                    )
                    .map_err(|e| e.to_string())?;
                return Ok(());
            }
            let id = current.id;
            self.cut_segment(id, &entry_id, ended_at as u64, None)
        }
    }

    // --- P1: Apps -------------------------------------------------------
}
