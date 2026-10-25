use serde::{Deserialize, Serialize};

pub const DEFAULT_TRACKING_START: &str = "07:00";
pub const DEFAULT_TRACKING_END: &str = "19:00";

pub(crate) fn parse_time_minutes(s: &str) -> Option<u32> {
    let mut parts = s.split(':');
    let h: u32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    if h < 24 && m < 60 && parts.next().is_none() {
        Some(h * 60 + m)
    } else {
        None
    }
}

pub(crate) fn normalize_time_str(s: &str, default_val: &str) -> String {
    if let Some(mins) = parse_time_minutes(s) {
        format!("{:02}:{:02}", mins / 60, mins % 60)
    } else {
        default_val.to_string()
    }
}

pub fn is_time_in_window(hour: u32, minute: u32, start_str: &str, end_str: &str) -> bool {
    let start = match parse_time_minutes(start_str) {
        Some(mins) => mins,
        None => return false,
    };
    let end = match parse_time_minutes(end_str) {
        Some(mins) => mins,
        None => return false,
    };

    if start == end {
        return false;
    }

    let current = hour * 60 + minute;
    if start < end {
        current >= start && current < end
    } else {
        current >= start || current < end
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DaySchedule {
    pub enabled: bool,
    pub start: String,
    pub end: String,
}

impl Default for DaySchedule {
    fn default() -> Self {
        Self {
            enabled: true,
            start: DEFAULT_TRACKING_START.to_string(),
            end: DEFAULT_TRACKING_END.to_string(),
        }
    }
}

impl DaySchedule {
    pub fn normalized(mut self) -> Self {
        self.start = normalize_time_str(&self.start, DEFAULT_TRACKING_START);
        self.end = normalize_time_str(&self.end, DEFAULT_TRACKING_END);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackingHours {
    pub enabled: bool,
    pub per_day: bool,
    pub default_start: String,
    pub default_end: String,
    pub monday: DaySchedule,
    pub tuesday: DaySchedule,
    pub wednesday: DaySchedule,
    pub thursday: DaySchedule,
    pub friday: DaySchedule,
    pub saturday: DaySchedule,
    pub sunday: DaySchedule,
}

impl Default for TrackingHours {
    fn default() -> Self {
        Self {
            enabled: true,
            per_day: false,
            default_start: DEFAULT_TRACKING_START.to_string(),
            default_end: DEFAULT_TRACKING_END.to_string(),
            monday: DaySchedule::default(),
            tuesday: DaySchedule::default(),
            wednesday: DaySchedule::default(),
            thursday: DaySchedule::default(),
            friday: DaySchedule::default(),
            saturday: DaySchedule::default(),
            sunday: DaySchedule::default(),
        }
    }
}

impl TrackingHours {
    pub fn normalized(mut self) -> Self {
        self.default_start = normalize_time_str(&self.default_start, DEFAULT_TRACKING_START);
        self.default_end = normalize_time_str(&self.default_end, DEFAULT_TRACKING_END);
        self.monday = self.monday.normalized();
        self.tuesday = self.tuesday.normalized();
        self.wednesday = self.wednesday.normalized();
        self.thursday = self.thursday.normalized();
        self.friday = self.friday.normalized();
        self.saturday = self.saturday.normalized();
        self.sunday = self.sunday.normalized();

        if !self.per_day {
            let single = DaySchedule {
                enabled: true,
                start: self.default_start.clone(),
                end: self.default_end.clone(),
            };
            self.monday = single.clone();
            self.tuesday = single.clone();
            self.wednesday = single.clone();
            self.thursday = single.clone();
            self.friday = single.clone();
            self.saturday = single.clone();
            self.sunday = single;
        }

        self
    }

    pub fn is_inside_window(&self, now_ms: u64) -> bool {
        use chrono::{DateTime, Datelike, Local, Timelike, Weekday};

        if !self.enabled {
            return true;
        }

        let dt = match DateTime::from_timestamp_millis(now_ms as i64) {
            Some(utc) => utc.with_timezone(&Local),
            None => return true,
        };

        let (start_str, end_str) = if self.per_day {
            let day = match dt.weekday() {
                Weekday::Mon => &self.monday,
                Weekday::Tue => &self.tuesday,
                Weekday::Wed => &self.wednesday,
                Weekday::Thu => &self.thursday,
                Weekday::Fri => &self.friday,
                Weekday::Sat => &self.saturday,
                Weekday::Sun => &self.sunday,
            };
            if !day.enabled {
                return false;
            }
            (&day.start, &day.end)
        } else {
            (&self.default_start, &self.default_end)
        };

        is_time_in_window(dt.hour(), dt.minute(), start_str, end_str)
    }
}
