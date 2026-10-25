//! Contract shape snapshots for HTTP/IPC/CLI transport envelopes.
//!
//! These tests lock the serialized JSON shape of shared transport types so that
//! accidental renames or field removals fail loudly. They are intentionally
//! minimal and deterministic.

use aevio_contracts::api::{ApiErrorBody, codes};
use aevio_contracts::sync::{
    ENTITY_TIMELINE, MAX_OPERATIONS_PER_PUSH, MAX_PAYLOAD_BYTES, SyncOperation, SyncPushRequest,
};
use serde_json::json;

#[test]
fn api_error_body_shape_is_stable() {
    let body = ApiErrorBody::new(codes::FORBIDDEN, "workspace role is not sufficient");
    let encoded = serde_json::to_value(&body).expect("serialize");
    assert_eq!(
        encoded,
        json!({"error": {"code": "FORBIDDEN", "message": "workspace role is not sufficient"}})
    );
}

#[test]
fn sync_push_request_shape_is_stable() {
    let request = SyncPushRequest {
        device_id: "device-1".to_owned(),
        operations: vec![SyncOperation {
            operation_id: "op-1".to_owned(),
            workspace_id: "ws-1".to_owned(),
            entity_type: ENTITY_TIMELINE.to_owned(),
            entity_id: "entry-1".to_owned(),
            operation: "upsert".to_owned(),
            payload: json!({"title": "hello"}),
            base_revision: Some(3),
        }],
    };
    let encoded = serde_json::to_value(&request).expect("serialize");
    assert_eq!(encoded["deviceId"], "device-1");
    assert_eq!(encoded["operations"][0]["operationId"], "op-1");
    assert_eq!(encoded["operations"][0]["workspaceId"], "ws-1");
    assert_eq!(encoded["operations"][0]["entityType"], "timeline");
    assert_eq!(encoded["operations"][0]["entityId"], "entry-1");
    assert_eq!(encoded["operations"][0]["operation"], "upsert");
    assert_eq!(encoded["operations"][0]["baseRevision"], 3);
}

#[test]
fn sync_limits_are_stable() {
    assert_eq!(MAX_OPERATIONS_PER_PUSH, 500);
    assert_eq!(MAX_PAYLOAD_BYTES, 256 * 1024);
}
