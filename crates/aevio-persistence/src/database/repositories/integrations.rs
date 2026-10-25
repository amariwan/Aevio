use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, QueryOrder, Set};

use crate::database::entities::integrations;
use aevio_core::integrations::{IntegrationConnection, Provider, SyncDirection};

pub struct IntegrationRepository<'db> {
    db: &'db DatabaseConnection,
}

impl<'db> IntegrationRepository<'db> {
    pub fn new(db: &'db DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn list(&self) -> Result<Vec<IntegrationConnection>, DbErr> {
        integrations::Entity::find()
            .order_by_asc(integrations::Column::Name)
            .all(self.db)
            .map(|rows| rows.into_iter().map(to_domain).collect())
    }

    pub fn upsert(
        &self,
        connection: IntegrationConnection,
    ) -> Result<IntegrationConnection, DbErr> {
        if let Some(row) = integrations::Entity::find_by_id(connection.id.clone()).one(self.db)? {
            let mut model: integrations::ActiveModel = row.into();
            model.provider = Set(connection.provider.as_str().to_owned());
            model.name = Set(connection.name);
            model.workspace_ref = Set(connection.workspace_ref);
            model.secret_ref = Set(connection.secret_ref);
            model.direction = Set(direction_str(connection.direction));
            model.enabled = Set(connection.enabled as i64);
            model.last_synced_at = Set(connection.last_synced_at.map(|value| value as i64));
            model.updated_at = Set(connection.updated_at as i64);
            model.update(self.db).map(to_domain)
        } else {
            let id = connection.id.clone();
            integrations::ActiveModel {
                id: Set(connection.id),
                provider: Set(connection.provider.as_str().to_owned()),
                name: Set(connection.name),
                workspace_ref: Set(connection.workspace_ref),
                secret_ref: Set(connection.secret_ref),
                direction: Set(direction_str(connection.direction)),
                enabled: Set(connection.enabled as i64),
                last_synced_at: Set(connection.last_synced_at.map(|value| value as i64)),
                created_at: Set(connection.created_at as i64),
                updated_at: Set(connection.updated_at as i64),
            }
            .insert(self.db)
            .map(|_| ())?;
            integrations::Entity::find_by_id(id)
                .one(self.db)?
                .map(to_domain)
                .ok_or_else(|| DbErr::RecordNotFound("integration".into()))
        }
    }
}

fn to_domain(row: integrations::Model) -> IntegrationConnection {
    IntegrationConnection {
        id: row.id,
        provider: parse_provider(&row.provider),
        name: row.name,
        workspace_ref: row.workspace_ref,
        secret_ref: row.secret_ref,
        direction: parse_direction(&row.direction),
        enabled: row.enabled != 0,
        last_synced_at: row.last_synced_at.map(|value| value.max(0) as u64),
        created_at: row.created_at.max(0) as u64,
        updated_at: row.updated_at.max(0) as u64,
    }
}

fn parse_provider(value: &str) -> Provider {
    match value {
        "linear" => Provider::Linear,
        "asana" => Provider::Asana,
        _ => Provider::ClickUp,
    }
}

fn parse_direction(value: &str) -> SyncDirection {
    match value {
        "pull" => SyncDirection::Pull,
        "push" => SyncDirection::Push,
        _ => SyncDirection::Bidirectional,
    }
}

fn direction_str(value: SyncDirection) -> String {
    match value {
        SyncDirection::Pull => "pull".to_owned(),
        SyncDirection::Push => "push".to_owned(),
        SyncDirection::Bidirectional => "bidirectional".to_owned(),
    }
}
