//! Shared sync protocol contract against real PostgreSQL: revisions, replay,
//! conflicts, cursors and tenant isolation.

mod support;

use axum::http::{Method, StatusCode};
use serde_json::json;
use support::{call, operation};

#[tokio::test]
async fn push_then_pull_round_trips_and_replays_idempotently() {
    let harness = pg_harness!("push_then_pull_round_trips_and_replays_idempotently");
    let workspace = harness.workspace.as_str();
    let op_a = harness.op_id("op-a");
    let first = operation(
        workspace,
        &op_a,
        "entity-1",
        json!({ "title": "Focus" }),
        None,
    );

    let pushed = harness
        .push(Some(&harness.token), "device-a", json!([first]))
        .await;
    assert_eq!(pushed.status, StatusCode::OK, "{:?}", pushed.body);
    assert_eq!(pushed.body["accepted"][0], op_a.as_str());
    assert_eq!(pushed.array_len("conflicts"), 0);
    let cursor = pushed.body["cursor"].as_u64().expect("cursor");
    assert!(cursor > 0, "a stored operation advances the cursor");

    let pulled = harness.pull(Some(&harness.token), workspace, 0, 500).await;
    assert_eq!(pulled.status, StatusCode::OK, "{:?}", pulled.body);
    assert_eq!(pulled.array_len("changes"), 1);
    assert_eq!(
        pulled.body["changes"][0]["operation"]["operationId"],
        op_a.as_str()
    );
    assert_eq!(
        pulled.body["changes"][0]["operation"]["payload"]["title"],
        "Focus"
    );
    assert_eq!(pulled.body["cursor"].as_u64(), Some(cursor));

    // The same operation id and content is an idempotent replay, not a new row.
    let replay = operation(
        workspace,
        &op_a,
        "entity-1",
        json!({ "title": "Focus" }),
        None,
    );
    let replayed = harness
        .push(Some(&harness.token), "device-a", json!([replay]))
        .await;
    assert_eq!(replayed.status, StatusCode::OK);
    assert_eq!(replayed.body["accepted"][0], op_a.as_str());
    assert_eq!(replayed.array_len("conflicts"), 0);
    assert!(
        replayed.body["cursor"].as_u64().expect("cursor") >= cursor,
        "the replay never moves the cursor backwards"
    );

    let after_replay = harness.pull(Some(&harness.token), workspace, 0, 500).await;
    assert_eq!(
        after_replay.array_len("changes"),
        1,
        "a replay must not create a second revision"
    );

    // The same id with different content is reported as a conflict.
    let conflicting = operation(
        workspace,
        &op_a,
        "entity-1",
        json!({ "title": "Something else" }),
        None,
    );
    let conflicted = harness
        .push(Some(&harness.token), "device-a", json!([conflicting]))
        .await;
    assert_eq!(conflicted.status, StatusCode::OK);
    assert_eq!(conflicted.array_len("accepted"), 0);
    assert_eq!(conflicted.body["conflicts"][0], op_a.as_str());

    let unchanged = harness.pull(Some(&harness.token), workspace, 0, 500).await;
    assert_eq!(unchanged.array_len("changes"), 1);
    assert_eq!(
        unchanged.body["changes"][0]["operation"]["payload"]["title"],
        "Focus"
    );
}

#[tokio::test]
async fn a_stale_base_revision_is_a_conflict_and_a_current_one_is_accepted() {
    let harness = pg_harness!("a_stale_base_revision_is_a_conflict_and_a_current_one_is_accepted");
    let workspace = harness.workspace.as_str();
    let created_id = harness.op_id("op-1");
    let stale_id = harness.op_id("op-2");
    let current_id = harness.op_id("op-3");

    let created = harness
        .push(
            Some(&harness.token),
            "device-a",
            json!([operation(
                workspace,
                &created_id,
                "entity-9",
                json!({ "v": 1 }),
                None
            )]),
        )
        .await;
    assert_eq!(created.status, StatusCode::OK);
    let revision = created.body["cursor"].as_u64().expect("revision");

    let stale = harness
        .push(
            Some(&harness.token),
            "device-a",
            json!([operation(
                workspace,
                &stale_id,
                "entity-9",
                json!({ "v": 2 }),
                Some(revision + 5_000)
            )]),
        )
        .await;
    assert_eq!(stale.status, StatusCode::OK);
    assert_eq!(stale.array_len("accepted"), 0);
    assert_eq!(stale.body["conflicts"][0], stale_id.as_str());

    let current = harness
        .push(
            Some(&harness.token),
            "device-a",
            json!([operation(
                workspace,
                &current_id,
                "entity-9",
                json!({ "v": 3 }),
                Some(revision)
            )]),
        )
        .await;
    assert_eq!(current.status, StatusCode::OK);
    assert_eq!(current.body["accepted"][0], current_id.as_str());
    assert_eq!(current.array_len("conflicts"), 0);
}

