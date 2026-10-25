//! Workspace, team and permission governance.
//!
//! Persistence is kept behind the infrastructure repository so policy
//! decisions remain independent from the SQLite representation.

use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use aevio_persistence::database::DatabaseConnection;
use aevio_persistence::database::entities::governance::{
    audit, departments, member_permissions, members, team_members, teams,
};
use aevio_persistence::database::repositories::governance::GovernanceRepository;

const WORKSPACE: &str = "workspace-local";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub currency: String,
    pub timezone: String,
    pub defaults: Value,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    pub id: String,
    pub name: String,
    pub currency: String,
    pub timezone: String,
    pub rate_strategy: String,
    pub auto_approve: bool,
    pub member_count: i64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Department {
    pub id: String,
    pub name: String,
    pub member_count: i64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub status: String,
    pub org_role: String,
    pub job_title: Option<String>,
    pub manager_id: Option<String>,
    pub default_team_id: Option<String>,
    pub department_id: Option<String>,
    pub team_roles: Vec<String>,
    pub permissions: Vec<String>,
    pub bill_rate_cents: Option<i64>,
    pub cost_rate_cents: Option<i64>,
    pub billable_default: bool,
    pub effective_defaults: Value,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub id: String,
    pub actor_id: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub at: i64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceSnapshot {
    pub workspace: Workspace,
    pub teams: Vec<Team>,
    pub departments: Vec<Department>,
    pub members: Vec<Member>,
    pub audit: Vec<AuditEvent>,
    pub can_manage: bool,
    pub can_view_finance: bool,
    pub can_manage_resourcing: bool,
    pub can_export: bool,
    pub is_manager: bool,
    pub is_viewer: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamInput {
    pub id: Option<String>,
    pub name: String,
    pub currency: String,
    pub timezone: String,
    pub rate_strategy: String,
    pub auto_approve: bool,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepartmentInput {
    pub id: Option<String>,
    pub name: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberInput {
    pub id: Option<String>,
    pub email: String,
    pub display_name: String,
    pub org_role: String,
    pub job_title: Option<String>,
    pub manager_id: Option<String>,
    pub default_team_id: Option<String>,
    pub department_id: Option<String>,
    pub team_ids: Vec<String>,
    pub team_role: String,
    pub permissions: Vec<String>,
    pub bill_rate_cents: Option<i64>,
    pub cost_rate_cents: Option<i64>,
    pub billable_default: bool,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultsInput {
    pub defaults: Value,
    pub apply_to_all: bool,
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
fn id() -> String {
    Uuid::now_v7().to_string()
}
fn json_text(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".into())
}
fn parse(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| json!({}))
}
fn db_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn allowed(db: &DatabaseConnection, actor: &str, permission: &str) -> Result<bool, String> {
    let repository = GovernanceRepository::new(db);
    if repository.member_role(actor).map_err(db_error)?.as_deref() == Some("admin") {
        return Ok(true);
    }
    repository
        .has_permission(actor, permission)
        .map_err(db_error)
}
fn team_role(db: &DatabaseConnection, actor: &str, role: &str) -> Result<bool, String> {
    GovernanceRepository::new(db)
        .has_team_role(actor, role)
        .map_err(db_error)
}
pub fn require(db: &DatabaseConnection, actor: &str, permission: &str) -> Result<(), String> {
    allowed(db, actor, permission)?
        .then_some(())
        .ok_or_else(|| format!("Permission denied: {permission}"))
}

fn audit_event(
    db: &DatabaseConnection,
    actor: &str,
    action: &str,
    entity_type: &str,
    entity_id: &str,
    before: Option<Value>,
    after: Option<Value>,
) -> Result<(), String> {
    GovernanceRepository::new(db)
        .add_audit(audit::ActiveModel {
            id: sea_orm::ActiveValue::Set(id()),
            workspace_id: sea_orm::ActiveValue::Set(WORKSPACE.into()),
            actor_id: sea_orm::ActiveValue::Set(actor.into()),
            action: sea_orm::ActiveValue::Set(action.into()),
            entity_type: sea_orm::ActiveValue::Set(entity_type.into()),
            entity_id: sea_orm::ActiveValue::Set(entity_id.into()),
            before_json: sea_orm::ActiveValue::Set(before.as_ref().map(json_text)),
            after_json: sea_orm::ActiveValue::Set(after.as_ref().map(json_text)),
            at: sea_orm::ActiveValue::Set(now()),
        })
        .map_err(db_error)
}

pub fn snapshot(db: &DatabaseConnection, actor: &str) -> Result<GovernanceSnapshot, String> {
    let repository = GovernanceRepository::new(db);
    let workspace = repository
        .workspace(WORKSPACE)
        .map_err(db_error)?
        .ok_or_else(|| "workspace not found".to_string())?;
    let team_rows = repository.teams().map_err(db_error)?;
    let department_rows = repository.departments().map_err(db_error)?;
    let member_rows = repository.members().map_err(db_error)?;
    let all_team_members = team_members::Entity::find().all(db).map_err(db_error)?;
    let all_member_permissions = member_permissions::Entity::find()
        .all(db)
        .map_err(db_error)?;
    let audit_rows = repository.audit().map_err(db_error)?;
    let teams = team_rows
        .into_iter()
        .map(|row| Team {
            member_count: all_team_members
                .iter()
                .filter(|item| item.team_id == row.id)
                .count() as i64,
            id: row.id,
            name: row.name,
            currency: row.currency,
            timezone: row.timezone,
            rate_strategy: row.rate_strategy,
            auto_approve: row.auto_approve != 0,
        })
        .collect();
    let departments = department_rows
        .into_iter()
        .map(|row| Department {
            member_count: member_rows
                .iter()
                .filter(|item| item.department_id.as_deref() == Some(row.id.as_str()))
                .count() as i64,
            id: row.id,
            name: row.name,
        })
        .collect();
    let members = member_rows
        .into_iter()
        .map(|row| Member {
            team_roles: all_team_members
                .iter()
                .filter(|item| item.member_id == row.id)
                .map(|item| item.team_role.clone())
                .collect(),
            permissions: all_member_permissions
                .iter()
                .filter(|item| item.member_id == row.id)
                .map(|item| item.permission.clone())
                .collect(),
            id: row.id,
            email: row.email,
            display_name: row.display_name,
            status: row.status,
            org_role: row.org_role,
            job_title: row.job_title,
            manager_id: row.manager_id,
            default_team_id: row.default_team_id,
            department_id: row.department_id,
            bill_rate_cents: row.bill_rate_cents,
            cost_rate_cents: row.cost_rate_cents,
            billable_default: row.billable_default != 0,
            effective_defaults: parse(&row.effective_defaults_json),
        })
        .collect();
    let audit = audit_rows
        .into_iter()
        .map(|row| AuditEvent {
            id: row.id,
            actor_id: row.actor_id,
            action: row.action,
            entity_type: row.entity_type,
            entity_id: row.entity_id,
            before: row.before_json.as_deref().map(parse),
            after: row.after_json.as_deref().map(parse),
            at: row.at,
        })
        .collect();
    Ok(GovernanceSnapshot {
        workspace: Workspace {
            id: workspace.id,
            name: workspace.name,
            currency: workspace.currency,
            timezone: workspace.timezone,
            defaults: parse(&workspace.defaults_json),
        },
        teams,
        departments,
        members,
        audit,
        can_manage: allowed(db, actor, "manage_members")?,
        can_view_finance: allowed(db, actor, "view_finance")?,
        can_manage_resourcing: allowed(db, actor, "manage_resourcing")?,
        can_export: allowed(db, actor, "export_data")?,
        is_manager: team_role(db, actor, "manager")?,
        is_viewer: team_role(db, actor, "viewer")?,
    })
}

pub fn save_team(db: &DatabaseConnection, actor: &str, input: TeamInput) -> Result<Team, String> {
    require(db, actor, "manage_teams")?;
    let id = input.id.unwrap_or_else(id);
    let ts = now();
    GovernanceRepository::new(db)
        .save_team(teams::ActiveModel {
            id: sea_orm::ActiveValue::Set(id.clone()),
            workspace_id: sea_orm::ActiveValue::Set(WORKSPACE.into()),
            name: sea_orm::ActiveValue::Set(input.name),
            currency: sea_orm::ActiveValue::Set(input.currency),
            timezone: sea_orm::ActiveValue::Set(input.timezone),
            rate_strategy: sea_orm::ActiveValue::Set(input.rate_strategy),
            auto_approve: sea_orm::ActiveValue::Set(i64::from(input.auto_approve)),
            created_at: sea_orm::ActiveValue::Set(ts),
            updated_at: sea_orm::ActiveValue::Set(ts),
        })
        .map_err(db_error)?;
    audit_event(db, actor, "upsert", "team", &id, None, None)?;
    let saved = teams::Entity::find_by_id(id)
        .one(db)
        .map_err(db_error)?
        .ok_or_else(|| "team not found after save".to_string())?;
    let count = team_members::Entity::find()
        .filter(team_members::Column::TeamId.eq(saved.id.clone()))
        .count(db)
        .map_err(db_error)? as i64;
    Ok(Team {
        id: saved.id,
        name: saved.name,
        currency: saved.currency,
        timezone: saved.timezone,
        rate_strategy: saved.rate_strategy,
        auto_approve: saved.auto_approve != 0,
        member_count: count,
    })
}

pub fn save_department(
    db: &DatabaseConnection,
    actor: &str,
    input: DepartmentInput,
) -> Result<Department, String> {
    require(db, actor, "manage_members")?;
    let id = input.id.unwrap_or_else(id);
    let ts = now();
    let name = input.name;
    GovernanceRepository::new(db)
        .save_department(departments::ActiveModel {
            id: sea_orm::ActiveValue::Set(id.clone()),
            workspace_id: sea_orm::ActiveValue::Set(WORKSPACE.into()),
            name: sea_orm::ActiveValue::Set(name.clone()),
            created_at: sea_orm::ActiveValue::Set(ts),
            updated_at: sea_orm::ActiveValue::Set(ts),
        })
        .map_err(db_error)?;
    audit_event(db, actor, "upsert", "department", &id, None, None)?;
    Ok(Department {
        id,
        name,
        member_count: 0,
    })
}

pub fn save_member(db: &DatabaseConnection, actor: &str, input: MemberInput) -> Result<(), String> {
    require(db, actor, "manage_members")?;
    let id = input.id.unwrap_or_else(id);
    let ts = now();
    let defaults = GovernanceRepository::new(db)
        .workspace(WORKSPACE)
        .map_err(db_error)?
        .map(|row| row.defaults_json)
        .unwrap_or_else(|| "{}".into());
    let row = members::ActiveModel {
        id: sea_orm::ActiveValue::Set(id.clone()),
        workspace_id: sea_orm::ActiveValue::Set(WORKSPACE.into()),
        email: sea_orm::ActiveValue::Set(input.email),
        display_name: sea_orm::ActiveValue::Set(input.display_name),
        status: sea_orm::ActiveValue::Set("active".into()),
        org_role: sea_orm::ActiveValue::Set(input.org_role),
        job_title: sea_orm::ActiveValue::Set(input.job_title),
        manager_id: sea_orm::ActiveValue::Set(input.manager_id),
        default_team_id: sea_orm::ActiveValue::Set(input.default_team_id),
        department_id: sea_orm::ActiveValue::Set(input.department_id),
        bill_rate_cents: sea_orm::ActiveValue::Set(input.bill_rate_cents),
        cost_rate_cents: sea_orm::ActiveValue::Set(input.cost_rate_cents),
        billable_default: sea_orm::ActiveValue::Set(i64::from(input.billable_default)),
        effective_defaults_json: sea_orm::ActiveValue::Set(defaults),
        created_at: sea_orm::ActiveValue::Set(ts),
        updated_at: sea_orm::ActiveValue::Set(ts),
    };
    let teams_for_member = input
        .team_ids
        .into_iter()
        .map(|team_id| team_members::ActiveModel {
            team_id: sea_orm::ActiveValue::Set(team_id),
            member_id: sea_orm::ActiveValue::Set(id.clone()),
            team_role: sea_orm::ActiveValue::Set(input.team_role.clone()),
        })
        .collect();
    let permissions_for_member = input
        .permissions
        .into_iter()
        .map(|permission| member_permissions::ActiveModel {
            member_id: sea_orm::ActiveValue::Set(id.clone()),
            permission: sea_orm::ActiveValue::Set(permission),
        })
        .collect();
    GovernanceRepository::new(db)
        .save_member(&id, row, teams_for_member, permissions_for_member)
        .map_err(db_error)?;
    audit_event(db, actor, "upsert", "member", &id, None, None)
}

pub fn save_defaults(
    db: &DatabaseConnection,
    actor: &str,
    input: DefaultsInput,
) -> Result<(), String> {
    require(db, actor, "manage_workspace")?;
    let text = json_text(&input.defaults);
    GovernanceRepository::new(db)
        .save_defaults(&text, input.apply_to_all, now())
        .map_err(db_error)?;
    audit_event(
        db,
        actor,
        if input.apply_to_all {
            "apply_defaults"
        } else {
            "update_defaults"
        },
        "workspace",
        WORKSPACE,
        None,
        Some(input.defaults),
    )
}
pub fn authorize(db: &DatabaseConnection, actor: &str, permission: &str) -> Result<bool, String> {
    allowed(db, actor, permission)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aevio_persistence::database::entities::governance::{
        audit, departments, member_permissions, members, team_members, teams, workspaces,
    };
    use aevio_persistence::database::sea_orm::{ConnectionTrait, Database, DbBackend, Schema};
    use sea_orm::{ActiveValue::Set, EntityTrait};

    fn connection() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").unwrap();
        let schema = Schema::new(DbBackend::Sqlite);
        for statement in [
            schema.create_table_from_entity(workspaces::Entity),
            schema.create_table_from_entity(teams::Entity),
            schema.create_table_from_entity(departments::Entity),
            schema.create_table_from_entity(members::Entity),
            schema.create_table_from_entity(team_members::Entity),
            schema.create_table_from_entity(member_permissions::Entity),
            schema.create_table_from_entity(audit::Entity),
        ] {
            db.execute(&statement).unwrap();
        }
        workspaces::Entity::insert(workspaces::ActiveModel {
            id: Set(WORKSPACE.into()),
            name: Set("My Workspace".into()),
            currency: Set("USD".into()),
            timezone: Set("UTC".into()),
            defaults_json: Set("{}".into()),
            created_at: Set(0),
            updated_at: Set(0),
        })
        .exec(&db)
        .unwrap();
        teams::Entity::insert(teams::ActiveModel {
            id: Set("team-default".into()),
            workspace_id: Set(WORKSPACE.into()),
            name: Set("Default Team".into()),
            currency: Set("USD".into()),
            timezone: Set("UTC".into()),
            rate_strategy: Set("member".into()),
            auto_approve: Set(0),
            created_at: Set(0),
            updated_at: Set(0),
        })
        .exec(&db)
        .unwrap();
        members::Entity::insert(members::ActiveModel {
            id: Set("member-local".into()),
            workspace_id: Set(WORKSPACE.into()),
            email: Set("local@aevio".into()),
            display_name: Set("Local Owner".into()),
            status: Set("active".into()),
            org_role: Set("admin".into()),
            job_title: Set(None),
            manager_id: Set(None),
            default_team_id: Set(Some("team-default".into())),
            department_id: Set(None),
            bill_rate_cents: Set(None),
            cost_rate_cents: Set(None),
            billable_default: Set(1),
            effective_defaults_json: Set("{}".into()),
            created_at: Set(0),
            updated_at: Set(0),
        })
        .exec(&db)
        .unwrap();
        team_members::Entity::insert(team_members::ActiveModel {
            team_id: Set("team-default".into()),
            member_id: Set("member-local".into()),
            team_role: Set("admin".into()),
        })
        .exec(&db)
        .unwrap();
        db
    }

    #[test]
    fn owner_can_manage_and_view_governance() {
        let db = connection();
        let snapshot = snapshot(&db, "member-local").unwrap();
        assert!(snapshot.can_manage && snapshot.can_view_finance && snapshot.can_export);
        assert!(!snapshot.is_viewer);
    }

    #[test]
    fn viewer_cannot_mutate_or_export_without_permission() {
        let db = connection();
        members::Entity::insert(members::ActiveModel {
            id: Set("viewer".into()),
            workspace_id: Set(WORKSPACE.into()),
            email: Set("viewer@local".into()),
            display_name: Set("Viewer".into()),
            status: Set("active".into()),
            org_role: Set("member".into()),
            job_title: Set(None),
            manager_id: Set(None),
            default_team_id: Set(None),
            department_id: Set(None),
            bill_rate_cents: Set(None),
            cost_rate_cents: Set(None),
            billable_default: Set(1),
            effective_defaults_json: Set("{}".into()),
            created_at: Set(1),
            updated_at: Set(1),
        })
        .exec(&db)
        .unwrap();
        team_members::Entity::insert(team_members::ActiveModel {
            team_id: Set("team-default".into()),
            member_id: Set("viewer".into()),
            team_role: Set("viewer".into()),
        })
        .exec(&db)
        .unwrap();
        assert!(snapshot(&db, "viewer").unwrap().is_viewer);
        assert!(
            save_department(
                &db,
                "viewer",
                DepartmentInput {
                    id: None,
                    name: "Blocked".into()
                }
            )
            .is_err()
        );
        assert!(!authorize(&db, "viewer", "export_data").unwrap());
    }

    #[test]
    fn applying_defaults_updates_members_and_audits() {
        let db = connection();
        save_defaults(
            &db,
            "member-local",
            DefaultsInput {
                defaults: json!({"minimumDuration": 60}),
                apply_to_all: true,
            },
        )
        .unwrap();
        let member = members::Entity::find_by_id("member-local")
            .one(&db)
            .unwrap()
            .unwrap();
        assert!(member.effective_defaults_json.contains("minimumDuration"));
        assert_eq!(audit::Entity::find().all(&db).unwrap().len(), 1);
    }
}
