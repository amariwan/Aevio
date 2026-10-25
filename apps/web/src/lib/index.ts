// Barrel file: controlled public API for shared lib modules.
export * from "./activity";
export * from "./api";
export * from "./breaks";
export * from "./clients";
export * from "./confidence";
export * from "./dates";
export * from "./entries";
export * from "./format";
export * from "./invoices";
export * from "./palette";
export * from "./platform";
export * from "./settings";
// `formatDuration` collides with `./format`; re-export the rest explicitly.
export type { Timer, TimerSegment } from "./timers";
export { elapsedMs } from "./timers";
export * from "./timesheetGrid";
export * from "./types";
