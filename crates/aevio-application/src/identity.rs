//! Identity, workspace access, session and membership policy.
//!
//! Every HTTP, IPC and CLI adapter delegates its decisions here. Adapters only
//! read the current role from storage and translate the result into their own
//! transport response; they must never re-derive a rule.

/// Workspace roles in ascending privilege order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    Viewer,
    Member,
    Admin,
    Owner,
}

impl Role {
    /// Ascending privilege order. Used for matrix tests and permission lists.
    pub const ALL: [Self; 4] = [Self::Viewer, Self::Member, Self::Admin, Self::Owner];

    /// Parses a persisted or wire role. Unknown values are never a role.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "viewer" => Some(Self::Viewer),
            "member" => Some(Self::Member),
            "admin" => Some(Self::Admin),
            "owner" => Some(Self::Owner),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Member => "member",
            Self::Admin => "admin",
            Self::Owner => "owner",
        }
    }

    pub const fn rank(self) -> u8 {
        match self {
            Self::Viewer => 0,
            Self::Member => 1,
            Self::Admin => 2,
            Self::Owner => 3,
        }
    }

    /// Whether this role satisfies `required`, including equal privilege.
    pub const fn allows(self, required: Self) -> bool {
        self.rank() >= required.rank()
    }
    /// The machine permission that corresponds to holding this role.
    pub const fn permission(self) -> Permission {
        match self {
            Self::Viewer => Permission::Read,
            Self::Member => Permission::Write,
            Self::Admin => Permission::Admin,
            Self::Owner => Permission::Admin,
        }
    }
}

/// Non-interactive credential permissions. `Admin` never equals `Owner`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Permission {
    Read,
    Write,
    Admin,
}

impl Permission {
    pub const ALL: [Self; 3] = [Self::Read, Self::Write, Self::Admin];

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Admin => "admin",
        }
    }

    pub const fn rank(self) -> u8 {
        match self {
            Self::Read => 0,
            Self::Write => 1,
            Self::Admin => 2,
        }
    }

    pub const fn allows(self, required: Self) -> bool {
        self.rank() >= required.rank()
    }
}

/// Explicit limits of a machine credential. An empty scope grants nothing, so a
/// missing or blank configuration can never degrade into a global super token.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServiceScope {
    workspaces: Vec<String>,
    permissions: Vec<Permission>,
}

impl ServiceScope {
    pub fn new<I, S, P>(workspaces: I, permissions: P) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
        P: IntoIterator<Item = Permission>,
    {
        let workspaces = workspaces
            .into_iter()
            .map(|value| value.as_ref().trim().to_owned())
            .filter(|value| !value.is_empty())
            .collect();
        let mut permissions = permissions.into_iter().collect::<Vec<_>>();
        permissions.sort_unstable();
        permissions.dedup();
        Self {
            workspaces,
            permissions: permissions.into_iter().collect(),
        }
    }

    /// Parses the `AEVIO_SERVER_TOKEN_WORKSPACES` / `..._PERMISSIONS` CSV form.
    pub fn from_csv(workspaces: &str, permissions: &str) -> Self {
        Self::new(
            workspaces.split(','),
            permissions
                .split(',')
                .filter_map(|value| Permission::parse(value.trim())),
        )
    }

    /// A credential without a workspace or without a permission is unusable.
    pub fn is_empty(&self) -> bool {
        self.workspaces.is_empty() || self.permissions.is_empty()
    }

    pub fn allows_workspace(&self, workspace_id: &str) -> bool {
        self.workspaces
            .iter()
            .any(|candidate| candidate == workspace_id)
    }

    pub fn allows_permission(&self, required: Permission) -> bool {
        self.permissions
            .iter()
            .any(|granted| granted.allows(required))
    }

    pub fn allows(&self, workspace_id: &str, required: Permission) -> bool {
        self.allows_workspace(workspace_id) && self.allows_permission(required)
    }

    pub fn workspaces(&self) -> &[String] {
        &self.workspaces
    }

    pub fn permissions(&self) -> &[Permission] {
        &self.permissions
    }
}

/// The authenticated caller behind one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    Account { account_id: String },
    Service { scope: ServiceScope },
}

impl Actor {
    pub fn account(account_id: impl Into<String>) -> Self {
        Self::Account {
            account_id: account_id.into(),
        }
    }

    pub fn service(scope: ServiceScope) -> Self {
        Self::Service { scope }
    }

    pub fn is_service(&self) -> bool {
        matches!(self, Self::Service { .. })
    }

    pub fn account_id(&self) -> Option<&str> {
        match self {
            Self::Account { account_id } => Some(account_id),
            Self::Service { .. } => None,
        }
    }

    /// The role a credential itself carries. Accounts take their role from the
    /// workspace membership, never from the credential.
    pub fn effective_role(&self) -> Option<Role> {
        match self {
            Self::Account { .. } => None,
            Self::Service { scope } => {
                if scope.allows_permission(Permission::Admin) {
                    Some(Role::Admin)
                } else if scope.allows_permission(Permission::Write) {
                    Some(Role::Member)
                } else if scope.allows_permission(Permission::Read) {
                    Some(Role::Viewer)
                } else {
                    None
                }
            }
        }
    }
}

