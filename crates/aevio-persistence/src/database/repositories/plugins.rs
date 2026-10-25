use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait, QueryFilter,
    QueryOrder, TransactionTrait,
};

use crate::database::entities::plugins::{
    audit_events, event_deliveries, events, grants, installation_history, installations, jobs,
    layout_preferences, operations, storage_entries, update_policies, workflow_runs,
};

pub struct PluginRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> PluginRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn list_installations(&self) -> Result<Vec<installations::Model>, DbErr> {
        installations::Entity::find()
            .order_by_asc(installations::Column::PluginId)
            .all(self.db)
    }

    pub fn installation(&self, plugin_id: &str) -> Result<Option<installations::Model>, DbErr> {
        installations::Entity::find()
            .filter(installations::Column::PluginId.eq(plugin_id.to_owned()))
            .one(self.db)
    }

    pub fn installation_by_id(&self, id: &str) -> Result<Option<installations::Model>, DbErr> {
        installations::Entity::find_by_id(id.to_owned()).one(self.db)
    }

    pub fn insert_installation(&self, model: installations::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn update_installation(&self, model: installations::ActiveModel) -> Result<(), DbErr> {
        model.update(self.db).map(|_| ())
    }

    pub fn delete_installation(&self, plugin_id: &str) -> Result<(), DbErr> {
        let Some(row) = self.installation(plugin_id)? else {
            return Ok(());
        };
        installations::Entity::delete_by_id(row.id)
            .exec(self.db)
            .map(|_| ())
    }

    pub fn delete_plugin_data(&self, installation_id: &str, plugin_id: &str) -> Result<(), DbErr> {
        let transaction = self.db.begin()?;
        event_deliveries::Entity::delete_many()
            .filter(
                event_deliveries::Column::SubscriberInstallationId.eq(installation_id.to_owned()),
            )
            .exec(&transaction)?;
        events::Entity::delete_many()
            .filter(events::Column::PublisherInstallationId.eq(installation_id.to_owned()))
            .exec(&transaction)?;
        storage_entries::Entity::delete_many()
            .filter(storage_entries::Column::InstallationId.eq(installation_id.to_owned()))
            .exec(&transaction)?;
        grants::Entity::delete_many()
            .filter(grants::Column::InstallationId.eq(installation_id.to_owned()))
            .exec(&transaction)?;
        jobs::Entity::delete_many()
            .filter(jobs::Column::InstallationId.eq(installation_id.to_owned()))
            .exec(&transaction)?;
        workflow_runs::Entity::delete_many()
            .filter(workflow_runs::Column::InstallationId.eq(installation_id.to_owned()))
            .exec(&transaction)?;
        layout_preferences::Entity::delete_many()
            .filter(layout_preferences::Column::InstallationId.eq(installation_id.to_owned()))
            .exec(&transaction)?;
        crate::database::entities::plugins::runtime_health::Entity::delete_by_id(
            installation_id.to_owned(),
        )
        .exec(&transaction)?;
        installation_history::Entity::delete_many()
            .filter(installation_history::Column::PluginId.eq(plugin_id.to_owned()))
            .exec(&transaction)?;
        update_policies::Entity::delete_by_id(plugin_id.to_owned()).exec(&transaction)?;
        transaction.commit()
    }

    pub fn history_for_plugin(
        &self,
        plugin_id: &str,
    ) -> Result<Vec<installation_history::Model>, DbErr> {
        installation_history::Entity::find()
            .filter(installation_history::Column::PluginId.eq(plugin_id.to_owned()))
            .all(self.db)
    }

    pub fn delete_history_for_plugin(&self, plugin_id: &str) -> Result<(), DbErr> {
        installation_history::Entity::delete_many()
            .filter(installation_history::Column::PluginId.eq(plugin_id.to_owned()))
            .exec(self.db)
            .map(|_| ())
    }

    pub fn insert_operation(&self, model: operations::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn insert_audit_event(&self, model: audit_events::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn list_grants(&self, installation_id: &str) -> Result<Vec<grants::Model>, DbErr> {
        grants::Entity::find()
            .filter(grants::Column::InstallationId.eq(installation_id.to_owned()))
            .all(self.db)
    }

    pub fn upsert_persistent_grant(
        &self,
        installation_id: &str,
        capability: &str,
        now: i64,
    ) -> Result<(), DbErr> {
        let existing = grants::Entity::find()
            .filter(grants::Column::InstallationId.eq(installation_id.to_owned()))
            .filter(grants::Column::Capability.eq(capability.to_owned()))
            .filter(grants::Column::Scope.is_null())
            .one(self.db)?;
        if let Some(existing) = existing {
            let mut model: grants::ActiveModel = existing.into();
            model.revoked_at = sea_orm::ActiveValue::Set(None);
            model.expires_at = sea_orm::ActiveValue::Set(None);
            model.created_at = sea_orm::ActiveValue::Set(now);
            model.update(self.db).map(|_| ())
        } else {
            grants::ActiveModel {
                id: sea_orm::ActiveValue::Set(uuid::Uuid::now_v7().to_string()),
                installation_id: sea_orm::ActiveValue::Set(installation_id.into()),
                capability: sea_orm::ActiveValue::Set(capability.into()),
                scope: sea_orm::ActiveValue::Set(None),
                expires_at: sea_orm::ActiveValue::Set(None),
                revoked_at: sea_orm::ActiveValue::Set(None),
                created_at: sea_orm::ActiveValue::Set(now),
            }
            .insert(self.db)
            .map(|_| ())
        }
    }

    pub fn revoke_grant(
        &self,
        installation_id: &str,
        capability: &str,
        now: i64,
    ) -> Result<(), DbErr> {
        grants::Entity::update_many()
            .col_expr(grants::Column::RevokedAt, Expr::value(Some(now)))
            .filter(grants::Column::InstallationId.eq(installation_id.to_owned()))
            .filter(grants::Column::Capability.eq(capability.to_owned()))
            .filter(grants::Column::RevokedAt.is_null())
            .exec(self.db)
            .map(|_| ())
    }

    pub fn storage_entry(
        &self,
        installation_id: &str,
        user_ref: &str,
        workspace_ref: &str,
        device_ref: &str,
        key: &str,
    ) -> Result<Option<storage_entries::Model>, DbErr> {
        storage_entries::Entity::find()
            .filter(storage_entries::Column::InstallationId.eq(installation_id.to_owned()))
            .filter(storage_entries::Column::UserRef.eq(user_ref.to_owned()))
            .filter(storage_entries::Column::WorkspaceRef.eq(workspace_ref.to_owned()))
            .filter(storage_entries::Column::DeviceRef.eq(device_ref.to_owned()))
            .filter(storage_entries::Column::Key.eq(key.to_owned()))
            .one(self.db)
    }

    pub fn insert_storage_entry(&self, model: storage_entries::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn update_storage_entry(&self, model: storage_entries::ActiveModel) -> Result<(), DbErr> {
        model.update(self.db).map(|_| ())
    }

    pub fn delete_storage_entry(&self, id: &str) -> Result<(), DbErr> {
        storage_entries::Entity::delete_by_id(id)
            .exec(self.db)
            .map(|_| ())
    }

    pub fn insert_history(&self, model: installation_history::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn latest_history(
        &self,
        plugin_id: &str,
    ) -> Result<Option<installation_history::Model>, DbErr> {
        installation_history::Entity::find()
            .filter(installation_history::Column::PluginId.eq(plugin_id.to_owned()))
            .order_by_desc(installation_history::Column::RecordedAt)
            .one(self.db)
    }

    pub fn delete_history(&self, id: &str) -> Result<(), DbErr> {
        installation_history::Entity::delete_by_id(id)
            .exec(self.db)
            .map(|_| ())
    }

    pub fn insert_job(&self, model: jobs::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn job(&self, installation_id: &str, job_id: &str) -> Result<Option<jobs::Model>, DbErr> {
        jobs::Entity::find()
            .filter(jobs::Column::InstallationId.eq(installation_id.to_owned()))
            .filter(jobs::Column::JobId.eq(job_id.to_owned()))
            .one(self.db)
    }

    pub fn job_by_id(&self, id: &str) -> Result<Option<jobs::Model>, DbErr> {
        jobs::Entity::find_by_id(id.to_owned()).one(self.db)
    }

    pub fn due_jobs(&self, now: i64) -> Result<Vec<jobs::Model>, DbErr> {
        jobs::Entity::find()
            .filter(jobs::Column::Enabled.eq(1))
            .filter(jobs::Column::State.eq("scheduled"))
            .filter(jobs::Column::NextRunAt.lte(now))
            .filter(
                Condition::any()
                    .add(jobs::Column::LeaseUntil.is_null())
                    .add(jobs::Column::LeaseUntil.lte(now)),
            )
            .order_by_asc(jobs::Column::NextRunAt)
            .all(self.db)
    }

    pub fn claim_job(
        &self,
        id: &str,
        now: i64,
        lease_until: i64,
    ) -> Result<Option<jobs::Model>, DbErr> {
        let result = jobs::Entity::update_many()
            .col_expr(jobs::Column::State, Expr::value("leased"))
            .col_expr(jobs::Column::LeaseUntil, Expr::value(Some(lease_until)))
            .col_expr(jobs::Column::UpdatedAt, Expr::value(now))
            .filter(jobs::Column::Id.eq(id.to_owned()))
            .filter(jobs::Column::Enabled.eq(1))
            .filter(jobs::Column::State.eq("scheduled"))
            .filter(jobs::Column::NextRunAt.lte(now))
            .filter(
                Condition::any()
                    .add(jobs::Column::LeaseUntil.is_null())
                    .add(jobs::Column::LeaseUntil.lte(now)),
            )
            .exec(self.db)?;
        if result.rows_affected == 0 {
            return Ok(None);
        }
        self.job_by_id(id)
    }

    pub fn update_job(&self, model: jobs::ActiveModel) -> Result<(), DbErr> {
        model.update(self.db).map(|_| ())
    }

    pub fn workflow_run(
        &self,
        installation_id: &str,
        workflow_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<workflow_runs::Model>, DbErr> {
        workflow_runs::Entity::find()
            .filter(workflow_runs::Column::InstallationId.eq(installation_id.to_owned()))
            .filter(workflow_runs::Column::WorkflowId.eq(workflow_id.to_owned()))
            .filter(workflow_runs::Column::IdempotencyKey.eq(idempotency_key.to_owned()))
            .one(self.db)
    }

    pub fn workflow_runs(&self, installation_id: &str) -> Result<Vec<workflow_runs::Model>, DbErr> {
        workflow_runs::Entity::find()
            .filter(workflow_runs::Column::InstallationId.eq(installation_id.to_owned()))
            .order_by_desc(workflow_runs::Column::CreatedAt)
            .all(self.db)
    }

    pub fn insert_workflow_run(&self, model: workflow_runs::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn update_workflow_run(&self, model: workflow_runs::ActiveModel) -> Result<(), DbErr> {
        model.update(self.db).map(|_| ())
    }

    pub fn insert_event(&self, model: events::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn event(&self, id: &str) -> Result<Option<events::Model>, DbErr> {
        events::Entity::find_by_id(id.to_owned()).one(self.db)
    }

    pub fn insert_event_delivery(&self, model: event_deliveries::ActiveModel) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn event_delivery(&self, id: &str) -> Result<Option<event_deliveries::Model>, DbErr> {
        event_deliveries::Entity::find_by_id(id.to_owned()).one(self.db)
    }

    pub fn due_event_deliveries(&self, now: i64) -> Result<Vec<event_deliveries::Model>, DbErr> {
        event_deliveries::Entity::find()
            .filter(
                Condition::any()
                    .add(event_deliveries::Column::Status.eq("pending"))
                    .add(event_deliveries::Column::Status.eq("running")),
            )
            .filter(
                Condition::any()
                    .add(event_deliveries::Column::LeaseUntil.is_null())
                    .add(event_deliveries::Column::LeaseUntil.lte(now)),
            )
            .order_by_asc(event_deliveries::Column::CreatedAt)
            .all(self.db)
    }

    pub fn claim_event_delivery(
        &self,
        id: &str,
        now: i64,
        lease_until: i64,
    ) -> Result<Option<event_deliveries::Model>, DbErr> {
        let result = event_deliveries::Entity::update_many()
            .col_expr(event_deliveries::Column::Status, Expr::value("running"))
            .col_expr(
                event_deliveries::Column::LeaseUntil,
                Expr::value(Some(lease_until)),
            )
            .col_expr(event_deliveries::Column::UpdatedAt, Expr::value(now))
            .filter(event_deliveries::Column::Id.eq(id.to_owned()))
            .filter(
                Condition::any()
                    .add(event_deliveries::Column::Status.eq("pending"))
                    .add(event_deliveries::Column::Status.eq("running")),
            )
            .filter(
                Condition::any()
                    .add(event_deliveries::Column::LeaseUntil.is_null())
                    .add(event_deliveries::Column::LeaseUntil.lte(now)),
            )
            .exec(self.db)?;
        if result.rows_affected == 0 {
            return Ok(None);
        }
        self.event_delivery(id)
    }

    pub fn update_event_delivery(&self, model: event_deliveries::ActiveModel) -> Result<(), DbErr> {
        model.update(self.db).map(|_| ())
    }

    pub fn update_policy(&self, plugin_id: &str) -> Result<Option<update_policies::Model>, DbErr> {
        update_policies::Entity::find_by_id(plugin_id.to_owned()).one(self.db)
    }

    pub fn upsert_update_policy(
        &self,
        plugin_id: &str,
        model: update_policies::ActiveModel,
    ) -> Result<(), DbErr> {
        if self.update_policy(plugin_id)?.is_some() {
            model.update(self.db).map(|_| ())
        } else {
            model.insert(self.db).map(|_| ())
        }
    }

    pub fn layout_preferences(&self) -> Result<Vec<layout_preferences::Model>, DbErr> {
        layout_preferences::Entity::find()
            .order_by_asc(layout_preferences::Column::OrderIndex)
            .order_by_desc(layout_preferences::Column::Pinned)
            .all(self.db)
    }

    pub fn layout_preference(
        &self,
        installation_id: &str,
        contribution_id: &str,
    ) -> Result<Option<layout_preferences::Model>, DbErr> {
        layout_preferences::Entity::find()
            .filter(layout_preferences::Column::InstallationId.eq(installation_id.to_owned()))
            .filter(layout_preferences::Column::ContributionId.eq(contribution_id.to_owned()))
            .one(self.db)
    }

    pub fn insert_layout_preference(
        &self,
        model: layout_preferences::ActiveModel,
    ) -> Result<(), DbErr> {
        model.insert(self.db).map(|_| ())
    }

    pub fn update_layout_preference(
        &self,
        model: layout_preferences::ActiveModel,
    ) -> Result<(), DbErr> {
        model.update(self.db).map(|_| ())
    }

    pub fn delete_layout_preferences(&self) -> Result<(), DbErr> {
        layout_preferences::Entity::delete_many()
            .exec(self.db)
            .map(|_| ())
    }

    pub fn runtime_health(
        &self,
        installation_id: &str,
    ) -> Result<Option<crate::database::entities::plugins::runtime_health::Model>, DbErr> {
        crate::database::entities::plugins::runtime_health::Entity::find_by_id(
            installation_id.to_owned(),
        )
        .one(self.db)
    }

    pub fn all_runtime_health(
        &self,
    ) -> Result<Vec<crate::database::entities::plugins::runtime_health::Model>, DbErr> {
        crate::database::entities::plugins::runtime_health::Entity::find().all(self.db)
    }

    pub fn upsert_runtime_health(
        &self,
        installation_id: &str,
        model: crate::database::entities::plugins::runtime_health::ActiveModel,
    ) -> Result<(), DbErr> {
        if self.runtime_health(installation_id)?.is_some() {
            model.update(self.db).map(|_| ())
        } else {
            model.insert(self.db).map(|_| ())
        }
    }
}
