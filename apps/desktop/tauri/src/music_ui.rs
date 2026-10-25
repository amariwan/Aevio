use tauri::{Manager, Runtime};

use crate::AppState;

const MAX_AUDIO_BYTES: u64 = 100 * 1024 * 1024;

pub fn response<R: Runtime>(
    context: tauri::UriSchemeContext<'_, R>,
    request: tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    let not_found = || {
        tauri::http::Response::builder()
            .status(tauri::http::StatusCode::NOT_FOUND)
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(b"music track not found".to_vec())
            .expect("static music response is valid")
    };
    if request.uri().host() != Some("track") {
        return not_found();
    }
    let track_id = request.uri().path().trim_matches('/');
    if track_id.is_empty() || track_id.contains('/') {
        return not_found();
    }
    let Some(state) = context.app_handle().try_state::<AppState>() else {
        return not_found();
    };
    let Ok(database) = aevio_persistence::database::connect(&state.database_path) else {
        return not_found();
    };
    let Ok(Some(track)) =
        aevio_persistence::database::repositories::music_tracks::MusicTrackRepository::new(
            &database,
        )
        .by_id(track_id)
    else {
        return not_found();
    };
    if track.kind != "local" {
        return not_found();
    }
    let Some(file_path) = track.file_path else {
        return not_found();
    };
    let Ok(path) = std::fs::canonicalize(file_path) else {
        return not_found();
    };
    let Some(root) = state.database_path.parent().map(|path| path.join("music")) else {
        return not_found();
    };
    let Ok(root) = std::fs::canonicalize(root) else {
        return not_found();
    };
    if !path.starts_with(root) {
        return not_found();
    }
    let Ok(metadata) = std::fs::metadata(&path) else {
        return not_found();
    };
    if !metadata.is_file() || metadata.len() > MAX_AUDIO_BYTES {
        return not_found();
    }
    let Ok(body) = std::fs::read(&path) else {
        return not_found();
    };
    tauri::http::Response::builder()
        .header("Content-Type", content_type(&path))
        .header("Content-Length", body.len().to_string())
        .header("Content-Disposition", "inline")
        .header("X-Content-Type-Options", "nosniff")
        .header("Cache-Control", "no-store")
        .body(body)
        .expect("music response headers are valid")
}

fn content_type(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "flac" => "audio/flac",
        "aac" => "audio/aac",
        _ => "application/octet-stream",
    }
}
