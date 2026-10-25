//! Harness for the PostgreSQL adapter contract tests.
//!
//! These tests run against a real PostgreSQL instance: there is no SQLite or
//! in-memory stand-in, because the point is to verify the adapter and its schema
//! where they actually run. Set `AEVIO_TEST_POSTGRES_URL` to a disposable
//! database; without it every test reports a skip instead of a result it did not
//! obtain.

// The harness is compiled into every test binary, and each binary uses a
// different subset of it. Unused helpers are intentional, not dead code.
#![allow(dead_code)]

use std::env;

use aevio_server::app::config::{DatabaseConfig, ServerConfig};
use aevio_server::app::core::database::postgres as postgres_database;
use aevio_server::app::core::database::{DatabaseBackend, Repository};
use aevio_server::app::services::auth::unix_now;
use aevio_server::app::state::AppState;
use aevio_server::app::views::router;
use aevio_services::identity::ServiceScope;
use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

/// Password that satisfies the registration policy.
pub const PASSWORD: &str = "correct horse battery";

pub struct Call {
    pub status: StatusCode,
    pub body: Value,
}

impl Call {
    /// The documented stable error code, or an empty string for a success body.
    pub fn code(&self) -> &str {
        self.body["error"]["code"].as_str().unwrap_or_default()
    }

    pub fn array_len(&self, key: &str) -> usize {
        self.body[key].as_array().map(Vec::len).unwrap_or(0)
    }
}

pub struct Harness {
    pub db: DatabaseConnection,
    /// Workspace this harness's service token is scoped to.
    pub workspace: String,
    pub token: String,
    state: AppState,
}

