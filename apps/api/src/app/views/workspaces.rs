use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::app::shared::ApiError;
use aevio_services::identity::{Role, decide_membership_change};

use crate::app::authz::{authenticate, require_workspace};
use crate::app::core::database::Repository;
use crate::app::services::auth::unix_now;
use crate::app::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberRequest {
    email: String,
    role: String,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Value>>, ApiError> {
    let db = state.connect().await?;
    let actor = authenticate(&headers, &state, db).await?;
    let account_id = actor.account_id().ok_or_else(|| {
        ApiError::forbidden("this endpoint requires an account session, not a service token")
    })?;
    let rows = Repository::new(db)
        .workspace_rows(account_id)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(
        rows.into_iter()
            .map(|(workspace, role)| json!({ "id": workspace.id, "name": workspace.name, "role": role }))
            .collect(),
    ))
}

pub async fn add_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<String>,
    Json(request): Json<MemberRequest>,
) -> Result<StatusCode, ApiError> {
    let db = state.connect().await?;
    let actor = authenticate(&headers, &state, db).await?;
    let actor_role = require_workspace(db, &state, &actor, &workspace_id, Role::Admin).await?;
    let repository = Repository::new(db);
    let member_id = repository
        .active_account_by_email(&request.email)
        .await
        .map_err(ApiError::internal)?
        .map(|account| account.id)
        .ok_or_else(|| ApiError::not_found("account does not exist"))?;
    let current_role = repository
        .membership_role(&member_id, &workspace_id)
        .await
        .map_err(ApiError::internal)?;
    let owners = repository
        .owner_count(&workspace_id)
        .await
        .map_err(ApiError::internal)?;
    let role =
        decide_membership_change(actor_role, current_role.as_deref(), &request.role, owners)?;
    repository
        .upsert_membership(
            workspace_id,
            member_id,
            role.as_str().to_owned(),
            unix_now(),
        )
        .await
        .map_err(ApiError::internal)?;
    Ok(StatusCode::NO_CONTENT)
}
