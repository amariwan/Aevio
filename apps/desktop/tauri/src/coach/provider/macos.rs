use crate::ai::sidecar::Sidecar;
use aevio_core::coach::{
    assistance::PROMPT_VERSION,
    models::{AssistRequest, Preferences, Proposal},
};
use std::time::Duration;

pub fn generate(
    request: &AssistRequest,
    preferences: &Preferences,
    memory: &[String],
) -> Result<Proposal, String> {
    // A short-lived, serialized coach request never queues behind classification.
    // Drop kills/reaps the process on success, error and timeout.
    let mut sidecar = Sidecar::new();
    let capabilities = sidecar.probe()?;
    if capabilities.llm != "available" {
        return Err(capabilities.llm);
    }
    let mut value:Proposal=sidecar.call("coach",serde_json::json!({"title":request.title,"reason":request.reason,"granularity":request.granularity,"estimateMinutes":request.estimate_minutes,"previousSteps":request.previous_steps,"style":preferences.style,"memory":memory}),Duration::from_secs(30))?;
    value.source = "on_device".into();
    value.prompt_version = PROMPT_VERSION.into();
    value.note = None;
    Ok(value)
}

pub fn generate_plan(
    day: &aevio_core::coach::models::Day,
) -> Result<aevio_core::coach::adaptive::PlanSelection, String> {
    let mut sidecar = Sidecar::new();
    if sidecar.probe()?.llm != "available" {
        return Err("On-device language model is unavailable".into());
    }
    let tasks=day.tasks.iter().filter(|task|!task.done).take(30).map(|task|serde_json::json!({"id":task.id,"title":task.title.chars().take(160).collect::<String>(),"minutes":task.estimate_minutes,"priority":task.priority})).collect::<Vec<_>>();
    sidecar.call("coach_plan",serde_json::json!({"goal":day.plan.as_ref().map(|plan|plan.goal.chars().take(500).collect::<String>()).unwrap_or_default(),"capacity":day.plan.as_ref().map(|plan|plan.available_minutes).unwrap_or(0),"tasks":tasks}),Duration::from_secs(30))
}
