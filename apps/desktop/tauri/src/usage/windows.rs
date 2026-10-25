pub struct Metadata {
    pub platform: &'static str,
    pub native_screen_time_available: bool,
    pub native_screen_time_note: &'static str,
}

pub fn metadata() -> Metadata {
    Metadata {
        platform: "windows",
        native_screen_time_available: false,
        native_screen_time_note: "Windows has no supported general desktop Screen Time report API for arbitrary applications. Aevio uses native foreground-window and session-lock signals.",
    }
}
