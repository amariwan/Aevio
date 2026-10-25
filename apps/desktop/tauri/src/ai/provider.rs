//! External AI provider (Ollama / OpenAI-compatible) for the Agent surface.

use serde::Deserialize;
use serde_json::Value;

use super::agent::AgentAnswer;

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
}

#[derive(Deserialize)]
struct OllamaMessage {
    content: String,
}

#[derive(Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Deserialize)]
struct OpenAiMessage {
    content: String,
}

/// Calls an externally configured AI provider. Returns an error when no provider
/// is configured or the call fails, so callers can fall back to local AI.
pub fn answer(
    message: &str,
    context: &Value,
    provider: &str,
    base_url: &str,
    model: &str,
    api_key: &str,
) -> Result<AgentAnswer, String> {
    if provider.is_empty() || base_url.is_empty() || model.is_empty() {
        return Err("no external AI provider configured".into());
    }
    let system = format!(
        "You are Aevio's local productivity assistant. Answer concisely in the user's language. \
         Use only the provided workspace context. Context: {}",
        context
    );
    match provider {
        "ollama" => call_ollama(base_url, model, &system, message),
        "openai" => call_openai(base_url, model, api_key, &system, message),
        other => Err(format!("unknown AI provider: {other}")),
    }
}

fn call_ollama(
    base_url: &str,
    model: &str,
    system: &str,
    message: &str,
) -> Result<AgentAnswer, String> {
    let url = format!("{}/api/chat", base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": message},
        ],
        "stream": false,
    });
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("ollama returned {}", resp.status()));
    }
    let parsed: OllamaChatResponse = resp.json().map_err(|e| e.to_string())?;
    let answer = parsed.message.content.trim().to_string();
    if answer.is_empty() {
        return Err("ollama returned an empty answer".into());
    }
    Ok(AgentAnswer {
        answer,
        source: "external-ai",
    })
}

fn call_openai(
    base_url: &str,
    model: &str,
    api_key: &str,
    system: &str,
    message: &str,
) -> Result<AgentAnswer, String> {
    let base = base_url.trim_end_matches('/');
    let url = if base.ends_with("/chat/completions") {
        base.to_string()
    } else if base.ends_with("/v1") {
        format!("{}/chat/completions", base)
    } else {
        format!("{}/v1/chat/completions", base)
    };
    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": message},
        ],
    });
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let mut req = client.post(&url).json(&body);
    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }
    let resp = req.send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("provider returned {}", resp.status()));
    }
    let parsed: OpenAiChatResponse = resp.json().map_err(|e| e.to_string())?;
    let answer = parsed
        .choices
        .first()
        .map(|c| c.message.content.trim().to_string())
        .unwrap_or_default();
    if answer.is_empty() {
        return Err("provider returned an empty answer".into());
    }
    Ok(AgentAnswer {
        answer,
        source: "external-ai",
    })
}
