CREATE TABLE IF NOT EXISTS coach_focus (
 id TEXT PRIMARY KEY, daily_task_id TEXT NOT NULL REFERENCES daily_tasks(id) ON DELETE CASCADE,
 segment_id INTEGER NOT NULL REFERENCES segments(id) ON DELETE CASCADE,
 goal TEXT NOT NULL, next_step TEXT NOT NULL, planned_minutes INTEGER NOT NULL CHECK(planned_minutes BETWEEN 2 AND 180),
 body_doubling INTEGER NOT NULL DEFAULT 0, state TEXT NOT NULL CHECK(state IN ('running','interrupted','ended')),
 started_at INTEGER NOT NULL, ended_at INTEGER, reminded_at INTEGER
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_coach_focus_running ON coach_focus(state) WHERE state='running';
CREATE INDEX IF NOT EXISTS idx_coach_focus_task ON coach_focus(daily_task_id,started_at);
CREATE TABLE IF NOT EXISTS focus_interruptions (
 id INTEGER PRIMARY KEY, focus_id TEXT NOT NULL REFERENCES coach_focus(id) ON DELETE CASCADE,
 at INTEGER NOT NULL, reason TEXT NOT NULL
);
