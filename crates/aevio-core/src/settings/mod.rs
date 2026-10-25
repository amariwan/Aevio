//! User settings: domain types, normalization, and durable persistence.

pub mod breaks;
mod models;
pub mod tracking;
mod validation;

pub use breaks::{
    BreakSettings, DEFAULT_BREAK_MINUTES, DEFAULT_SNOOZE_MINUTES, DEFAULT_WORK_MINUTES,
    MAX_BREAK_MESSAGE_CHARS, MAX_SCHEDULED_BREAKS, SNOOZE_CHOICES, ScheduledBreak, Weekdays,
};
pub use models::{
    AiSuggest, CloseBehavior, DEFAULT_WEEKLY_TARGET_HOURS, Language, MAX_AUTO_ACCEPT_PERCENT,
    MAX_RETENTION_DAYS, MAX_WEEKLY_TARGET_HOURS, MIN_AUTO_ACCEPT_PERCENT, Mode,
    NotificationPlacement, Palette, Settings, Shape, SizeMode, StopwatchReminder,
};
pub use tracking::{
    DEFAULT_TRACKING_END, DEFAULT_TRACKING_START, DaySchedule, TrackingHours, is_time_in_window,
};
