//! Account, session and startup contract against real PostgreSQL.

mod support;

use aevio_server::app::core::database::postgres::entities::tokens;
use aevio_server::app::services::auth::{hash_token, unix_now};
use axum::http::{Method, StatusCode};
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, EntityTrait};
use serde_json::json;
use support::{Harness, PASSWORD, call};

/// Rewrites a stored session so the expiry and revocation policy can be tested
/// without waiting for a real clock.
async fn rewrite_session(
    harness: &Harness,
    token: &str,
    mutate: impl FnOnce(&mut tokens::ActiveModel),
) {
    let row = tokens::Entity::find_by_id(hash_token(token))
        .one(&harness.db)
        .await
        .expect("query session")
        .expect("session row exists");
    let mut model: tokens::ActiveModel = row.into();
    mutate(&mut model);
    model.update(&harness.db).await.expect("update session");
}

#[tokio::test]
async fn health_reports_the_postgres_storage() {
    let harness = pg_harness!("health_reports_the_postgres_storage");
    let health = harness.call(Method::GET, "/healthz", None, None).await;
    assert_eq!(health.status, StatusCode::OK);
    assert_eq!(health.body["status"], "ok");
    assert_eq!(health.body["storage"], "postgres");
    assert_eq!(health.body["protocolVersion"], 1);

    let ready = harness.call(Method::GET, "/readyz", None, None).await;
    assert_eq!(ready.status, StatusCode::OK);
    assert_eq!(ready.body["status"], "ok");

    harness.db.close_by_ref().await.expect("close test pool");
    let unavailable = harness.call(Method::GET, "/readyz", None, None).await;
    assert_eq!(unavailable.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(unavailable.body["status"], "unavailable");
}

#[tokio::test]
async fn login_issues_a_session_that_lists_its_workspaces() {
    let harness = pg_harness!("login_issues_a_session_that_lists_its_workspaces");
    let email = harness.email("owner");
    let account = harness.account(&email).await;
    harness.grant(&harness.workspace, &account, "owner").await;

    let token = harness.session(&email).await;
    let listed = harness
        .call(Method::GET, "/api/v1/workspaces", Some(&token), None)
        .await;
    assert_eq!(listed.status, StatusCode::OK, "{:?}", listed.body);
    let rows = listed.body.as_array().expect("workspace list");
    assert!(
        rows.iter()
            .any(|row| row["id"] == harness.workspace.as_str() && row["role"] == "owner"),
        "workspace with owner role missing: {:?}",
        listed.body
    );
}

#[tokio::test]
async fn email_identity_is_case_insensitive() {
    let harness = pg_harness!("email_identity_is_case_insensitive");
    let email = harness.email("cased");
    harness.account(&email).await;

    let login = harness
        .call(
            Method::POST,
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": email.to_uppercase(), "password": PASSWORD })),
        )
        .await;
    assert_eq!(login.status, StatusCode::OK, "{:?}", login.body);
}

#[tokio::test]
async fn invalid_credentials_and_missing_tokens_are_unauthenticated() {
    let harness = pg_harness!("invalid_credentials_and_missing_tokens_are_unauthenticated");
    let email = harness.email("member");
    harness.account(&email).await;

    let wrong = harness
        .call(
            Method::POST,
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": email, "password": "not the password" })),
        )
        .await;
    assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);
    assert_eq!(wrong.code(), "UNAUTHENTICATED");

    let anonymous = harness
        .call(Method::GET, "/api/v1/workspaces", None, None)
        .await;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
    assert_eq!(anonymous.code(), "UNAUTHENTICATED");

    let unknown = harness
        .call(
            Method::GET,
            "/api/v1/workspaces",
            Some("unknown-token"),
            None,
        )
        .await;
    assert_eq!(unknown.status, StatusCode::UNAUTHORIZED);
    assert_eq!(unknown.code(), "UNAUTHENTICATED");
}

#[tokio::test]
async fn registration_can_be_disabled_and_is_validated() {
    let harness = pg_harness!("registration_can_be_disabled_and_is_validated");
    let disabled = call(
        harness.app_without_registration(),
        Method::POST,
        "/api/v1/auth/register",
        None,
        Some(json!({ "email": harness.email("blocked"), "password": PASSWORD })),
    )
    .await;
    assert_eq!(disabled.status, StatusCode::NOT_FOUND);
    assert_eq!(disabled.code(), "REGISTRATION_DISABLED");

    let weak = harness
        .call(
            Method::POST,
            "/api/v1/auth/register",
            None,
            Some(json!({ "email": harness.email("short"), "password": "short" })),
        )
        .await;
    assert_eq!(weak.status, StatusCode::BAD_REQUEST);
    assert_eq!(weak.code(), "INVALID_REQUEST");

    let email = harness.email("duplicate");
    harness.account(&email).await;
    let duplicate = harness.register(&email).await;
    assert_eq!(duplicate.status, StatusCode::CONFLICT);
    assert_eq!(duplicate.code(), "CONFLICT");
}

#[tokio::test]
async fn sessions_expire_and_revocation_wins() {
    let harness = pg_harness!("sessions_expire_and_revocation_wins");
    let email = harness.email("session");
    harness.account(&email).await;

    let expired = harness.session(&email).await;
    rewrite_session(&harness, &expired, |model| {
        model.created_at = Set(unix_now() - 31 * 24 * 60 * 60);
    })
    .await;
    let rejected = harness
        .call(Method::GET, "/api/v1/workspaces", Some(&expired), None)
        .await;
    assert_eq!(rejected.status, StatusCode::UNAUTHORIZED);
    assert_eq!(rejected.code(), "UNAUTHENTICATED");

    let revoked = harness.session(&email).await;
    rewrite_session(&harness, &revoked, |model| {
        model.revoked_at = Set(Some(unix_now()));
    })
    .await;
    let denied = harness
        .call(Method::GET, "/api/v1/workspaces", Some(&revoked), None)
        .await;
    assert_eq!(denied.status, StatusCode::UNAUTHORIZED);
    assert_eq!(denied.code(), "UNAUTHENTICATED");

    // A second session of the same account stays usable: revocation is per
    // session, not per account.
    let live = harness.session(&email).await;
    let ok = harness
        .call(Method::GET, "/api/v1/workspaces", Some(&live), None)
        .await;
    assert_eq!(ok.status, StatusCode::OK);
}

#[tokio::test]
async fn a_service_token_cannot_list_account_workspaces() {
    let harness = pg_harness!("a_service_token_cannot_list_account_workspaces");
    let listed = harness
        .call(
            Method::GET,
            "/api/v1/workspaces",
            Some(&harness.token),
            None,
        )
        .await;
    assert_eq!(listed.status, StatusCode::FORBIDDEN);
    assert_eq!(listed.code(), "FORBIDDEN");
}
