//! Durable, host-executed plugin workflow actions.

use aevio_contracts::plugin::{
    PluginLifecycleState, PluginWorkflowRequest, PluginWorkflowRunSummary,
};
use aevio_persistence::database::entities::plugins::workflow_runs;
use aevio_persistence::database::repositories::plugins::PluginRepository;
use sea_orm::{ActiveValue::Set, DatabaseConnection};
use serde_json::Value;
use uuid::Uuid;

use super::plugins::{PluginServiceError, parse_state};

const MAX_IDEMPOTENCY_KEY: usize = 256;
const MAX_TRIGGER: usize = 120;
const MAX_ATTEMPTS: i64 = 3;

pub fn run(
    db: &DatabaseConnection,
    request: PluginWorkflowRequest,
    now: i64,
    device_ref: &str,
) -> Result<
    (
        PluginWorkflowRunSummary,
        Vec<super::plugins_host::PluginRuntimeEffect>,
    ),
    PluginServiceError,
> {
    validate_request(&request)?;
    let repository = PluginRepository::new(db);
    let installation = repository
        .installation(&request.plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(request.plugin_id.clone()))?;
    if parse_state(&installation.state)? != PluginLifecycleState::Active {
        return Err(PluginServiceError::InvalidState(
            "plugin must be active".into(),
        ));
    }
    let manifest: aevio_contracts::plugin::PluginManifest =
        serde_json::from_str(&installation.manifest_json)
            .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    let action = manifest
        .contributes
        .workflow_actions
        .iter()
        .find(|action| action.id == request.workflow_id)
        .ok_or_else(|| PluginServiceError::NotFound(request.workflow_id.clone()))?;
    let grants = super::plugins::effective_permissions(db, &installation, now)?;
    for capability in &action.required_capabilities {
        if !grants.iter().any(|grant| grant == capability) {
            return Err(PluginServiceError::Permission(format!(
                "workflow action requires {capability}"
            )));
        }
    }
    let mut run = repository.workflow_run(
        &installation.id,
        &request.workflow_id,
        &request.idempotency_key,
    )?;
    if let Some(existing) = &run {
        if existing.status == "succeeded" || existing.status == "cancelled" {
            return Ok((to_summary(existing, &request.plugin_id)?, Vec::new()));
        }
        if existing.status == "running" || existing.attempt_count >= existing.max_attempts {
            return Ok((to_summary(existing, &request.plugin_id)?, Vec::new()));
        }
    }
    if run.is_none() {
        let input_json = serde_json::to_string(&request.input)
            .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
        let model = workflow_runs::ActiveModel {
            id: Set(Uuid::now_v7().to_string()),
            installation_id: Set(installation.id.clone()),
            workflow_id: Set(request.workflow_id.clone()),
            trigger: Set(request.trigger.clone()),
            idempotency_key: Set(request.idempotency_key.clone()),
            status: Set("pending".into()),
            attempt_count: Set(0),
            max_attempts: Set(MAX_ATTEMPTS),
            input_json: Set(input_json),
            output_json: Set(None),
            last_error: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            completed_at: Set(None),
        };
        repository.insert_workflow_run(model)?;
        run = repository.workflow_run(
            &installation.id,
            &request.workflow_id,
            &request.idempotency_key,
        )?;
    }
    let current =
        run.ok_or_else(|| PluginServiceError::InvalidState("workflow run disappeared".into()))?;
    let mut running: workflow_runs::ActiveModel = current.clone().into();
    running.status = Set("running".into());
    running.attempt_count = Set(current.attempt_count + 1);
    running.updated_at = Set(now);
    running.last_error = Set(None);
    repository.update_workflow_run(running)?;
    let execution = super::plugins_runtime::execute_workflow_action_with_effects_for_device(
        db, &request, device_ref,
    );
    match execution {
        Ok((output, effects)) => {
            let mut completed: workflow_runs::ActiveModel = current.into();
            completed.status = Set("succeeded".into());
            completed.attempt_count = Set(completed.attempt_count.unwrap() + 1);
            completed.output_json =
                Set(Some(serde_json::to_string(&output).map_err(|error| {
                    PluginServiceError::Serialization(error.to_string())
                })?));
            completed.updated_at = Set(now);
            completed.completed_at = Set(Some(now));
            repository.update_workflow_run(completed)?;
            let saved = repository
                .workflow_run(
                    &installation.id,
                    &request.workflow_id,
                    &request.idempotency_key,
                )?
                .ok_or_else(|| {
                    PluginServiceError::InvalidState("workflow run disappeared".into())
                })?;
            Ok((to_summary(&saved, &request.plugin_id)?, effects))
        }
        Err(error) => {
            let mut failed: workflow_runs::ActiveModel = current.into();
            let attempts = failed.attempt_count.unwrap() + 1;
            failed.status = Set(if attempts >= MAX_ATTEMPTS {
                "dead_letter"
            } else {
                "failed"
            }
            .into());
            failed.attempt_count = Set(attempts);
            failed.last_error = Set(Some(error.to_string()));
            failed.updated_at = Set(now);
            repository.update_workflow_run(failed)?;
            Err(error)
        }
    }
}

