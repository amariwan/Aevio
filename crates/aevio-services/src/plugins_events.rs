//! Durable, capability-checked plugin event publication and workflow delivery.
//!
//! Tauri events remain the low-latency observation path for active UI hosts.
//! This module is the durable path: an event is recorded before it is exposed
//! to subscribers, and workflow deliveries are leased and retried separately
//! so a crashed desktop process cannot lose an event-triggered action.

use aevio_contracts::plugin::{
    EventSubscription, PluginEventPublicationSummary, PluginLifecycleState, PluginWorkflowRequest,
    capabilities,
};
use aevio_persistence::database::entities::plugins::{event_deliveries, events};
use aevio_persistence::database::repositories::plugins::PluginRepository;
use sea_orm::{ActiveValue::Set, DatabaseConnection};
use serde_json::Value;
use uuid::Uuid;

use super::plugins::{CapabilityGateway, PluginServiceError, audit, parse_state};
use super::plugins_host::PluginRuntimeEffect;

const MAX_EVENT_PAYLOAD_BYTES: usize = 64 * 1024;
const MAX_DELIVERIES_PER_SWEEP: usize = 32;
const DELIVERY_LEASE_MS: i64 = 5 * 60 * 1000;
const MAX_RETRY_DELAY_MS: i64 = 60 * 60 * 1000;
const MAX_ATTEMPTS: i64 = 3;

#[derive(Debug, Clone)]
pub struct EventPublication {
    pub event_name: String,
    pub summary: PluginEventPublicationSummary,
}

#[derive(Debug, Default)]
pub struct EventDispatch {
    pub effects: Vec<PluginRuntimeEffect>,
    pub delivered: usize,
    pub failed: usize,
    pub dead_lettered: usize,
}

pub fn publish(
    db: &DatabaseConnection,
    plugin_id: &str,
    event_type: &str,
    payload: &Value,
    now: i64,
) -> Result<EventPublication, PluginServiceError> {
    validate_event_type(event_type)?;
    let repository = PluginRepository::new(db);
    let publisher = repository
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    if parse_state(&publisher.state)? != PluginLifecycleState::Active {
        return Err(PluginServiceError::InvalidState(
            "plugin must be active".into(),
        ));
    }
    let grants = super::plugins::effective_permissions(db, &publisher, now)?;
    CapabilityGateway::new(plugin_id, grants).require(capabilities::EVENTS_PUBLISH)?;
    let payload_json = serde_json::to_string(payload)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    if payload_json.len() > MAX_EVENT_PAYLOAD_BYTES {
        return Err(PluginServiceError::Permission(
            "plugin event payload is too large".into(),
        ));
    }

    let event_id = Uuid::now_v7().to_string();
    repository.insert_event(events::ActiveModel {
        id: Set(event_id.clone()),
        publisher_installation_id: Set(publisher.id.clone()),
        publisher_plugin_id: Set(plugin_id.into()),
        event_type: Set(event_type.into()),
        payload_json: Set(payload_json),
        occurred_at: Set(now),
    })?;

    let mut delivery_count = 0usize;
    for subscriber in repository.list_installations()? {
        if parse_state(&subscriber.state)? != PluginLifecycleState::Active {
            continue;
        }
        let manifest = match serde_json::from_str::<aevio_contracts::plugin::PluginManifest>(
            &subscriber.manifest_json,
        ) {
            Ok(manifest) => manifest,
            Err(_) => continue,
        };
        for subscription in manifest
            .event_subscriptions
            .iter()
            .filter(|subscription| subscription_matches(subscription, event_type))
        {
            if !subscription_is_authorized(db, &subscriber, subscription)? {
                continue;
            }
            let Some(workflow_id) = subscription.workflow_id.as_deref() else {
                continue;
            };
            let delivery = event_deliveries::ActiveModel {
                id: Set(Uuid::now_v7().to_string()),
                event_id: Set(event_id.clone()),
                subscriber_installation_id: Set(subscriber.id.clone()),
                workflow_id: Set(workflow_id.into()),
                status: Set("pending".into()),
                attempt_count: Set(0),
                max_attempts: Set(MAX_ATTEMPTS),
                lease_until: Set(None),
                last_error: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                completed_at: Set(None),
            };
            repository.insert_event_delivery(delivery)?;
            delivery_count += 1;
        }
    }
    audit(
        &repository,
        &publisher.id,
        "event_published",
        now,
        plugin_id,
    )?;
    Ok(EventPublication {
        event_name: format!("plugin-event:{plugin_id}:{event_type}"),
        summary: PluginEventPublicationSummary {
            event_id,
            event_type: event_type.into(),
            delivery_count,
            occurred_at: now,
        },
    })
}

