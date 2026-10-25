//! Durable, offline-safe scheduling for plugin workflow jobs.

use aevio_contracts::plugin::{
    MAX_PLUGIN_JOB_INTERVAL_MS, MIN_PLUGIN_JOB_INTERVAL_MS, PluginJobScheduleRequest,
    PluginJobSummary, PluginLifecycleState, capabilities,
};
use aevio_persistence::database::entities::plugins::jobs;
use aevio_persistence::database::repositories::plugins::PluginRepository;
use sea_orm::{ActiveValue::Set, DatabaseConnection};
use uuid::Uuid;

use super::plugins::{CapabilityGateway, PluginServiceError, audit, parse_state};

pub use aevio_contracts::plugin::{
    MAX_PLUGIN_JOB_INTERVAL_MS as MAX_JOB_INTERVAL_MS,
    MIN_PLUGIN_JOB_INTERVAL_MS as MIN_JOB_INTERVAL_MS,
};
const MAX_JOB_PAYLOAD_BYTES: usize = 64 * 1024;
pub const DEFAULT_MAX_ATTEMPTS: i64 = 3;
pub const JOB_LEASE_MS: i64 = 5 * 60 * 1000;
const MAX_RETRY_DELAY_MS: i64 = 60 * 60 * 1000;

pub fn schedule(
    db: &DatabaseConnection,
    request: PluginJobScheduleRequest,
    now: i64,
) -> Result<PluginJobSummary, PluginServiceError> {
    if !(MIN_PLUGIN_JOB_INTERVAL_MS..=MAX_PLUGIN_JOB_INTERVAL_MS).contains(&request.interval_ms) {
        return Err(PluginServiceError::Permission(
            "plugin job interval must be between 1 minute and 30 days".into(),
        ));
    }
    let repository = PluginRepository::new(db);
    let installation = repository
        .installation(&request.plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(request.plugin_id.clone()))?;
    if parse_state(&installation.state)? != PluginLifecycleState::Active {
        return Err(PluginServiceError::InvalidState(
            "plugin must be active".into(),
        ));
    }
    let manifest = serde_json::from_str::<aevio_contracts::plugin::PluginManifest>(
        &installation.manifest_json,
    )
    .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    let grants = super::plugins::effective_permissions(db, &installation, now)?;
    CapabilityGateway::new(&request.plugin_id, grants).require(capabilities::JOBS_SCHEDULE)?;
    let job = manifest
        .contributes
        .jobs
        .iter()
        .find(|job| job.id == request.job_id)
        .ok_or_else(|| PluginServiceError::NotFound(request.job_id.clone()))?;
    if request.interval_ms < job.min_interval_ms {
        return Err(PluginServiceError::Permission(
            "requested interval is below the manifest minimum".into(),
        ));
    }
    let payload_json = serde_json::to_string(&request.payload)
        .map_err(|error| PluginServiceError::Serialization(error.to_string()))?;
    if payload_json.len() > MAX_JOB_PAYLOAD_BYTES {
        return Err(PluginServiceError::Permission(
            "plugin job payload is too large".into(),
        ));
    }
    let next_run_at = now.saturating_add(request.interval_ms as i64);
    if let Some(existing) = repository.job(&installation.id, &request.job_id)? {
        let mut model: jobs::ActiveModel = existing.clone().into();
        model.interval_ms = Set(request.interval_ms as i64);
        model.next_run_at = Set(next_run_at);
        model.payload_json = Set(payload_json);
        model.enabled = Set(1);
        model.attempt_count = Set(0);
        model.max_attempts = Set(DEFAULT_MAX_ATTEMPTS);
        model.lease_until = Set(None);
        model.last_error = Set(None);
        model.state = Set("scheduled".into());
        model.updated_at = Set(now);
        repository.update_job(model)?;
        audit(
            &repository,
            &installation.id,
            "job_scheduled",
            now,
            &request.plugin_id,
        )?;
        return repository
            .job(&installation.id, &request.job_id)?
            .map(|job| to_summary(job, request.plugin_id.clone()))
            .ok_or_else(|| PluginServiceError::NotFound(request.job_id));
    }
    let id = Uuid::now_v7().to_string();
    repository.insert_job(jobs::ActiveModel {
        id: Set(id),
        installation_id: Set(installation.id.clone()),
        job_id: Set(request.job_id.clone()),
        interval_ms: Set(request.interval_ms as i64),
        next_run_at: Set(next_run_at),
        last_run_at: Set(None),
        run_count: Set(0),
        enabled: Set(1),
        payload_json: Set(payload_json),
        attempt_count: Set(0),
        max_attempts: Set(DEFAULT_MAX_ATTEMPTS),
        lease_until: Set(None),
        last_error: Set(None),
        state: Set("scheduled".into()),
        created_at: Set(now),
        updated_at: Set(now),
    })?;
    audit(
        &repository,
        &installation.id,
        "job_scheduled",
        now,
        &request.plugin_id,
    )?;
    repository
        .job(&installation.id, &request.job_id)?
        .map(|job| to_summary(job, request.plugin_id.clone()))
        .ok_or_else(|| PluginServiceError::NotFound(request.job_id))
}

