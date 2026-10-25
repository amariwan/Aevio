use aevio_contracts::plugin::{PluginLifecycleState, capabilities};
use aevio_persistence::database::entities::plugins::storage_entries;
use aevio_persistence::database::repositories::plugins::PluginRepository;
use sea_orm::{ActiveValue::Set, DatabaseConnection};
use serde_json::Value;
use uuid::Uuid;

use super::plugins::{
    CapabilityGateway, PluginExecutionContext, PluginServiceError, audit,
    effective_permissions_for_context,
};

const USER_REF: &str = "local-user";
const LEGACY_DEVICE_REF: &str = "legacy-device";
const MAX_STORAGE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone)]
struct StorageContext {
    user_ref: &'static str,
    workspace_ref: &'static str,
    device_ref: String,
}

pub fn get(
    db: &DatabaseConnection,
    plugin_id: &str,
    key: &str,
) -> Result<Option<Value>, PluginServiceError> {
    get_for_device(db, plugin_id, key, LEGACY_DEVICE_REF)
}

pub fn get_for_device(
    db: &DatabaseConnection,
    plugin_id: &str,
    key: &str,
    device_ref: &str,
) -> Result<Option<Value>, PluginServiceError> {
    validate_key(key)?;
    let (repository, installation, context) =
        access(db, plugin_id, capabilities::STORAGE_OWN_READ, device_ref)?;
    let Some(entry) = repository.storage_entry(
        &installation.id,
        context.user_ref,
        context.workspace_ref,
        &context.device_ref,
        key,
    )?
    else {
        return Ok(None);
    };
    serde_json::from_str(&entry.value_json)
        .map(Some)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))
}

pub fn set(
    db: &DatabaseConnection,
    plugin_id: &str,
    key: &str,
    value: Value,
    now: i64,
) -> Result<(), PluginServiceError> {
    set_for_device(db, plugin_id, key, value, now, LEGACY_DEVICE_REF)
}

pub fn set_for_device(
    db: &DatabaseConnection,
    plugin_id: &str,
    key: &str,
    value: Value,
    now: i64,
    device_ref: &str,
) -> Result<(), PluginServiceError> {
    validate_key(key)?;
    let (repository, installation, context) =
        access(db, plugin_id, capabilities::STORAGE_OWN_WRITE, device_ref)?;
    let value_json = serde_json::to_string(&value)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    let size_bytes = value_json.len() as i64;
    if size_bytes as u64 > MAX_STORAGE_BYTES {
        return Err(PluginServiceError::Permission(
            "plugin storage value exceeds the host limit".into(),
        ));
    }
    if let Some(entry) = repository.storage_entry(
        &installation.id,
        context.user_ref,
        context.workspace_ref,
        &context.device_ref,
        key,
    )? {
        let mut model: storage_entries::ActiveModel = entry.into();
        model.value_json = Set(value_json);
        model.size_bytes = Set(size_bytes);
        model.updated_at = Set(now);
        repository.update_storage_entry(model)?;
    } else {
        repository.insert_storage_entry(storage_entries::ActiveModel {
            id: Set(Uuid::now_v7().to_string()),
            installation_id: Set(installation.id.clone()),
            user_ref: Set(context.user_ref.into()),
            workspace_ref: Set(context.workspace_ref.into()),
            device_ref: Set(context.device_ref.clone()),
            key: Set(key.into()),
            value_json: Set(value_json),
            schema_version: Set(1),
            size_bytes: Set(size_bytes),
            updated_at: Set(now),
        })?;
    }
    audit(
        &repository,
        &installation.id,
        "storage_set",
        now,
        context.workspace_ref,
    )?;
    Ok(())
}

pub fn delete(
    db: &DatabaseConnection,
    plugin_id: &str,
    key: &str,
    now: i64,
) -> Result<(), PluginServiceError> {
    delete_for_device(db, plugin_id, key, now, LEGACY_DEVICE_REF)
}

pub fn delete_for_device(
    db: &DatabaseConnection,
    plugin_id: &str,
    key: &str,
    now: i64,
    device_ref: &str,
) -> Result<(), PluginServiceError> {
    validate_key(key)?;
    let (repository, installation, context) =
        access(db, plugin_id, capabilities::STORAGE_OWN_WRITE, device_ref)?;
    if let Some(entry) = repository.storage_entry(
        &installation.id,
        context.user_ref,
        context.workspace_ref,
        &context.device_ref,
        key,
    )? {
        repository.delete_storage_entry(&entry.id)?;
        audit(
            &repository,
            &installation.id,
            "storage_delete",
            now,
            context.workspace_ref,
        )?;
    }
    Ok(())
}

fn access<'db>(
    db: &'db DatabaseConnection,
    plugin_id: &str,
    capability: &str,
    device_ref: &str,
) -> Result<
    (
        PluginRepository<'db>,
        aevio_persistence::database::entities::plugins::installations::Model,
        StorageContext,
    ),
    PluginServiceError,
> {
    let repository = PluginRepository::new(db);
    let installation = repository
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    let state: PluginLifecycleState = serde_json::from_str(&format!("\"{}\"", installation.state))
        .map_err(|_| PluginServiceError::InvalidState(installation.state.clone()))?;
    if state != PluginLifecycleState::Active {
        return Err(PluginServiceError::InvalidState(
            "plugin must be active".into(),
        ));
    }
    let context = PluginExecutionContext::local(&installation, device_ref)?;
    let gateway = CapabilityGateway::new(
        plugin_id,
        effective_permissions_for_context(
            db,
            &installation,
            &context,
            aevio_core::timers::now_epoch_ms() as i64,
        )?,
    );
    gateway.require(capability)?;
    let scope = match installation.scope.as_str() {
        "personal" => StorageContext {
            user_ref: USER_REF,
            workspace_ref: "personal",
            device_ref: device_ref.to_owned(),
        },
        "workspace" => StorageContext {
            user_ref: USER_REF,
            workspace_ref: "default",
            device_ref: device_ref.to_owned(),
        },
        _ => {
            return Err(PluginServiceError::InvalidState(
                "plugin installation scope is invalid".into(),
            ));
        }
    };
    Ok((repository, installation, scope))
}

fn validate_key(key: &str) -> Result<(), PluginServiceError> {
    if key.is_empty() || key.len() > 256 || key.chars().any(char::is_control) {
        return Err(PluginServiceError::Permission(
            "plugin storage keys must contain 1-256 non-control characters".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_key;

    #[test]
    fn storage_keys_reject_empty_and_control_characters() {
        assert!(validate_key("plugin.setting").is_ok());
        assert!(validate_key("").is_err());
        assert!(validate_key("bad\nkey").is_err());
    }
}
