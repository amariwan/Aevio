//! A local MCP stdio server backed by the same typed IPC protocol as the CLI.
//! It deliberately exposes read-only tools first: external agents must not
//! mutate time records without an explicit approval workflow.

use std::io::{self, BufRead, Write};
use std::path::Path;

use aevio_core::EntryFilter;
use aevio_infrastructure::protocol::Operation;
use serde_json::{json, Map, Value};

use crate::client::Client;

const PROTOCOL_VERSION: &str = "2025-11-25";
const DEFAULT_RANGE_MS: u64 = 30 * 24 * 60 * 60 * 1000;
const MAX_LIMIT: u32 = 200;

pub fn serve(data_dir: Option<&Path>) -> Result<(), String> {
    let client = Client::new(data_dir)?;
    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| error.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = serde_json::from_str(&line)
            .map_err(|error| format!("invalid JSON-RPC request: {error}"))?;
        if let Some(response) = handle(&client, &request) {
            serde_json::to_writer(&mut stdout, &response).map_err(|error| error.to_string())?;
            stdout.write_all(b"\n").map_err(|error| error.to_string())?;
            stdout.flush().map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn handle(client: &Client, request: &Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {
                "tools": {"listChanged": false},
                "resources": {"subscribe": false, "listChanged": false},
                "prompts": {"listChanged": false}
            },
            "serverInfo": {"name": "aevio", "version": env!("CARGO_PKG_VERSION")},
            "instructions": "Aevio exposes local-first, read-only time and project context."
        })),
        "notifications/initialized" => return None,
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools()})),
        "resources/list" => Ok(json!({"resources": resources()})),
        "resources/read" => read_resource(client, request.get("params").unwrap_or(&Value::Null)),
        "prompts/list" => Ok(json!({"prompts": prompts()})),
        "prompts/get" => get_prompt(request.get("params").unwrap_or(&Value::Null)),
        "tools/call" => call_tool(client, request.get("params").unwrap_or(&Value::Null)),
        _ => Err((-32601, format!("method not found: {method}"))),
    };
    let id = id?;
    Some(match result {
        Ok(result) => json!({"jsonrpc":"2.0", "id": id, "result": result}),
        Err((code, message)) => {
            json!({"jsonrpc":"2.0", "id": id, "error": {"code": code, "message": message}})
        }
    })
}

