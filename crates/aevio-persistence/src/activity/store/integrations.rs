use super::*;

impl ActivityStore {
    pub fn list_integrations(&self) -> Result<Vec<IntegrationConnection>, String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            return crate::database::repositories::integrations::IntegrationRepository::new(
                &database,
            )
            .list()
            .map_err(|error| error.to_string());
        }
        #[cfg(not(test))]
        return Err("database path unavailable for integrations read".into());
        #[cfg(test)]
        {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT id, provider, name, workspace_ref, secret_ref, direction, enabled,
                    last_synced_at, created_at, updated_at FROM integrations
             ORDER BY name COLLATE NOCASE;",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(IntegrationConnection {
                        id: row.get(0)?,
                        provider: parse_provider(&row.get::<_, String>(1)?),
                        name: row.get(2)?,
                        workspace_ref: row.get(3)?,
                        secret_ref: row.get(4)?,
                        direction: parse_direction(&row.get::<_, String>(5)?),
                        enabled: row.get::<_, i64>(6)? != 0,
                        last_synced_at: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
                        created_at: row.get::<_, i64>(8)? as u64,
                        updated_at: row.get::<_, i64>(9)? as u64,
                    })
                })
                .map_err(|e| e.to_string())?;
            rows.map(|row| row.map_err(|e| e.to_string())).collect()
        }
    }

    pub fn upsert_integration(
        &mut self,
        connection: IntegrationConnection,
    ) -> Result<IntegrationConnection, String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            return crate::database::repositories::integrations::IntegrationRepository::new(
                &database,
            )
            .upsert(connection)
            .map_err(|error| error.to_string());
        }
        #[cfg(not(test))]
        return Err("database path unavailable for integration write".into());
        #[cfg(test)]
        {
            let id = connection.id.clone();
            self.conn.execute(
            "INSERT INTO integrations (id, provider, name, workspace_ref, secret_ref, direction, enabled, last_synced_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET name = excluded.name, secret_ref = excluded.secret_ref,
               direction = excluded.direction, enabled = excluded.enabled, updated_at = excluded.updated_at;",
            params![connection.id, connection.provider.as_str(), connection.name, connection.workspace_ref,
                connection.secret_ref, direction_str(connection.direction), connection.enabled as i64,
                connection.last_synced_at.map(|v| v as i64), connection.created_at as i64, connection.updated_at as i64],
        ).map_err(|e| e.to_string())?;
            self.list_integrations()?
                .into_iter()
                .find(|item| item.id == id)
                .ok_or_else(|| "Integration was not persisted.".to_string())
        }
    }
}

#[cfg(test)]
fn parse_provider(value: &str) -> Provider {
    match value {
        "linear" => Provider::Linear,
        "asana" => Provider::Asana,
        _ => Provider::ClickUp,
    }
}

#[cfg(test)]
fn parse_direction(value: &str) -> SyncDirection {
    match value {
        "pull" => SyncDirection::Pull,
        "push" => SyncDirection::Push,
        _ => SyncDirection::Bidirectional,
    }
}

#[cfg(test)]
fn direction_str(value: SyncDirection) -> &'static str {
    match value {
        SyncDirection::Pull => "pull",
        SyncDirection::Push => "push",
        SyncDirection::Bidirectional => "bidirectional",
    }
}
