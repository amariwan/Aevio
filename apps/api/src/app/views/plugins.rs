use std::fs::{self, OpenOptions};
use std::io::Write;

use axum::Json;
use axum::body::{Body, Bytes};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use serde::Deserialize;

use crate::app::authz::authenticate;
use crate::app::core::database::{DatabaseBackend, PluginRegistryRecord, Repository};
use crate::app::services::auth::unix_now;
use crate::app::shared::ApiError;
use crate::app::state::AppState;
use aevio_contracts::plugin::{MAX_ASSET_BYTES, PluginCatalogEntry, PluginManifest, PluginRelease};
use aevio_services::identity::{Actor, Permission};

/// Returns signed-release metadata for built-ins and persisted remote plugins.
#[derive(Debug, Default, Deserialize)]
pub struct CatalogQuery {
    pub q: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublisherKeyRequest {
    pub key_id: String,
    pub public_key: String,
    pub expires_at: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CatalogQuery>,
) -> Result<axum::Json<Vec<PluginCatalogEntry>>, ApiError> {
    let db: &DatabaseBackend = state.connect().await?;
    authenticate(&headers, &state, db).await?;
    let query_text = query.q.unwrap_or_default().trim().to_ascii_lowercase();
    let entries = catalog(db).await?;
    Ok(axum::Json(
        entries
            .into_iter()
            .filter(|entry| {
                query
                    .category
                    .as_deref()
                    .is_none_or(|category| entry.manifest.category == category)
            })
            .filter(|entry| {
                query_text.is_empty()
                    || format!("{} {}", entry.manifest.name, entry.manifest.description)
                        .to_ascii_lowercase()
                        .contains(&query_text)
            })
            .collect(),
    ))
}

pub async fn detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(plugin_id): Path<String>,
) -> Result<axum::Json<PluginCatalogEntry>, ApiError> {
    let db: &DatabaseBackend = state.connect().await?;
    authenticate(&headers, &state, db).await?;
    catalog(db)
        .await?
        .into_iter()
        .find(|entry| entry.manifest.id == plugin_id)
        .map(axum::Json)
        .ok_or_else(|| ApiError::not_found("plugin does not exist"))
}

pub async fn publish(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((plugin_id, version)): Path<(String, String)>,
    body: Bytes,
) -> Result<axum::Json<PluginRelease>, ApiError> {
    if body.len() as u64 > MAX_ASSET_BYTES {
        return Err(ApiError::invalid_request(
            "plugin artifact exceeds the host limit",
        ));
    }
    let db: &DatabaseBackend = state.connect().await?;
    let actor = authenticate(&headers, &state, db).await?;
    require_registry_admin(&actor)?;
    let signature = required_header(&headers, "x-aevio-plugin-signature")?;
    let key_id = required_text_header(&headers, "x-aevio-plugin-key-id")?;
    let channel = headers
        .get("x-aevio-plugin-channel")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("stable")
        .trim()
        .to_ascii_lowercase();
    if channel.is_empty()
        || channel.len() > 32
        || !channel
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(ApiError::invalid_request(
            "plugin release channel is invalid",
        ));
    }
    let manifest = aevio_infrastructure::plugins_package::read_package_manifest(&body)
        .map_err(|error| ApiError::invalid_request(error.to_string()))?;
    if manifest.id != plugin_id || manifest.version != version {
        return Err(ApiError::invalid_request(
            "release path does not match the package manifest",
        ));
    }
    let public_key = manifest
        .publisher
        .public_key
        .as_deref()
        .ok_or_else(|| ApiError::invalid_request("plugin publisher key is missing"))?;
    if !Repository::new(db)
        .trusted_publisher_key(&manifest.publisher.id, &key_id, public_key, unix_now())
        .await
        .map_err(ApiError::internal)?
    {
        return Err(ApiError::forbidden(
            "plugin publisher key is not trusted or has expired",
        ));
    }
    let release = PluginRelease {
        version,
        artifact_digest: aevio_infrastructure::plugins::hex_digest(&body),
        artifact_size: body.len() as u64,
        signature: Some(signature),
        channel,
        published_at: unix_now(),
        revoked: false,
    };
    aevio_infrastructure::plugins_package::verify_package(&body, &release)
        .map_err(|error| ApiError::invalid_request(error.to_string()))?;
    let artifact_path =
        write_artifact(&state, &release.artifact_digest, &body).map_err(ApiError::internal)?;
    Repository::new(db)
        .upsert_plugin_release(&manifest, &release, unix_now())
        .await
        .map_err(ApiError::internal)?;
    eprintln!(
        "plugin release published: {} {} at {}",
        manifest.id,
        release.version,
        artifact_path.display()
    );
    Ok(axum::Json(release))
}

