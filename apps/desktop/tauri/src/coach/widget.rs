//! Existing widget window; Tauri implements these operations on all desktops.
use tauri::{AppHandle, Manager};
pub fn set_visible(app: &AppHandle, visible: bool) -> Result<(), String> {
    if visible {
        crate::widget::init(app).map_err(|e| e.to_string())?;
    }
    if let Some(window) = app.get_webview_window(crate::widget::LABEL) {
        if visible {
            window.show()
        } else {
            window.hide()
        }
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