#[tokio::test]
async fn pull_resumes_from_a_cursor_without_duplicates() {
    let harness = pg_harness!("pull_resumes_from_a_cursor_without_duplicates");
    let workspace = harness.workspace.as_str();
    let first = harness.op_id("op-1");
    let second = harness.op_id("op-2");
    let third = harness.op_id("op-3");
    let operations = json!([
        operation(workspace, &first, "entity-1", json!({ "n": 1 }), None),
        operation(workspace, &second, "entity-2", json!({ "n": 2 }), None),
        operation(workspace, &third, "entity-3", json!({ "n": 3 }), None),
    ]);
    let pushed = harness
        .push(Some(&harness.token), "device-a", operations)
        .await;
    assert_eq!(pushed.status, StatusCode::OK);
    assert_eq!(pushed.array_len("accepted"), 3);

    let page = harness.pull(Some(&harness.token), workspace, 0, 2).await;
    assert_eq!(page.status, StatusCode::OK);
    assert_eq!(page.array_len("changes"), 2, "the limit is honoured");
    let cursor = page.body["cursor"].as_u64().expect("cursor");
    let second_revision = page.body["changes"][1]["revision"]
        .as_u64()
        .expect("second revision");
    assert_eq!(cursor, second_revision);

    let next = harness
        .pull(Some(&harness.token), workspace, cursor, 2)
        .await;
    assert_eq!(next.array_len("changes"), 1);
    assert!(
        next.body["changes"][0]["revision"]
            .as_u64()
            .expect("revision")
            > cursor
    );

    let exhausted = harness
        .pull(
            Some(&harness.token),
            workspace,
            next.body["cursor"].as_u64().expect("cursor"),
            2,
        )
        .await;
    assert_eq!(exhausted.array_len("changes"), 0);
}

/// Two devices can store the same operation id at the same moment. The primary
/// key decides, and the losing writer reports a replay instead of failing the
/// push with a storage error.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_duplicate_pushes_resolve_as_a_replay_not_a_failure() {
    let harness = pg_harness!("concurrent_duplicate_pushes_resolve_as_a_replay_not_a_failure");
    let workspace = harness.workspace.clone();
    let operation_id = harness.op_id("op-race");
    let first = operation(
        &workspace,
        &operation_id,
        "entity-race",
        json!({ "v": 1 }),
        None,
    );
    let second = first.clone();

    let (left, right) = tokio::join!(
        harness.push(Some(&harness.token), "device-a", json!([first])),
        harness.push(Some(&harness.token), "device-b", json!([second])),
    );
    for call in [&left, &right] {
        assert_eq!(call.status, StatusCode::OK, "{:?}", call.body);
        assert_eq!(call.body["accepted"][0], operation_id.as_str());
        assert_eq!(call.array_len("conflicts"), 0);
    }

    let pulled = harness.pull(Some(&harness.token), &workspace, 0, 100).await;
    assert_eq!(
        pulled.array_len("changes"),
        1,
        "the duplicate push must not create a second revision"
    );
}

#[tokio::test]
async fn workspaces_are_isolated_between_accounts() {
    let harness = pg_harness!("workspaces_are_isolated_between_accounts");
    let owned = harness.workspace.clone();
    let owner_email = harness.email("owner");
    let owner = harness.account(&owner_email).await;
    harness.grant(&owned, &owner, "owner").await;

    let other_workspace = format!("{owned}-other");
    let outsider_email = harness.email("outsider");
    let outsider = harness.account(&outsider_email).await;
    harness.grant(&other_workspace, &outsider, "member").await;
    let outsider_session = harness.session(&outsider_email).await;
    let intrusion_id = harness.op_id("op-x");
    let own_id = harness.op_id("op-y");

    let intrusion = harness
        .push(
            Some(&outsider_session),
            "device-b",
            json!([operation(
                &owned,
                &intrusion_id,
                "entity-x",
                json!({}),
                None
            )]),
        )
        .await;
    assert_eq!(
        intrusion.status,
        StatusCode::FORBIDDEN,
        "{:?}",
        intrusion.body
    );
    assert_eq!(intrusion.code(), "FORBIDDEN");

    let snooping = harness.pull(Some(&outsider_session), &owned, 0, 100).await;
    assert_eq!(snooping.status, StatusCode::FORBIDDEN);
    assert_eq!(snooping.code(), "FORBIDDEN");

    let own_push = harness
        .push(
            Some(&outsider_session),
            "device-b",
            json!([operation(
                &other_workspace,
                &own_id,
                "entity-y",
                json!({}),
                None
            )]),
        )
        .await;
    assert_eq!(own_push.status, StatusCode::OK, "{:?}", own_push.body);

    let owner_pull = harness.pull(Some(&harness.token), &owned, 0, 100).await;
    assert_eq!(owner_pull.status, StatusCode::OK);
    assert_eq!(
        owner_pull.array_len("changes"),
        0,
        "another workspace's operations never leak into this one"
    );
}

