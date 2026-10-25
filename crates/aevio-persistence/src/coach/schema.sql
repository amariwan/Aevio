-- Legacy personal tasks remain the source of truth; projects retain tasks.id.
CREATE TABLE IF NOT EXISTS coach_tasks (
 id TEXT PRIMARY KEY, title TEXT NOT NULL, notes TEXT,
 priority INTEGER NOT NULL DEFAULT 3, status TEXT NOT NULL DEFAULT 'open',
 due_at INTEGER, reminder_at INTEGER, estimate_ms INTEGER,
 created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, completed_at INTEGER, notified_at INTEGER
);
CREATE INDEX IF NOT EXISTS idx_coach_tasks_status_due ON coach_tasks(status,due_at,priority);
CREATE TABLE IF NOT EXISTS daily_plans (
 day TEXT PRIMARY KEY, time_zone TEXT NOT NULL, goal TEXT NOT NULL DEFAULT '',
 notes TEXT NOT NULL DEFAULT '', available_minutes INTEGER NOT NULL CHECK(available_minutes BETWEEN 0 AND 1440),
 energy INTEGER CHECK(energy BETWEEN 1 AND 5), state TEXT NOT NULL CHECK(state IN ('planned','skipped')),
 revision INTEGER NOT NULL DEFAULT 1, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS daily_tasks (
 id TEXT PRIMARY KEY, day TEXT NOT NULL REFERENCES daily_plans(day) ON DELETE CASCADE,
 personal_task_id TEXT REFERENCES coach_tasks(id) ON DELETE CASCADE,
 project_task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
 estimate_minutes INTEGER NOT NULL CHECK(estimate_minutes BETWEEN 0 AND 1440),
 important INTEGER NOT NULL DEFAULT 0 CHECK(important IN (0,1)), position INTEGER NOT NULL,
 postponements INTEGER NOT NULL DEFAULT 0,
 CHECK((personal_task_id IS NULL) != (project_task_id IS NULL)),
 UNIQUE(day, personal_task_id), UNIQUE(day, project_task_id)
);
CREATE INDEX IF NOT EXISTS idx_daily_tasks_day ON daily_tasks(day,position);
CREATE TABLE IF NOT EXISTS task_steps (
 daily_task_id TEXT NOT NULL REFERENCES daily_tasks(id) ON DELETE CASCADE,
 position INTEGER NOT NULL, title TEXT NOT NULL, done INTEGER NOT NULL CHECK(done IN (0,1)),
 PRIMARY KEY(daily_task_id,position)
);
CREATE TABLE IF NOT EXISTS daily_reviews (
 day TEXT PRIMARY KEY REFERENCES daily_plans(day) ON DELETE CASCADE,
 outcome TEXT NOT NULL CHECK(outcome IN ('achieved','partially_achieved','not_achieved','changed','not_reviewed')),
 worked TEXT NOT NULL, obstacles TEXT NOT NULL, tomorrow TEXT NOT NULL,
 revision INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS coach_preferences (id INTEGER PRIMARY KEY CHECK(id=1), value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS task_time_links (
 entry_id TEXT PRIMARY KEY REFERENCES time_entries(id) ON DELETE CASCADE,
 daily_task_id TEXT NOT NULL REFERENCES daily_tasks(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_task_time_links_task ON task_time_links(daily_task_id);
CREATE TABLE IF NOT EXISTS coach_interventions (
 id INTEGER PRIMARY KEY, day TEXT NOT NULL, kind TEXT NOT NULL, at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_coach_interventions_at ON coach_interventions(at);
