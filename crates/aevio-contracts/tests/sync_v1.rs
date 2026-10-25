use aevio_contracts::sync::{SyncPullResponse, SyncPushRequest, SyncPushResponse};
use serde_json::json;

#[test]
fn push_v1_preserves_names_null_revision_and_payload() {
    let wire = json!({
        "deviceId": "device-1",
        "operations": [{
            "operationId": "operation-1",
            "workspaceId": "workspace-1",
            "entityType": "timeline_event",
            "entityId": "event-1",
            "operation": "upsert",
            "payload": {"title": "Explicit publication", "startedAt": 1792600000123_u64},
            "baseRevision": null
        }]
    });
    let request: SyncPushRequest = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(request.operations[0].base_revision, None);
    assert_eq!(serde_json::to_value(request).unwrap(), wire);
}

#[test]
fn responses_preserve_cursors_conflicts_and_timestamps() {
    let push = json!({"accepted": ["op-1"], "conflicts": ["op-2"], "cursor": 42});
    let response: SyncPushResponse = serde_json::from_value(push.clone()).unwrap();
    assert_eq!(serde_json::to_value(response).unwrap(), push);

    let pull = json!({
        "changes": [{
            "revision": 42,
            "operation": {
                "operationId": "op-1", "workspaceId": "ws-1",
                "entityType": "timeline_event", "entityId": "event-1",
                "operation": "delete", "payload": {}, "baseRevision": 41
            },
            "createdAt": 1792600000
        }],
        "cursor": 42
    });
    let response: SyncPullResponse = serde_json::from_value(pull.clone()).unwrap();
    assert_eq!(response.changes[0].created_at, 1792600000);
    assert_eq!(response.changes[0].operation.base_revision, Some(41));
    assert_eq!(serde_json::to_value(response).unwrap(), pull);
}

#[test]
fn malformed_required_fields_and_negative_revisions_are_rejected() {
    assert!(serde_json::from_value::<SyncPushRequest>(json!({"operations": []})).is_err());
    assert!(
        serde_json::from_value::<SyncPullResponse>(json!({
            "changes": [], "cursor": -1
        }))
        .is_err()
    );
}
