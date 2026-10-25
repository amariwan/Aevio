use aevio_contracts::plugin::{PluginManifest, PluginRelease};

use super::{DatabaseBackend, Repository, postgres, sqlite};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRegistryRecord {
    pub plugin_id: String,
    pub manifest_json: String,
    pub release: PluginRelease,
}

impl<'db> Repository<'db> {
    pub async fn plugin_registry_rows(&self) -> Result<Vec<PluginRegistryRecord>, String> {
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| sqlite::Repository::new(&database).plugin_registry_rows())
                .map(|rows| {
                    rows.into_iter()
                        .filter_map(plugin_registry_record)
                        .collect()
                })
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .plugin_registry_rows()
                .await
                .map(|rows| {
                    rows.into_iter()
                        .filter_map(plugin_registry_record)
                        .collect()
                })
                .map_err(|error| error.to_string()),
        }
    }

    pub async fn plugin_release(
        &self,
        plugin_id: &str,
        version: &str,
    ) -> Result<Option<PluginRegistryRecord>, String> {
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| {
                    sqlite::Repository::new(&database).plugin_release(plugin_id, version)
                })
                .map(|row| row.and_then(plugin_registry_record))
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .plugin_release(plugin_id, version)
                .await
                .map(|row| row.and_then(plugin_registry_record))
                .map_err(|error| error.to_string()),
        }
    }

    pub async fn upsert_plugin_release(
        &self,
        manifest: &PluginManifest,
        release: &PluginRelease,
        now: i64,
    ) -> Result<(), String> {
        let manifest_json = serde_json::to_string(manifest).map_err(|error| error.to_string())?;
        let signature = release
            .signature
            .clone()
            .ok_or_else(|| "registry releases require a signature".to_owned())?;
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| {
                    sqlite::Repository::new(&database).upsert_plugin_release(
                        manifest.id.clone(),
                        manifest.publisher.id.clone(),
                        manifest_json.clone(),
                        release.version.clone(),
                        release.artifact_digest.clone(),
                        release.artifact_size as i64,
                        signature.clone(),
                        release.channel.clone(),
                        release.published_at,
                        now,
                    )
                })
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .upsert_plugin_release(
                    manifest.id.clone(),
                    manifest.publisher.id.clone(),
                    manifest_json,
                    release.version.clone(),
                    release.artifact_digest.clone(),
                    release.artifact_size as i64,
                    signature,
                    release.channel.clone(),
                    release.published_at,
                    now,
                )
                .await
                .map_err(|error| error.to_string()),
        }
    }

    pub async fn trusted_publisher_key(
        &self,
        publisher_id: &str,
        key_id: &str,
        public_key: &str,
        now: i64,
    ) -> Result<bool, String> {
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| {
                    sqlite::Repository::new(&database).trusted_publisher_key(
                        publisher_id,
                        key_id,
                        public_key,
                        now,
                    )
                })
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .trusted_publisher_key(publisher_id, key_id, public_key, now)
                .await
                .map_err(|error| error.to_string()),
        }
    }

    pub async fn rotate_publisher_key(
        &self,
        publisher_id: String,
        key_id: String,
        public_key: String,
        expires_at: Option<i64>,
        now: i64,
    ) -> Result<(), String> {
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| {
                    sqlite::Repository::new(&database).rotate_publisher_key(
                        publisher_id,
                        key_id,
                        public_key,
                        expires_at,
                        now,
                    )
                })
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .rotate_publisher_key(publisher_id, key_id, public_key, expires_at, now)
                .await
                .map_err(|error| error.to_string()),
        }
    }

    pub async fn revoke_plugin_release(
        &self,
        plugin_id: &str,
        version: &str,
    ) -> Result<bool, String> {
        match self.database {
            DatabaseBackend::Sqlite(path) => sqlite::connect(path)
                .and_then(|database| {
                    sqlite::Repository::new(&database).revoke_plugin_release(plugin_id, version)
                })
                .map_err(|error| error.to_string()),
            DatabaseBackend::Postgres(database) => postgres::Repository::new(database)
                .revoke_plugin_release(plugin_id, version)
                .await
                .map_err(|error| error.to_string()),
        }
    }
}

fn plugin_registry_record(row: impl PluginRegistryRowFields) -> Option<PluginRegistryRecord> {
    let manifest_json = row.manifest_json().to_owned();
    serde_json::from_str::<PluginManifest>(&manifest_json).ok()?;
    Some(PluginRegistryRecord {
        plugin_id: row.plugin_id().to_owned(),
        manifest_json,
        release: PluginRelease {
            version: row.version().to_owned(),
            artifact_digest: row.artifact_digest().to_owned(),
            artifact_size: u64::try_from(row.artifact_size()).ok()?,
            signature: Some(row.signature().to_owned()),
            channel: row.channel().to_owned(),
            published_at: row.published_at(),
            revoked: row.revoked(),
        },
    })
}

trait PluginRegistryRowFields {
    fn plugin_id(&self) -> &str;
    fn manifest_json(&self) -> &str;
    fn version(&self) -> &str;
    fn artifact_digest(&self) -> &str;
    fn artifact_size(&self) -> i64;
    fn signature(&self) -> &str;
    fn channel(&self) -> &str;
    fn published_at(&self) -> i64;
    fn revoked(&self) -> bool;
}

macro_rules! impl_plugin_registry_row_fields {
    ($type:ty) => {
        impl PluginRegistryRowFields for $type {
            fn plugin_id(&self) -> &str {
                &self.plugin_id
            }
            fn manifest_json(&self) -> &str {
                &self.manifest_json
            }
            fn version(&self) -> &str {
                &self.version
            }
            fn artifact_digest(&self) -> &str {
                &self.artifact_digest
            }
            fn artifact_size(&self) -> i64 {
                self.artifact_size
            }
            fn signature(&self) -> &str {
                &self.signature
            }
            fn channel(&self) -> &str {
                &self.channel
            }
            fn published_at(&self) -> i64 {
                self.published_at
            }
            fn revoked(&self) -> bool {
                self.revoked
            }
        }
    };
}

impl_plugin_registry_row_fields!(sqlite::PluginRegistryRow);
impl_plugin_registry_row_fields!(postgres::PluginRegistryRow);
