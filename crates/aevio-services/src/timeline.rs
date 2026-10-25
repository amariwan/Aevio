//! Personal, device-spanning activity timeline.
//!
//! Raw activity remains local. A publication is an explicit, user initiated
//! projection of one segment into a workspace. The table is deliberately
//! separate from `segments`, so group visibility can never be inferred from
//! capture data alone.

use serde::{Deserialize, Serialize};

use aevio_persistence::database::repositories::timeline::{SyncApply, TimelineRepository};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEvent {
    pub id: String,
    pub app: String,
    pub title: String,
    pub kind: String,
    pub domain: Option<String>,
    pub started_at: u64,
    pub ended_at: u64,
    pub duration_ms: u64,
    pub visibility: String,
    pub workspace_id: Option<String>,
    pub published_at: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishInput {
    pub segment_id: i64,
    pub workspace_id: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineDevice {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub last_seen_at: u64,
    pub tracking_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineSyncState {
    pub mode: String,
    pub server_url: Option<String>,
    pub status: String,
    pub pending_publications: u64,
    pub last_success_at: Option<u64>,
    pub last_error: Option<String>,
}

// Preserve the public desktop/CLI import path while sharing the wire contract.
pub use aevio_contracts::sync::{
    SyncChange, SyncOperation, SyncPullResponse, SyncPushRequest, SyncPushResponse,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineSyncInput {
    pub server_url: String,
    pub bearer_token: String,
    pub device_id: String,
    pub workspace_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineSyncResult {
    pub accepted: usize,
    pub conflicts: usize,
    pub applied: usize,
    pub cursor: u64,
}

pub fn list_events(
    db: &sea_orm::DatabaseConnection,
    start_ms: u64,
    end_ms: u64,
    now_ms: u64,
    workspace_id: Option<&str>,
) -> Result<Vec<TimelineEvent>, String> {
    if start_ms >= end_ms {
        return Err("timeline range must have a positive duration".into());
    }
    let (segments, publications, remote_rows) = TimelineRepository::new(db)
        .event_data(start_ms as i64, end_ms as i64, workspace_id)
        .map_err(|error| error.to_string())?;
    let publications = publications
        .into_iter()
        .map(|row| (row.segment_id, row))
        .collect::<std::collections::HashMap<_, _>>();
    let mut events = segments
        .into_iter()
        .map(|segment| {
            let ended_at = segment.ended_at.unwrap_or(now_ms as i64) as u64;
            let effective_start = (segment.started_at as u64).max(start_ms);
            let effective_end = ended_at.min(end_ms);
            let publication = publications.get(&segment.id);
            TimelineEvent {
                id: format!("segment-{}", segment.id),
                app: segment.app,
                title: publication
                    .and_then(|row| row.title_override.clone())
                    .unwrap_or(segment.title),
                kind: segment.kind,
                domain: segment.domain,
                started_at: segment.started_at as u64,
                ended_at,
                duration_ms: effective_end.saturating_sub(effective_start),
                visibility: publication
                    .map(|row| row.visibility.clone())
                    .unwrap_or_else(|| "private".to_owned()),
                workspace_id: publication.and_then(|row| row.workspace_id.clone()),
                published_at: publication
                    .and_then(|row| row.published_at.map(|value| value as u64)),
            }
        })
        .collect::<Vec<_>>();
    for remote in remote_rows {
        let event: TimelineEvent =
            serde_json::from_str(&remote.payload).map_err(|error| error.to_string())?;
        if event.started_at < end_ms && event.ended_at > start_ms {
            events.push(event);
        }
    }
    events.sort_by_key(|event| (event.started_at, event.id.clone()));
    Ok(events)
}

pub fn publish(
    db: &sea_orm::DatabaseConnection,
    input: PublishInput,
    now_ms: u64,
) -> Result<TimelineEvent, String> {
    let segment_id = input.segment_id;
    let repository = TimelineRepository::new(db);
    if !repository
        .segment_exists(segment_id)
        .map_err(|error| error.to_string())?
    {
        return Err("activity segment not found".into());
    }
    let workspace_id = input
        .workspace_id
        .as_deref()
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| "workspace id is required for shared activity".to_string())?;
    repository
        .publish(segment_id, workspace_id, input.title, now_ms as i64)
        .map_err(|error| error.to_string())?;
    let event = list_events(db, 0, now_ms.saturating_add(1), now_ms, None)?
        .into_iter()
        .find(|event| event.id == format!("segment-{segment_id}"))
        .ok_or_else(|| "published segment could not be read back".to_string())?;
    if let Some(_workspace_id) = &event.workspace_id {
        repository
            .enqueue_outbox(
                &event.id,
                "upsert",
                &serde_json::to_string(&event).map_err(|error| error.to_string())?,
                now_ms as i64,
            )
            .and_then(|_| repository.mark_pending())
            .map_err(|error| error.to_string())?;
    }
    Ok(event)
}

pub fn unpublish(
    db: &sea_orm::DatabaseConnection,
    segment_id: i64,
    now_ms: u64,
) -> Result<(), String> {
    let repository = TimelineRepository::new(db);
    let workspace_id = repository
        .unpublish(segment_id, now_ms as i64)
        .map_err(|error| error.to_string())?;
    if workspace_id.is_some() {
        repository
            .enqueue_outbox(
                &format!("segment-{segment_id}"),
                "delete",
                &serde_json::json!({ "id": format!("segment-{segment_id}") }).to_string(),
                now_ms as i64,
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn list_devices(db: &sea_orm::DatabaseConnection) -> Result<Vec<TimelineDevice>, String> {
    TimelineRepository::new(db)
        .list_devices()
        .map_err(|error| error.to_string())
        .map(|rows| rows.into_iter().map(device_from_model).collect())
}

pub fn register_device(
    db: &sea_orm::DatabaseConnection,
    device_id: &str,
    now_ms: u64,
) -> Result<(), String> {
    TimelineRepository::new(db)
        .register_device(device_id, now_ms as i64, std::env::consts::OS)
        .map_err(|error| error.to_string())
}

fn device_from_model(
    model: aevio_persistence::database::entities::timeline_devices::Model,
) -> TimelineDevice {
    TimelineDevice {
        id: model.id,
        name: model.name,
        platform: model.platform,
        last_seen_at: model.last_seen_at as u64,
        tracking_enabled: model.tracking_enabled != 0,
    }
}

pub fn sync_state(db: &sea_orm::DatabaseConnection) -> Result<TimelineSyncState, String> {
    let (row, pending_publications) = TimelineRepository::new(db)
        .sync_state()
        .map_err(|error| error.to_string())?;
    let mut state = row
        .map(|row| TimelineSyncState {
            mode: row.mode,
            server_url: row.server_url,
            status: row.status,
            pending_publications: 0,
            last_success_at: row.last_success_at.map(|value| value as u64),
            last_error: row.last_error,
        })
        .unwrap_or(TimelineSyncState {
            mode: "standalone".into(),
            server_url: None,
            status: "local".into(),
            pending_publications: 0,
            last_success_at: None,
            last_error: None,
        });
    state.pending_publications = pending_publications;
    Ok(state)
}

pub fn pending_operations(
    db: &sea_orm::DatabaseConnection,
    workspace_id: &str,
) -> Result<Vec<SyncOperation>, String> {
    TimelineRepository::new(db)
        .pending_operations()
        .map_err(|error| error.to_string())
        .map(|rows| {
            rows.into_iter()
                .map(|row| SyncOperation {
                    operation_id: row.id,
                    workspace_id: workspace_id.to_owned(),
                    entity_type: row.entity_type,
                    entity_id: row.entity_id,
                    operation: row.operation,
                    payload: serde_json::from_str(&row.payload).unwrap_or_default(),
                    base_revision: None,
                })
                .collect()
        })
}

pub fn sync_cursor(db: &sea_orm::DatabaseConnection, workspace_id: &str) -> Result<u64, String> {
    TimelineRepository::new(db)
        .cursor(workspace_id)
        .map_err(|error| error.to_string())
}

pub fn set_sync_connecting(
    db: &sea_orm::DatabaseConnection,
    server_url: &str,
) -> Result<(), String> {
    TimelineRepository::new(db)
        .set_connecting(server_url)
        .map_err(|error| error.to_string())
}

pub fn apply_sync_response(
    db: &sea_orm::DatabaseConnection,
    workspace_id: &str,
    push: &SyncPushResponse,
    pull: &SyncPullResponse,
    now_ms: u64,
) -> Result<usize, String> {
    let mut remote_upserts = Vec::new();
    let mut remote_deletes = Vec::new();
    for change in &pull.changes {
        if push.accepted.contains(&change.operation.operation_id) {
            continue;
        }
        let id = format!("remote-{}", change.operation.operation_id);
        if change.operation.operation == "delete" {
            remote_deletes.push(id);
        } else {
            remote_upserts.push((
                id,
                change.operation.workspace_id.clone(),
                change.operation.payload.to_string(),
            ));
        }
    }
    TimelineRepository::new(db)
        .apply_sync(SyncApply {
            accepted: &push.accepted,
            conflicts: &push.conflicts,
            remote_upserts: &remote_upserts,
            remote_deletes: &remote_deletes,
            workspace_id,
            cursor: pull.cursor,
            now_ms: now_ms as i64,
        })
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aevio_persistence::database::entities::{
        timeline_publications, timeline_remote_events, timeline_segments, timeline_sync_cursors,
        timeline_sync_outbox, timeline_sync_state,
    };
    use sea_orm::{ActiveValue::Set, ConnectionTrait, Database, DbBackend, EntityTrait, Schema};

    fn database() -> sea_orm::DatabaseConnection {
        let database = Database::connect("sqlite::memory:").unwrap();
        let schema = Schema::new(DbBackend::Sqlite);
        for statement in [
            schema.create_table_from_entity(timeline_segments::Entity),
            schema.create_table_from_entity(timeline_publications::Entity),
            schema.create_table_from_entity(timeline_sync_outbox::Entity),
            schema.create_table_from_entity(timeline_sync_state::Entity),
            schema.create_table_from_entity(timeline_sync_cursors::Entity),
            schema.create_table_from_entity(timeline_remote_events::Entity),
        ] {
            database.execute(&statement).unwrap();
        }
        database
    }

    #[test]
    fn segments_are_private_until_explicitly_published() {
        let conn = database();
        timeline_segments::Entity::insert(timeline_segments::ActiveModel {
            app: Set("Editor".into()),
            title: Set("Private draft".into()),
            kind: Set("app".into()),
            started_at: Set(100),
            ended_at: Set(Some(500)),
            domain: Set(None),
            id: Set(1),
        })
        .exec(&conn)
        .unwrap();
        let before = list_events(&conn, 0, 1_000, 1_000, None).unwrap();
        assert_eq!(before[0].visibility, "private");
        let published = publish(
            &conn,
            PublishInput {
                segment_id: 1,
                workspace_id: Some("team".into()),
                title: Some("Project work".into()),
            },
            2_000,
        )
        .unwrap();
        assert_eq!(published.visibility, "shared");
        assert_eq!(published.title, "Project work");
        assert_eq!(TimelineRepository::new(&conn).sync_state().unwrap().1, 1);
        unpublish(&conn, 1, 3_000).unwrap();
        assert_eq!(
            list_events(&conn, 0, 1_000, 1_000, None).unwrap()[0].visibility,
            "private"
        );
        assert_eq!(
            TimelineRepository::new(&conn)
                .pending_operations()
                .unwrap()
                .len(),
            2
        );
    }
}
