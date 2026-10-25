use aevio_contracts::plugin::{PluginLifecycleState, capabilities};
use aevio_persistence::database::repositories::plugins::PluginRepository;
use sea_orm::DatabaseConnection;

use super::plugins::{
    CapabilityGateway, PluginServiceError, audit, effective_permissions, parse_state,
};

const MAX_TITLE_CHARS: usize = 200;
const MAX_BODY_CHARS: usize = 2_000;

pub fn show(
    db: &DatabaseConnection,
    plugin_id: &str,
    title: &str,
    body: &str,
    now: i64,
) -> Result<(), PluginServiceError> {
    validate_text(title, MAX_TITLE_CHARS, "title")?;
    validate_text(body, MAX_BODY_CHARS, "body")?;
    let repository = PluginRepository::new(db);
    let installation = repository
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    if parse_state(&installation.state)? != PluginLifecycleState::Active {
        return Err(PluginServiceError::InvalidState(
            "plugin must be active".into(),
        ));
    }
    CapabilityGateway::new(plugin_id, effective_permissions(db, &installation, now)?)
        .require(capabilities::NOTIFICATIONS_SHOW)?;
    audit(
        &repository,
        &installation.id,
        "notification_requested",
        now,
        plugin_id,
    )?;
    Ok(())
}

fn validate_text(value: &str, maximum: usize, field: &str) -> Result<(), PluginServiceError> {
    if value.trim().is_empty()
        || value.chars().count() > maximum
        || value.chars().any(char::is_control)
    {
        return Err(PluginServiceError::Permission(format!(
            "plugin notification {field} is invalid"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_text;

    #[test]
    fn notification_text_is_bounded_and_non_control() {
        assert!(validate_text("Title", 10, "title").is_ok());
        assert!(validate_text("", 10, "title").is_err());
        assert!(validate_text("bad\ntext", 20, "body").is_err());
        assert!(validate_text("123456", 5, "body").is_err());
    }
}