pub fn dispatch_due(
    db: &DatabaseConnection,
    now: i64,
    device_ref: &str,
) -> Result<EventDispatch, PluginServiceError> {
    let repository = PluginRepository::new(db);
    let mut dispatch = EventDispatch::default();
    for candidate in repository
        .due_event_deliveries(now)?
        .into_iter()
        .take(MAX_DELIVERIES_PER_SWEEP)
    {
        let Some(delivery) = repository.claim_event_delivery(
            &candidate.id,
            now,
            now.saturating_add(DELIVERY_LEASE_MS),
        )?
        else {
            continue;
        };
        let Some(event) = repository.event(&delivery.event_id)? else {
            mark_failure(&repository, delivery, now, "event record is missing")?;
            dispatch.failed += 1;
            continue;
        };
        let Some(subscriber) =
            repository.installation_by_id(&delivery.subscriber_installation_id)?
        else {
            mark_failure(
                &repository,
                delivery,
                now,
                "subscriber installation is missing",
            )?;
            dispatch.failed += 1;
            continue;
        };
        let input = serde_json::from_str::<Value>(&event.payload_json)
            .map_err(|error| PluginServiceError::Serialization(error.to_string()));
        let result = input.and_then(|input| {
            super::plugins_workflows::run(
                db,
                PluginWorkflowRequest {
                    plugin_id: subscriber.plugin_id.clone(),
                    workflow_id: delivery.workflow_id.clone(),
                    trigger: format!("event:{}", event.event_type),
                    idempotency_key: format!("event:{}:delivery:{}", event.id, delivery.id),
                    input,
                },
                now,
                device_ref,
            )
        });
        match result {
            Ok((_summary, effects)) => {
                mark_success(&repository, delivery, now)?;
                dispatch.effects.extend(effects);
                dispatch.delivered += 1;
            }
            Err(error) => {
                let exhausted = delivery.attempt_count + 1 >= delivery.max_attempts.max(1);
                mark_failure(&repository, delivery, now, &error.to_string())?;
                if exhausted {
                    dispatch.dead_lettered += 1;
                } else {
                    dispatch.failed += 1;
                }
            }
        }
    }
    Ok(dispatch)
}

pub fn authorize_subscription(
    db: &DatabaseConnection,
    plugin_id: &str,
    event_type: &str,
) -> Result<(), PluginServiceError> {
    validate_event_type(event_type)?;
    let repository = PluginRepository::new(db);
    let installation = repository
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    if parse_state(&installation.state)? != PluginLifecycleState::Active {
        return Err(PluginServiceError::InvalidState(
            "plugin must be active".into(),
        ));
    }
    let manifest = serde_json::from_str::<aevio_contracts::plugin::PluginManifest>(
        &installation.manifest_json,
    )
    .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    let grants = super::plugins::effective_permissions(
        db,
        &installation,
        aevio_core::timers::now_epoch_ms() as i64,
    )?;
    CapabilityGateway::new(plugin_id, grants.clone()).require(capabilities::EVENTS_SUBSCRIBE)?;
    let subscription = manifest
        .event_subscriptions
        .iter()
        .find(|subscription| subscription_matches(subscription, event_type))
        .ok_or_else(|| {
            PluginServiceError::Permission("event is not declared by the plugin manifest".into())
        })?;
    for capability in &subscription.required_capabilities {
        CapabilityGateway::new(plugin_id, grants.clone()).require(capability)?;
    }
    Ok(())
}

