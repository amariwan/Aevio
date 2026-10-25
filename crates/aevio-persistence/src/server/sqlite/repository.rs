use aevio_application::identity::{Role, normalize_email};
use aevio_application::replay::{ReplayDecision, classify_replay};
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{Expr, ExprTrait, Func, OnConflict};
use sea_orm::{
    ActiveValue::Set, DatabaseConnection, DatabaseTransaction, DbErr, PaginatorTrait, QueryOrder,
    QuerySelect, TransactionTrait, TryInsertResult,
};
use serde_json::Value;

use super::entities::{
    accounts, memberships, operations, plugin_publisher_keys, plugin_registry,
    plugin_registry_releases, plugin_workspace_desired_state, tokens, workspaces,
};
use aevio_contracts::sync::{SyncChange, SyncOperation, SyncPushRequest, SyncPushResponse};

/// A persisted session before the expiry and revocation policy is applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    pub account_id: String,
    pub created_at: i64,
    pub revoked_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRegistryRow {
    pub plugin_id: String,
    pub manifest_json: String,
    pub version: String,
    pub artifact_digest: String,
    pub artifact_size: i64,
    pub signature: String,
    pub channel: String,
    pub published_at: i64,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginWorkspaceDesiredStateRow {
    pub workspace_id: String,
    pub plugin_id: String,
    pub requested_version: Option<String>,
    pub channel: String,
    pub enabled: bool,
    pub requested_permissions_json: String,
    pub revision: i64,
    pub updated_at: i64,
}

pub struct Repository<'db> {
    db: &'db DatabaseConnection,
}

fn workspace_plugin_state(
    row: plugin_workspace_desired_state::Model,
) -> PluginWorkspaceDesiredStateRow {
    PluginWorkspaceDesiredStateRow {
        workspace_id: row.workspace_id,
        plugin_id: row.plugin_id,
        requested_version: row.requested_version,
        channel: row.channel,
        enabled: row.enabled != 0,
        requested_permissions_json: row.requested_permissions_json,
        revision: row.revision,
        updated_at: row.updated_at,
    }
}

