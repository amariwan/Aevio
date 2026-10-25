//! Auth primitives shared by every adapter: token hashing, password hashing
//! and password verification. Adapters must not re-derive these rules or pick
//! their own Argon2 parameters.
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand_core::OsRng;
use sha2::{Digest, Sha256};

/// Seconds since the Unix epoch, saturating on clock errors.
pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

/// SHA-256 hex digest of a bearer token. Used as the storage key for sessions.
pub fn hash_token(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

/// Why a password operation failed. Adapters map these to transport errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordError {
    HashFailed,
    InvalidHash,
}

impl PasswordError {
    pub const fn message(self) -> &'static str {
        match self {
            Self::HashFailed => "password hashing failed",
            Self::InvalidHash => "stored password hash is not readable",
        }
    }
}

/// Hashes a password with the canonical Argon2 parameters. A single source so
/// every adapter stores comparable hashes.
pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| PasswordError::HashFailed)
}

/// Verifies a password against a stored encoded hash.
pub fn verify_password(password: &str, encoded: &str) -> Result<(), PasswordError> {
    let parsed = PasswordHash::new(encoded).map_err(|_| PasswordError::InvalidHash)?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| PasswordError::InvalidHash)
}

#[cfg(test)]
mod tests {
    use super::{hash_password, verify_password};

    #[test]
    fn password_hashes_round_trip_and_reject_wrong_passwords() {
        let hash = hash_password("correct horse battery").expect("hash");
        assert!(verify_password("correct horse battery", &hash).is_ok());
        assert!(verify_password("wrong password", &hash).is_err());
        assert!(verify_password("correct horse battery", "not-a-hash").is_err());
    }
}
