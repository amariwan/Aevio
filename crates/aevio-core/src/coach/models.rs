use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Plan {
    pub day: String,
    pub time_zone: String,
    pub goal: String,
    pub notes: String,
    pub available_minutes: u32,
    pub energy: Option<u8>,
    pub state: String,
    pub revision: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Review {
    pub outcome: String,
    pub worked: String,
    pub obstacles: String,
    pub tomorrow: String,
    pub revision: u32,
}

impl Default for Review {
    fn default() -> Self {
        Self {
            outcome: "not_reviewed".into(),
            worked: String::new(),
            obstacles: String::new(),
            tomorrow: String::new(),
            revision: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preferences {
    pub enabled: bool,
    pub morning_planning: bool,
    pub reminders: bool,
    pub automatic_hints: bool,
    pub ai_enabled: bool,
    pub memory_enabled: bool,
    pub style: Style,
    pub quiet_start: u16,
    pub quiet_end: u16,
    pub cooldown_minutes: u16,
    pub review_minute: u16,
    pub visual_timer: bool,
    #[serde(default)]
    pub widget_context: bool,
    #[serde(default = "default_widget_visible")]
    pub widget_visible: bool,
    #[serde(default)]
    pub planning_buffer_minutes: u16,
    #[serde(default)]
    pub personal_intelligence_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    Minimal,
    #[default]
    Calm,
    Supportive,
    Structured,
    Direct,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRef {
    pub source: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogTask {
    pub reminder_at: Option<u64>,
    pub task: TaskRef,
    pub title: String,
    pub notes: String,
    pub done: bool,
    pub priority: i64,
    pub estimate_minutes: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Step {
    pub title: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedTask {
    pub reminder_at: Option<u64>,
    pub id: String,
    pub task: TaskRef,
    pub title: String,
    pub notes: String,
    pub done: bool,
    pub priority: i64,
    pub estimate_minutes: u32,
    pub important: bool,
    pub position: i64,
    pub steps: Vec<Step>,
    pub actual_ms: u64,
    pub postponements: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    pub day: String,
    pub time_zone: String,
    pub plan: Option<Plan>,
    pub review: Review,
    pub tasks: Vec<PlannedTask>,
    pub catalog: Vec<CatalogTask>,
    pub carryover: Vec<PlannedTask>,
    pub preferences: Preferences,
    pub tracked_ms: u64,
    pub unassigned_ms: u64,
    pub planned_minutes: u32,
    pub overbooked_minutes: u32,
    pub changes: u32,
    pub time_entries: Vec<TimeEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Command {
    SavePlan {
        plan: Plan,
    },
    SaveReview {
        review: Review,
        revision: u32,
    },
    AddTask {
        task: TaskRef,
        estimate_minutes: u32,
        important: bool,
    },
    CreateTask {
        reminder_at: Option<u64>,
        title: String,
        notes: String,
        priority: u8,
        estimate_minutes: u32,
        important: bool,
    },
    EditTask {
        reminder_at: Option<u64>,
        id: String,
        title: String,
        notes: String,
        priority: u8,
        estimate_minutes: u32,
        important: bool,
    },
    CompleteTask {
        id: String,
        done: bool,
    },
    MoveTask {
        id: String,
        target_day: String,
    },
    RemoveTask {
        id: String,
    },
    Reorder {
        ids: Vec<String>,
    },
    SaveSteps {
        id: String,
        steps: Vec<Step>,
    },
    LinkTime {
        id: String,
        entry_id: String,
    },
    UnlinkTime {
        entry_id: String,
    },
    SavePreferences {
        preferences: Preferences,
    },
    EraseCoach,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssistRequest {
    pub title: String,
    pub reason: String,
    pub granularity: u8,
    pub estimate_minutes: u32,
    pub previous_steps: Vec<String>,
    pub use_ai: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Proposal {
    pub steps: Vec<String>,
    pub next_step: String,
    pub source: String,
    pub note: Option<String>,
    pub prompt_version: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeEntry {
    pub id: String,
    pub description: String,
    pub duration_ms: u64,
    pub assigned_to: Option<String>,
}

fn default_widget_visible() -> bool {
    true
}
