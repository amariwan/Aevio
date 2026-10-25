//! Small authenticated LAN bridge for the independent watchOS companion.
//!
//! The bridge deliberately exposes only the watch surface. Durable state and
//! all mutations remain in the Rust stores; the listener is only a transport.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;

use aevio_infrastructure::timers_store::Timer;
use serde::Serialize;
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::commands;
use crate::timers::now_epoch_ms;
use crate::AppState;

pub const PORT: u16 = 48_731;
const TOKEN_FILE: &str = "watch-bridge.token";
const MAX_REQUEST_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingInfo {
    pub enabled: bool,
    pub port: u16,
    pub token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WatchState {
    timers: Vec<Timer>,
    now_ms: u64,
}

pub fn start(app: AppHandle, data_dir: PathBuf) {
    let token = match load_or_create_token(&data_dir) {
        Ok(token) => token,
        Err(error) => {
            eprintln!("watch bridge disabled: {error}");
            return;
        }
    };

    thread::spawn(move || {
        let listener = match TcpListener::bind(("0.0.0.0", PORT)) {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!("watch bridge unavailable on port {PORT}: {error}");
                return;
            }
        };

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let app = app.clone();
                    let token = token.clone();
                    thread::spawn(move || handle_connection(stream, &app, &token));
                }
                Err(error) => eprintln!("watch bridge connection failed: {error}"),
            }
        }
    });
}

pub fn pairing_info(data_dir: &Path) -> Result<PairingInfo, String> {
    let token = load_or_create_token(data_dir)?;
    Ok(PairingInfo {
        enabled: true,
        port: PORT,
        token,
    })
}

fn load_or_create_token(data_dir: &Path) -> Result<String, String> {
    let path = data_dir.join(TOKEN_FILE);
    if let Ok(token) = fs::read_to_string(&path) {
        let token = token.trim().to_string();
        if token.len() >= 32 {
            return Ok(token);
        }
    }

    let token = Uuid::new_v4().to_string();
    fs::write(&path, format!("{token}\n"))
        .map_err(|error| format!("could not write watch pairing token: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("could not protect watch pairing token: {error}"))?;
    }
    Ok(token)
}

fn handle_connection(mut stream: TcpStream, app: &AppHandle, token: &str) {
    let Some(request) = read_request(&mut stream) else {
        write_response(&mut stream, 400, "invalid request", None);
        return;
    };
    if request.authorization.as_deref() != Some(token) {
        write_response(&mut stream, 401, "unauthorized", None);
        return;
    }

    let result = match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/v1/watch/state") => state(app),
        ("POST", "/v1/watch/session/start") => start_session(app, request.body.as_deref()),
        ("POST", "/v1/watch/session/stop") => stop_session(app),
        _ => timer_action(app, &request.method, &request.path),
    };

    match result {
        Ok(body) => write_response(&mut stream, 200, "ok", Some(&body)),
        Err((status, message)) => write_response(&mut stream, status, &message, None),
    }
}

fn state(app: &AppHandle) -> Result<Vec<u8>, (u16, String)> {
    let timers = app
        .state::<AppState>()
        .store
        .lock()
        .map_err(|error| (500, error.to_string()))?
        .snapshot()
        .map_err(|error| (500, error.to_string()))?;
    serde_json::to_vec(&WatchState {
        timers,
        now_ms: now_epoch_ms(),
    })
    .map_err(|error| (500, error.to_string()))
}

fn start_session(app: &AppHandle, body: Option<&[u8]>) -> Result<Vec<u8>, (u16, String)> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Input {
        kind: Option<String>,
        label: Option<String>,
    }
    let input = body
        .map(serde_json::from_slice::<Input>)
        .transpose()
        .map_err(|error| (400, format!("invalid request body: {error}")))?
        .unwrap_or(Input {
            kind: None,
            label: None,
        });
    commands::start_session(
        app.clone(),
        input.kind.unwrap_or_else(|| "focus".to_string()),
        input.label,
    )
    .map_err(|error| (400, error))?;
    state(app)
}

fn stop_session(app: &AppHandle) -> Result<Vec<u8>, (u16, String)> {
    commands::stop_session(app.clone()).map_err(|error| (400, error))?;
    state(app)
}

fn timer_action(app: &AppHandle, method: &str, path: &str) -> Result<Vec<u8>, (u16, String)> {
    let Some(rest) = path.strip_prefix("/v1/watch/timers/") else {
        return Err((404, "not found".to_string()));
    };
    if method != "POST" {
        return Err((405, "method not allowed".to_string()));
    }
    let Some((id, action)) = rest.rsplit_once('/') else {
        return Err((404, "not found".to_string()));
    };
    let result = match action {
        "start" => commands::start_timer(app.clone(), id.to_string()),
        "pause" => commands::pause_timer(app.clone(), id.to_string()),
        _ => return Err((404, "not found".to_string())),
    };
    result
        .map(|timers| {
            serde_json::to_vec(&WatchState {
                timers,
                now_ms: now_epoch_ms(),
            })
            .unwrap_or_default()
        })
        .map_err(|error| (400, error))
}

struct Request {
    method: String,
    path: String,
    authorization: Option<String>,
    body: Option<Vec<u8>>,
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut first = String::new();
    reader.read_line(&mut first).ok()?;
    let mut parts = first.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
    let mut authorization = None;
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        if line == "\r\n" || line == "\n" {
            break;
        }
        let (name, value) = line.split_once(':')?;
        match name.trim().to_ascii_lowercase().as_str() {
            "authorization" => {
                authorization = value.trim().strip_prefix("Bearer ").map(str::to_string)
            }
            "content-length" => content_length = value.trim().parse().ok()?,
            _ => {}
        }
    }
    if content_length > MAX_REQUEST_BYTES {
        return None;
    }
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body).ok()?;
    Some(Request {
        method,
        path,
        authorization,
        body: (!body.is_empty()).then_some(body),
    })
}

fn write_response(stream: &mut TcpStream, status: u16, message: &str, body: Option<&[u8]>) {
    let body = body.unwrap_or({
        // The error response is intentionally tiny and does not echo request data.
        message.as_bytes()
    });
    let content_type = if body.first() == Some(&b'{') {
        "application/json"
    } else {
        "text/plain; charset=utf-8"
    };
    let response = format!(
        "HTTP/1.1 {status} {message}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(body);
}
