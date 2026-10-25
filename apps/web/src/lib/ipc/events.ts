// Single TypeScript source of truth for Tauri event names.
// Must stay in sync with `pub const EVENT_*` in src-tauri/src (checked by
// scripts/check-events.mjs). Re-exported from ../api for backwards compat.
export const ACTIVITY_CHANGED = "activity-changed";
export const ACTIVITY_TICK = "activity-tick";
export const SETTINGS_CHANGED = "settings-changed";
export const ENTRIES_CHANGED = "entries-changed";
export const TIMERS_CHANGED = "timers-changed";
/** Payload: `{ entryId }`. */
export const SUGGESTION_READY = "suggestion-ready";
/** Payload: `AiStatus`. */
export const AI_STATUS_CHANGED = "ai-status-changed";
export const ENERGY_CHANGED = "energy-changed";
/** Payload: `CoachSnapshot`. */
export const COACH_DAY_CHANGED = "coach-day-changed";
export const COACH_CHANGED = "coach-changed";
/** Main window only: the Pulse panel asked for today's review queue. */
export const OPEN_REVIEW = "open-review";
/** Payload: `UpdateStatus`. */
export const UPDATE_STATUS = "update-status";
/** Main window only: the reminder asked for Settings > Notifications. */
export const OPEN_BREAK_SETTINGS = "open-break-settings";
/** Payload: `BreakState`. */
export const BREAK_STATE_CHANGED = "break-state-changed";
