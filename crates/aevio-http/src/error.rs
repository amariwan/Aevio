//! One HTTP error envelope and one status mapping for every server adapter.

use std::fmt::Display;

use aevio_application::identity::{AccessDenial, MembershipChangeError};
use aevio_contracts::api::{ApiErrorBody, codes};
use aevio_contracts::sync::PushValidationError;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// A failed request, ready to become an HTTP response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    pub fn unauthenticated(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, codes::UNAUTHENTICATED, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, codes::FORBIDDEN, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, codes::NOT_FOUND, message)
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, codes::INVALID_REQUEST, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, codes::CONFLICT, message)
    }

    pub fn registration_disabled() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            codes::REGISTRATION_DISABLED,
            "registration is disabled",
        )
    }

    /// Unexpected server-side failure. The detail is logged, never returned, so
    /// SQL fragments, paths, and secrets cannot reach the client.
    pub fn internal(error: impl Display) -> Self {
        eprintln!("aevio-server internal error: {error}");
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            codes::INTERNAL,
            "internal server error",
        )
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn body(&self) -> ApiErrorBody {
        ApiErrorBody::new(self.code, self.message.clone())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body())).into_response()
    }
}

impl From<AccessDenial> for ApiError {
    fn from(denial: AccessDenial) -> Self {
        Self::forbidden(denial.message())
    }
}

impl From<PushValidationError> for ApiError {
    fn from(error: PushValidationError) -> Self {
        Self::invalid_request(error.message())
    }
}

impl From<MembershipChangeError> for ApiError {
    fn from(error: MembershipChangeError) -> Self {
        match error {
            MembershipChangeError::UnknownRole => Self::invalid_request(error.message()),
            MembershipChangeError::CannotGrant => Self::forbidden(error.message()),
            MembershipChangeError::LastOwner => Self::conflict(error.message()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ApiError;
    use aevio_application::identity::{AccessDenial, MembershipChangeError};
    use aevio_contracts::sync::PushValidationError;
    use axum::http::StatusCode;

    #[test]
    fn envelope_has_exactly_one_error_object() {
        let body = ApiError::forbidden("workspace role is not sufficient").body();
        let encoded = serde_json::to_value(&body).expect("serialize");
        assert_eq!(encoded["error"]["code"], "FORBIDDEN");
        assert_eq!(
            encoded["error"]["message"],
            "workspace role is not sufficient"
        );
        assert_eq!(encoded.as_object().expect("object").len(), 1);
        assert_eq!(encoded["error"].as_object().expect("detail").len(), 2);
    }

    #[test]
    fn every_constructor_uses_a_documented_status() {
        let cases = [
            (ApiError::unauthenticated("x"), StatusCode::UNAUTHORIZED),
            (ApiError::forbidden("x"), StatusCode::FORBIDDEN),
            (ApiError::not_found("x"), StatusCode::NOT_FOUND),
            (ApiError::invalid_request("x"), StatusCode::BAD_REQUEST),
            (ApiError::conflict("x"), StatusCode::CONFLICT),
            (ApiError::registration_disabled(), StatusCode::NOT_FOUND),
            (ApiError::internal("x"), StatusCode::INTERNAL_SERVER_ERROR),
        ];
        for (error, status) in cases {
            assert_eq!(error.status(), status);
            assert!(
                error
                    .code()
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c == '_')
            );
        }
    }

    #[test]
    fn internal_errors_do_not_echo_the_cause() {
        let error = ApiError::internal("SELECT * FROM accounts -- secret");
        assert_eq!(error.message(), "internal server error");
        assert!(!error.message().contains("SELECT"));
        assert_eq!(error.code(), "INTERNAL");
    }

    #[test]
    fn policy_denials_map_to_stable_statuses() {
        let denied = ApiError::from(AccessDenial::InsufficientRole);
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);
        assert_eq!(denied.code(), "FORBIDDEN");

        let invalid = ApiError::from(PushValidationError::UnregisteredEntityType);
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
        assert_eq!(invalid.code(), "INVALID_REQUEST");

        assert_eq!(
            ApiError::from(MembershipChangeError::UnknownRole).status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ApiError::from(MembershipChangeError::CannotGrant).status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            ApiError::from(MembershipChangeError::LastOwner).status(),
            StatusCode::CONFLICT
        );
    }
}
