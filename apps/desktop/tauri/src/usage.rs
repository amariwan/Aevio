//! Cross-platform usage reporting.
//!
//! The source of truth is the native foreground-window sampler. macOS does
//! not expose the Screen Time report database through a supported desktop API,
//! so the response explicitly reports that limitation instead of scraping
//! private system data.

use std::collections::HashMap;

use serde::Serialize;

use crate::{activity, permissions};

#[cfg_attr(target_os = "macos", path = "usage/macos.rs")]
#[cfg_attr(windows, path = "usage/windows.rs")]
#[cfg_attr(not(any(target_os = "macos", windows)), path = "usage/unsupported.rs")]
mod platform;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageProvider {
    pub platform: String,
    pub source: String,
    pub native_screen_time_available: bool,
    pub native_screen_time_note: String,
    pub permissions: Vec<UsagePermission>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsagePermission {
    pub id: String,
    pub granted: bool,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageItem {
    pub name: String,
    pub kind: String,
    pub identifier: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub since_ms: u64,
    pub until_ms: u64,
    pub active_ms: u64,
    pub idle_ms: u64,
    pub break_ms: u64,
    pub unreviewed: u64,
    pub current: Option<String>,
    pub items: Vec<UsageItem>,
    pub provider: UsageProvider,
}

pub fn snapshot_for(app: &tauri::AppHandle, since_ms: u64) -> Result<UsageSnapshot, String> {
    let activity = activity::snapshot_for(app, since_ms)?;
    let now = crate::timers::now_epoch_ms();
    let mut items: HashMap<(String, String, Option<String>), u64> = HashMap::new();

    for segment in &activity.segments {
        if segment.kind == activity::KIND_BREAK {
            continue;
        }
        let end = segment.ended_at.unwrap_or(now).min(now);
        let duration_ms = end.saturating_sub(segment.started_at.max(since_ms));
        if duration_ms == 0 {
            continue;
        }
        let (kind, name, identifier) = match segment.domain.as_deref() {
            Some(domain) => (
                "website".to_string(),
                domain.to_string(),
                segment.url.clone(),
            ),
            None => (
                "app".to_string(),
                segment.app.clone(),
                segment.bundle_id.clone(),
            ),
        };
        *items.entry((kind, name, identifier)).or_default() += duration_ms;
    }

    let mut items: Vec<_> = items
        .into_iter()
        .map(|((kind, name, identifier), duration_ms)| UsageItem {
            name,
            kind,
            identifier,
            duration_ms,
        })
        .collect();
    items.sort_by_key(|item| std::cmp::Reverse(item.duration_ms));

    Ok(UsageSnapshot {
        since_ms,
        until_ms: now,
        active_ms: activity.tracked_ms,
        idle_ms: activity.idle_ms,
        break_ms: activity.break_ms,
        unreviewed: activity.unreviewed,
        current: activity.current.map(|segment| segment.app),
        items,
        provider: provider(),
    })
}

fn provider() -> UsageProvider {
    let metadata = platform::metadata();
    let permissions = permissions::check_permissions()
        .into_iter()
        .map(|permission| UsagePermission {
            id: permission.id,
            granted: permission.granted,
            required: permission.required,
        })
        .collect();
    UsageProvider {
        platform: metadata.platform.to_string(),
        source: "native-foreground-sampling".to_string(),
        native_screen_time_available: metadata.native_screen_time_available,
        native_screen_time_note: metadata.native_screen_time_note.to_string(),
        permissions,
    }
}
