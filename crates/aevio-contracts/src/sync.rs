//! Sync v1 wire contract. Timestamps retain their existing units.
//!
//! Only registered entity types and operations are replicated. A push that
//! carries anything else is rejected instead of being stored blindly, so the
//! server never becomes a generic row-replication channel.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Most operations one push may carry.
pub const MAX_OPERATIONS_PER_PUSH: usize = 500;
/// Largest single operation payload, in bytes of canonical JSON.
pub const MAX_PAYLOAD_BYTES: usize = 256 * 1024;

/// The only entity type currently shareable between devices.
pub const ENTITY_TIMELINE: &str = "timeline";

/// Registered operations per entity type.
pub fn registered_operations(entity_type: &str) -> &'static [&'static str] {
    match entity_type {
        ENTITY_TIMELINE => &["upsert", "delete"],
        _ => &[],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushValidationError {
    EmptyDevice,
    NoOperations,
    TooManyOperations,
    EmptyWorkspace,
    EmptyOperationId,
    EmptyEntityId,
    UnregisteredEntityType,
    UnsupportedOperation,
    PayloadTooLarge,
}

impl PushValidationError {
    /// Client-facing prose for the corresponding error code.
    pub fn message(self) -> &'static str {
        match self {
            Self::EmptyDevice => "device id is required",
            Self::NoOperations => "a push must carry at least one operation",
            Self::TooManyOperations => "a push carries too many operations",
            Self::EmptyWorkspace => "workspace id is required",
            Self::EmptyOperationId => "operation id is required",
            Self::EmptyEntityId => "entity id is required",
            Self::UnregisteredEntityType => "entity type is not registered for sync",
            Self::UnsupportedOperation => "operation is not registered for this entity type",
            Self::PayloadTooLarge => "operation payload is too large",
        }
    }
}

/// Validates a push envelope and every operation in it.
pub fn validate_push_request(request: &SyncPushRequest) -> Result<(), PushValidationError> {
    if request.device_id.trim().is_empty() {
        return Err(PushValidationError::EmptyDevice);
    }
    if request.operations.is_empty() {
        return Err(PushValidationError::NoOperations);
    }
    if request.operations.len() > MAX_OPERATIONS_PER_PUSH {
        return Err(PushValidationError::TooManyOperations);
    }
    request.operations.iter().try_for_each(validate_operation)
}

fn validate_operation(operation: &SyncOperation) -> Result<(), PushValidationError> {
    if operation.operation_id.trim().is_empty() {
        return Err(PushValidationError::EmptyOperationId);
    }
    if operation.workspace_id.trim().is_empty() {
        return Err(PushValidationError::EmptyWorkspace);
    }
    if operation.entity_id.trim().is_empty() {
        return Err(PushValidationError::EmptyEntityId);
    }
    let allowed = registered_operations(&operation.entity_type);
    if allowed.is_empty() {
        return Err(PushValidationError::UnregisteredEntityType);
    }
    if !allowed.contains(&operation.operation.as_str()) {
        return Err(PushValidationError::UnsupportedOperation);
    }
    if canonical_payload(&operation.payload).len() > MAX_PAYLOAD_BYTES {
        return Err(PushValidationError::PayloadTooLarge);
    }
    Ok(())
}

/// Deterministic JSON text: object keys are sorted at every depth.
pub fn canonical_payload(payload: &Value) -> String {
    let mut out = String::new();
    write_canonical(payload, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_unstable();
            out.push('{');
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(key.clone()).to_string());
                out.push(':');
                write_canonical(&map[key], out);
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

impl SyncOperation {
    /// Content identity of this operation, independent of its operation id.
    ///
    /// Two operations with the same id and the same fingerprint are an
    /// idempotent replay; the same id with a different fingerprint is a
    /// conflict, never a silent overwrite.
    pub fn fingerprint(&self) -> String {
        let payload = canonical_payload(&self.payload);
        let parts = [
            self.entity_type.as_str(),
            self.entity_id.as_str(),
            self.operation.as_str(),
            self.workspace_id.as_str(),
            payload.as_str(),
        ];
        let mut out = String::new();
        for part in parts {
            out.push_str(&part.len().to_string());
            out.push(':');
            out.push_str(part);
            out.push(';');
        }
        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncOperation {
    pub operation_id: String,
    pub workspace_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub operation: String,
    pub payload: serde_json::Value,
    pub base_revision: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPushRequest {
    pub device_id: String,
    pub operations: Vec<SyncOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncChange {
    pub revision: u64,
    pub operation: SyncOperation,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPushResponse {
    pub accepted: Vec<String>,
    pub conflicts: Vec<String>,
    pub cursor: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPullResponse {
    pub changes: Vec<SyncChange>,
    pub cursor: u64,
}
