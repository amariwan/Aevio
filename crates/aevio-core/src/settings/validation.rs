//! Canonical settings normalization.

use super::models::{
    MAX_AUTO_ACCEPT_PERCENT, MAX_RETENTION_DAYS, MAX_WEEKLY_TARGET_HOURS, MIN_AUTO_ACCEPT_PERCENT,
    Settings,
};

const MAX_CUSTOM_PROMPT_CHARS: usize = 1_000;

impl Settings {
    /// The only place a value is allowed in. Keeps persistence and validation
    /// in one spot so a command can never write something unloadable.
    pub fn normalized(mut self) -> Self {
        self.retention_days = self.retention_days.min(MAX_RETENTION_DAYS);
        self.weekly_target_hours = self.weekly_target_hours.clamp(1, MAX_WEEKLY_TARGET_HOURS);
        self.auto_accept_percent = self
            .auto_accept_percent
            .clamp(MIN_AUTO_ACCEPT_PERCENT, MAX_AUTO_ACCEPT_PERCENT);
        if self.ai_custom_prompt.chars().count() > MAX_CUSTOM_PROMPT_CHARS {
            self.ai_custom_prompt = self
                .ai_custom_prompt
                .chars()
                .take(MAX_CUSTOM_PROMPT_CHARS)
                .collect();
        }
        self.tracking_hours = self.tracking_hours.normalized();
        self.breaks = self.breaks.normalized();
        self.stopwatch_reminder.after_minutes =
            self.stopwatch_reminder.after_minutes.clamp(30, 720);
        self
    }
}
