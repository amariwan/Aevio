//! Native daily productivity summary window.

use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

pub const LABEL: &str = "summary";
const WIDTH: f64 = 420.0;
const HEIGHT: f64 = 555.0;
const EDGE: i32 = 24;

pub fn open(app: &AppHandle) -> tauri::Result<()> {
    let window = if let Some(window) = app.get_webview_window(LABEL) {
        window
    } else {
        let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
            .title("Aevio Productivity Coach")
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
        make_global(&window);
        window
    };
    place(&window);
    window.show()?;
    window.set_focus()?;
    Ok(())
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.hide();
    }
}

fn place(window: &WebviewWindow) {
    let Ok(Some(monitor)) = window.primary_monitor() else {
        return;
    };
    let area = monitor.work_area();
    let scale = monitor.scale_factor();
    let width = (WIDTH * scale).round() as i32;
    let height = (HEIGHT * scale).round() as i32;
    let x = area.position.x + area.size.width as i32 - width - EDGE;
    let y = area.position.y + area.size.height as i32 - height - EDGE;
    let _ = window.set_position(PhysicalPosition::new(x, y));
    let _ = window.set_size(LogicalSize::new(WIDTH, HEIGHT));
}

#[cfg(target_os = "macos")]
fn make_global(window: &WebviewWindow) {
    use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
    let target = window.clone();
    let _ = window.run_on_main_thread(move || {
        let Ok(pointer) = target.ns_window() else {
            return;
        };
        // SAFETY: ns_window returns the live NSWindow on AppKit's main thread.
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
fn make_global(window: &WebviewWindow) {
    let _ = window.set_visible_on_all_workspaces(true);
}
