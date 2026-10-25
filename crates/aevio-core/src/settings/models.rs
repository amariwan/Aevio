//! User preferences - palette, mode, size mode, close behavior, tray, retention.
//!
//! Storage policy: a single JSON file under the XDG config home
//! (`$XDG_CONFIG_HOME/aevio/settings.json`, or `~/.config/aevio/...`).
//! Preferences are config, not data: they survive deleting the app-data
//! directory, and they are portable between machines the way `.dotfiles` are.
//!
//! The file is written atomically (temp + rename) and a corrupt file is moved
//! aside rather than silently reset, matching `timers.rs`.

use serde::{Deserialize, Serialize};

use super::breaks::BreakSettings;
use super::tracking::TrackingHours;

/// 0 means keep everything. Anything else is a day count, clamped to a decade
/// so a typo cannot ask the sweeper to walk a table for a century.
pub const MAX_RETENTION_DAYS: u32 = 3650;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Palette {
    #[default]
    Zinc,
    Forest,
    Ocean,
    Violet,
    Sunset,
    Mono,
    Claude,
    Neobrutualism,
    Supabase,
    Vercel,
    Notebook,
    #[serde(rename = "light-green")]
    LightGreen,
    Zen,
    #[serde(rename = "astro-vista")]
    AstroVista,
    Whatsapp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    #[default]
    System,
    Light,
    Dark,
}

/// The language used for user-facing application text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Language {
    #[default]
    System,
    De,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SizeMode {
    Compact,
    #[default]
    Normal,
    Relaxed,
    VeryRelaxed,
}

/// How panels are drawn: spaced rounded cards, or edge-to-edge sections split
/// by plain dividers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Shape {
    #[default]
    Rounded,
    Sharper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum CloseBehavior {
    Quit,
    #[default]
    Hide,
}

/// Placement of app-owned reminder, live-break, and welcome-back panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum NotificationPlacement {
    TopLeft,
    TopRight,
    #[serde(alias = "centerMiddle")]
    TopCenter,
    BottomLeft,
    #[default]
    BottomRight,
}

/// Reminds once per current stopwatch run, rather than accumulated time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StopwatchReminder {
    pub enabled: bool,
    pub after_minutes: u32,
}

impl Default for StopwatchReminder {
    fn default() -> Self {
        Self {
            enabled: true,
            after_minutes: 180,
        }
    }
}

/// What the AI suggests for each entry (Aevio's "Suggestion level").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AiSuggest {
    Category,
    #[default]
    CategoryProject,
}

/// Auto-accept thresholds below this would approve too many wrong entries to
/// be useful; the UI offers 85-99.
pub const MIN_AUTO_ACCEPT_PERCENT: u8 = 50;
pub const MAX_AUTO_ACCEPT_PERCENT: u8 = 100;
/// The work week the Calendar, My Timesheet, and Timesheets measure against.
pub const DEFAULT_WEEKLY_TARGET_HOURS: u16 = 40;
pub const MAX_WEEKLY_TARGET_HOURS: u16 = 168;

/// Every field is `#[serde(default)]`, so a settings file written by an older
/// build loads with new fields defaulted instead of failing to parse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub palette: Palette,
    pub mode: Mode,
    pub language: Language,
    pub shape: Shape,
    pub size_mode: SizeMode,
    /// What the window's close button does while the tray is enabled.
    pub close_behavior: CloseBehavior,
    pub tray_enabled: bool,
    /// Activity history older than this many days is deleted. 0 = forever.
    pub retention_days: u32,
    pub ai_suggest: AiSuggest,
    /// Approve an entry without review when every suggested field clears
    /// `auto_accept_percent`.
    pub auto_accept: bool,
    pub auto_accept_percent: u8,
    /// Appended to the Foundation Model's instructions.
    pub ai_custom_prompt: String,
    /// AI provider: "none", "ollama", or "openai".
    pub ai_provider: String,
    /// Base URL for the AI provider (e.g. http://localhost:11434).
    pub ai_base_url: String,
    /// Model name for the AI provider.
    pub ai_model: String,
    /// Optional API key for the AI provider.
    pub ai_api_key: String,
    /// Expected work hours per week. A day's target is a fifth of it.
    pub weekly_target_hours: u16,
    pub tracking_hours: TrackingHours,
    pub breaks: BreakSettings,
    pub stopwatch_reminder: StopwatchReminder,
    pub notification_placement: NotificationPlacement,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            palette: Palette::default(),
            mode: Mode::default(),
            language: Language::default(),
            shape: Shape::default(),
            size_mode: SizeMode::default(),
            close_behavior: CloseBehavior::Hide,
            tray_enabled: true,
            retention_days: 0,
            ai_suggest: AiSuggest::CategoryProject,
            auto_accept: true,
            auto_accept_percent: 95,
            ai_custom_prompt: String::new(),
            ai_provider: String::new(),
            ai_base_url: String::new(),
            ai_model: String::new(),
            ai_api_key: String::new(),
            weekly_target_hours: DEFAULT_WEEKLY_TARGET_HOURS,
            tracking_hours: TrackingHours::default(),
            breaks: BreakSettings::default(),
            stopwatch_reminder: StopwatchReminder::default(),
            notification_placement: NotificationPlacement::default(),
        }
    }
}