pub fn cancel(
    db: &DatabaseConnection,
    plugin_id: &str,
    workflow_id: &str,
    idempotency_key: &str,
    now: i64,
) -> Result<(), PluginServiceError> {
    let repository = PluginRepository::new(db);
    let installation = repository
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    let Some(run) = repository.workflow_run(&installation.id, workflow_id, idempotency_key)? else {
        return Ok(());
    };
    if run.status == "succeeded" {
        return Err(PluginServiceError::InvalidState(
            "completed workflow cannot be cancelled".into(),
        ));
    }
    let mut model: workflow_runs::ActiveModel = run.into();
    model.status = Set("cancelled".into());
    model.updated_at = Set(now);
    model.completed_at = Set(Some(now));
    repository
        .update_workflow_run(model)
        .map_err(PluginServiceError::from)
}

pub fn history(
    db: &DatabaseConnection,
    plugin_id: &str,
) -> Result<Vec<PluginWorkflowRunSummary>, PluginServiceError> {
    let repository = PluginRepository::new(db);
    let installation = repository
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    repository
        .workflow_runs(&installation.id)?
        .iter()
        .map(|run| to_summary(run, plugin_id))
        .collect()
}

fn validate_request(request: &PluginWorkflowRequest) -> Result<(), PluginServiceError> {
    if request.workflow_id.is_empty()
        || request.workflow_id.len() > 200
        || !request
            .workflow_id
            .starts_with(&format!("{}/", request.plugin_id))
        || request.idempotency_key.is_empty()
        || request.idempotency_key.len() > MAX_IDEMPOTENCY_KEY
        || request.idempotency_key.chars().any(char::is_control)
        || request.trigger.is_empty()
        || request.trigger.len() > MAX_TRIGGER
        || request.trigger.chars().any(char::is_control)
    {
        return Err(PluginServiceError::Permission(
            "workflow request is invalid".into(),
        ));
    }
    Ok(())
}

fn to_summary(
    run: &workflow_runs::Model,
    plugin_id: &str,
) -> Result<PluginWorkflowRunSummary, PluginServiceError> {
    let input = serde_json::from_str(&run.input_json)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    let output: Option<Value> = run
        .output_json
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    Ok(PluginWorkflowRunSummary {
        id: run.id.clone(),
        plugin_id: plugin_id.into(),
        workflow_id: run.workflow_id.clone(),
        trigger: run.trigger.clone(),
        idempotency_key: run.idempotency_key.clone(),
        status: run.status.clone(),
        attempt_count: run.attempt_count.max(0) as u64,
        max_attempts: run.max_attempts.max(0) as u64,
        input,
        output,
        last_error: run.last_error.clone(),
        created_at: run.created_at,
        completed_at: run.completed_at,
    })
}

#[cfg(test)]
mod tests {
    use super::validate_request;
    use aevio_contracts::plugin::PluginWorkflowRequest;
    use serde_json::json;

    #[test]
    fn workflow_requests_require_namespaced_identity_and_idempotency() {
        let valid = PluginWorkflowRequest {
            plugin_id: "dev.example".into(),
            workflow_id: "dev.example/run".into(),
            trigger: "manual".into(),
            idempotency_key: "run-1".into(),
            input: json!({}),
        };
        assert!(validate_request(&valid).is_ok());
        assert!(
            validate_request(&PluginWorkflowRequest {
                workflow_id: "other/run".into(),
                ..valid.clone()
            })
            .is_err()
        );
        assert!(
            validate_request(&PluginWorkflowRequest {
                idempotency_key: "bad\nkey".into(),
                ..valid
            })
            .is_err()
        );
    }
}