impl Harness {
    /// `None` means no PostgreSQL URL is configured, so the caller must report a
    /// skip rather than pass silently.
    pub async fn start() -> Option<Self> {
        let url = env::var("AEVIO_TEST_POSTGRES_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())?;
        let db = postgres_database::connect(&url)
            .await
            .expect("connect to PostgreSQL");
        postgres_database::initialize(&db)
            .await
            .expect("create server schema");
        // A unique workspace per harness keeps parallel test binaries isolated
        // without truncating a database that another test may be using.
        let workspace = format!("workspace-{}", Uuid::now_v7());
        let config = ServerConfig {
            database: DatabaseConfig::Postgres(url),
            token: format!("test-token-{}", Uuid::now_v7()),
            bind: "127.0.0.1:0".into(),
            workspace: None,
            allow_registration: true,
            admin_email: None,
            admin_password: None,
            workspace_name: "Integration workspace".into(),
            service_scope: ServiceScope::from_csv(&workspace, "read,write"),
            plugin_artifact_dir: "test-plugin-artifacts".into(),
        };
        let backend = DatabaseBackend::Postgres(std::sync::Arc::new(db.clone()));
        Repository::new(&backend)
            .ensure_workspace(
                workspace.clone(),
                "Integration workspace".into(),
                unix_now(),
            )
            .await
            .expect("workspace row");
        let state = AppState::new(&config, backend);
        Some(Self {
            db,
            workspace,
            token: config.token,
            state,
        })
    }

    pub fn app(&self) -> Router {
        router(self.state.clone())
    }

    /// A second router over the same storage with self-registration turned off.
    pub fn app_without_registration(&self) -> Router {
        let mut state = self.state.clone();
        state.allow_registration = false;
        router(state)
    }

    pub async fn call(
        &self,
        method: Method,
        uri: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> Call {
        call(self.app(), method, uri, token, body).await
    }

    pub async fn register(&self, email: &str) -> Call {
        self.call(
            Method::POST,
            "/api/v1/auth/register",
            None,
            Some(json!({ "email": email, "password": PASSWORD })),
        )
        .await
    }

    /// Registers the given address and returns its account id.
    pub async fn account(&self, email: &str) -> String {
        let call = self.register(email).await;
        assert_eq!(
            call.status,
            StatusCode::OK,
            "register {email}: {:?}",
            call.body
        );
        call.body["accountId"]
            .as_str()
            .expect("registration returns an account id")
            .to_owned()
    }

    pub fn email(&self, label: &str) -> String {
        format!("{label}-{}@example.test", Uuid::now_v7())
    }

    /// Logs in and returns the session token.
    pub async fn session(&self, email: &str) -> String {
        let call = self
            .call(
                Method::POST,
                "/api/v1/auth/login",
                None,
                Some(json!({ "email": email, "password": PASSWORD })),
            )
            .await;
        assert_eq!(
            call.status,
            StatusCode::OK,
            "login {email}: {:?}",
            call.body
        );
        call.body["accessToken"]
            .as_str()
            .expect("login returns an access token")
            .to_owned()
    }

    /// Grants a membership directly through the repository, the way the admin
    /// bootstrap does, so a test can start from any role. The workspace row is
    /// created first, because memberships reference it.
    pub async fn grant(&self, workspace: &str, account_id: &str, role: &str) {
        let backend = DatabaseBackend::Postgres(std::sync::Arc::new(self.db.clone()));
        let repository = Repository::new(&backend);
        repository
            .ensure_workspace(
                workspace.to_owned(),
                "Integration workspace".into(),
                unix_now(),
            )
            .await
            .expect("workspace row");
        repository
            .upsert_membership(
                workspace.to_owned(),
                account_id.to_owned(),
                role.to_owned(),
                unix_now(),
            )
            .await
            .expect("membership row");
    }

    /// An operation id that is unique to this harness run. `sync_operations`
    /// keys on the operation id globally, so a fixed literal would replay an
    /// earlier run's row, in this or any other workspace.
    pub fn op_id(&self, label: &str) -> String {
        format!("{}-{label}", self.workspace)
    }

    pub async fn push(&self, token: Option<&str>, device: &str, operations: Value) -> Call {
        self.call(
            Method::POST,
            "/api/v1/sync/push",
            token,
            Some(json!({ "deviceId": device, "operations": operations })),
        )
        .await
    }

    pub async fn pull(
        &self,
        token: Option<&str>,
        workspace: &str,
        cursor: u64,
        limit: u32,
    ) -> Call {
        self.call(
            Method::GET,
            &format!("/api/v1/sync/pull?workspace_id={workspace}&cursor={cursor}&limit={limit}"),
            token,
            None,
        )
        .await
    }

    pub async fn add_member(
        &self,
        token: Option<&str>,
        workspace: &str,
        email: &str,
        role: &str,
    ) -> Call {
        self.call(
            Method::POST,
            &format!("/api/v1/workspaces/{workspace}/members"),
            token,
            Some(json!({ "email": email, "role": role })),
        )
        .await
    }
}

/// One registered sync operation, already in the public wire shape.
pub fn operation(
    workspace: &str,
    operation_id: &str,
    entity_id: &str,
    payload: Value,
    base_revision: Option<u64>,
) -> Value {
    json!({
        "operationId": operation_id,
        "workspaceId": workspace,
        "entityType": "timeline",
        "entityId": entity_id,
        "operation": "upsert",
        "payload": payload,
        "baseRevision": base_revision,
    })
}

pub async fn call(
    app: Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> Call {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = match body {
        Some(value) => {
            request = request.header(header::CONTENT_TYPE, "application/json");
            Body::from(value.to_string())
        }
        None => Body::empty(),
    };
    let response = app
        .oneshot(request.body(body).expect("request builds"))
        .await
        .expect("router answers");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body reads")
        .to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice::<Value>(&bytes)
            .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes).into_owned() }))
    };
    Call { status, body }
}

/// Prints the reason a test produced no result. Tests stay green only because
/// they explicitly recorded the skip, never because they silently did nothing.
pub fn skipped(test: &str) {
    eprintln!("skipped {test}: AEVIO_TEST_POSTGRES_URL is not set");
}

/// Starts a harness or records an explicit skip and returns from the test.
#[macro_export]
macro_rules! pg_harness {
    ($test:expr) => {
        match $crate::support::Harness::start().await {
            Some(harness) => harness,
            None => {
                $crate::support::skipped($test);
                return;
            }
        }
    };
}
