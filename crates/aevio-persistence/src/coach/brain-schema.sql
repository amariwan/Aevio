CREATE TABLE IF NOT EXISTS brain_dump_entries (
 id TEXT PRIMARY KEY, content TEXT NOT NULL, category TEXT NOT NULL CHECK(category IN ('task','idea','reminder','note','question','later')),
 converted_task_id TEXT REFERENCES coach_tasks(id) ON DELETE SET NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS coach_routines (id TEXT PRIMARY KEY,title TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS coach_routine_items (
 routine_id TEXT NOT NULL REFERENCES coach_routines(id) ON DELETE CASCADE, position INTEGER NOT NULL,
 title TEXT NOT NULL, estimate_minutes INTEGER NOT NULL CHECK(estimate_minutes BETWEEN 0 AND 1440),
 PRIMARY KEY(routine_id,position)
);
CREATE TABLE IF NOT EXISTS coach_routine_runs (
 routine_id TEXT NOT NULL REFERENCES coach_routines(id) ON DELETE CASCADE,
 day TEXT NOT NULL REFERENCES daily_plans(day) ON DELETE CASCADE, PRIMARY KEY(routine_id,day)
);
