//! Artifact verification and package-boundary checks.
//!
//! This module verifies bytes before a caller may stage them. It does not
//! execute plugin code, choose permissions, or provide a database handle.

use std::path::Path;

use aevio_contracts::plugin::{MAX_ASSET_BYTES, PluginManifest, PluginRelease};
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactVerificationError {
    TooLarge,
    DigestMismatch,
    MissingSignature,
    InvalidSignatureEncoding,
    InvalidPublisherKey,
    SignatureMismatch,
    InvalidManifest(String),
    UnsafePackagePath,
}

impl std::fmt::Display for ArtifactVerificationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ArtifactVerificationError {}

#[derive(Debug, Clone)]
pub struct VerifiedPluginArtifact {
    pub digest: String,
    pub manifest: PluginManifest,
}

pub fn verify_artifact(
    bytes: &[u8],
    manifest: PluginManifest,
    release: &PluginRelease,
) -> Result<VerifiedPluginArtifact, ArtifactVerificationError> {
    if bytes.len() as u64 > MAX_ASSET_BYTES {
        return Err(ArtifactVerificationError::TooLarge);
    }
    manifest
        .validate()
        .map_err(|error| ArtifactVerificationError::InvalidManifest(error.to_string()))?;
    let digest = hex_digest(bytes);
    if !constant_time_equal(&digest, &release.artifact_digest) {
        return Err(ArtifactVerificationError::DigestMismatch);
    }
    let signature_text = release
        .signature
        .as_deref()
        .ok_or(ArtifactVerificationError::MissingSignature)?;
    let public_key_text = manifest
        .publisher
        .public_key
        .as_deref()
        .ok_or(ArtifactVerificationError::InvalidPublisherKey)?;
    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature_text)
        .map_err(|_| ArtifactVerificationError::InvalidSignatureEncoding)?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| ArtifactVerificationError::InvalidSignatureEncoding)?;
    let key_bytes = base64::engine::general_purpose::STANDARD
        .decode(public_key_text)
        .map_err(|_| ArtifactVerificationError::InvalidPublisherKey)?;
    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| ArtifactVerificationError::InvalidPublisherKey)?;
    let key = VerifyingKey::from_bytes(&key_array)
        .map_err(|_| ArtifactVerificationError::InvalidPublisherKey)?;
    key.verify(bytes, &signature)
        .map_err(|_| ArtifactVerificationError::SignatureMismatch)?;
    Ok(VerifiedPluginArtifact { digest, manifest })
}

pub fn validate_package_path(path: &str) -> Result<(), ArtifactVerificationError> {
    let candidate = Path::new(path);
    if path.is_empty()
        || candidate.is_absolute()
        || candidate.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        })
    {
        return Err(ArtifactVerificationError::UnsafePackagePath);
    }
    Ok(())
}

pub fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn constant_time_equal(left: &str, right: &str) -> bool {
    left.len() == right.len()
        && left
            .as_bytes()
            .iter()
            .zip(right.as_bytes())
            .fold(0u8, |difference, (left, right)| difference | (left ^ right))
            == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use aevio_contracts::plugin::{
        MANIFEST_VERSION, PluginCompatibility, PluginEntrypoints, PluginPlatform, PublisherIdentity,
    };
    use ed25519_dalek::{Signer, SigningKey};

    fn manifest(public_key: String) -> PluginManifest {
        PluginManifest {
            manifest_version: MANIFEST_VERSION,
            id: "dev.aevio.test".into(),
            name: "Test".into(),
            version: "1.0.0".into(),
            publisher: PublisherIdentity {
                id: "test".into(),
                display_name: "Test".into(),
                verified: true,
                public_key: Some(public_key),
            },
            description: "Test".into(),
            category: "test".into(),
            compatibility: PluginCompatibility {
                aevio: ">=0.9.0, <1.0.0".into(),
                plugin_api: "^1.0.0".into(),
            },
            platforms: vec![PluginPlatform::Desktop],
            entrypoints: PluginEntrypoints::default(),
            icons: Vec::new(),
            screenshots: Vec::new(),
            permissions: Vec::new(),
            dependencies: Vec::new(),
            optional_dependencies: Vec::new(),
            contributes: Default::default(),
            event_subscriptions: Vec::new(),
            offline: Default::default(),
            data_schema: None,
        }
    }

    #[test]
    fn accepts_a_signed_untampered_artifact() {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let bytes = b"plugin bytes";
        let public_key = base64::engine::general_purpose::STANDARD
            .encode(signing_key.verifying_key().to_bytes());
        let release = PluginRelease {
            version: "1.0.0".into(),
            artifact_digest: hex_digest(bytes),
            artifact_size: bytes.len() as u64,
            signature: Some(
                base64::engine::general_purpose::STANDARD
                    .encode(signing_key.sign(bytes).to_bytes()),
            ),
            channel: "stable".into(),
            published_at: 0,
            revoked: false,
        };
        assert!(verify_artifact(bytes, manifest(public_key), &release).is_ok());
    }

    #[test]
    fn rejects_tampered_bytes_and_unsafe_paths() {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let bytes = b"plugin bytes";
        let public_key = base64::engine::general_purpose::STANDARD
            .encode(signing_key.verifying_key().to_bytes());
        let release = PluginRelease {
            version: "1.0.0".into(),
            artifact_digest: hex_digest(bytes),
            artifact_size: bytes.len() as u64,
            signature: Some(
                base64::engine::general_purpose::STANDARD
                    .encode(signing_key.sign(bytes).to_bytes()),
            ),
            channel: "stable".into(),
            published_at: 0,
            revoked: false,
        };
        assert!(matches!(
            verify_artifact(b"tampered", manifest(public_key), &release),
            Err(ArtifactVerificationError::DigestMismatch)
        ));
        assert_eq!(
            validate_package_path("../plugin.wasm"),
            Err(ArtifactVerificationError::UnsafePackagePath)
        );
    }
}
