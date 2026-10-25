use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;

use crate::app::state::AppState;

pub async fn health() -> impl IntoResponse {
    readiness_response(true, "process")
}

pub async fn readiness(State(state): State<AppState>) -> impl IntoResponse {
    readiness_response(state.database.ping().await, state.database.storage_name())
}

fn readiness_response(ready: bool, storage: &'static str) -> impl IntoResponse {
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let state = if ready { "ok" } else { "unavailable" };
    (
        status,
        Json(json!({
            "status": state,
            "protocolVersion": 1,
            "storage": storage
        })),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aevio_services::identity::ServiceScope;
    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    use super::readiness;
    use crate::app::core::database::DatabaseBackend;
    use crate::app::state::AppState;

    fn state(database_path: std::path::PathBuf) -> AppState {
        AppState {
            database: Arc::new(DatabaseBackend::Sqlite(database_path)),
            token: Arc::new("test-token".into()),
            workspace: None,
            allow_registration: false,
            service_scope: Arc::new(ServiceScope::from_csv("workspace", "read")),
            plugin_artifact_dir: Arc::new(std::path::PathBuf::from("test-artifacts")),
        }
    }

    #[tokio::test]
    async fn readiness_fails_when_sqlite_cannot_be_opened() {
        let path =
            std::path::PathBuf::from("/aevio-readiness-parent-does-not-exist/database.sqlite");
        let response = readiness(State(state(path))).await.into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
