use axum::Json;
use axum::extract::State;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::app::shared::ApiError;
use aevio_services::auth::{hash_password, verify_password};
use aevio_services::identity::{normalize_email, validate_registration};

use crate::app::config::ServerConfig;
use crate::app::core::database::Repository;
use crate::app::services::auth::{hash_token, unix_now};
use crate::app::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    email: String,
    password: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<Credentials>,
) -> Result<Json<Value>, ApiError> {
    let db = state.connect().await?;
    let repository = Repository::new(db);
    let account = repository
        .active_account_by_email(&request.email)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::unauthenticated("invalid credentials"))?;
    verify_password(&request.password, &account.password_hash)
        .map_err(|_| ApiError::unauthenticated("invalid credentials"))?;
    let token = Uuid::now_v7().to_string();
    repository
        .insert_token(hash_token(&token), account.id.clone(), unix_now())
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(
        json!({ "accessToken": token, "accountId": account.id }),
    ))
}

pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<Credentials>,
) -> Result<Json<Value>, ApiError> {
    if !state.allow_registration {
        return Err(ApiError::registration_disabled());
    }
    validate_registration(&request.email, &request.password).map_err(ApiError::invalid_request)?;
    let email = normalize_email(&request.email);
    let password_hash = hash_password(&request.password)
        .map_err(|_| ApiError::internal("password hashing failed"))?;
    let account_id = Uuid::now_v7().to_string();
    let db = state.connect().await?;
    let repository = Repository::new(db);
    match repository
        .insert_account_if_unused(account_id.clone(), email.clone(), password_hash, unix_now())
        .await
    {
        Ok(true) => Ok(Json(json!({ "accountId": account_id }))),
        Ok(false) => Err(ApiError::conflict("email is already registered")),
        // A concurrent registration can win the unique constraint. Re-checking
        // the address keeps the answer correct without matching driver strings.
        Err(error) => {
            if repository
                .active_account_by_email(&email)
                .await
                .map_err(ApiError::internal)?
                .is_some()
            {
                Err(ApiError::conflict("email is already registered"))
            } else {
                Err(ApiError::internal(error))
            }
        }
    }
}

/// Bootstraps the operator account and workspace once. Existing rows win.
pub async fn seed_admin(
    db: &crate::app::core::database::DatabaseBackend,
    config: &ServerConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(email) = config.admin_email.as_deref() else {
        return Ok(());
    };
    let password = config
        .admin_password
        .as_deref()
        .ok_or("admin password is required")?;
    let email = normalize_email(email);
    let repository = Repository::new(db);
    let account_id = format!("account-{}", &hash_token(&email)[..16]);
    if repository.active_account_by_email(&email).await?.is_none() {
        let password_hash =
            hash_password(password).map_err(|_| "admin password could not be hashed")?;
        repository
            .insert_account(account_id.clone(), email, password_hash, unix_now())
            .await?;
    }
    let workspace = config
        .workspace
        .clone()
        .unwrap_or_else(|| "workspace-main".into());
    repository
        .ensure_workspace(workspace.clone(), config.workspace_name.clone(), unix_now())
        .await?;
    repository
        .upsert_membership(workspace, account_id, "owner".into(), unix_now())
        .await?;
    Ok(())
}
