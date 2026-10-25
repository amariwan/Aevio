//! Model-backed local answers for the Agent surface.

use serde::Serialize;
use serde_json::Value;

use super::sidecar::Sidecar;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAnswer {
    pub answer: String,
    pub source: &'static str,
}

#[derive(Debug, serde::Deserialize)]
struct SidecarAnswer {
    answer: String,
}

/// Uses the on-device model when available. Callers retain a deterministic
/// fallback so unsupported platforms never pretend that generation happened.
pub fn answer(message: &str, context: &Value) -> Result<AgentAnswer, String> {
    let mut sidecar = Sidecar::new();
    let capabilities = sidecar.probe()?;
    if capabilities.llm != "available" {
        return Err(capabilities.llm);
    }
    let response: SidecarAnswer = sidecar.call(
        "agent",
        serde_json::json!({"message": message, "context": context}),
        super::sidecar::TIMEOUT_CLASSIFY,
    )?;
    if response.answer.trim().is_empty() {
        return Err("the on-device agent returned an empty answer".into());
    }
    Ok(AgentAnswer {
        answer: response.answer,
        source: "on_device",
    })
}
