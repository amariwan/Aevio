//! Full-monitor break window. It blocks the active display without locking
//! the operating-system session.

use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

pub const LABEL: &str = "break-screen";

pub fn sync(app: &AppHandle, visible: bool) {
    let window = app.get_webview_window(LABEL);
    if !visible {
        if let Some(window) = window {
            let _ = window.hide();
        }
        return;
    }

    let window = match window {
        Some(window) => window,
        None => match create(app) {
            Ok(window) => window,
            Err(error) => {
                eprintln!("could not open the full-screen break window: {error}");
                return;
            }
        },
    };
    fit_to_active_monitor(app, &window);
    let _ = window.set_always_on_top(true);
    let _ = window.set_focusable(true);
    let _ = window.show();
    let _ = window.set_focus();
}

fn fit_to_active_monitor(app: &AppHandle, window: &WebviewWindow) {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|point| app.monitor_from_point(point.x, point.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        return;
    };
    let scale = monitor.scale_factor();
    let size = monitor.size();
    let position = monitor.position();
    let _ = window.set_size(LogicalSize::new(
        f64::from(size.width) / scale,
        f64::from(size.height) / scale,
    ));
    let _ = window.set_position(PhysicalPosition::new(position.x, position.y));
}

fn create(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let language = crate::i18n::from_settings(&app.state::<crate::AppState>().settings_snapshot());
    let title = match language {
        crate::i18n::ResolvedLanguage::De => "Aevio Pause",
        crate::i18n::ResolvedLanguage::En => "Aevio break",
    };
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title(title)
        .inner_size(1280.0, 800.0)
        .decorations(false)
        .transparent(true)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .focused(true)
        .focusable(true)
        .build()
}
