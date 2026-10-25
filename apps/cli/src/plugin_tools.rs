use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use aevio_contracts::plugin::{PluginManifest, PluginRelease, MAX_ASSET_BYTES, MAX_MANIFEST_BYTES};
use aevio_infrastructure::plugins::{hex_digest, validate_package_path};
use aevio_infrastructure::plugins_package::verify_package;
use serde_json::{json, Value};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const MAX_FILES: usize = 256;

pub fn validate_manifest(path: &Path) -> Result<Value, String> {
    let manifest = read_manifest(path)?;
    Ok(json!({
        "id": manifest.id,
        "name": manifest.name,
        "version": manifest.version,
        "publisher": manifest.publisher.id,
        "valid": true,
    }))
}

pub fn inspect_package(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("cannot read package: {error}"))?;
    let (manifest, files) = read_archive(&bytes)?;
    Ok(json!({
        "id": manifest.id,
        "name": manifest.name,
        "version": manifest.version,
        "publisher": manifest.publisher.id,
        "digest": hex_digest(&bytes),
        "size": bytes.len(),
        "files": files,
    }))
}

pub fn verify_signed_package(path: &Path, signature: &str) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("cannot read package: {error}"))?;
    let (manifest, _) = read_archive(&bytes)?;
    let release = PluginRelease {
        version: manifest.version.clone(),
        artifact_digest: hex_digest(&bytes),
        artifact_size: bytes.len() as u64,
        signature: Some(signature.to_owned()),
        channel: "local".into(),
        published_at: 0,
        revoked: false,
    };
    let verified = verify_package(&bytes, &release).map_err(|error| error.to_string())?;
    Ok(json!({
        "valid": true,
        "id": verified.artifact.manifest.id,
        "version": verified.artifact.manifest.version,
        "digest": verified.artifact.digest,
    }))
}

pub fn pack(source: &Path, output: &Path) -> Result<Value, String> {
    let files = collect_files(source)?;
    let manifest = parse_manifest_bytes(
        files
            .get("manifest.json")
            .ok_or_else(|| "source directory must contain manifest.json".to_owned())?,
    )?;
    if output.exists() {
        return Err(format!("output already exists: {}", output.display()));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create output directory: {error}"))?;
    }
    let temporary = output.with_extension(format!("aevio-plugin.tmp-{}", std::process::id()));
    let result = write_zip(&temporary, &files).and_then(|()| {
        fs::rename(&temporary, output).map_err(|error| format!("cannot publish package: {error}"))
    });
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    let bytes = fs::read(output).map_err(|error| format!("cannot read packed package: {error}"))?;
    Ok(json!({
        "id": manifest.id,
        "version": manifest.version,
        "output": output,
        "digest": hex_digest(&bytes),
        "signed": false,
        "warning": "This package is unsigned and cannot be installed through the production Store.",
    }))
}

fn read_manifest(path: &Path) -> Result<PluginManifest, String> {
    let bytes = fs::read(path).map_err(|error| format!("cannot read manifest: {error}"))?;
    parse_manifest_bytes(&bytes)
}

fn parse_manifest_bytes(bytes: &[u8]) -> Result<PluginManifest, String> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err("manifest exceeds the host limit".into());
    }
    let manifest: PluginManifest =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid manifest JSON: {error}"))?;
    manifest.validate().map_err(|error| error.to_string())?;
    Ok(manifest)
}

fn collect_files(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    if !root.is_dir() {
        return Err(format!("source is not a directory: {}", root.display()));
    }
    let mut files = BTreeMap::new();
    collect_files_inner(root, root, &mut files)?;
    if files.len() > MAX_FILES {
        return Err("source contains too many files".into());
    }
    Ok(files)
}

