use axum::Json;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use serde::Deserialize;

use crate::app::shared::ApiError;
use crate::app::shared::contracts::{
    SyncPullResponse, SyncPushRequest, SyncPushResponse, validate_push_request,
};
use aevio_services::identity::Role;

use crate::app::authz::{authenticate, require_workspace};
use crate::app::core::database::Repository;
use crate::app::state::AppState;

const DEFAULT_PULL_LIMIT: u32 = 500;
const MAX_PULL_LIMIT: u32 = 2_000;

#[derive(Debug, Deserialize)]
pub struct PullQuery {
    workspace_id: String,
    cursor: Option<u64>,
    limit: Option<u32>,
}

pub async fn push(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SyncPushRequest>,
) -> Result<Json<SyncPushResponse>, ApiError> {
    validate_push_request(&request)?;
    let db = state.connect().await?;
    let actor = authenticate(&headers, &state, db).await?;
    for operation in &request.operations {
        require_workspace(db, &state, &actor, &operation.workspace_id, Role::Member).await?;
    }
    Repository::new(db)
        .push(&request)
        .await
        .map(Json)
        .map_err(ApiError::internal)
}

pub async fn pull(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PullQuery>,
) -> Result<Json<SyncPullResponse>, ApiError> {
    let cursor = query.cursor.unwrap_or(0);
    let limit = u64::from(
        query
            .limit
            .unwrap_or(DEFAULT_PULL_LIMIT)
            .clamp(1, MAX_PULL_LIMIT),
    );
    let db = state.connect().await?;
    let actor = authenticate(&headers, &state, db).await?;
    require_workspace(db, &state, &actor, &query.workspace_id, Role::Viewer).await?;
    let changes = Repository::new(db)
        .pull(&query.workspace_id, cursor, limit)
        .await
        .map_err(ApiError::internal)?;
    let next_cursor = changes
        .last()
        .map(|change| change.revision)
        .unwrap_or(cursor);
    Ok(Json(SyncPullResponse {
        changes,
        cursor: next_cursor,
    }))
}

#[cfg(test)]
mod tests {
    use aevio_contracts::sync::SyncPushRequest;
    use serde_json::json;

    #[test]
    fn pull_query_accepts_the_existing_snake_case_wire_form() {
        let query: super::PullQuery =
            serde_json::from_value(json!({"workspace_id": "ws", "cursor": 7, "limit": 10}))
                .expect("query");
        assert_eq!(query.workspace_id, "ws");
        assert_eq!(query.cursor, Some(7));
        assert_eq!(query.limit, Some(10));
    }

    #[test]
    fn push_wire_shape_is_camel_case() {
        let request: SyncPushRequest = serde_json::from_value(json!({
            "deviceId": "device",
            "operations": [{
                "operationId": "op",
                "workspaceId": "ws",
                "entityType": "timeline",
                "entityId": "event",
                "operation": "upsert",
                "payload": {},
                "baseRevision": null
            }]
        }))
        .expect("request");
        assert_eq!(request.device_id, "device");
        assert_eq!(request.operations.len(), 1);
    }
}