pub async fn register_publisher_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(publisher_id): Path<String>,
    Json(request): Json<PublisherKeyRequest>,
) -> Result<StatusCode, ApiError> {
    let db: &DatabaseBackend = state.connect().await?;
    let actor = authenticate(&headers, &state, db).await?;
    require_registry_admin(&actor)?;
    if publisher_id.is_empty()
        || publisher_id.len() > 160
        || publisher_id.chars().any(char::is_control)
        || request.key_id.is_empty()
        || request.key_id.len() > 120
        || request.key_id.chars().any(char::is_control)
    {
        return Err(ApiError::invalid_request(
            "publisher key identity is invalid",
        ));
    }
    if request
        .expires_at
        .is_some_and(|expires_at| expires_at <= unix_now())
    {
        return Err(ApiError::invalid_request(
            "publisher key expiration must be in the future",
        ));
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&request.public_key)
        .map_err(|_| ApiError::invalid_request("publisher public key is invalid"))?;
    if decoded.len() != 32 {
        return Err(ApiError::invalid_request("publisher public key is invalid"));
    }
    Repository::new(db)
        .rotate_publisher_key(
            publisher_id,
            request.key_id,
            request.public_key,
            request.expires_at,
            unix_now(),
        )
        .await
        .map_err(|error| {
            if error.contains("immutable") {
                ApiError::conflict("publisher key id is already bound to another key")
            } else {
                ApiError::internal(error)
            }
        })?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn revoke(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((plugin_id, version)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let db: &DatabaseBackend = state.connect().await?;
    let actor = authenticate(&headers, &state, db).await?;
    require_registry_admin(&actor)?;
    Repository::new(db)
        .revoke_plugin_release(&plugin_id, &version)
        .await
        .map_err(ApiError::internal)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn download(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((plugin_id, version)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let db: &DatabaseBackend = state.connect().await?;
    authenticate(&headers, &state, db).await?;
    let record = Repository::new(db)
        .plugin_release(&plugin_id, &version)
        .await
        .map_err(ApiError::internal)?
        .filter(|record| !record.release.revoked)
        .ok_or_else(|| ApiError::not_found("plugin release does not exist"))?;
    let path = artifact_path(&state, &record.release.artifact_digest)
        .ok_or_else(|| ApiError::internal("plugin artifact digest is invalid"))?;
    let bytes =
        fs::read(&path).map_err(|_| ApiError::not_found("plugin artifact is unavailable"))?;
    if bytes.len() as u64 != record.release.artifact_size
        || aevio_infrastructure::plugins::hex_digest(&bytes) != record.release.artifact_digest
    {
        return Err(ApiError::internal(
            "plugin artifact failed integrity verification",
        ));
    }
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/zip"),
    );
    response_headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response_headers.insert(
        header::ETAG,
        HeaderValue::from_str(&format!("\"{}\"", record.release.artifact_digest))
            .map_err(ApiError::internal)?,
    );
    Ok((StatusCode::OK, response_headers, Body::from(bytes)).into_response())
}

async fn catalog(db: &DatabaseBackend) -> Result<Vec<PluginCatalogEntry>, ApiError> {
    let mut entries = aevio_services::plugins::public_catalog();
    let rows = Repository::new(db)
        .plugin_registry_rows()
        .await
        .map_err(ApiError::internal)?;
    for row in rows {
        merge_registry_release(&mut entries, row)?;
    }
    Ok(entries)
}

fn merge_registry_release(
    entries: &mut Vec<PluginCatalogEntry>,
    row: PluginRegistryRecord,
) -> Result<(), ApiError> {
    if row.release.revoked {
        if let Some(entry) = entries
            .iter_mut()
            .find(|entry| entry.manifest.id == row.plugin_id)
        {
            entry
                .releases
                .retain(|release| release.version != row.release.version);
        }
        return Ok(());
    }
    let manifest: PluginManifest =
        serde_json::from_str(&row.manifest_json).map_err(ApiError::internal)?;
    if manifest.id != row.plugin_id {
        return Err(ApiError::internal(
            "plugin registry manifest identity mismatch",
        ));
    }
    if let Some(entry) = entries
        .iter_mut()
        .find(|entry| entry.manifest.id == row.plugin_id)
    {
        entry
            .releases
            .retain(|release| release.version != row.release.version);
        entry.releases.push(row.release);
    } else {
        entries.push(PluginCatalogEntry {
            manifest,
            releases: vec![row.release],
            installed: None,
        });
    }
    Ok(())
}

fn require_registry_admin(actor: &Actor) -> Result<(), ApiError> {
    match actor {
        Actor::Service { scope } if scope.allows_permission(Permission::Admin) => Ok(()),
        _ => Err(ApiError::forbidden(
            "plugin publication requires an admin service credential",
        )),
    }
}

fn required_header(headers: &HeaderMap, name: &str) -> Result<String, ApiError> {
    let value = headers
        .get(name)
        .ok_or_else(|| ApiError::invalid_request("required plugin release header is missing"))?
        .to_str()
        .map_err(|_| ApiError::invalid_request("plugin release header is invalid"))?
        .trim();
    let decoded = base64::engine::general_purpose::STANDARD.decode(value);
    let valid_signature = decoded.is_ok_and(|bytes| bytes.len() == 64);
    if value.is_empty() || value.len() > 512 || !valid_signature {
        return Err(ApiError::invalid_request(
            "plugin release signature is invalid",
        ));
    }
    Ok(value.to_owned())
}

fn required_text_header(headers: &HeaderMap, name: &str) -> Result<String, ApiError> {
    let value = headers
        .get(name)
        .ok_or_else(|| ApiError::invalid_request("required plugin release header is missing"))?
        .to_str()
        .map_err(|_| ApiError::invalid_request("plugin release header is invalid"))?
        .trim();
    if value.is_empty() || value.len() > 120 || value.chars().any(char::is_control) {
        return Err(ApiError::invalid_request(
            "plugin release header is invalid",
        ));
    }
    Ok(value.to_owned())
}

fn artifact_path(state: &AppState, digest: &str) -> Option<std::path::PathBuf> {
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(
        state
            .plugin_artifact_dir
            .join("sha256")
            .join(&digest[..2])
            .join(digest),
    )
}

fn write_artifact(
    state: &AppState,
    digest: &str,
    bytes: &[u8],
) -> Result<std::path::PathBuf, std::io::Error> {
    let path = artifact_path(state, digest)
        .expect("the release digest is generated by the local SHA-256 implementation");
    if path.is_file() {
        return Ok(path);
    }
    let parent = path
        .parent()
        .expect("content-addressed artifact has a parent");
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".{digest}.{}.tmp", uuid::Uuid::now_v7()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    match fs::rename(&temporary, &path) {
        Ok(()) => Ok(path),
        Err(_error) if path.is_file() => {
            let _ = fs::remove_file(&temporary);
            Ok(path)
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{require_registry_admin, required_header, required_text_header};
    use aevio_services::identity::{Actor, Permission, ServiceScope};
    use axum::http::{HeaderMap, HeaderValue};
    use base64::Engine;

    #[test]
    fn release_headers_are_strictly_validated() {
        let mut headers = HeaderMap::new();
        headers.insert("x-aevio-plugin-signature", HeaderValue::from_static("nope"));
        assert!(required_header(&headers, "x-aevio-plugin-signature").is_err());
        headers.insert(
            "x-aevio-plugin-signature",
            HeaderValue::from_str(&base64::engine::general_purpose::STANDARD.encode([0_u8; 64]))
                .expect("header"),
        );
        assert!(required_header(&headers, "x-aevio-plugin-signature").is_ok());
        headers.insert("x-aevio-plugin-key-id", HeaderValue::from_static("key-1"));
        assert_eq!(
            required_text_header(&headers, "x-aevio-plugin-key-id").expect("key id"),
            "key-1"
        );
    }

    #[test]
    fn only_admin_service_credentials_can_publish() {
        let admin = Actor::service(ServiceScope::new(["workspace"], [Permission::Admin]));
        let writer = Actor::service(ServiceScope::new(["workspace"], [Permission::Write]));
        assert!(require_registry_admin(&admin).is_ok());
        assert!(require_registry_admin(&writer).is_err());
        assert!(require_registry_admin(&Actor::account("account")).is_err());
    }
}
