//! Capability-shaped host calls exposed to bounded external WASM components.

use aevio_contracts::plugin::PluginJobScheduleRequest;
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};

use super::plugins_runtime::publish_event;

#[derive(Debug, Clone)]
pub enum PluginRuntimeEffect {
    Event { event_name: String, payload: Value },
    Notification { title: String, body: String },
}

pub fn wasm_host_call(
    db: &DatabaseConnection,
    plugin_id: &str,
    method: &str,
    payload: &Value,
    effects: &mut Vec<PluginRuntimeEffect>,
) -> Result<Value, String> {
    wasm_host_call_for_device(db, plugin_id, method, payload, effects, "legacy-device")
}

pub fn wasm_host_call_for_device(
    db: &DatabaseConnection,
    plugin_id: &str,
    method: &str,
    payload: &Value,
    effects: &mut Vec<PluginRuntimeEffect>,
    device_ref: &str,
) -> Result<Value, String> {
    match method {
        "plugin.info" => Ok(json!({ "pluginId": plugin_id })),
        "plugin.storage.get" => {
            let key = string_field(payload, "key", "storage.get")?;
            super::plugins_storage::get_for_device(db, plugin_id, key, device_ref)
                .map(|value| value.unwrap_or(Value::Null))
                .map_err(|error| error.to_string())
        }
        "plugin.storage.set" => {
            let key = string_field(payload, "key", "storage.set")?;
            let value = payload
                .get("value")
                .cloned()
                .ok_or_else(|| "storage.set requires a value".to_owned())?;
            super::plugins_storage::set_for_device(
                db,
                plugin_id,
                key,
                value,
                aevio_core::timers::now_epoch_ms() as i64,
                device_ref,
            )
            .map(|()| Value::Null)
            .map_err(|error| error.to_string())
        }
        "plugin.storage.delete" => {
            let key = string_field(payload, "key", "storage.delete")?;
            super::plugins_storage::delete_for_device(
                db,
                plugin_id,
                key,
                aevio_core::timers::now_epoch_ms() as i64,
                device_ref,
            )
            .map(|()| Value::Null)
            .map_err(|error| error.to_string())
        }
        "plugin.jobs.schedule" => {
            let job_id = string_field(payload, "jobId", "jobs.schedule")?;
            let interval_ms = payload
                .get("intervalMs")
                .and_then(Value::as_u64)
                .ok_or_else(|| "jobs.schedule requires intervalMs".to_owned())?;
            let request = PluginJobScheduleRequest {
                plugin_id: plugin_id.into(),
                job_id: job_id.into(),
                interval_ms,
                payload: payload.get("payload").cloned().unwrap_or_default(),
            };
            let summary = super::plugins_jobs::schedule(
                db,
                request,
                aevio_core::timers::now_epoch_ms() as i64,
            )
            .map_err(|error| error.to_string())?;
            serde_json::to_value(summary).map_err(|error| error.to_string())
        }
        "plugin.jobs.cancel" => {
            let job_id = string_field(payload, "jobId", "jobs.cancel")?;
            super::plugins_jobs::cancel(
                db,
                plugin_id,
                job_id,
                aevio_core::timers::now_epoch_ms() as i64,
            )
            .map(|()| Value::Null)
            .map_err(|error| error.to_string())
        }
        "plugin.events.publish" => {
            let event = string_field(payload, "event", "events.publish")?;
            let event_payload = payload.get("payload").unwrap_or(&Value::Null);
            let event_name = publish_event(db, plugin_id, event, event_payload)
                .map_err(|error| error.to_string())?;
            effects.push(PluginRuntimeEffect::Event {
                event_name: event_name.clone(),
                payload: event_payload.clone(),
            });
            Ok(json!({ "event": event_name }))
        }
        "plugin.notifications.show" => {
            let title = string_field(payload, "title", "notifications.show")?;
            let body = string_field(payload, "body", "notifications.show")?;
            super::plugins_notifications::show(
                db,
                plugin_id,
                title,
                body,
                aevio_core::timers::now_epoch_ms() as i64,
            )
            .map_err(|error| error.to_string())?;
            effects.push(PluginRuntimeEffect::Notification {
                title: title.into(),
                body: body.into(),
            });
            Ok(Value::Null)
        }
        _ => Err("host method is not available for this command".into()),
    }
}

fn string_field<'a>(payload: &'a Value, field: &str, method: &str) -> Result<&'a str, String> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{method} requires {field}"))
}
