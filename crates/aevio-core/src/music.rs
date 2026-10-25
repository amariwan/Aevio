//! The built-in focus music catalog. Playback remains a frontend concern;
//! this module owns the durable catalog the desktop app exposes to it.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicTrack {
    pub id: String,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub source_url: Option<String>,
}
