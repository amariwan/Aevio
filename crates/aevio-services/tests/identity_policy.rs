//! Identity, access and membership policy behaviour.

use aevio_services::identity::*;

fn scope(workspaces: &[&str], permissions: &[Permission]) -> ServiceScope {
    ServiceScope::new(workspaces, permissions.iter().copied())
}

#[test]
fn role_matrix_is_a_total_order() {
    for (required_rank, required) in Role::ALL.iter().enumerate() {
        for (actor_rank, actor) in Role::ALL.iter().enumerate() {
            assert_eq!(
                actor.allows(*required),
                actor_rank >= required_rank,
                "{} requesting {}",
                actor.as_str(),
                required.as_str()
            );
        }
    }
    for role in Role::ALL {
        assert_eq!(Role::parse(role.as_str()), Some(role));
        assert_eq!(
            Role::ALL
                .iter()
                .filter(|candidate| role.allows(**candidate))
                .count(),
            role.rank() as usize + 1
        );
    }
}

#[test]
fn unknown_roles_and_permissions_are_never_granted() {
    for invalid in ["", "Owner", "superuser", " member ", "member ", "owner\n"] {
        assert_eq!(Role::parse(invalid), None, "{invalid:?}");
        assert_eq!(Permission::parse(invalid), None, "{invalid:?}");
    }
}

#[test]
fn empty_service_scope_denies_everything() {
    let empty = ServiceScope::default();
    assert!(empty.is_empty());
    assert!(!empty.allows_workspace("workspace"));
    assert!(!empty.allows("workspace", Permission::Read));
    assert_eq!(Actor::service(empty.clone()).effective_role(), None);
    assert!(matches!(
        authorize(&Actor::service(empty), "workspace", Role::Viewer, None),
        Err(AccessDenial::WorkspaceNotAllowed)
    ));
    assert!(scope(&["  ", ""], &[Permission::Read]).is_empty());
    assert!(scope(&["workspace"], &[]).is_empty());
}

#[test]
fn service_scope_is_limited_to_named_workspaces_and_permissions() {
    let read_only = Actor::service(scope(&["workspace-a"], &[Permission::Read]));
    assert_eq!(
        authorize(&read_only, "workspace-a", Role::Viewer, None),
        Ok(())
    );
    assert_eq!(
        authorize(&read_only, "workspace-a", Role::Member, None),
        Err(AccessDenial::MissingPermission)
    );
    assert_eq!(
        authorize(&read_only, "workspace-b", Role::Viewer, None),
        Err(AccessDenial::WorkspaceNotAllowed)
    );
    assert_eq!(read_only.effective_role(), Some(Role::Viewer));

    let writer = Actor::service(scope(&["workspace-a"], &[Permission::Write]));
    assert_eq!(
        authorize(&writer, "workspace-a", Role::Member, None),
        Ok(())
    );
    assert_eq!(writer.effective_role(), Some(Role::Member));

    let admin = Actor::service(scope(
        &["workspace-a"],
        &[Permission::Write, Permission::Admin],
    ));
    assert_eq!(authorize(&admin, "workspace-a", Role::Admin, None), Ok(()));
    assert_eq!(admin.effective_role(), Some(Role::Admin));
}

#[test]
fn service_scope_never_becomes_an_owner() {
    let admin = Actor::service(scope(&["workspace"], &[Permission::Admin]));
    assert_ne!(admin.effective_role(), Some(Role::Owner));
    assert_eq!(
        decide_membership_change(admin.effective_role().expect("role"), None, "owner", 1),
        Err(MembershipChangeError::CannotGrant)
    );
}

#[test]
fn service_scope_parses_csv_configuration() {
    let parsed = ServiceScope::from_csv(" workspace-a , workspace-b ,, ", "read, admin,bogus");
    assert_eq!(parsed.workspaces(), ["workspace-a", "workspace-b"]);
    assert_eq!(parsed.permissions(), [Permission::Read, Permission::Admin]);
    assert!(ServiceScope::from_csv("", "read").is_empty());
    assert!(ServiceScope::from_csv("workspace", "bogus").is_empty());
}

