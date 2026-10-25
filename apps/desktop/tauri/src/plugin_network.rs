//! Bounded, origin-approved network capability for external plugins.

use std::time::Duration;

use aevio_contracts::plugin::{PluginNetworkRequest, PluginNetworkResponse};
use tauri::AppHandle;

use crate::commands::with_database_reader;

const MAX_REQUEST_BODY: usize = 256 * 1024;
const MAX_RESPONSE_BODY: usize = 1024 * 1024;

#[tauri::command(rename = "plugin_network_fetch")]
pub async fn fetch(
    app: AppHandle,
    plugin_id: String,
    request: PluginNetworkRequest,
) -> Result<PluginNetworkResponse, String> {
    if request
        .body
        .as_ref()
        .is_some_and(|body| body.len() > MAX_REQUEST_BODY)
    {
        return Err("plugin network request body exceeds the host limit".into());
    }
    let method = request.method.trim().to_ascii_uppercase();
    if !matches!(method.as_str(), "GET" | "POST") {
        return Err("plugin network method is not allowed".into());
    }
    let url = url::Url::parse(request.url.trim()).map_err(|error| error.to_string())?;
    if !matches!(url.scheme(), "https" | "http") || url.host_str().is_none() {
        return Err("plugin network URL must use HTTP(S)".into());
    }
    with_database_reader(&app, |database| {
        aevio_services::plugins::authorize_network_origin(database, &plugin_id, url.as_str())
            .map_err(|error| error.to_string())
    })?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| error.to_string())?;
    let builder = match method.as_str() {
        "GET" => client.get(url),
        "POST" => client.post(url).body(request.body.unwrap_or_default()),
        _ => unreachable!(),
    };
    let response = builder.send().await.map_err(|error| error.to_string())?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BODY as u64)
    {
        return Err("plugin network response exceeds the host limit".into());
    }
    let status = response.status().as_u16();
    let body = read_bounded(response).await?;
    Ok(PluginNetworkResponse { status, body })
}

async fn read_bounded(mut response: reqwest::Response) -> Result<String, String> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BODY {
            return Err("plugin network response exceeds the host limit".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| "plugin network response is not UTF-8".into())
}