pub fn validate_event_type(event_type: &str) -> Result<(), PluginServiceError> {
    if event_type.is_empty()
        || event_type.len() > 128
        || !event_type.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '-' | '_' | '/')
        })
    {
        return Err(PluginServiceError::Permission(
            "invalid plugin event type".into(),
        ));
    }
    Ok(())
}

fn subscription_matches(subscription: &EventSubscription, event_type: &str) -> bool {
    subscription
        .event_type
        .as_deref()
        .unwrap_or(&subscription.id)
        == event_type
}

fn subscription_is_authorized(
    db: &DatabaseConnection,
    installation: &aevio_persistence::database::entities::plugins::installations::Model,
    subscription: &EventSubscription,
) -> Result<bool, PluginServiceError> {
    let grants = super::plugins::effective_permissions(
        db,
        installation,
        aevio_core::timers::now_epoch_ms() as i64,
    )?;
    if !CapabilityGateway::new(&installation.plugin_id, grants.clone())
        .require(capabilities::EVENTS_SUBSCRIBE)
        .is_ok()
    {
        return Ok(false);
    }
    Ok(subscription
        .required_capabilities
        .iter()
        .all(|capability| grants.iter().any(|grant| grant == capability)))
}

fn mark_success(
    repository: &PluginRepository<'_>,
    delivery: event_deliveries::Model,
    now: i64,
) -> Result<(), PluginServiceError> {
    let mut model: event_deliveries::ActiveModel = delivery.clone().into();
    model.status = Set("succeeded".into());
    model.attempt_count = Set(delivery.attempt_count.saturating_add(1));
    model.lease_until = Set(None);
    model.updated_at = Set(now);
    model.completed_at = Set(Some(now));
    repository.update_event_delivery(model)?;
    Ok(())
}