impl<'db> Repository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn workspace_plugin_states(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<PluginWorkspaceDesiredStateRow>, DbErr> {
        plugin_workspace_desired_state::Entity::find()
            .filter(plugin_workspace_desired_state::Column::WorkspaceId.eq(workspace_id))
            .order_by_asc(plugin_workspace_desired_state::Column::PluginId)
            .all(self.db)
            .map(|rows| rows.into_iter().map(workspace_plugin_state).collect())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn upsert_workspace_plugin_state(
        &self,
        workspace_id: String,
        plugin_id: String,
        requested_version: Option<String>,
        channel: String,
        enabled: bool,
        requested_permissions_json: String,
        now: i64,
    ) -> Result<PluginWorkspaceDesiredStateRow, DbErr> {
        let existing = plugin_workspace_desired_state::Entity::find_by_id((
            workspace_id.clone(),
            plugin_id.clone(),
        ))
        .one(self.db)?;
        let revision = existing.as_ref().map_or(1, |row| row.revision + 1);
        plugin_workspace_desired_state::Entity::insert(
            plugin_workspace_desired_state::ActiveModel {
                workspace_id: Set(workspace_id.clone()),
                plugin_id: Set(plugin_id.clone()),
                requested_version: Set(requested_version),
                channel: Set(channel),
                enabled: Set(i64::from(enabled)),
                requested_permissions_json: Set(requested_permissions_json),
                revision: Set(revision),
                updated_at: Set(now),
            },
        )
        .on_conflict(
            OnConflict::columns([
                plugin_workspace_desired_state::Column::WorkspaceId,
                plugin_workspace_desired_state::Column::PluginId,
            ])
            .update_columns([
                plugin_workspace_desired_state::Column::RequestedVersion,
                plugin_workspace_desired_state::Column::Channel,
                plugin_workspace_desired_state::Column::Enabled,
                plugin_workspace_desired_state::Column::RequestedPermissionsJson,
                plugin_workspace_desired_state::Column::Revision,
                plugin_workspace_desired_state::Column::UpdatedAt,
            ])
            .to_owned(),
        )
        .exec_without_returning(self.db)?;
        plugin_workspace_desired_state::Entity::find_by_id((workspace_id, plugin_id))
            .one(self.db)?
            .map(workspace_plugin_state)
            .ok_or_else(|| DbErr::RecordNotFound("workspace plugin state".into()))
    }

    pub fn plugin_registry_rows(&self) -> Result<Vec<PluginRegistryRow>, DbErr> {
        let plugins = plugin_registry::Entity::find().all(self.db)?;
        let releases = plugin_registry_releases::Entity::find()
            .order_by_asc(plugin_registry_releases::Column::PluginId)
            .order_by_desc(plugin_registry_releases::Column::PublishedAt)
            .all(self.db)?;
        let manifests = plugins
            .into_iter()
            .map(|plugin| (plugin.plugin_id, plugin.manifest_json))
            .collect::<std::collections::HashMap<_, _>>();
        Ok(releases
            .into_iter()
            .filter_map(|release| {
                manifests
                    .get(&release.plugin_id)
                    .cloned()
                    .map(|manifest_json| PluginRegistryRow {
                        plugin_id: release.plugin_id,
                        manifest_json,
                        version: release.version,
                        artifact_digest: release.artifact_digest,
                        artifact_size: release.artifact_size,
                        signature: release.signature,
                        channel: release.channel,
                        published_at: release.published_at,
                        revoked: release.revoked != 0,
                    })
            })
            .collect())
    }

    pub fn plugin_release(
        &self,
        plugin_id: &str,
        version: &str,
    ) -> Result<Option<PluginRegistryRow>, DbErr> {
        let Some(release) = plugin_registry_releases::Entity::find_by_id((
            plugin_id.to_owned(),
            version.to_owned(),
        ))
        .one(self.db)?
        else {
            return Ok(None);
        };
        let manifest_json = plugin_registry::Entity::find_by_id(plugin_id.to_owned())
            .one(self.db)?
            .map(|plugin| plugin.manifest_json);
        Ok(manifest_json.map(|manifest_json| PluginRegistryRow {
            plugin_id: release.plugin_id,
            manifest_json,
            version: release.version,
            artifact_digest: release.artifact_digest,
            artifact_size: release.artifact_size,
            signature: release.signature,
            channel: release.channel,
            published_at: release.published_at,
            revoked: release.revoked != 0,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn upsert_plugin_release(
        &self,
        plugin_id: String,
        publisher_id: String,
        manifest_json: String,
        version: String,
        artifact_digest: String,
        artifact_size: i64,
        signature: String,
        channel: String,
        published_at: i64,
        now: i64,
    ) -> Result<(), DbErr> {
        let transaction = self.db.begin()?;
        if let Some(existing) =
            plugin_registry_releases::Entity::find_by_id((plugin_id.clone(), version.clone()))
                .one(&transaction)?
        {
            if existing.revoked != 0
                || existing.artifact_digest != artifact_digest
                || existing.artifact_size != artifact_size
                || existing.signature != signature
                || existing.channel != channel
                || existing.published_at != published_at
            {
                return Err(DbErr::Custom(
                    "plugin release versions are immutable".into(),
                ));
            }
            transaction.commit()?;
            return Ok(());
        }
        plugin_registry::Entity::insert(plugin_registry::ActiveModel {
            plugin_id: Set(plugin_id.clone()),
            publisher_id: Set(publisher_id),
            manifest_json: Set(manifest_json),
            created_at: Set(now),
            updated_at: Set(now),
        })
        .on_conflict(
            OnConflict::column(plugin_registry::Column::PluginId)
                .update_columns([
                    plugin_registry::Column::PublisherId,
                    plugin_registry::Column::ManifestJson,
                    plugin_registry::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec_without_returning(&transaction)?;
        plugin_registry_releases::Entity::insert(plugin_registry_releases::ActiveModel {
            plugin_id: Set(plugin_id),
            version: Set(version),
            artifact_digest: Set(artifact_digest),
            artifact_size: Set(artifact_size),
            signature: Set(signature),
            channel: Set(channel),
            published_at: Set(published_at),
            revoked: Set(0),
        })
        .on_conflict_do_nothing()
        .exec_without_returning(&transaction)?;
        transaction.commit()
    }

    pub fn trusted_publisher_key(
        &self,
        publisher_id: &str,
        key_id: &str,
        public_key: &str,
        now: i64,
    ) -> Result<bool, DbErr> {
        Ok(
            plugin_publisher_keys::Entity::find_by_id((publisher_id.to_owned(), key_id.to_owned()))
                .one(self.db)?
                .is_some_and(|key| {
                    key.public_key == public_key
                        && key.revoked_at.is_none()
                        && key.expires_at.is_none_or(|expires_at| expires_at > now)
                }),
        )
    }

    pub fn rotate_publisher_key(
        &self,
        publisher_id: String,
        key_id: String,
        public_key: String,
        expires_at: Option<i64>,
        now: i64,
    ) -> Result<(), DbErr> {
        let transaction = self.db.begin()?;
        if let Some(existing) =
            plugin_publisher_keys::Entity::find_by_id((publisher_id.clone(), key_id.clone()))
                .one(&transaction)?
        {
            if existing.public_key == public_key && existing.revoked_at.is_none() {
                transaction.commit()?;
                return Ok(());
            }
            return Err(DbErr::Custom("publisher key ids are immutable".into()));
        }
        plugin_publisher_keys::Entity::update_many()
            .col_expr(
                plugin_publisher_keys::Column::RevokedAt,
                Expr::value(Some(now)),
            )
            .filter(plugin_publisher_keys::Column::PublisherId.eq(publisher_id.clone()))
            .filter(plugin_publisher_keys::Column::RevokedAt.is_null())
            .exec(&transaction)?;
        plugin_publisher_keys::Entity::insert(plugin_publisher_keys::ActiveModel {
            publisher_id: Set(publisher_id),
            key_id: Set(key_id),
            public_key: Set(public_key),
            created_at: Set(now),
            expires_at: Set(expires_at),
            revoked_at: Set(None),
        })
        .exec_without_returning(&transaction)?;
        transaction.commit()
    }

    pub fn revoke_plugin_release(&self, plugin_id: &str, version: &str) -> Result<bool, DbErr> {
        let result = plugin_registry_releases::Entity::update_many()
            .col_expr(plugin_registry_releases::Column::Revoked, Expr::value(1))
            .filter(plugin_registry_releases::Column::PluginId.eq(plugin_id.to_owned()))
            .filter(plugin_registry_releases::Column::Version.eq(version.to_owned()))
            .filter(plugin_registry_releases::Column::Revoked.eq(0))
            .exec(self.db)?;
        Ok(result.rows_affected > 0)
    }

    /// Email identity is case-insensitive; the filter runs in the database and
    /// never loads every account into memory.
    pub fn active_account_by_email(&self, email: &str) -> Result<Option<accounts::Model>, DbErr> {
        let normalized = normalize_email(email);
        if normalized.is_empty() {
            return Ok(None);
        }
        accounts::Entity::find()
            .filter(accounts::Column::Status.eq("active"))
            .filter(Expr::expr(Func::lower(Expr::col(accounts::Column::Email))).eq(normalized))
            .one(self.db)
    }

    pub fn session_for_token(&self, token_hash: &str) -> Result<Option<SessionRecord>, DbErr> {
        tokens::Entity::find_by_id(token_hash.to_owned())
            .one(self.db)
            .map(|row| {
                row.map(|row| SessionRecord {
                    account_id: row.account_id,
                    created_at: row.created_at,
                    revoked_at: row.revoked_at,
                })
            })
    }

    pub fn insert_account(
        &self,
        id: String,
        email: String,
        password_hash: String,
        now: i64,
    ) -> Result<(), DbErr> {
        accounts::ActiveModel {
            id: Set(id),
            email: Set(email),
            password_hash: Set(password_hash),
            status: Set("active".into()),
            created_at: Set(now),
        }
        .insert(self.db)
        .map(|_| ())
    }

    /// `None` means the address is already registered.
    pub fn insert_account_if_unused(
        &self,
        id: String,
        email: String,
        password_hash: String,
        now: i64,
    ) -> Result<bool, DbErr> {
        if self.active_account_by_email(&email)?.is_some() {
            return Ok(false);
        }
        self.insert_account(id, email, password_hash, now)
            .map(|_| true)
    }

    pub fn insert_token(
        &self,
        token_hash: String,
        account_id: String,
        now: i64,
    ) -> Result<(), DbErr> {
        tokens::ActiveModel {
            token_hash: Set(token_hash),
            account_id: Set(account_id),
            created_at: Set(now),
            revoked_at: Set(None),
        }
        .insert(self.db)
        .map(|_| ())
    }

    /// Idempotent: a repeated call is a no-op and a racing insert resolves on
    /// the primary key instead of failing.
    pub fn ensure_workspace(&self, id: String, name: String, now: i64) -> Result<(), DbErr> {
        workspaces::Entity::insert(workspaces::ActiveModel {
            id: Set(id),
            name: Set(name),
            created_at: Set(now),
        })
        .on_conflict_do_nothing()
        .exec_without_returning(self.db)
        .map(|_| ())
    }

    pub fn workspace_rows(
        &self,
        account_id: &str,
    ) -> Result<Vec<(workspaces::Model, String)>, DbErr> {
        let memberships = memberships::Entity::find()
            .filter(memberships::Column::AccountId.eq(account_id.to_owned()))
            .all(self.db)?;
        let ids = memberships
            .iter()
            .map(|row| row.workspace_id.clone())
            .collect::<Vec<_>>();
        let rows = workspaces::Entity::find()
            .filter(workspaces::Column::Id.is_in(ids))
            .all(self.db)?;
        let by_id = rows
            .into_iter()
            .map(|row| (row.id.clone(), row))
            .collect::<std::collections::HashMap<_, _>>();
        Ok(memberships
            .into_iter()
            .filter_map(|membership| {
                by_id
                    .get(&membership.workspace_id)
                    .cloned()
                    .map(|workspace| (workspace, membership.role))
            })
            .collect())
    }

    pub fn membership_role(
        &self,
        account_id: &str,
        workspace_id: &str,
    ) -> Result<Option<String>, DbErr> {
        Ok(
            memberships::Entity::find_by_id((workspace_id.to_owned(), account_id.to_owned()))
                .one(self.db)?
                .map(|row| row.role),
        )
    }

    pub fn owner_count(&self, workspace_id: &str) -> Result<u64, DbErr> {
        memberships::Entity::find()
            .filter(memberships::Column::WorkspaceId.eq(workspace_id.to_owned()))
            .filter(memberships::Column::Role.eq(Role::Owner.as_str()))
            .count(self.db)
    }

    /// One statement, so two concurrent grants for the same pair cannot both
    /// take the "row does not exist" branch. `created_at` keeps its original
    /// value when the membership already exists.
    pub fn upsert_membership(
        &self,
        workspace_id: String,
        account_id: String,
        role: String,
        now: i64,
    ) -> Result<(), DbErr> {
        memberships::Entity::insert(memberships::ActiveModel {
            workspace_id: Set(workspace_id),
            account_id: Set(account_id),
            role: Set(role),
            created_at: Set(now),
        })
        .on_conflict(
            OnConflict::columns([
                memberships::Column::WorkspaceId,
                memberships::Column::AccountId,
            ])
            .update_column(memberships::Column::Role)
            .to_owned(),
        )
        .exec_without_returning(self.db)
        .map(|_| ())
    }

    pub fn push(&self, request: &SyncPushRequest) -> Result<SyncPushResponse, DbErr> {
        self.db
            .transaction::<_, SyncPushResponse, DbErr>(|tx| {
                let mut cursor = operations::Entity::find()
                    .order_by_desc(operations::Column::Revision)
                    .one(tx)?
                    .map(|row| std::cmp::max(row.revision, 0) as u64)
                    .unwrap_or(0);
                let mut accepted = Vec::new();
                let mut conflicts = Vec::new();
                for operation in &request.operations {
                    if let Some(existing) =
                        operations::Entity::find_by_id(operation.operation_id.clone()).one(tx)?
                    {
                        record_replay(&existing, operation, &mut accepted, &mut conflicts);
                        continue;
                    }
                    let latest = operations::Entity::find()
                        .filter(operations::Column::WorkspaceId.eq(operation.workspace_id.clone()))
                        .filter(operations::Column::EntityType.eq(operation.entity_type.clone()))
                        .filter(operations::Column::EntityId.eq(operation.entity_id.clone()))
                        .order_by_desc(operations::Column::Revision)
                        .one(tx)?
                        .map(|row| row.revision as u64);
                    if operation.base_revision.is_some() && operation.base_revision != latest {
                        conflicts.push(operation.operation_id.clone());
                        continue;
                    }
                    // A second writer can store the same operation id between the
                    // read above and this insert: the primary key decides and the
                    // loser reports a replay or a conflict instead of an error.
                    let (stored, written_here) = store_operation(
                        tx,
                        &request.device_id,
                        operation,
                        cursor.saturating_add(1),
                    )?;
                    cursor = std::cmp::max(cursor, std::cmp::max(stored.revision, 0) as u64);
                    if written_here {
                        accepted.push(operation.operation_id.clone());
                    } else {
                        record_replay(&stored, operation, &mut accepted, &mut conflicts);
                    }
                }
                Ok(SyncPushResponse {
                    accepted,
                    conflicts,
                    cursor,
                })
            })
            .map_err(|error| DbErr::Custom(error.to_string()))
    }

    pub fn pull(
        &self,
        workspace_id: &str,
        cursor: u64,
        limit: u64,
    ) -> Result<Vec<SyncChange>, DbErr> {
        operations::Entity::find()
            .filter(operations::Column::WorkspaceId.eq(workspace_id.to_owned()))
            .filter(operations::Column::Revision.gt(cursor as i64))
            .order_by_asc(operations::Column::Revision)
            .limit(limit)
            .all(self.db)
            .map(|rows| rows.into_iter().map(change_from_row).collect())
    }
}

/// Stores one operation unless its id is already taken, and returns the stored
/// row plus whether this transaction wrote it. The primary key decides a
/// concurrent duplicate; the row is read back so the caller sees the revision
/// that actually won, not the one this attempt proposed.
fn store_operation(
    tx: &DatabaseTransaction,
    device_id: &str,
    operation: &SyncOperation,
    revision: u64,
) -> Result<(operations::Model, bool), DbErr> {
    let affected = operations::Entity::insert(operations::ActiveModel {
        operation_id: Set(operation.operation_id.clone()),
        workspace_id: Set(operation.workspace_id.clone()),
        device_id: Set(device_id.to_owned()),
        entity_type: Set(operation.entity_type.clone()),
        entity_id: Set(operation.entity_id.clone()),
        operation: Set(operation.operation.clone()),
        payload: Set(operation.payload.to_string()),
        base_revision: Set(operation.base_revision.map(|value| value as i64)),
        revision: Set(i64::try_from(revision).unwrap_or(i64::MAX)),
        created_at: Set(now()),
    })
    .on_conflict_do_nothing()
    .exec_without_returning(tx)?;
    let written_here = matches!(affected, TryInsertResult::Inserted(rows) if rows > 0);
    let stored = operations::Entity::find_by_id(operation.operation_id.clone())
        .one(tx)?
        .ok_or_else(|| {
            DbErr::RecordNotFound(format!(
                "sync operation {} is neither inserted nor readable",
                operation.operation_id
            ))
        })?;
    Ok((stored, written_here))
}

/// An already stored operation id is either the same content (an idempotent
/// replay) or a conflict, reported per operation rather than as a failed push.
fn record_replay(
    existing: &operations::Model,
    incoming: &SyncOperation,
    accepted: &mut Vec<String>,
    conflicts: &mut Vec<String>,
) {
    if classify_replay(Some(&stored_fingerprint(existing)), &incoming.fingerprint())
        == ReplayDecision::Idempotent
    {
        accepted.push(incoming.operation_id.clone());
    } else {
        conflicts.push(incoming.operation_id.clone());
    }
}

/// Recomputes the stored content fingerprint from persisted columns. A payload
/// that cannot be parsed becomes `null`, so it never matches an incoming
/// fingerprint and the replay is reported as a conflict instead of succeeding.
fn stored_fingerprint(row: &operations::Model) -> String {
    SyncOperation {
        operation_id: row.operation_id.clone(),
        workspace_id: row.workspace_id.clone(),
        entity_type: row.entity_type.clone(),
        entity_id: row.entity_id.clone(),
        operation: row.operation.clone(),
        payload: serde_json::from_str::<Value>(&row.payload).unwrap_or(Value::Null),
        base_revision: None,
    }
    .fingerprint()
}

fn change_from_row(row: operations::Model) -> SyncChange {
    SyncChange {
        revision: row.revision as u64,
        operation: SyncOperation {
            operation_id: row.operation_id,
            workspace_id: row.workspace_id,
            entity_type: row.entity_type,
            entity_id: row.entity_id,
            operation: row.operation,
            payload: serde_json::from_str::<Value>(&row.payload).unwrap_or_default(),
            base_revision: row.base_revision.map(|value| value as u64),
        },
        created_at: row.created_at as u64,
    }
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::sea_orm::Database;

    fn repository_fixture() -> DatabaseConnection {
        let database = Database::connect("sqlite::memory:").expect("database");
        super::super::initialize(&database).expect("schema");
        database
    }

    fn operation(payload: Value) -> SyncOperation {
        SyncOperation {
            operation_id: "operation".into(),
            workspace_id: "workspace".into(),
            entity_type: "timeline".into(),
            entity_id: "event".into(),
            operation: "upsert".into(),
            payload,
            base_revision: None,
        }
    }

    fn push_request(payload: Value) -> SyncPushRequest {
        SyncPushRequest {
            device_id: "device".into(),
            operations: vec![operation(payload)],
        }
    }

    #[test]
    fn typed_server_repository_preserves_push_conflict_and_pull_semantics() {
        let database = repository_fixture();
        let repository = Repository::new(&database);
        repository
            .insert_account(
                "account".into(),
                "user@example.test".into(),
                "hash".into(),
                1,
            )
            .expect("account");
        repository
            .ensure_workspace("workspace".into(), "Workspace".into(), 1)
            .expect("workspace");
        repository
            .upsert_membership("workspace".into(), "account".into(), "owner".into(), 1)
            .expect("membership");
        let request = push_request(serde_json::json!({"title": "Focus"}));
        let pushed = repository.push(&request).expect("push");
        assert_eq!(pushed.accepted, vec!["operation"]);
        assert!(pushed.conflicts.is_empty());
        assert_eq!(
            repository.push(&request).expect("idempotent push").accepted,
            vec!["operation"]
        );
        let changes = repository.pull("workspace", 0, 10).expect("pull");
        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes[0].operation.payload,
            serde_json::json!({"title": "Focus"})
        );
    }

    #[test]
    fn replaying_an_operation_with_other_content_is_a_conflict() {
        let database = repository_fixture();
        let repository = Repository::new(&database);
        repository
            .push(&push_request(serde_json::json!({"title": "Focus"})))
            .expect("push");
        let replayed = repository
            .push(&push_request(serde_json::json!({"title": "Deep work"})))
            .expect("replay");
        assert_eq!(replayed.accepted, Vec::<String>::new());
        assert_eq!(replayed.conflicts, vec!["operation"]);
        assert_eq!(repository.pull("workspace", 0, 10).expect("pull").len(), 1);
    }

    #[test]
    fn the_same_operation_id_twice_in_one_push_is_stored_once() {
        let database = repository_fixture();
        let repository = Repository::new(&database);
        let operation = operation(serde_json::json!({"title": "Focus"}));
        let request = SyncPushRequest {
            device_id: "device".into(),
            operations: vec![operation.clone(), operation],
        };
        let pushed = repository.push(&request).expect("push");
        assert_eq!(pushed.accepted, vec!["operation", "operation"]);
        assert!(pushed.conflicts.is_empty());
        let changes = repository.pull("workspace", 0, 10).expect("pull");
        assert_eq!(
            changes.len(),
            1,
            "the duplicate id is a replay, not a second row"
        );
        assert_eq!(
            pushed.cursor, changes[0].revision,
            "the response cursor is the revision that was actually stored"
        );
    }

    #[test]
    fn payload_key_order_does_not_change_the_fingerprint() {
        let database = repository_fixture();
        let repository = Repository::new(&database);
        let first: Value = serde_json::from_str(r#"{"a":1,"b":2}"#).unwrap();
        let second: Value = serde_json::from_str(r#"{"b":2,"a":1}"#).unwrap();
        repository.push(&push_request(first)).expect("push");
        let replayed = repository.push(&push_request(second)).expect("replay");
        assert_eq!(replayed.accepted, vec!["operation"]);
        assert!(replayed.conflicts.is_empty());
    }

    #[test]
    fn membership_and_owner_queries_reflect_storage() {
        let database = repository_fixture();
        let repository = Repository::new(&database);
        for (id, email) in [("a", "a@example.test"), ("b", "B@Example.test")] {
            repository
                .insert_account(id.into(), email.into(), "hash".into(), 1)
                .expect("account");
        }
        repository
            .ensure_workspace("workspace".into(), "Workspace".into(), 1)
            .expect("workspace");
        repository
            .upsert_membership("workspace".into(), "a".into(), "owner".into(), 1)
            .expect("membership");
        assert_eq!(repository.owner_count("workspace").expect("count"), 1);
        assert_eq!(
            repository.membership_role("a", "workspace").expect("role"),
            Some("owner".into())
        );
        assert_eq!(
            repository.membership_role("b", "workspace").expect("role"),
            None
        );
        assert_eq!(repository.owner_count("other").expect("count"), 0);
    }

    #[test]
    fn accounts_are_found_case_insensitively_without_scanning() {
        let database = repository_fixture();
        let repository = Repository::new(&database);
        repository
            .insert_account(
                "account".into(),
                "owner@example.test".into(),
                "hash".into(),
                1,
            )
            .expect("account");
        assert!(
            repository
                .active_account_by_email(" OWNER@Example.Test ")
                .expect("lookup")
                .is_some()
        );
        assert!(
            repository
                .active_account_by_email("other@example.test")
                .expect("lookup")
                .is_none()
        );
        assert!(
            repository
                .active_account_by_email("  ")
                .expect("lookup")
                .is_none()
        );
        assert!(
            !repository
                .insert_account_if_unused(
                    "second".into(),
                    "Owner@Example.test".into(),
                    "hash".into(),
                    1
                )
                .expect("insert")
        );
    }

    #[test]
    fn stored_sessions_expose_expiry_inputs() {
        let database = repository_fixture();
        let repository = Repository::new(&database);
        repository
            .insert_account(
                "account".into(),
                "owner@example.test".into(),
                "hash".into(),
                1,
            )
            .expect("account");
        repository
            .insert_token("hash".into(), "account".into(), 10)
            .expect("token");
        let session = repository
            .session_for_token("hash")
            .expect("session")
            .expect("present");
        assert_eq!(session.account_id, "account");
        assert_eq!(session.created_at, 10);
        assert_eq!(session.revoked_at, None);
        assert!(
            repository
                .session_for_token("missing")
                .expect("session")
                .is_none()
        );
    }

    #[test]
    fn plugin_registry_release_upsert_is_idempotent_and_retrievable() {
        let database = repository_fixture();
        let repository = Repository::new(&database);
        repository
            .upsert_plugin_release(
                "dev.example.plugin".into(),
                "publisher.example".into(),
                "{\"id\":\"dev.example.plugin\"}".into(),
                "1.0.0".into(),
                "a".repeat(64),
                42,
                "signature".into(),
                "stable".into(),
                10,
                10,
            )
            .expect("release");
        repository
            .upsert_plugin_release(
                "dev.example.plugin".into(),
                "publisher.example".into(),
                "{\"id\":\"dev.example.plugin\"}".into(),
                "1.0.0".into(),
                "a".repeat(64),
                42,
                "signature".into(),
                "stable".into(),
                10,
                11,
            )
            .expect("idempotent release");
        let rows = repository.plugin_registry_rows().expect("rows");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].artifact_digest, "a".repeat(64));
        assert_eq!(rows[0].channel, "stable");
        assert!(
            repository
                .upsert_plugin_release(
                    "dev.example.plugin".into(),
                    "publisher.example".into(),
                    "{\"id\":\"dev.example.plugin\"}".into(),
                    "1.0.0".into(),
                    "b".repeat(64),
                    43,
                    "signature-2".into(),
                    "beta".into(),
                    11,
                    12,
                )
                .is_err()
        );
        assert!(
            repository
                .plugin_release("dev.example.plugin", "missing")
                .expect("lookup")
                .is_none()
        );
    }

    #[test]
    fn workspace_plugin_desired_state_upsert_increments_revision() {
        let database = repository_fixture();
        let repository = Repository::new(&database);
        repository
            .ensure_workspace("workspace".into(), "Workspace".into(), 1)
            .expect("workspace");
        let first = repository
            .upsert_workspace_plugin_state(
                "workspace".into(),
                "dev.example.plugin".into(),
                Some("1.2.3".into()),
                "stable".into(),
                true,
                "[\"tasks:read\"]".into(),
                10,
            )
            .expect("first desired state");
        assert_eq!(first.revision, 1);
        assert_eq!(first.requested_version.as_deref(), Some("1.2.3"));

        let second = repository
            .upsert_workspace_plugin_state(
                "workspace".into(),
                "dev.example.plugin".into(),
                Some("1.3.0".into()),
                "beta".into(),
                false,
                "[]".into(),
                11,
            )
            .expect("updated desired state");
        assert_eq!(second.revision, 2);
        assert!(!second.enabled);
        assert_eq!(second.channel, "beta");

        let states = repository
            .workspace_plugin_states("workspace")
            .expect("desired states");
        assert_eq!(states, vec![second]);
    }

    #[test]
    fn publisher_key_rotation_revokes_previous_key_and_honours_expiry() {
        let database = repository_fixture();
        let repository = Repository::new(&database);
        repository
            .rotate_publisher_key(
                "publisher.example".into(),
                "key-1".into(),
                "a".repeat(44),
                Some(100),
                10,
            )
            .expect("first key");
        assert!(
            repository
                .trusted_publisher_key("publisher.example", "key-1", &"a".repeat(44), 50)
                .expect("trust lookup")
        );
        assert!(
            !repository
                .trusted_publisher_key("publisher.example", "key-1", &"a".repeat(44), 100)
                .expect("expired lookup")
        );
        repository
            .rotate_publisher_key(
                "publisher.example".into(),
                "key-2".into(),
                "b".repeat(44),
                None,
                60,
            )
            .expect("rotation");
        assert!(
            !repository
                .trusted_publisher_key("publisher.example", "key-1", &"a".repeat(44), 60)
                .expect("revoked lookup")
        );
        assert!(
            repository
                .trusted_publisher_key("publisher.example", "key-2", &"b".repeat(44), 60)
                .expect("new lookup")
        );
    }
}