pub fn cancel(
    db: &DatabaseConnection,
    plugin_id: &str,
    job_id: &str,
    now: i64,
) -> Result<(), PluginServiceError> {
    let repository = PluginRepository::new(db);
    let installation = repository
        .installation(plugin_id)?
        .ok_or_else(|| PluginServiceError::NotFound(plugin_id.into()))?;
    let Some(job) = repository.job(&installation.id, job_id)? else {
        return Ok(());
    };
    let mut model: jobs::ActiveModel = job.into();
    model.enabled = Set(0);
    model.state = Set("cancelled".into());
    model.lease_until = Set(None);
    model.updated_at = Set(now);
    repository.update_job(model)?;
    audit(
        &repository,
        &installation.id,
        "job_cancelled",
        now,
        plugin_id,
    )?;
    Ok(())
}

pub fn due(db: &DatabaseConnection, now: i64) -> Result<Vec<PluginJobSummary>, PluginServiceError> {
    let repository = PluginRepository::new(db);
    let mut due = Vec::new();
    for job in repository.due_jobs(now)? {
        let Some(installation) = repository.installation_by_id(&job.installation_id)? else {
            continue;
        };
        if parse_state(&installation.state)? == PluginLifecycleState::Active {
            due.push(to_summary(job, installation.plugin_id));
        }
    }
    Ok(due)
}

pub fn mark_run(
    db: &DatabaseConnection,
    job_id: &str,
    now: i64,
) -> Result<PluginJobSummary, PluginServiceError> {
    let repository = PluginRepository::new(db);
    let job = repository
        .job_by_id(job_id)?
        .ok_or_else(|| PluginServiceError::NotFound(job_id.into()))?;
    let mut model: jobs::ActiveModel = job.clone().into();
    model.last_run_at = Set(Some(now));
    model.run_count = Set(job.run_count.saturating_add(1));
    model.next_run_at = Set(now.saturating_add(job.interval_ms));
    model.attempt_count = Set(0);
    model.lease_until = Set(None);
    model.last_error = Set(None);
    model.state = Set("scheduled".into());
    model.updated_at = Set(now);
    repository.update_job(model)?;
    let updated = repository
        .job_by_id(job_id)?
        .ok_or_else(|| PluginServiceError::NotFound(job_id.into()))?;
    let installation = repository
        .installation_by_id(&updated.installation_id)?
        .ok_or_else(|| PluginServiceError::NotFound(updated.installation_id.clone()))?;
    Ok(to_summary(updated, installation.plugin_id))
}

pub fn claim(
    db: &DatabaseConnection,
    job_id: &str,
    now: i64,
) -> Result<Option<PluginJobSummary>, PluginServiceError> {
    let repository = PluginRepository::new(db);
    let Some(job) = repository.claim_job(job_id, now, now.saturating_add(JOB_LEASE_MS))? else {
        return Ok(None);
    };
    let installation = repository
        .installation_by_id(&job.installation_id)?
        .ok_or_else(|| PluginServiceError::NotFound(job.installation_id.clone()))?;
    Ok(Some(to_summary(job, installation.plugin_id)))
}

pub fn mark_failure(
    db: &DatabaseConnection,
    job_id: &str,
    now: i64,
    error: &str,
) -> Result<PluginJobSummary, PluginServiceError> {
    let repository = PluginRepository::new(db);
    let job = repository
        .job_by_id(job_id)?
        .ok_or_else(|| PluginServiceError::NotFound(job_id.into()))?;
    let attempt_count = job.attempt_count.saturating_add(1);
    let exhausted = attempt_count >= job.max_attempts.max(1);
    let retry_delay = job
        .interval_ms
        .max(1)
        .saturating_mul(1_i64 << attempt_count.saturating_sub(1).min(6) as u32)
        .min(MAX_RETRY_DELAY_MS);
    let mut model: jobs::ActiveModel = job.into();
    model.attempt_count = Set(attempt_count);
    model.last_error = Set(Some(error.chars().take(2_000).collect()));
    model.lease_until = Set(None);
    model.state = Set(if exhausted {
        "dead_letter"
    } else {
        "scheduled"
    }
    .into());
    model.enabled = Set(if exhausted { 0 } else { 1 });
    model.next_run_at = Set(if exhausted {
        now
    } else {
        now.saturating_add(retry_delay)
    });
    model.updated_at = Set(now);
    repository.update_job(model)?;
    let updated = repository
        .job_by_id(job_id)?
        .ok_or_else(|| PluginServiceError::NotFound(job_id.into()))?;
    let installation = repository
        .installation_by_id(&updated.installation_id)?
        .ok_or_else(|| PluginServiceError::NotFound(updated.installation_id.clone()))?;
    Ok(to_summary(updated, installation.plugin_id))
}