#[tokio::test]
async fn the_service_token_is_limited_to_its_own_workspace() {
    let harness = pg_harness!("the_service_token_is_limited_to_its_own_workspace");
    let outside = format!("{}-outside", harness.workspace);
    let operation_id = harness.op_id("op-z");
    let pushed = harness
        .push(
            Some(&harness.token),
            "device-a",
            json!([operation(
                &outside,
                &operation_id,
                "entity-z",
                json!({}),
                None
            )]),
        )
        .await;
    assert_eq!(pushed.status, StatusCode::FORBIDDEN);
    assert_eq!(pushed.code(), "FORBIDDEN");
}

#[tokio::test]
async fn unregistered_payloads_are_rejected_before_storage() {
    let harness = pg_harness!("unregistered_payloads_are_rejected_before_storage");
    let workspace = harness.workspace.as_str();
    let unknown_type_id = harness.op_id("op-u");
    let unknown_operation_id = harness.op_id("op-v");

    let mut unknown_type = operation(workspace, &unknown_type_id, "entity-u", json!({}), None);
    unknown_type["entityType"] = json!("account");
    let rejected = harness
        .push(Some(&harness.token), "device-a", json!([unknown_type]))
        .await;
    assert_eq!(rejected.status, StatusCode::BAD_REQUEST);
    assert_eq!(rejected.code(), "INVALID_REQUEST");

    let mut unknown_operation = operation(
        workspace,
        &unknown_operation_id,
        "entity-v",
        json!({}),
        None,
    );
    unknown_operation["operation"] = json!("truncate");
    let unsupported = harness
        .push(Some(&harness.token), "device-a", json!([unknown_operation]))
        .await;
    assert_eq!(unsupported.status, StatusCode::BAD_REQUEST);
    assert_eq!(unsupported.code(), "INVALID_REQUEST");

    let empty = harness
        .push(Some(&harness.token), "device-a", json!([]))
        .await;
    assert_eq!(empty.status, StatusCode::BAD_REQUEST);
    assert_eq!(empty.code(), "INVALID_REQUEST");

    let stored = harness.pull(Some(&harness.token), workspace, 0, 100).await;
    assert_eq!(
        stored.array_len("changes"),
        0,
        "a rejected push stores nothing"
    );
}

#[tokio::test]
async fn stored_changes_survive_a_server_restart() {
    let harness = pg_harness!("stored_changes_survive_a_server_restart");
    let workspace = harness.workspace.as_str();
    let operation_id = harness.op_id("op-r");
    let pushed = harness
        .push(
            Some(&harness.token),
            "device-a",
            json!([operation(
                workspace,
                &operation_id,
                "entity-r",
                json!({ "v": 1 }),
                None
            )]),
        )
        .await;
    assert_eq!(pushed.status, StatusCode::OK);
    let cursor = pushed.body["cursor"].as_u64().expect("cursor");

    // A fresh router and state over the same database behave like a restarted
    // process: no in-memory cursor or cache is required to continue.
    let restarted = harness.app();
    let resumed = call(
        restarted,
        Method::GET,
        &format!("/api/v1/sync/pull?workspace_id={workspace}&cursor=0&limit=100"),
        Some(&harness.token),
        None,
    )
    .await;
    assert_eq!(resumed.status, StatusCode::OK, "{:?}", resumed.body);
    assert_eq!(resumed.array_len("changes"), 1);
    assert_eq!(resumed.body["cursor"].as_u64(), Some(cursor));
}

#[tokio::test]
async fn a_delete_tombstone_is_replayed_to_a_fresh_device() {
    let harness = pg_harness!("a_delete_tombstone_is_replayed_to_a_fresh_device");
    let workspace = harness.workspace.as_str();
    let upsert_id = harness.op_id("op-upsert");
    let delete_id = harness.op_id("op-delete");

    let upsert = operation(
        workspace,
        &upsert_id,
        "entity-t",
        json!({ "title": "Focus" }),
        None,
    );
    let pushed_upsert = harness
        .push(Some(&harness.token), "device-a", json!([upsert]))
        .await;
    assert_eq!(
        pushed_upsert.status,
        StatusCode::OK,
        "{:?}",
        pushed_upsert.body
    );

    let mut delete = operation(workspace, &delete_id, "entity-t", json!({}), None);
    delete["operation"] = json!("delete");
    let pushed_delete = harness
        .push(Some(&harness.token), "device-a", json!([delete]))
        .await;
    assert_eq!(
        pushed_delete.status,
        StatusCode::OK,
        "{:?}",
        pushed_delete.body
    );

    // A device that never saw the upsert still receives the delete tombstone.
    let pulled = harness.pull(Some(&harness.token), workspace, 0, 500).await;
    assert_eq!(pulled.status, StatusCode::OK, "{:?}", pulled.body);
    assert_eq!(pulled.array_len("changes"), 2);
    let ops: Vec<&str> = (0..2)
        .map(|i| {
            pulled.body["changes"][i]["operation"]["operation"]
                .as_str()
                .unwrap()
        })
        .collect();
    assert!(
        ops.contains(&"delete"),
        "delete tombstone must survive in the log"
    );
}
