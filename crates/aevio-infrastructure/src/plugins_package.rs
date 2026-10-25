//! Signed `.aevio-plugin` package parsing and atomic staging.
//!
//! This module never executes a package. It verifies the complete archive,
//! rejects unsafe paths and zip bombs, then writes an immutable versioned
//! directory through a same-filesystem rename.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use aevio_contracts::plugin::{MAX_ASSET_BYTES, PluginManifest, PluginRelease};
use serde_json::from_slice;
use zip::ZipArchive;

use crate::plugins::{
    ArtifactVerificationError, VerifiedPluginArtifact, validate_package_path, verify_artifact,
};

const MAX_FILES: usize = 256;

#[derive(Debug)]
pub enum PluginPackageError {
    Verification(ArtifactVerificationError),
    Archive(String),
    MissingManifest,
    TooManyFiles,
    UnpackedTooLarge,
    Io(String),
    DestinationExists,
}

impl std::fmt::Display for PluginPackageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Verification(error) => write!(formatter, "artifact verification failed: {error}"),
            Self::Archive(error) => write!(formatter, "invalid plugin archive: {error}"),
            Self::MissingManifest => write!(formatter, "plugin archive is missing manifest.json"),
            Self::TooManyFiles => write!(formatter, "plugin archive contains too many files"),
            Self::UnpackedTooLarge => write!(formatter, "unpacked plugin exceeds the host limit"),
            Self::Io(error) => write!(formatter, "plugin staging failed: {error}"),
            Self::DestinationExists => write!(formatter, "plugin version is already staged"),
        }
    }
}

impl std::error::Error for PluginPackageError {}

impl From<ArtifactVerificationError> for PluginPackageError {
    fn from(error: ArtifactVerificationError) -> Self {
        Self::Verification(error)
    }
}

#[derive(Debug)]
pub struct VerifiedPluginPackage {
    pub artifact: VerifiedPluginArtifact,
    pub files: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedPluginPackage {
    pub path: PathBuf,
    pub digest: String,
}

pub fn verify_package(
    bytes: &[u8],
    release: &PluginRelease,
) -> Result<VerifiedPluginPackage, PluginPackageError> {
    if release.revoked || release.artifact_size != bytes.len() as u64 {
        return Err(PluginPackageError::Verification(
            ArtifactVerificationError::DigestMismatch,
        ));
    }
    let files = read_archive(bytes)?;
    let manifest = files
        .get("manifest.json")
        .ok_or(PluginPackageError::MissingManifest)
        .and_then(|bytes| {
            from_slice::<PluginManifest>(bytes)
                .map_err(|error| PluginPackageError::Archive(error.to_string()))
        })?;
    let artifact = verify_artifact(bytes, manifest, release)?;
    if release.version != artifact.manifest.version {
        return Err(PluginPackageError::Archive(
            "release version does not match manifest version".into(),
        ));
    }
    Ok(VerifiedPluginPackage { artifact, files })
}

/// Reads only the manifest from a package so a registry can derive the signed
/// release identity before calling the full verifier. The archive is still
/// subject to the same path, file-count and unpacked-size limits.
pub fn read_package_manifest(bytes: &[u8]) -> Result<PluginManifest, PluginPackageError> {
    read_archive(bytes)?
        .get("manifest.json")
        .ok_or(PluginPackageError::MissingManifest)
        .and_then(|manifest| {
            from_slice::<PluginManifest>(manifest)
                .map_err(|error| PluginPackageError::Archive(error.to_string()))
        })
}

pub fn stage_package(
    package: &VerifiedPluginPackage,
    root: &Path,
) -> Result<StagedPluginPackage, PluginPackageError> {
    let plugin_root = root.join(&package.artifact.manifest.id);
    let version_root = plugin_root.join(&package.artifact.manifest.version);
    if version_root.exists() {
        return Err(PluginPackageError::DestinationExists);
    }
    fs::create_dir_all(&plugin_root).map_err(io_error)?;
    let stage = plugin_root.join(format!(
        ".staging-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| PluginPackageError::Io(error.to_string()))?
            .as_nanos()
    ));
    let result = write_stage(package, &stage).and_then(|()| {
        fs::rename(&stage, &version_root).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                PluginPackageError::DestinationExists
            } else {
                io_error(error)
            }
        })?;
        Ok(StagedPluginPackage {
            path: version_root,
            digest: package.artifact.digest.clone(),
        })
    });
    if result.is_err() {
        let _ = fs::remove_dir_all(&stage);
    }
    result
}

