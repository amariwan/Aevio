// Feature-local Tauri IPC wrappers for timers.
// Delegates to the shared IPC layer; no business logic here.
export {
  addTimerSegment,
  createTimer,
  deleteTimer,
  deleteTimerSegment,
  listTimers,
  pauseTimer,
  renameTimer,
  resetTimer,
  startTimer,
  TIMERS_CHANGED,
  updateTimerSegment,
} from "@/lib/api";