pub fn event_name(plugin_id: &str, job_id: &str) -> String {
    format!("plugin-job:{plugin_id}:{job_id}")
}

fn to_summary(row: jobs::Model, plugin_id: String) -> PluginJobSummary {
    PluginJobSummary {
        id: row.id,
        plugin_id,
        job_id: row.job_id,
        interval_ms: row.interval_ms.max(0) as u64,
        next_run_at: row.next_run_at,
        last_run_at: row.last_run_at,
        run_count: row.run_count.max(0) as u64,
        enabled: row.enabled != 0,
        state: row.state,
        attempt_count: row.attempt_count.max(0) as u64,
        max_attempts: row.max_attempts.max(1) as u64,
        last_error: row.last_error,
        payload: serde_json::from_str(&row.payload_json).unwrap_or(serde_json::Value::Null),
    }
}

#[cfg(test)]
mod tests {
    use super::{cancel, claim, due, event_name, mark_failure, mark_run, schedule};
    use aevio_contracts::plugin::{PluginInstallationScope, PluginJobScheduleRequest};
    use aevio_persistence::bootstrap::migrations::run_migrations;
    use aevio_persistence::database::connect;
    use rusqlite::Connection;
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

    fn activate_project_planning(db: &sea_orm::DatabaseConnection) {
        let plugin_id = "dev.aevio.project-planning";
        super::super::plugins::install_builtin(db, plugin_id, PluginInstallationScope::Personal, 1)
            .unwrap();
        let permissions = super::super::plugins::catalog(db)
            .unwrap()
            .into_iter()
            .find(|entry| entry.manifest.id == plugin_id)
            .unwrap()
            .manifest
            .permissions
            .into_iter()
            .map(|permission| permission.id)
            .collect();
        super::super::plugins::grant_permissions(db, plugin_id, permissions, 2).unwrap();
        super::super::plugins::set_enabled(db, plugin_id, true, 3).unwrap();
    }

    #[test]
    fn schedule_due_run_and_cancel_are_durable() {
        let (_directory, db) = database();
        activate_project_planning(&db);
        let plugin_id = "dev.aevio.project-planning";
        let job_id = "dev.aevio.project-planning/refresh";
        let scheduled = schedule(
            &db,
            PluginJobScheduleRequest {
                plugin_id: plugin_id.into(),
                job_id: job_id.into(),
                interval_ms: 900_000,
                payload: json!({"source": "test"}),
            },
            10,
        )
        .unwrap();
        assert_eq!(scheduled.plugin_id, plugin_id);
        assert!(due(&db, scheduled.next_run_at - 1).unwrap().is_empty());
        let pending = due(&db, scheduled.next_run_at).unwrap();
        assert_eq!(pending.len(), 1);
        let claimed = claim(&db, &pending[0].id, scheduled.next_run_at)
            .unwrap()
            .unwrap();
        assert_eq!(claimed.state, "leased");
        let run = mark_run(&db, &claimed.id, scheduled.next_run_at).unwrap();
        assert_eq!(run.run_count, 1);
        cancel(&db, plugin_id, job_id, run.next_run_at).unwrap();
        assert!(
            due(&db, run.next_run_at + run.interval_ms as i64)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            event_name(plugin_id, job_id),
            "plugin-job:dev.aevio.project-planning:dev.aevio.project-planning/refresh"
        );
    }

    #[test]
    fn failed_jobs_retry_then_enter_dead_letter() {
        let (_directory, db) = database();
        activate_project_planning(&db);
        let scheduled = schedule(
            &db,
            PluginJobScheduleRequest {
                plugin_id: "dev.aevio.project-planning".into(),
                job_id: "dev.aevio.project-planning/refresh".into(),
                interval_ms: 900_000,
                payload: json!({"source": "test"}),
            },
            10,
        )
        .unwrap();
        let first = claim(&db, &scheduled.id, scheduled.next_run_at)
            .unwrap()
            .unwrap();
        let retry = mark_failure(&db, &first.id, 20, "temporary delivery failure").unwrap();
        assert_eq!(retry.state, "scheduled");
        assert_eq!(retry.attempt_count, 1);
        assert_eq!(
            retry.last_error.as_deref(),
            Some("temporary delivery failure")
        );
        let second = claim(&db, &retry.id, retry.next_run_at).unwrap().unwrap();
        let retry_again = mark_failure(
            &db,
            &second.id,
            retry.next_run_at,
            "second delivery failure",
        )
        .unwrap();
        let third = claim(&db, &retry_again.id, retry_again.next_run_at)
            .unwrap()
            .unwrap();
        let dead = mark_failure(
            &db,
            &third.id,
            retry_again.next_run_at,
            "third delivery failure",
        )
        .unwrap();
        assert_eq!(dead.state, "dead_letter");
        assert!(!dead.enabled);
    }
}