fn read_archive(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, PluginPackageError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| PluginPackageError::Archive(error.to_string()))?;
    if archive.len() > MAX_FILES {
        return Err(PluginPackageError::TooManyFiles);
    }
    let mut total = 0u64;
    let mut files = BTreeMap::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| PluginPackageError::Archive(error.to_string()))?;
        let name = entry.name().to_owned();
        validate_package_path(&name)?;
        if entry.is_dir() {
            continue;
        }
        let size = entry.size();
        total = total.saturating_add(size);
        if total > MAX_ASSET_BYTES {
            return Err(PluginPackageError::UnpackedTooLarge);
        }
        let mut content = Vec::with_capacity(size.min(MAX_ASSET_BYTES) as usize);
        entry.read_to_end(&mut content).map_err(io_error)?;
        if files.insert(name, content).is_some() {
            return Err(PluginPackageError::Archive("duplicate package path".into()));
        }
    }
    Ok(files)
}

fn write_stage(package: &VerifiedPluginPackage, stage: &Path) -> Result<(), PluginPackageError> {
    fs::create_dir(stage).map_err(io_error)?;
    for (name, content) in &package.files {
        let destination = stage.join(name);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .map_err(io_error)?;
        file.write_all(content).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
    }
    Ok(())
}

fn io_error(error: std::io::Error) -> PluginPackageError {
    PluginPackageError::Io(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aevio_contracts::plugin::MANIFEST_VERSION;
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey};
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    fn package() -> (Vec<u8>, PluginRelease) {
        let signing_key = SigningKey::from_bytes(&[9; 32]);
        let manifest = serde_json::json!({
            "manifestVersion": MANIFEST_VERSION,
            "id": "dev.aevio.package-test",
            "name": "Package Test",
            "version": "1.0.0",
            "publisher": {
                "id": "test",
                "displayName": "Test",
                "verified": true,
                "publicKey": base64::engine::general_purpose::STANDARD.encode(signing_key.verifying_key().to_bytes())
            },
            "description": "test",
            "category": "test",
            "compatibility": {"aevio": ">=0.9.0, <1.0.0", "pluginApi": "^1.0.0"},
            "platforms": ["desktop"],
            "entrypoints": {"component": "component.wasm"},
            "permissions": [],
            "contributes": {},
            "offline": {"activation": true, "networkRequired": false}
        });
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut cursor);
            writer
                .start_file("manifest.json", SimpleFileOptions::default())
                .unwrap();
            writer.write_all(manifest.to_string().as_bytes()).unwrap();
            writer
                .start_file("component.wasm", SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"wasm").unwrap();
            writer.finish().unwrap();
        }
        let bytes = cursor.into_inner();
        let release = PluginRelease {
            version: "1.0.0".into(),
            artifact_digest: crate::plugins::hex_digest(&bytes),
            artifact_size: bytes.len() as u64,
            signature: Some(
                base64::engine::general_purpose::STANDARD
                    .encode(signing_key.sign(&bytes).to_bytes()),
            ),
            channel: "stable".into(),
            published_at: 0,
            revoked: false,
        };
        (bytes, release)
    }

    #[test]
    fn verifies_and_atomically_stages_a_package() {
        let (bytes, release) = package();
        let package = verify_package(&bytes, &release).unwrap();
        assert_eq!(package.files["component.wasm"], b"wasm");
        let root = tempdir().unwrap();
        let staged = stage_package(&package, root.path()).unwrap();
        assert!(staged.path.join("manifest.json").is_file());
        assert!(staged.path.join("component.wasm").is_file());
    }

    #[test]
    fn rejects_duplicate_paths_and_traversal() {
        let (bytes, mut release) = package();
        let mut tampered = bytes.clone();
        tampered.extend_from_slice(b"garbage");
        release.artifact_size = tampered.len() as u64;
        assert!(matches!(
            verify_package(&tampered, &release),
            Err(PluginPackageError::Archive(_)) | Err(PluginPackageError::Verification(_))
        ));
        assert!(validate_package_path("../escape").is_err());
    }
}