fn mark_failure(
    repository: &PluginRepository<'_>,
    delivery: event_deliveries::Model,
    now: i64,
    error: &str,
) -> Result<(), PluginServiceError> {
    let attempt_count = delivery.attempt_count.saturating_add(1);
    let exhausted = attempt_count >= delivery.max_attempts.max(1);
    let retry_delay = (5_000_i64
        .saturating_mul(1_i64 << attempt_count.saturating_sub(1).min(6) as u32))
    .min(MAX_RETRY_DELAY_MS);
    let mut model: event_deliveries::ActiveModel = delivery.into();
    model.status = Set(if exhausted { "dead_letter" } else { "pending" }.into());
    model.attempt_count = Set(attempt_count);
    model.lease_until = Set((!exhausted).then_some(now.saturating_add(retry_delay)));
    model.last_error = Set(Some(error.chars().take(2_000).collect()));
    model.updated_at = Set(now);
    model.completed_at = Set(exhausted.then_some(now));
    repository.update_event_delivery(model)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{dispatch_due, publish, validate_event_type};
    use aevio_contracts::plugin::{WorkflowContribution, capabilities};
    use aevio_persistence::bootstrap::migrations::run_migrations;
    use aevio_persistence::database::connect;
    use aevio_persistence::database::entities::plugins::installations;
    use aevio_persistence::database::repositories::plugins::PluginRepository;
    use rusqlite::Connection;
    use sea_orm::ActiveValue::Set;
    use serde_json::json;
    use tempfile::tempdir;

    fn database() -> (tempfile::TempDir, sea_orm::DatabaseConnection) {
        let directory = tempdir().unwrap();
        let path = directory.path().join("activity.db");
        let mut connection = Connection::open(&path).unwrap();
        run_migrations(&mut connection).unwrap();
        drop(connection);
        (directory, connect(&path).unwrap())
    }

    fn insert_active(
        db: &sea_orm::DatabaseConnection,
        plugin_id: &str,
        mut manifest: aevio_contracts::plugin::PluginManifest,
        permissions: &[&str],
        now: i64,
    ) {
        manifest.id = plugin_id.into();
        let repository = PluginRepository::new(db);
        repository
            .insert_installation(installations::ActiveModel {
                id: Set(format!("installation-{plugin_id}")),
                plugin_id: Set(plugin_id.into()),
                publisher_id: Set(manifest.publisher.id.clone()),
                version: Set(manifest.version.clone()),
                scope: Set("personal".into()),
                state: Set("active".into()),
                manifest_json: Set(serde_json::to_string(&manifest).unwrap()),
                granted_permissions_json: Set(json!(permissions).to_string()),
                artifact_digest: Set("test".into()),
                artifact_path: Set(None),
                revision: Set(1),
                last_error: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                last_activated_at: Set(Some(now)),
            })
            .unwrap();
    }

    #[test]
    fn event_types_are_lowercase_and_bounded() {
        assert!(validate_event_type("tasks.completed.v1").is_ok());
        assert!(validate_event_type("Tasks.Completed.v1").is_err());
        assert!(validate_event_type("bad event").is_err());
    }

    #[test]
    fn publication_persists_authorized_workflow_delivery() {
        let (_directory, db) = database();
        let now = 1_000;
        let catalog = super::super::plugins::catalog(&db).unwrap();
        let mut publisher = catalog
            .iter()
            .find(|entry| entry.manifest.id == "dev.aevio.project-planning")
            .unwrap()
            .manifest
            .clone();
        publisher
            .permissions
            .push(aevio_contracts::plugin::PluginPermissionRequest {
                id: capabilities::EVENTS_PUBLISH.into(),
                scope: None,
                reason: None,
            });
        let mut subscriber = catalog
            .iter()
            .find(|entry| entry.manifest.id == "dev.aevio.music")
            .unwrap()
            .manifest
            .clone();
        subscriber
            .permissions
            .push(aevio_contracts::plugin::PluginPermissionRequest {
                id: capabilities::EVENTS_SUBSCRIBE.into(),
                scope: None,
                reason: None,
            });
        subscriber.contributes.workflow_actions = vec![WorkflowContribution {
            id: "dev.aevio.music/on-task".into(),
            title: "Handle task completion".into(),
            required_capabilities: vec![],
        }];
        subscriber.event_subscriptions = vec![aevio_contracts::plugin::EventSubscription {
            id: "dev.aevio.music/on-task-subscription".into(),
            event_type: Some("tasks.completed.v1".into()),
            workflow_id: Some("dev.aevio.music/on-task".into()),
            required_capabilities: vec![],
        }];
        insert_active(
            &db,
            "dev.aevio.project-planning",
            publisher,
            &[capabilities::EVENTS_PUBLISH],
            now,
        );
        insert_active(
            &db,
            "dev.aevio.music",
            subscriber,
            &[capabilities::EVENTS_SUBSCRIBE],
            now,
        );

        let publication = publish(
            &db,
            "dev.aevio.project-planning",
            "tasks.completed.v1",
            &json!({ "taskId": "task-1" }),
            now,
        )
        .unwrap();
        assert_eq!(publication.summary.delivery_count, 1);
        assert_eq!(publication.summary.event_type, "tasks.completed.v1");
        let deliveries = PluginRepository::new(&db)
            .due_event_deliveries(now)
            .unwrap();
        assert_eq!(deliveries.len(), 1);
        assert_eq!(deliveries[0].workflow_id, "dev.aevio.music/on-task");
        assert_eq!(deliveries[0].status, "pending");

        let dispatch = dispatch_due(&db, now, "test-device").unwrap();
        assert_eq!(dispatch.delivered, 0);
        assert_eq!(dispatch.failed, 1);
        assert!(
            PluginRepository::new(&db)
                .due_event_deliveries(now)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            PluginRepository::new(&db)
                .due_event_deliveries(now + 5_000)
                .unwrap()
                .len(),
            1
        );
    }
}
