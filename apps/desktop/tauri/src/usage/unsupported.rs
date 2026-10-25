pub struct Metadata {
    pub platform: &'static str,
    pub native_screen_time_available: bool,
    pub native_screen_time_note: &'static str,
}

pub fn metadata() -> Metadata {
    Metadata {
        platform: "unsupported",
        native_screen_time_available: false,
        native_screen_time_note:
            "This platform does not currently expose a supported native usage provider in Aevio.",
    }
}
