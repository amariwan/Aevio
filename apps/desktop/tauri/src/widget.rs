//! The always-visible compact timer widget.
//!
//! The widget is deliberately a separate window from the tray Pulse panel:
//! Pulse is transient and focus-driven, while this window remains visible
//! above other applications and across virtual desktops.

use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "widget";

const WIDTH: f64 = 420.0;
const HEIGHT: f64 = 54.0;
pub const EXPANDED_HEIGHT: f64 = 320.0;
const EDGE: i32 = 16;

/// Creates and shows the widget once during app startup.
pub fn init(app: &AppHandle) -> tauri::Result<()> {
    if let Some(state) = app.try_state::<crate::AppState>() {
        let _store = state
            .activity
            .lock()
            .map_err(|error| tauri::Error::Io(std::io::Error::other(error.to_string())))?;
        let database = aevio_persistence::database::connect(&state.database_path)
            .map_err(|error| tauri::Error::Io(std::io::Error::other(error.to_string())))?;
        let preferences =
            aevio_persistence::database::repositories::coach::CoachMetadataRepository::new(
                &database,
            )
            .preferences()
            .map_err(|error| tauri::Error::Io(std::io::Error::other(error)))?;
        if !preferences.widget_visible {
            return Ok(());
        }
    }
    if app.get_webview_window(LABEL).is_some() {
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("Aevio Timer")
        .inner_size(WIDTH, HEIGHT)
        .decorations(false)
        .transparent(true)
        .shadow(true)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .focused(false)
        .build()?;

    place(&window);
    make_global(&window);
    let _ = window.show();
    Ok(())
}

pub fn resize(app: &AppHandle, height: f64) {
    if let Some(window) = app.get_webview_window(LABEL) {
        // macOS ignores programmatic height changes for a non-resizable window
        // in some AppKit configurations. Keep user resizing disabled while
        // temporarily allowing this controlled layout transition.
        let _ = window.set_resizable(true);
        let _ = window.set_size(LogicalSize::new(
            WIDTH,
            height.clamp(HEIGHT, EXPANDED_HEIGHT),
        ));
        let _ = window.set_resizable(false);
    }
}

fn place(window: &tauri::WebviewWindow) {
    let Ok(Some(monitor)) = window.primary_monitor() else {
        return;
    };
    let area = monitor.work_area();
    let scale = monitor.scale_factor();
    let width = (WIDTH * scale).round() as i32;
    let x = area.position.x + ((area.size.width as i32 - width) / 2);
    let y = area.position.y + EDGE;
    let _ = window.set_position(PhysicalPosition::new(x, y));
    let _ = window.set_size(LogicalSize::new(WIDTH, HEIGHT));
}

#[cfg(target_os = "macos")]
fn make_global(window: &tauri::WebviewWindow) {
    use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};

    let target = window.clone();
    let _ = window.run_on_main_thread(move || {
        let Ok(pointer) = target.ns_window() else {
            return;
        };
        // SAFETY: `ns_window` returns the live NSWindow and this closure runs
        // on AppKit's main thread.
        let native = unsafe { &*pointer.cast::<NSWindow>() };
        native.setHidesOnDeactivate(false);
        native.setCollectionBehavior(
            native.collectionBehavior()
                | NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Stationary,
        );
    });
}

#[cfg(not(target_os = "macos"))]
fn make_global(window: &tauri::WebviewWindow) {
    let _ = window.set_visible_on_all_workspaces(true);
}
