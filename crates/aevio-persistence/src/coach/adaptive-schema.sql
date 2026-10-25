CREATE TABLE IF NOT EXISTS coach_memory (
 id TEXT PRIMARY KEY, content TEXT NOT NULL, kind TEXT NOT NULL CHECK(kind IN ('explicit','confirmed')),
 updated_at INTEGER NOT NULL
);