#[test]
fn account_access_follows_persisted_membership() {
    let account = Actor::account("account-1");
    assert_eq!(
        authorize(&account, "workspace", Role::Viewer, Some("viewer")),
        Ok(())
    );
    assert_eq!(
        authorize(&account, "workspace", Role::Admin, Some("admin")),
        Ok(())
    );
    assert_eq!(
        authorize(&account, "workspace", Role::Admin, Some("member")),
        Err(AccessDenial::InsufficientRole)
    );
    assert_eq!(
        authorize(&account, "workspace", Role::Viewer, None),
        Err(AccessDenial::NotAMember)
    );
    assert_eq!(
        authorize(&account, "workspace", Role::Viewer, Some("superuser")),
        Err(AccessDenial::UnknownRole)
    );
    assert_eq!(
        authorize(&account, "  ", Role::Viewer, Some("owner")),
        Err(AccessDenial::EmptyWorkspace)
    );
    assert_eq!(account.effective_role(), None);
    assert_eq!(account.account_id(), Some("account-1"));
}

#[test]
fn sessions_expire_and_revocation_wins() {
    let now = 1_000_000;
    assert!(token_is_live(now, None, now));
    assert!(token_is_live(now - TOKEN_TTL_SECONDS, None, now));
    assert!(!token_is_live(now - TOKEN_TTL_SECONDS - 1, None, now));
    assert!(!token_is_live(now, Some(now), now));
    assert!(
        token_is_live(now + 30, None, now),
        "a server clock behind the writer must not reject a fresh session"
    );
    assert!(
        token_is_live_with_ttl(now, None, now, 0),
        "a zero ttl still grants the issue second"
    );
    assert!(!token_is_live_with_ttl(now, None, now + 1, 0));
    assert!(!token_is_live_with_ttl(now - 2, None, now, 1));
}

#[test]
fn membership_changes_respect_rank_and_the_last_owner() {
    assert_eq!(
        decide_membership_change(Role::Admin, None, "member", 0),
        Ok(Role::Member)
    );
    assert_eq!(
        decide_membership_change(Role::Owner, None, "owner", 1),
        Ok(Role::Owner)
    );
    assert_eq!(
        decide_membership_change(Role::Admin, None, "owner", 1),
        Err(MembershipChangeError::CannotGrant)
    );
    assert_eq!(
        decide_membership_change(Role::Owner, Some("owner"), "admin", 1),
        Err(MembershipChangeError::LastOwner)
    );
    assert_eq!(
        decide_membership_change(Role::Owner, Some("owner"), "admin", 2),
        Ok(Role::Admin)
    );
    assert_eq!(
        decide_membership_change(Role::Owner, Some("owner"), "owner", 1),
        Ok(Role::Owner)
    );
    assert_eq!(
        decide_membership_change(Role::Owner, None, "root", 1),
        Err(MembershipChangeError::UnknownRole)
    );
}

#[test]
fn registration_input_is_normalized_and_bounded() {
    assert_eq!(
        normalize_email("  Owner@Example.TEST "),
        "owner@example.test"
    );
    assert_eq!(
        validate_registration("owner@example.test", "correct horse battery"),
        Ok(())
    );
    assert!(validate_registration("   ", "correct horse battery").is_err());
    assert!(validate_registration("owner@example.test", "short").is_err());
    assert_eq!(MIN_PASSWORD_LENGTH, 12);
}

#[test]
fn workspace_role_uses_persisted_role_or_the_credential_ceiling() {
    let account = Actor::account("account-1");
    assert_eq!(workspace_role(&account, Some("admin")), Some(Role::Admin));
    assert_eq!(workspace_role(&account, Some("superuser")), None);
    assert_eq!(workspace_role(&account, None), None);

    let service = Actor::service(scope(&["workspace-a"], &[Permission::Write]));
    assert_eq!(workspace_role(&service, Some("owner")), Some(Role::Member));
}