fn tools() -> Vec<Value> {
    vec![
        json!({"name":"aevio_status","description":"Read today's tracking status and totals.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}),
        json!({"name":"aevio_list_projects","description":"List active and archived Aevio projects.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}),
        json!({"name":"aevio_list_categories","description":"List Aevio categories used for time classification.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}),
        json!({
            "name":"aevio_list_time_entries",
            "description":"List time entries in a bounded timestamp range. Defaults to the last 30 days.",
            "inputSchema":{"type":"object","properties":{
                "startMs":{"type":"integer","minimum":0},"endMs":{"type":"integer","minimum":1},
                "status":{"type":"string","enum":["pending","approved"]},"search":{"type":"string"},
                "limit":{"type":"integer","minimum":1,"maximum":200,"default":50}
            },"additionalProperties":false}
        }),
        json!({"name":"aevio_list_clients","description":"List Aevio clients and their project relationships.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}),
        json!({
            "name":"aevio_pending_reviews",
            "description":"List bounded time entries that still need review.",
            "inputSchema":{"type":"object","properties":{
                "startMs":{"type":"integer","minimum":0},"endMs":{"type":"integer","minimum":1},
                "limit":{"type":"integer","minimum":1,"maximum":200,"default":50}
            },"additionalProperties":false}
        }),
        json!({
            "name":"aevio_report",
            "description":"Summarize tracked time for a bounded range by project, client, category, app, status or total.",
            "inputSchema":{"type":"object","required":["startMs","endMs"],"properties":{
                "startMs":{"type":"integer","minimum":0},"endMs":{"type":"integer","minimum":1},
                "groupBy":{"type":"string","enum":["project","client","category","app","status","none"],"default":"project"}
            },"additionalProperties":false}
        }),
    ]
}

fn resources() -> Vec<Value> {
    vec![json!({
        "uri":"aevio://context",
        "name":"Aevio local context",
        "description":"Current local tracking, project and category context.",
        "mimeType":"application/json"
    })]
}

fn prompts() -> Vec<Value> {
    vec![json!({
        "name":"aevio_daily_review",
        "description":"Review today's tracking state and decide the next safe local step.",
        "arguments":[]
    })]
}

fn read_resource(client: &Client, params: &Value) -> Result<Value, (i64, String)> {
    if params.get("uri").and_then(Value::as_str) != Some("aevio://context") {
        return Err((-32602, "unknown Aevio resource".into()));
    }
    let context = json!({
        "status": response_data(client.request(Operation::Status { day_start: day_start_now() }))?,
        "projects": response_data(client.request(Operation::ProjectsList {}))?,
        "categories": response_data(client.request(Operation::CategoriesList {}))?,
        "clients": response_data(client.request(Operation::ClientsList {}))?
    });
    Ok(
        json!({"contents":[{"uri":"aevio://context","mimeType":"application/json","text":serde_json::to_string_pretty(&context).unwrap_or_else(|_| "{}".into())}]}),
    )
}

fn get_prompt(params: &Value) -> Result<Value, (i64, String)> {
    if params.get("name").and_then(Value::as_str) != Some("aevio_daily_review") {
        return Err((-32602, "unknown Aevio prompt".into()));
    }
    Ok(json!({
        "description":"Review today's local Aevio state without changing anything.",
        "messages":[{"role":"user","content":{"type":"text","text":"Use the aevio://context resource and aevio_pending_reviews tool to summarize today's tracking state, identify missing reviews, and propose one small next step. Do not mutate data."}}]
    }))
}

fn call_tool(client: &Client, params: &Value) -> Result<Value, (i64, String)> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((-32602, "tools/call requires a tool name".into()))?;
    let arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let response = match name {
        "aevio_status" => client.request(Operation::Status {
            day_start: day_start_now(),
        }),
        "aevio_list_projects" => client.request(Operation::ProjectsList {}),
        "aevio_list_categories" => client.request(Operation::CategoriesList {}),
        "aevio_list_clients" => client.request(Operation::ClientsList {}),
        "aevio_pending_reviews" => {
            let now = epoch_ms();
            let end_ms = integer(&arguments, "endMs").unwrap_or(now);
            let start_ms = integer(&arguments, "startMs")
                .unwrap_or_else(|| end_ms.saturating_sub(DEFAULT_RANGE_MS));
            let limit = integer(&arguments, "limit")
                .unwrap_or(50)
                .clamp(1, MAX_LIMIT as u64) as u32;
            client.request(Operation::EntriesList {
                filter: EntryFilter {
                    start_ms,
                    end_ms,
                    status: Some("pending".into()),
                    ..EntryFilter::default()
                },
                limit,
            })
        }
        "aevio_report" => {
            let start_ms = integer(&arguments, "startMs")
                .ok_or((-32602, "aevio_report requires startMs".into()))?;
            let end_ms = integer(&arguments, "endMs")
                .ok_or((-32602, "aevio_report requires endMs".into()))?;
            if end_ms <= start_ms {
                return Err((-32602, "endMs must be greater than startMs".into()));
            }
            let group_by = arguments
                .get("groupBy")
                .and_then(Value::as_str)
                .unwrap_or("project");
            if !matches!(
                group_by,
                "project" | "client" | "category" | "app" | "status" | "none"
            ) {
                return Err((-32602, "groupBy is invalid".into()));
            }
            client.request(Operation::Report {
                filter: EntryFilter {
                    start_ms,
                    end_ms,
                    ..EntryFilter::default()
                },
                boundaries: vec![start_ms, end_ms],
                group_by: group_by.into(),
            })
        }
        "aevio_list_time_entries" => {
            let now = epoch_ms();
            let end_ms = integer(&arguments, "endMs").unwrap_or(now);
            let start_ms = integer(&arguments, "startMs")
                .unwrap_or_else(|| end_ms.saturating_sub(DEFAULT_RANGE_MS));
            if end_ms <= start_ms {
                return Err((-32602, "endMs must be greater than startMs".into()));
            }
            let limit = integer(&arguments, "limit")
                .unwrap_or(50)
                .clamp(1, MAX_LIMIT as u64) as u32;
            client.request(Operation::EntriesList {
                filter: EntryFilter {
                    start_ms,
                    end_ms,
                    status: arguments
                        .get("status")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    search: arguments
                        .get("search")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    ..EntryFilter::default()
                },
                limit,
            })
        }
        _ => return Err((-32602, format!("unknown Aevio tool: {name}"))),
    };
    if response.ok {
        let data = response.data.unwrap_or(Value::Null);
        Ok(
            json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&data).unwrap_or_else(|_| "null".into())}],"structuredContent":data,"isError":false}),
        )
    } else {
        let message = response
            .error
            .map(|error| error.message)
            .unwrap_or_else(|| "Aevio returned an unknown error".into());
        Ok(json!({"content":[{"type":"text","text":message}],"isError":true}))
    }
}

fn response_data(
    response: aevio_infrastructure::protocol::Response,
) -> Result<Value, (i64, String)> {
    if response.ok {
        Ok(response.data.unwrap_or(Value::Null))
    } else {
        Err((
            -32000,
            response
                .error
                .map(|error| error.message)
                .unwrap_or_else(|| "Aevio returned an unknown error".into()),
        ))
    }
}

fn integer(arguments: &Map<String, Value>, key: &str) -> Option<u64> {
    arguments.get(key).and_then(Value::as_u64)
}

fn epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

fn day_start_now() -> u64 {
    let now = chrono::Local::now();
    now.date_naive()
        .and_hms_opt(0, 0, 0)
        .and_then(|date| date.and_local_timezone(*now.offset()).single())
        .map_or_else(epoch_ms, |date| date.timestamp_millis().max(0) as u64)
}
