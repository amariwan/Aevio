//! Push envelope validation and operation identity.

use aevio_contracts::sync::{
    MAX_OPERATIONS_PER_PUSH, PushValidationError, SyncOperation, SyncPushRequest,
    canonical_payload, registered_operations, validate_push_request,
};
use serde_json::{Value, json};

fn operation(
    operation_id: &str,
    workspace_id: &str,
    entity_type: &str,
    operation: &str,
    payload: Value,
) -> SyncOperation {
    SyncOperation {
        operation_id: operation_id.into(),
        workspace_id: workspace_id.into(),
        entity_type: entity_type.into(),
        entity_id: "entity-1".into(),
        operation: operation.into(),
        payload,
        base_revision: None,
    }
}

fn push(operations: Vec<SyncOperation>) -> SyncPushRequest {
    SyncPushRequest {
        device_id: "device-1".into(),
        operations,
    }
}

fn valid_operation() -> SyncOperation {
    operation(
        "operation-1",
        "workspace-1",
        "timeline",
        "upsert",
        json!({"title": "Focus"}),
    )
}

#[test]
fn registered_operations_are_explicit() {
    assert_eq!(registered_operations("timeline"), &["upsert", "delete"]);
    for unregistered in ["timeline_event", "segments", "", "accounts"] {
        assert!(
            registered_operations(unregistered).is_empty(),
            "{unregistered}"
        );
    }
}

#[test]
fn a_complete_push_is_accepted() {
    assert_eq!(
        validate_push_request(&push(vec![valid_operation()])),
        Ok(())
    );
}

#[test]
fn envelope_violations_are_rejected() {
    let mut empty_device = push(vec![valid_operation()]);
    empty_device.device_id = "   ".into();
    assert_eq!(
        validate_push_request(&empty_device),
        Err(PushValidationError::EmptyDevice)
    );

    let mut no_operations = push(Vec::new());
    no_operations.device_id = "device-1".into();
    assert_eq!(
        validate_push_request(&no_operations),
        Err(PushValidationError::NoOperations)
    );

    let too_many = vec![valid_operation(); MAX_OPERATIONS_PER_PUSH + 1];
    assert_eq!(
        validate_push_request(&push(too_many)),
        Err(PushValidationError::TooManyOperations)
    );
}

#[test]
fn operation_violations_are_rejected() {
    let mut empty_operation_id = valid_operation();
    empty_operation_id.operation_id = String::new();
    assert_eq!(
        validate_push_request(&push(vec![empty_operation_id])),
        Err(PushValidationError::EmptyOperationId)
    );

    let mut empty_workspace = valid_operation();
    empty_workspace.workspace_id = " ".into();
    assert_eq!(
        validate_push_request(&push(vec![empty_workspace])),
        Err(PushValidationError::EmptyWorkspace)
    );

    let mut empty_entity = valid_operation();
    empty_entity.entity_id = String::new();
    assert_eq!(
        validate_push_request(&push(vec![empty_entity])),
        Err(PushValidationError::EmptyEntityId)
    );

    let mut unregistered = valid_operation();
    unregistered.entity_type = "accounts".into();
    assert_eq!(
        validate_push_request(&push(vec![unregistered])),
        Err(PushValidationError::UnregisteredEntityType)
    );

    let mut unsupported = valid_operation();
    unsupported.operation = "truncate".into();
    assert_eq!(
        validate_push_request(&push(vec![unsupported])),
        Err(PushValidationError::UnsupportedOperation)
    );

    let mut oversized = valid_operation();
    oversized.payload = json!({"title": "x".repeat(256 * 1024)});
    assert_eq!(
        validate_push_request(&push(vec![oversized])),
        Err(PushValidationError::PayloadTooLarge)
    );
}

#[test]
fn every_violation_carries_client_facing_prose() {
    let all = [
        PushValidationError::EmptyDevice,
        PushValidationError::NoOperations,
        PushValidationError::TooManyOperations,
        PushValidationError::EmptyWorkspace,
        PushValidationError::EmptyOperationId,
        PushValidationError::EmptyEntityId,
        PushValidationError::UnregisteredEntityType,
        PushValidationError::UnsupportedOperation,
        PushValidationError::PayloadTooLarge,
    ];
    for error in all {
        let message = error.message();
        assert!(!message.is_empty());
        assert!(
            message
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_lowercase())
        );
    }
}

#[test]
fn canonical_payload_is_independent_of_key_order() {
    let left: Value = serde_json::from_str(r#"{"b":{"y":1,"x":2},"a":[{"d":4,"c":3}]}"#).unwrap();
    let right: Value = serde_json::from_str(r#"{"a":[{"c":3,"d":4}],"b":{"x":2,"y":1}}"#).unwrap();
    assert_eq!(canonical_payload(&left), canonical_payload(&right));
    assert_eq!(
        canonical_payload(&left),
        r#"{"a":[{"c":3,"d":4}],"b":{"x":2,"y":1}}"#
    );
}

#[test]
fn canonical_payload_keeps_array_order_and_scalar_types() {
    assert_eq!(canonical_payload(&json!([2, 1])), "[2,1]");
    assert_eq!(canonical_payload(&json!(null)), "null");
    assert_eq!(canonical_payload(&json!({"n": 1})), r#"{"n":1}"#);
    assert_eq!(canonical_payload(&json!({"n": "1"})), r#"{"n":"1"}"#);
}

#[test]
fn fingerprint_matches_equal_content_and_differs_otherwise() {
    let base = valid_operation();
    let reordered = SyncOperation {
        payload: serde_json::from_str(r#"{"title":"Focus"}"#).unwrap(),
        ..valid_operation()
    };
    assert_eq!(base.fingerprint(), reordered.fingerprint());

    let mut different_payload = valid_operation();
    different_payload.payload = json!({"title": "Deep work"});
    assert_ne!(base.fingerprint(), different_payload.fingerprint());

    let mut different_operation_id = valid_operation();
    different_operation_id.operation_id = "operation-2".into();
    assert_eq!(
        base.fingerprint(),
        different_operation_id.fingerprint(),
        "the operation id is not part of the content identity"
    );

    for (entity_type, entity_id, operation, workspace) in [
        ("timeline", "entity-2", "upsert", "workspace-1"),
        ("timeline", "entity-1", "delete", "workspace-1"),
        ("timeline", "entity-1", "upsert", "workspace-2"),
    ] {
        let mut changed = valid_operation();
        changed.entity_type = entity_type.into();
        changed.entity_id = entity_id.into();
        changed.operation = operation.into();
        changed.workspace_id = workspace.into();
        assert_ne!(base.fingerprint(), changed.fingerprint());
    }
}

#[test]
fn fingerprint_is_unambiguous_across_field_boundaries() {
    let a = SyncOperation {
        operation_id: "a".into(),
        workspace_id: "ws".into(),
        entity_type: "timeline".into(),
        entity_id: "x|y".into(),
        operation: "upsert".into(),
        payload: json!({}),
        base_revision: None,
    };
    let b = SyncOperation {
        entity_id: "x".into(),
        operation: "y".into(),
        ..a.clone()
    };
    assert_ne!(a.fingerprint(), b.fingerprint());
}
