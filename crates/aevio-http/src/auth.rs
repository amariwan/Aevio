//! Shared HTTP auth primitives: bearer extraction and constant-time token
//! comparison. Both server adapters delegate to these instead of re-deriving
//! them, so the security behaviour cannot drift between dialects.
use axum::http::HeaderMap;

use crate::error::ApiError;

/// Extracts the trimmed bearer token from an Authorization header.
pub fn bearer_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::unauthenticated("bearer token is required"))
}

/// Constant-time comparison of two token strings via their SHA-256 digests,
/// so response timing does not reveal a matching prefix.
pub fn tokens_match(presented: &str, expected: &str) -> bool {
    use sha2::{Digest, Sha256};
    let presented = Sha256::digest(presented.as_bytes());
    let expected = Sha256::digest(expected.as_bytes());
    presented.len() == expected.len()
        && presented
            .iter()
            .zip(expected)
            .fold(0_u8, |difference, (left, right)| {
                difference | (left ^ right)
            })
            == 0
}