/// Why a workspace operation is not authorized. All variants map to a denial;
/// they stay distinct so adapters can log the precise cause without leaking it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessDenial {
    EmptyWorkspace,
    WorkspaceNotAllowed,
    MissingPermission,
    NotAMember,
    UnknownRole,
    InsufficientRole,
}

impl AccessDenial {
    pub const fn message(self) -> &'static str {
        match self {
            Self::EmptyWorkspace => "workspace id is required",
            Self::WorkspaceNotAllowed => "workspace is not allowed",
            Self::MissingPermission => "token is not permitted to perform this operation",
            Self::NotAMember => "workspace membership is required",
            Self::UnknownRole => "workspace role is not recognized",
            Self::InsufficientRole => "workspace role is not sufficient",
        }
    }
}

/// Authorizes one workspace operation.
///
/// `member_role` is the caller's persisted role in `workspace_id`, or `None`
/// when the caller has no membership. Service credentials ignore it and are
/// limited by their explicit scope instead.
pub fn authorize(
    actor: &Actor,
    workspace_id: &str,
    required: Role,
    member_role: Option<&str>,
) -> Result<(), AccessDenial> {
    if workspace_id.trim().is_empty() {
        return Err(AccessDenial::EmptyWorkspace);
    }
    match actor {
        Actor::Service { scope } => {
            if !scope.allows_workspace(workspace_id) {
                return Err(AccessDenial::WorkspaceNotAllowed);
            }
            if !scope.allows_permission(required.permission()) {
                return Err(AccessDenial::MissingPermission);
            }
            Ok(())
        }
        Actor::Account { .. } => {
            let role = member_role
                .ok_or(AccessDenial::NotAMember)
                .and_then(|value| Role::parse(value).ok_or(AccessDenial::UnknownRole))?;
            if role.allows(required) {
                Ok(())
            } else {
                Err(AccessDenial::InsufficientRole)
            }
        }
    }
}

/// The privilege ceiling this actor holds in a workspace.
///
/// Adapters use it where a role is the input of another rule, for example when
/// deciding which membership roles the caller may grant.
pub fn workspace_role(actor: &Actor, member_role: Option<&str>) -> Option<Role> {
    match actor {
        Actor::Account { .. } => member_role.and_then(Role::parse),
        Actor::Service { .. } => actor.effective_role(),
    }
}

/// Sessions are short-lived even when nobody revokes them.
pub const TOKEN_TTL_SECONDS: i64 = 30 * 24 * 60 * 60;

/// Whether a persisted session may still authenticate.
pub fn token_is_live(created_at: i64, revoked_at: Option<i64>, now: i64) -> bool {
    token_is_live_with_ttl(created_at, revoked_at, now, TOKEN_TTL_SECONDS)
}

pub fn token_is_live_with_ttl(
    created_at: i64,
    revoked_at: Option<i64>,
    now: i64,
    ttl_seconds: i64,
) -> bool {
    if revoked_at.is_some() {
        return false;
    }
    now.saturating_sub(created_at) <= ttl_seconds.max(0)
}

/// Why a membership change is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MembershipChangeError {
    UnknownRole,
    CannotGrant,
    LastOwner,
}

impl MembershipChangeError {
    pub const fn message(self) -> &'static str {
        match self {
            Self::UnknownRole => "unsupported workspace role",
            Self::CannotGrant => "workspace role exceeds the caller's own role",
            Self::LastOwner => "the last workspace owner cannot be demoted",
        }
    }
}

/// Decides one membership upsert.
///
/// `owner_count` is the number of owners in the workspace before the change.
pub fn decide_membership_change(
    actor: Role,
    current_role: Option<&str>,
    requested: &str,
    owner_count: u64,
) -> Result<Role, MembershipChangeError> {
    let requested = Role::parse(requested).ok_or(MembershipChangeError::UnknownRole)?;
    if !actor.allows(requested) {
        return Err(MembershipChangeError::CannotGrant);
    }
    let demotes_owner =
        current_role.and_then(Role::parse) == Some(Role::Owner) && requested != Role::Owner;
    if demotes_owner && owner_count <= 1 {
        return Err(MembershipChangeError::LastOwner);
    }
    Ok(requested)
}

/// Accounts are compared case-insensitively; storage keeps one canonical form.
pub fn normalize_email(raw: &str) -> String {
    raw.trim().to_lowercase()
}

/// Minimum password length for every account-creating adapter.
pub const MIN_PASSWORD_LENGTH: usize = 12;

/// Validates self-service registration input.
pub fn validate_registration(email: &str, password: &str) -> Result<(), &'static str> {
    if normalize_email(email).is_empty() || password.len() < MIN_PASSWORD_LENGTH {
        return Err("email and a password of at least 12 characters are required");
    }
    Ok(())
}
