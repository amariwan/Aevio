use chrono::Weekday;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::tracking::{normalize_time_str, parse_time_minutes};

pub const DEFAULT_WORK_MINUTES: u16 = 50;
pub const DEFAULT_BREAK_MINUTES: u16 = 5;
pub const DEFAULT_SNOOZE_MINUTES: u16 = 5;
pub const SNOOZE_CHOICES: [u16; 3] = [5, 10, 15];
pub const MAX_BREAK_MESSAGE_CHARS: usize = 120;
pub const MAX_SCHEDULED_BREAKS: usize = 12;
const MAX_SCHEDULE_LABEL_CHARS: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Weekdays {
    pub mon: bool,
    pub tue: bool,
    pub wed: bool,
    pub thu: bool,
    pub fri: bool,
    pub sat: bool,
    pub sun: bool,
}

impl Default for Weekdays {
    fn default() -> Self {
        Self {
            mon: true,
            tue: true,
            wed: true,
            thu: true,
            fri: true,
            sat: false,
            sun: false,
        }
    }
}

impl Weekdays {
    pub fn includes(&self, weekday: Weekday) -> bool {
        match weekday {
            Weekday::Mon => self.mon,
            Weekday::Tue => self.tue,
            Weekday::Wed => self.wed,
            Weekday::Thu => self.thu,
            Weekday::Fri => self.fri,
            Weekday::Sat => self.sat,
            Weekday::Sun => self.sun,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScheduledBreak {
    pub id: String,
    pub label: String,
    pub at: String,
    pub minutes: u16,
    pub days: Weekdays,
    pub enabled: bool,
}

impl Default for ScheduledBreak {
    fn default() -> Self {
        Self {
            id: String::new(),
            label: "Break".to_string(),
            at: "12:30".to_string(),
            minutes: 30,
            days: Weekdays::default(),
            enabled: true,
        }
    }
}

impl ScheduledBreak {
    pub fn at_minutes(&self) -> u32 {
        parse_time_minutes(&self.at).unwrap_or(0)
    }

    pub(crate) fn normalized(mut self) -> Self {
        if self.id.trim().is_empty() {
            self.id = Uuid::now_v7().to_string();
        }
        let label: String = self
            .label
            .trim()
            .chars()
            .take(MAX_SCHEDULE_LABEL_CHARS)
            .collect();
        self.label = if label.is_empty() {
            "Break".to_string()
        } else {
            label
        };
        self.at = normalize_time_str(&self.at, "12:30");
        self.minutes = self.minutes.clamp(1, 180);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BreakSettings {
    pub enabled: bool,
    pub work_minutes: u16,
    pub break_minutes: u16,
    pub snooze_minutes: u16,
    pub message: String,
    pub pause_stopwatches: bool,
    pub chime: bool,
    pub schedules: Vec<ScheduledBreak>,
}

impl Default for BreakSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            work_minutes: DEFAULT_WORK_MINUTES,
            break_minutes: DEFAULT_BREAK_MINUTES,
            snooze_minutes: DEFAULT_SNOOZE_MINUTES,
            message: String::new(),
            pause_stopwatches: false,
            chime: false,
            schedules: Vec::new(),
        }
    }
}

impl BreakSettings {
    pub(crate) fn normalized(mut self) -> Self {
        self.work_minutes = self.work_minutes.clamp(15, 180);
        self.break_minutes = self.break_minutes.clamp(1, 60);
        if !SNOOZE_CHOICES.contains(&self.snooze_minutes) {
            self.snooze_minutes = DEFAULT_SNOOZE_MINUTES;
        }
        let message: String = self
            .message
            .trim()
            .chars()
            .take(MAX_BREAK_MESSAGE_CHARS)
            .collect();
        self.message = message.trim_end().to_string();
        self.schedules.truncate(MAX_SCHEDULED_BREAKS);
        self.schedules = self
            .schedules
            .into_iter()
            .map(ScheduledBreak::normalized)
            .collect();
        self
    }
}