fn collect_files_inner(
    root: &Path,
    current: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    let mut entries = fs::read_dir(current)
        .map_err(|error| format!("cannot read source directory: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("cannot enumerate source directory: {error}"))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect source entry: {error}"))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("symlinks are not allowed: {}", path.display()));
        }
        if metadata.is_dir() {
            collect_files_inner(root, &path, files)?;
            continue;
        }
        if !metadata.is_file() {
            return Err(format!("unsupported source entry: {}", path.display()));
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        validate_package_path(&relative).map_err(|error| error.to_string())?;
        let bytes = fs::read(&path).map_err(|error| format!("cannot read source file: {error}"))?;
        let total = files
            .values()
            .map(Vec::len)
            .sum::<usize>()
            .saturating_add(bytes.len());
        if total as u64 > MAX_ASSET_BYTES {
            return Err("source exceeds the unpacked package limit".into());
        }
        if files.insert(relative, bytes).is_some() {
            return Err("source contains duplicate package paths".into());
        }
    }
    Ok(())
}

fn write_zip(path: &Path, files: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("cannot create package: {error}"))?;
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, bytes) in files {
        writer
            .start_file(name, options)
            .map_err(|error| format!("cannot add package file: {error}"))?;
        writer
            .write_all(bytes)
            .map_err(|error| format!("cannot write package file: {error}"))?;
    }
    let file = writer
        .finish()
        .map_err(|error| format!("cannot finish package: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("cannot sync package: {error}"))
}

fn read_archive(bytes: &[u8]) -> Result<(PluginManifest, Vec<Value>), String> {
    if bytes.len() as u64 > MAX_ASSET_BYTES {
        return Err("package exceeds the host limit".into());
    }
    let mut archive =
        ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|error| error.to_string())?;
    if archive.len() > MAX_FILES {
        return Err("package contains too many files".into());
    }
    let mut total = 0u64;
    let mut seen = BTreeMap::new();
    let mut manifest_bytes = None;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let name = entry.name().to_owned();
        validate_package_path(&name).map_err(|error| error.to_string())?;
        if entry.is_dir() {
            continue;
        }
        total = total.saturating_add(entry.size());
        if total > MAX_ASSET_BYTES {
            return Err("package exceeds the unpacked limit".into());
        }
        let mut content = Vec::new();
        entry
            .read_to_end(&mut content)
            .map_err(|error| error.to_string())?;
        if seen.insert(name.clone(), content.len()).is_some() {
            return Err("package contains duplicate paths".into());
        }
        if name == "manifest.json" {
            manifest_bytes = Some(content);
        }
    }
    let manifest = parse_manifest_bytes(
        manifest_bytes
            .as_deref()
            .ok_or_else(|| "package is missing manifest.json".to_owned())?,
    )?;
    let files = seen
        .into_iter()
        .map(|(path, size)| json!({ "path": path, "size": size }))
        .collect();
    Ok((manifest, files))
}

#[cfg(test)]
mod tests {
    use super::{inspect_package, pack, validate_manifest};
    use aevio_contracts::plugin::MANIFEST_VERSION;

    #[test]
    fn pack_and_inspect_round_trip_a_valid_source_tree() {
        let root = std::env::temp_dir().join(format!(
            "aevio-plugin-tools-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::fs::write(
            root.join("manifest.json"),
            serde_json::json!({
                "manifestVersion": MANIFEST_VERSION,
                "id": "dev.aevio.cli-test",
                "name": "CLI Test",
                "version": "1.0.0",
                "publisher": {"id": "test", "displayName": "Test"},
                "description": "CLI test package",
                "category": "test",
                "compatibility": {"aevio": ">=0.9.0, <1.0.0", "pluginApi": "^1.0.0"},
                "platforms": ["desktop"],
                "entrypoints": {},
                "contributes": {},
                "offline": {"activation": true, "networkRequired": false}
            })
            .to_string(),
        )
        .unwrap();
        std::fs::write(root.join("assets/icon.png"), b"not-an-image").unwrap();
        let output = root.with_extension("aevio-plugin");
        let packed = pack(&root, &output).unwrap();
        assert_eq!(packed["id"], "dev.aevio.cli-test");
        assert_eq!(
            validate_manifest(&root.join("manifest.json")).unwrap()["valid"],
            true
        );
        let inspected = inspect_package(&output).unwrap();
        assert_eq!(inspected["version"], "1.0.0");
        assert_eq!(inspected["files"].as_array().unwrap().len(), 2);
        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_file(output).unwrap();
    }
}
