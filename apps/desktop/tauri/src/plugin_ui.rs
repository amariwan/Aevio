use tauri::{Manager, Runtime};

use crate::AppState;

pub fn response<R: Runtime>(
    context: tauri::UriSchemeContext<'_, R>,
    request: tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    let not_found = || {
        tauri::http::Response::builder()
            .status(tauri::http::StatusCode::NOT_FOUND)
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(b"plugin UI resource not found".to_vec())
            .expect("static plugin UI response is valid")
    };
    let Some(plugin_id) = request.uri().host() else {
        return not_found();
    };
    let relative_path = request.uri().path().trim_start_matches('/');
    let Some(state) = context.app_handle().try_state::<AppState>() else {
        return not_found();
    };
    let Ok(database) = aevio_persistence::database::connect(&state.database_path) else {
        return not_found();
    };
    let Ok(path) =
        aevio_services::plugins_artifacts::ui_asset_path(&database, plugin_id, relative_path)
    else {
        return not_found();
    };
    let Ok(body) = std::fs::read(path) else {
        return not_found();
    };
    let content_type = match relative_path.rsplit('.').next().unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    };
    tauri::http::Response::builder()
        .header("Content-Type", content_type)
        .header("Content-Security-Policy", plugin_ui_csp())
        .header("X-Content-Type-Options", "nosniff")
        .header("Cross-Origin-Resource-Policy", "same-origin")
        .header("Referrer-Policy", "no-referrer")
        .header("Cache-Control", "no-store")
        .body(body)
        .expect("static plugin UI response headers are valid")
}

fn plugin_ui_csp() -> &'static str {
    "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'; sandbox allow-scripts"
}
