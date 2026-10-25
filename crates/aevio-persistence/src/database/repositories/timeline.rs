use sea_orm::entity::prelude::*;
use sea_orm::sea_query::Condition;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveValue::Set, DatabaseConnection, DbErr, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};

use crate::database::entities::{
    timeline_devices, timeline_publications, timeline_remote_events, timeline_segments,
    timeline_sync_cursors, timeline_sync_outbox, timeline_sync_state,
};

pub struct TimelineRepository<'db> {
    db: &'db DatabaseConnection,
}

pub type EventData = (
    Vec<timeline_segments::Model>,
    Vec<timeline_publications::Model>,
    Vec<timeline_remote_events::Model>,
);

pub struct SyncApply<'a> {
    pub accepted: &'a [String],
    pub conflicts: &'a [String],
    pub remote_upserts: &'a [(String, String, String)],
    pub remote_deletes: &'a [String],
    pub workspace_id: &'a str,
    pub cursor: u64,
    pub now_ms: i64,
}

impl<'db> TimelineRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn list_devices(&self) -> Result<Vec<timeline_devices::Model>, DbErr> {
        timeline_devices::Entity::find()
            .filter(timeline_devices::Column::RevokedAt.is_null())
            .order_by_desc(timeline_devices::Column::LastSeenAt)
            .all(self.db)
    }

    pub fn event_data(
        &self,
        start_ms: i64,
        end_ms: i64,
        workspace_id: Option<&str>,
    ) -> Result<EventData, DbErr> {
        let segments = timeline_segments::Entity::find()
            .filter(timeline_segments::Column::StartedAt.lt(end_ms))
            .filter(
                Condition::any()
                    .add(timeline_segments::Column::EndedAt.is_null())
                    .add(timeline_segments::Column::EndedAt.gt(start_ms)),
            )
            .order_by_asc(timeline_segments::Column::StartedAt)
            .order_by_asc(timeline_segments::Column::Id)
            .all(self.db)?;
        let ids = segments.iter().map(|row| row.id).collect::<Vec<_>>();
        let publications = if ids.is_empty() {
            Vec::new()
        } else {
            timeline_publications::Entity::find()
                .filter(timeline_publications::Column::SegmentId.is_in(ids))
                .all(self.db)?
        };
        let remote = match workspace_id {
            Some(workspace_id) => timeline_remote_events::Entity::find()
                .filter(timeline_remote_events::Column::DeletedAt.is_null())
                .filter(timeline_remote_events::Column::WorkspaceId.eq(workspace_id.to_owned()))
                .all(self.db)?,
            None => Vec::new(),
        };
        Ok((segments, publications, remote))
    }

    pub fn segment_exists(&self, segment_id: i64) -> Result<bool, DbErr> {
        Ok(timeline_segments::Entity::find_by_id(segment_id)
            .one(self.db)?
            .is_some())
    }

    pub fn publish(
        &self,
        segment_id: i64,
        workspace_id: &str,
        title: Option<String>,
        now_ms: i64,
    ) -> Result<(), DbErr> {
        timeline_publications::Entity::insert(timeline_publications::ActiveModel {
            segment_id: Set(segment_id),
            visibility: Set("shared".to_owned()),
            workspace_id: Set(Some(workspace_id.to_owned())),
            title_override: Set(title),
            published_at: Set(Some(now_ms)),
            updated_at: Set(now_ms),
        })
        .on_conflict(
            OnConflict::column(timeline_publications::Column::SegmentId)
                .update_columns([
                    timeline_publications::Column::Visibility,
                    timeline_publications::Column::WorkspaceId,
                    timeline_publications::Column::TitleOverride,
                    timeline_publications::Column::PublishedAt,
                    timeline_publications::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(self.db)
        .map(|_| ())
    }

    pub fn unpublish(&self, segment_id: i64, now_ms: i64) -> Result<Option<String>, DbErr> {
        let previous = timeline_publications::Entity::find_by_id(segment_id)
            .one(self.db)?
            .and_then(|row| row.workspace_id);
        let changed = timeline_publications::Entity::update_many()
            .col_expr(
                timeline_publications::Column::Visibility,
                Expr::value("private"),
            )
            .col_expr(
                timeline_publications::Column::WorkspaceId,
                Expr::value(None::<String>),
            )
            .col_expr(
                timeline_publications::Column::UpdatedAt,
                Expr::value(now_ms),
            )
            .filter(timeline_publications::Column::SegmentId.eq(segment_id))
            .exec(self.db)?;
        if changed.rows_affected == 0 {
            // Preserve the legacy application-facing error contract. The old
            // legacy SQLite path returned this exact message and callers surface it
            // through the Tauri command boundary.
            return Err(DbErr::Custom("timeline publication not found".into()));
        }
        Ok(previous)
    }

    pub fn enqueue_outbox(
        &self,
        entity_id: &str,
        operation: &str,
        payload: &str,
        now_ms: i64,
    ) -> Result<(), DbErr> {
        timeline_sync_outbox::ActiveModel {
            id: Set(uuid::Uuid::now_v7().to_string()),
            entity_type: Set("timeline".to_owned()),
            entity_id: Set(entity_id.to_owned()),
            operation: Set(operation.to_owned()),
            payload: Set(payload.to_owned()),
            state: Set("pending".to_owned()),
            attempts: Set(0),
            last_error: Set(None),
            created_at: Set(now_ms),
            updated_at: Set(now_ms),
        }
        .insert(self.db)
        .map(|_| ())
    }

    pub fn mark_pending(&self) -> Result<(), DbErr> {
        timeline_sync_state::Entity::update_many()
            .col_expr(timeline_sync_state::Column::Mode, Expr::value("server"))
            .col_expr(timeline_sync_state::Column::Status, Expr::value("pending"))
            .filter(timeline_sync_state::Column::Id.eq(1))
            .exec(self.db)
            .map(|_| ())
    }

    pub fn register_device(
        &self,
        device_id: &str,
        now_ms: i64,
        platform: &str,
    ) -> Result<(), DbErr> {
        timeline_devices::Entity::insert(timeline_devices::ActiveModel {
            id: Set(device_id.to_owned()),
            name: Set(device_id.to_owned()),
            platform: Set(platform.to_owned()),
            last_seen_at: Set(now_ms),
            tracking_enabled: Set(1),
            created_at: Set(now_ms),
            revoked_at: Set(None),
        })
        .on_conflict(
            OnConflict::column(timeline_devices::Column::Id)
                .update_columns([
                    timeline_devices::Column::LastSeenAt,
                    timeline_devices::Column::RevokedAt,
                ])
                .to_owned(),
        )
        .exec(self.db)
        .map(|_| ())
    }

    pub fn sync_state(&self) -> Result<(Option<timeline_sync_state::Model>, u64), DbErr> {
        let state = timeline_sync_state::Entity::find_by_id(1).one(self.db)?;
        let pending = timeline_sync_outbox::Entity::find()
            .filter(timeline_sync_outbox::Column::State.eq("pending"))
            .count(self.db)?;
        Ok((state, pending))
    }

    pub fn pending_operations(&self) -> Result<Vec<timeline_sync_outbox::Model>, DbErr> {
        timeline_sync_outbox::Entity::find()
            .filter(timeline_sync_outbox::Column::State.eq("pending"))
            .order_by_asc(timeline_sync_outbox::Column::CreatedAt)
            .limit(500)
            .all(self.db)
    }

    pub fn cursor(&self, workspace_id: &str) -> Result<u64, DbErr> {
        Ok(
            timeline_sync_cursors::Entity::find_by_id(workspace_id.to_owned())
                .one(self.db)?
                .and_then(|row| row.cursor.parse().ok())
                .unwrap_or(0),
        )
    }

    pub fn set_connecting(&self, server_url: &str) -> Result<(), DbErr> {
        timeline_sync_state::Entity::update_many()
            .col_expr(timeline_sync_state::Column::Mode, Expr::value("server"))
            .col_expr(
                timeline_sync_state::Column::ServerUrl,
                Expr::value(Some(server_url)),
            )
            .col_expr(
                timeline_sync_state::Column::Status,
                Expr::value("connecting"),
            )
            .col_expr(
                timeline_sync_state::Column::LastError,
                Expr::value(None::<String>),
            )
            .filter(timeline_sync_state::Column::Id.eq(1))
            .exec(self.db)
            .map(|_| ())
    }

    pub fn apply_sync(&self, request: SyncApply<'_>) -> Result<usize, DbErr> {
        let SyncApply {
            accepted,
            conflicts,
            remote_upserts,
            remote_deletes,
            workspace_id,
            cursor,
            now_ms,
        } = request;
        let tx = self.db.begin()?;
        for id in accepted {
            timeline_sync_outbox::Entity::update_many()
                .col_expr(timeline_sync_outbox::Column::State, Expr::value("sent"))
                .col_expr(timeline_sync_outbox::Column::UpdatedAt, Expr::value(now_ms))
                .filter(timeline_sync_outbox::Column::Id.eq(id.clone()))
                .exec(&tx)?;
        }
        for id in conflicts {
            timeline_sync_outbox::Entity::update_many()
                .col_expr(timeline_sync_outbox::Column::State, Expr::value("failed"))
                .col_expr(
                    timeline_sync_outbox::Column::LastError,
                    Expr::value(Some("conflict")),
                )
                .col_expr(timeline_sync_outbox::Column::UpdatedAt, Expr::value(now_ms))
                .filter(timeline_sync_outbox::Column::Id.eq(id.clone()))
                .exec(&tx)?;
        }
        for id in remote_deletes {
            timeline_remote_events::Entity::update_many()
                .col_expr(
                    timeline_remote_events::Column::DeletedAt,
                    Expr::value(Some(now_ms)),
                )
                .col_expr(
                    timeline_remote_events::Column::UpdatedAt,
                    Expr::value(now_ms),
                )
                .filter(timeline_remote_events::Column::Id.eq(id.clone()))
                .exec(&tx)?;
        }
        for (id, remote_workspace, payload) in remote_upserts {
            timeline_remote_events::Entity::insert(timeline_remote_events::ActiveModel {
                id: Set(id.clone()),
                workspace_id: Set(remote_workspace.clone()),
                payload: Set(payload.clone()),
                updated_at: Set(now_ms),
                deleted_at: Set(None),
            })
            .on_conflict(
                OnConflict::column(timeline_remote_events::Column::Id)
                    .update_columns([
                        timeline_remote_events::Column::WorkspaceId,
                        timeline_remote_events::Column::Payload,
                        timeline_remote_events::Column::UpdatedAt,
                        timeline_remote_events::Column::DeletedAt,
                    ])
                    .to_owned(),
            )
            .exec(&tx)?;
        }
        timeline_sync_cursors::Entity::insert(timeline_sync_cursors::ActiveModel {
            workspace_id: Set(workspace_id.to_owned()),
            cursor: Set(cursor.to_string()),
            last_success_at: Set(Some(now_ms)),
            last_error: Set(None),
        })
        .on_conflict(
            OnConflict::column(timeline_sync_cursors::Column::WorkspaceId)
                .update_columns([
                    timeline_sync_cursors::Column::Cursor,
                    timeline_sync_cursors::Column::LastSuccessAt,
                    timeline_sync_cursors::Column::LastError,
                ])
                .to_owned(),
        )
        .exec(&tx)?;
        timeline_sync_state::Entity::update_many()
            .col_expr(
                timeline_sync_state::Column::Status,
                Expr::value("connected"),
            )
            .col_expr(
                timeline_sync_state::Column::LastSuccessAt,
                Expr::value(Some(now_ms)),
            )
            .col_expr(
                timeline_sync_state::Column::LastError,
                Expr::value(None::<String>),
            )
            .filter(timeline_sync_state::Column::Id.eq(1))
            .exec(&tx)?;
        tx.commit()?;
        Ok(remote_upserts.len() + remote_deletes.len())
    }
}
