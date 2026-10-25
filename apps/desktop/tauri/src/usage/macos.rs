pub struct Metadata {
    pub platform: &'static str,
    pub native_screen_time_available: bool,
    pub native_screen_time_note: &'static str,
}

pub fn metadata() -> Metadata {
    Metadata {
        platform: "macos",
        native_screen_time_available: false,
        native_screen_time_note: "macOS does not provide a supported desktop API for reading the System Settings Screen Time report. Aevio uses native foreground-window, idle, lock, sleep, and browser automation signals.",
    }
}
