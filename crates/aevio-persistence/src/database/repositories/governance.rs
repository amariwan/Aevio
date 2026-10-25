use sea_orm::entity::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};

use crate::database::entities::governance::{
    audit, departments, member_permissions, members, team_members, teams, workspaces,
};

pub struct GovernanceRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> GovernanceRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn workspace(&self, id: &str) -> Result<Option<workspaces::Model>, DbErr> {
        workspaces::Entity::find_by_id(id.to_owned()).one(self.db)
    }
    pub fn teams(&self) -> Result<Vec<teams::Model>, DbErr> {
        teams::Entity::find()
            .filter(teams::Column::WorkspaceId.eq("workspace-local"))
            .order_by_asc(teams::Column::Name)
            .all(self.db)
    }
    pub fn departments(&self) -> Result<Vec<departments::Model>, DbErr> {
        departments::Entity::find()
            .filter(departments::Column::WorkspaceId.eq("workspace-local"))
            .order_by_asc(departments::Column::Name)
            .all(self.db)
    }
    pub fn members(&self) -> Result<Vec<members::Model>, DbErr> {
        members::Entity::find()
            .filter(members::Column::WorkspaceId.eq("workspace-local"))
            .order_by_asc(members::Column::DisplayName)
            .all(self.db)
    }
    pub fn team_members(&self, member_id: &str) -> Result<Vec<team_members::Model>, DbErr> {
        team_members::Entity::find()
            .filter(team_members::Column::MemberId.eq(member_id.to_owned()))
            .all(self.db)
    }
    pub fn permissions(&self, member_id: &str) -> Result<Vec<member_permissions::Model>, DbErr> {
        member_permissions::Entity::find()
            .filter(member_permissions::Column::MemberId.eq(member_id.to_owned()))
            .all(self.db)
    }
    pub fn audit(&self) -> Result<Vec<audit::Model>, DbErr> {
        audit::Entity::find()
            .filter(audit::Column::WorkspaceId.eq("workspace-local"))
            .order_by_desc(audit::Column::At)
            .limit(100)
            .all(self.db)
    }
    pub fn member_role(&self, id: &str) -> Result<Option<String>, DbErr> {
        members::Entity::find_by_id(id.to_owned())
            .filter(members::Column::WorkspaceId.eq("workspace-local"))
            .filter(members::Column::Status.eq("active"))
            .select_only()
            .column(members::Column::OrgRole)
            .into_tuple::<String>()
            .one(self.db)
    }
    pub fn has_permission(&self, member_id: &str, permission: &str) -> Result<bool, DbErr> {
        member_permissions::Entity::find_by_id((member_id.to_owned(), permission.to_owned()))
            .one(self.db)
            .map(|v| v.is_some())
    }
    pub fn has_team_role(&self, member_id: &str, role: &str) -> Result<bool, DbErr> {
        team_members::Entity::find()
            .filter(team_members::Column::MemberId.eq(member_id.to_owned()))
            .filter(team_members::Column::TeamRole.eq(role.to_owned()))
            .one(self.db)
            .map(|v| v.is_some())
    }
    pub fn save_team(&self, row: teams::ActiveModel) -> Result<(), DbErr> {
        teams::Entity::insert(row)
            .on_conflict(
                OnConflict::column(teams::Column::Id)
                    .update_columns([
                        teams::Column::Name,
                        teams::Column::Currency,
                        teams::Column::Timezone,
                        teams::Column::RateStrategy,
                        teams::Column::AutoApprove,
                        teams::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(self.db)
            .map(|_| ())
    }
    pub fn save_department(&self, row: departments::ActiveModel) -> Result<(), DbErr> {
        departments::Entity::insert(row)
            .on_conflict(
                OnConflict::column(departments::Column::Id)
                    .update_columns([departments::Column::Name, departments::Column::UpdatedAt])
                    .to_owned(),
            )
            .exec(self.db)
            .map(|_| ())
    }
    pub fn save_member(
        &self,
        id: &str,
        row: members::ActiveModel,
        teams_for_member: Vec<team_members::ActiveModel>,
        permissions_for_member: Vec<member_permissions::ActiveModel>,
    ) -> Result<(), DbErr> {
        let tx = self.db.begin()?;
        members::Entity::insert(row)
            .on_conflict(
                OnConflict::column(members::Column::Id)
                    .update_columns([
                        members::Column::Email,
                        members::Column::DisplayName,
                        members::Column::OrgRole,
                        members::Column::JobTitle,
                        members::Column::ManagerId,
                        members::Column::DefaultTeamId,
                        members::Column::DepartmentId,
                        members::Column::BillRateCents,
                        members::Column::CostRateCents,
                        members::Column::BillableDefault,
                        members::Column::Status,
                        members::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&tx)?;
        team_members::Entity::delete_many()
            .filter(team_members::Column::MemberId.eq(id.to_owned()))
            .exec(&tx)?;
        member_permissions::Entity::delete_many()
            .filter(member_permissions::Column::MemberId.eq(id.to_owned()))
            .exec(&tx)?;
        if !teams_for_member.is_empty() {
            team_members::Entity::insert_many(teams_for_member).exec(&tx)?;
        }
        if !permissions_for_member.is_empty() {
            member_permissions::Entity::insert_many(permissions_for_member).exec(&tx)?;
        }
        tx.commit()
    }
    pub fn save_defaults(&self, text: &str, apply_to_all: bool, now: i64) -> Result<(), DbErr> {
        let tx = self.db.begin()?;
        workspaces::Entity::update_many()
            .col_expr(
                workspaces::Column::DefaultsJson,
                Expr::value(text.to_owned()),
            )
            .col_expr(workspaces::Column::UpdatedAt, Expr::value(now))
            .filter(workspaces::Column::Id.eq("workspace-local"))
            .exec(&tx)?;
        if apply_to_all {
            members::Entity::update_many()
                .col_expr(
                    members::Column::EffectiveDefaultsJson,
                    Expr::value(text.to_owned()),
                )
                .col_expr(members::Column::UpdatedAt, Expr::value(now))
                .filter(members::Column::WorkspaceId.eq("workspace-local"))
                .filter(members::Column::Status.eq("active"))
                .exec(&tx)?;
        }
        tx.commit()
    }
    pub fn add_audit(&self, row: audit::ActiveModel) -> Result<(), DbErr> {
        row.insert(self.db).map(|_| ())
    }
}
