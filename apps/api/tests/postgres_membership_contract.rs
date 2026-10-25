//! Workspace membership and role rules, verified through the shared HTTP surface and
//! the membership rows they leave behind.

mod support;

use axum::http::StatusCode;

#[tokio::test]
async fn members_are_added_by_email_and_unknown_accounts_are_not_found() {
    let harness = pg_harness!("members_are_added_by_email_and_unknown_accounts_are_not_found");
    let owner_email = harness.email("owner");
    let owner = harness.account(&owner_email).await;
    harness.grant(&harness.workspace, &owner, "owner").await;
    let owner_session = harness.session(&owner_email).await;

    let invited_email = harness.email("invited");
    harness.account(&invited_email).await;

    let added = harness
        .add_member(
            Some(&owner_session),
            &harness.workspace,
            &invited_email.to_uppercase(),
            "member",
        )
        .await;
    assert_eq!(added.status, StatusCode::NO_CONTENT, "{:?}", added.body);

    let missing = harness
        .add_member(
            Some(&owner_session),
            &harness.workspace,
            &harness.email("nobody"),
            "member",
        )
        .await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert_eq!(missing.code(), "NOT_FOUND");
}

#[tokio::test]
async fn the_role_matrix_is_enforced_for_every_change() {
    let harness = pg_harness!("the_role_matrix_is_enforced_for_every_change");
    let workspace = harness.workspace.clone();

    let owner_email = harness.email("owner");
    let owner = harness.account(&owner_email).await;
    harness.grant(&workspace, &owner, "owner").await;
    let owner_session = harness.session(&owner_email).await;

    let member_email = harness.email("member");
    let member = harness.account(&member_email).await;
    harness.grant(&workspace, &member, "member").await;
    let member_session = harness.session(&member_email).await;

    // A member may not grant itself a higher role.
    let escalation = harness
        .add_member(Some(&member_session), &workspace, &member_email, "admin")
        .await;
    assert_eq!(escalation.status, StatusCode::FORBIDDEN);
    assert_eq!(escalation.code(), "FORBIDDEN");

    // The owner may promote the member to admin.
    let promoted = harness
        .add_member(Some(&owner_session), &workspace, &member_email, "admin")
        .await;
    assert_eq!(
        promoted.status,
        StatusCode::NO_CONTENT,
        "{:?}",
        promoted.body
    );

    // An admin still cannot create an owner.
    let owner_grant = harness
        .add_member(Some(&member_session), &workspace, &member_email, "owner")
        .await;
    assert_eq!(owner_grant.status, StatusCode::FORBIDDEN);
    assert_eq!(owner_grant.code(), "FORBIDDEN");

    // The last owner cannot be demoted, even by itself.
    let last_owner = harness
        .add_member(Some(&owner_session), &workspace, &owner_email, "member")
        .await;
    assert_eq!(last_owner.status, StatusCode::CONFLICT);
    assert_eq!(last_owner.code(), "CONFLICT");

    // Unknown roles are a request error, not a role.
    let unknown = harness
        .add_member(Some(&owner_session), &workspace, &member_email, "superuser")
        .await;
    assert_eq!(unknown.status, StatusCode::BAD_REQUEST);
    assert_eq!(unknown.code(), "INVALID_REQUEST");

    // A viewer cannot change membership at all.
    let viewer_email = harness.email("viewer");
    let viewer = harness.account(&viewer_email).await;
    harness.grant(&workspace, &viewer, "viewer").await;
    let viewer_session = harness.session(&viewer_email).await;
    let denied = harness
        .add_member(Some(&viewer_session), &workspace, &member_email, "member")
        .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_second_owner_may_be_created_once_the_caller_is_also_an_owner() {
    let harness = pg_harness!("a_second_owner_may_be_created_once_the_caller_is_also_an_owner");
    let workspace = harness.workspace.clone();

    let first_email = harness.email("first");
    let first = harness.account(&first_email).await;
    harness.grant(&workspace, &first, "owner").await;
    let first_session = harness.session(&first_email).await;

    let second_email = harness.email("second");
    harness.account(&second_email).await;

    let promoted = harness
        .add_member(Some(&first_session), &workspace, &second_email, "owner")
        .await;
    assert_eq!(
        promoted.status,
        StatusCode::NO_CONTENT,
        "{:?}",
        promoted.body
    );

    // With two owners the demotion of one is allowed again.
    let demoted = harness
        .add_member(Some(&first_session), &workspace, &first_email, "admin")
        .await;
    assert_eq!(demoted.status, StatusCode::NO_CONTENT, "{:?}", demoted.body);

    // The promoted account really holds the role: it can read the workspace list.
    let second_session = harness.session(&second_email).await;
    let listed = harness
        .call(
            axum::http::Method::GET,
            "/api/v1/workspaces",
            Some(&second_session),
            None,
        )
        .await;
    assert_eq!(listed.status, StatusCode::OK, "{:?}", listed.body);
    let rows = listed.body.as_array().expect("workspace list");
    assert!(
        rows.iter()
            .any(|row| row["id"] == workspace.as_str() && row["role"] == "owner"),
        "second owner missing: {:?}",
        listed.body
    );
}
