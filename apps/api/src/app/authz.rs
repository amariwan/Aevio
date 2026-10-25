use aevio_http::auth::{bearer_token, tokens_match};
use aevio_http::error::ApiError;
use aevio_services::identity::{
    AccessDenial, Actor, Role, authorize, token_is_live, workspace_role,
};
use axum::http::HeaderMap;

use crate::app::core::database::{DatabaseBackend, Repository};
use crate::app::services::auth::{hash_token, unix_now};
use crate::app::state::AppState;

/// Authenticates the bearer credential of one request.
///
/// The deployment token becomes a scoped service actor, never a super user.
pub async fn authenticate(
    headers: &HeaderMap,
    state: &AppState,
    db: &DatabaseBackend,
) -> Result<Actor, ApiError> {
    let token = bearer_token(headers)?;
    if tokens_match(token, &state.token) {
        return Ok(Actor::service(state.service_scope()));
    }
    let session = Repository::new(db)
        .session_for_token(&hash_token(token))
        .await
        .map_err(ApiError::internal)?
        .filter(|session| token_is_live(session.created_at, session.revoked_at, unix_now()))
        .ok_or_else(|| ApiError::unauthenticated("invalid or expired bearer token"))?;
    Ok(Actor::account(session.account_id))
}

/// Authorizes one workspace operation and returns the caller's effective role.
pub async fn require_workspace(
    db: &DatabaseBackend,
    state: &AppState,
    actor: &Actor,
    workspace_id: &str,
    required: Role,
) -> Result<Role, ApiError> {
    if let Some(allowed) = state.workspace.as_deref()
        && allowed != workspace_id
    {
        return Err(ApiError::from(AccessDenial::WorkspaceNotAllowed));
    }
    let member_role = match actor {
        Actor::Service { .. } => None,
        Actor::Account { account_id } => Repository::new(db)
            .membership_role(account_id, workspace_id)
            .await
            .map_err(ApiError::internal)?,
    };
    authorize(actor, workspace_id, required, member_role.as_deref()).map_err(ApiError::from)?;
    workspace_role(actor, member_role.as_deref())
        .ok_or_else(|| ApiError::from(AccessDenial::InsufficientRole))
}
