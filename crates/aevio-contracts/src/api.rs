//! Stable HTTP error contract shared by every server adapter.
//!
//! Clients branch on `code`, never on the prose in `message`. Adding a code is
//! compatible; renaming one is a breaking change for every consumer.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorBody {
    pub error: ApiErrorDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorDetail {
    pub code: String,
    pub message: String,
}

impl ApiErrorBody {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            error: ApiErrorDetail {
                code: code.to_owned(),
                message: message.into(),
            },
        }
    }
}

/// Machine-readable codes. Keep these stable across adapters and versions.
pub mod codes {
    /// The credentials are missing, unknown, expired, or revoked.
    pub const UNAUTHENTICATED: &str = "UNAUTHENTICATED";
    /// The caller is authenticated but may not perform this operation.
    pub const FORBIDDEN: &str = "FORBIDDEN";
    /// The requested resource does not exist or is not visible to the caller.
    pub const NOT_FOUND: &str = "NOT_FOUND";
    /// The request is malformed or violates a documented limit.
    pub const INVALID_REQUEST: &str = "INVALID_REQUEST";
    /// The request conflicts with current state, including replayed payloads.
    pub const CONFLICT: &str = "CONFLICT";
    /// Self-service registration is disabled on this server.
    pub const REGISTRATION_DISABLED: &str = "REGISTRATION_DISABLED";
    /// An unexpected server-side failure; the message stays free of secrets.
    pub const INTERNAL: &str = "INTERNAL";
}

#[cfg(test)]
mod tests {
    use super::{ApiErrorBody, codes};

    #[test]
    fn serializes_the_documented_envelope() {
        let body = ApiErrorBody::new(codes::FORBIDDEN, "workspace role is not sufficient");
        let encoded = serde_json::to_value(&body).expect("serialize");
        assert_eq!(encoded["error"]["code"], "FORBIDDEN");
        assert_eq!(
            encoded["error"]["message"],
            "workspace role is not sufficient"
        );
        assert_eq!(encoded.as_object().expect("object").len(), 1);
    }

    #[test]
    fn codes_are_upper_snake_case_and_unique() {
        let all = [
            codes::UNAUTHENTICATED,
            codes::FORBIDDEN,
            codes::NOT_FOUND,
            codes::INVALID_REQUEST,
            codes::CONFLICT,
            codes::REGISTRATION_DISABLED,
            codes::INTERNAL,
        ];
        for code in all {
            assert_eq!(code, code.to_uppercase(), "{code}");
            assert!(code.chars().all(|c| c.is_ascii_uppercase() || c == '_'));
        }
        let mut sorted = all.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), all.len());
    }
}
