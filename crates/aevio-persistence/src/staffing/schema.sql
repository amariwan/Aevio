CREATE TABLE IF NOT EXISTS staffing_freelancers (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, email TEXT, status TEXT NOT NULL DEFAULT 'lead',
  location TEXT, timezone TEXT NOT NULL DEFAULT 'UTC', seniority TEXT, bio TEXT,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS staffing_skills (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, category TEXT);
CREATE TABLE IF NOT EXISTS staffing_freelancer_skills (
  freelancer_id TEXT NOT NULL REFERENCES staffing_freelancers(id) ON DELETE CASCADE,
  skill_id TEXT NOT NULL REFERENCES staffing_skills(id) ON DELETE CASCADE,
  level INTEGER NOT NULL DEFAULT 1, PRIMARY KEY(freelancer_id, skill_id)
);
CREATE TABLE IF NOT EXISTS staffing_availability (
  id TEXT PRIMARY KEY, freelancer_id TEXT NOT NULL REFERENCES staffing_freelancers(id) ON DELETE CASCADE,
  start_date TEXT NOT NULL, end_date TEXT NOT NULL, available_minutes INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'available'
);
CREATE TABLE IF NOT EXISTS staffing_requests (
  id TEXT PRIMARY KEY, project_id TEXT REFERENCES projects(id), role TEXT NOT NULL,
  skills_json TEXT NOT NULL DEFAULT '[]', start_date TEXT NOT NULL, end_date TEXT NOT NULL,
  requested_minutes INTEGER NOT NULL DEFAULT 0, status TEXT NOT NULL DEFAULT 'open',
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS staffing_assignments (
  id TEXT PRIMARY KEY, freelancer_id TEXT NOT NULL REFERENCES staffing_freelancers(id),
  project_id TEXT REFERENCES projects(id), request_id TEXT REFERENCES staffing_requests(id), role TEXT NOT NULL,
  start_date TEXT NOT NULL, end_date TEXT NOT NULL, planned_minutes INTEGER NOT NULL DEFAULT 0,
  buy_rate_cents INTEGER NOT NULL DEFAULT 0, sell_rate_cents INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'planned'
);
CREATE TABLE IF NOT EXISTS staffing_contracts (
  id TEXT PRIMARY KEY, freelancer_id TEXT NOT NULL REFERENCES staffing_freelancers(id), kind TEXT NOT NULL,
  start_date TEXT NOT NULL, end_date TEXT, notice_days INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'draft', document_ref TEXT
);
CREATE TABLE IF NOT EXISTS staffing_pipeline (
  id TEXT PRIMARY KEY, freelancer_id TEXT NOT NULL REFERENCES staffing_freelancers(id) ON DELETE CASCADE,
  stage TEXT NOT NULL DEFAULT 'lead', next_action_at INTEGER, notes TEXT
);
CREATE INDEX IF NOT EXISTS idx_staffing_availability_period ON staffing_availability(start_date,end_date,status);
CREATE INDEX IF NOT EXISTS idx_staffing_requests_status ON staffing_requests(status,start_date);
CREATE INDEX IF NOT EXISTS idx_staffing_assignments_period ON staffing_assignments(start_date,end_date,status);
CREATE INDEX IF NOT EXISTS idx_staffing_pipeline_stage ON staffing_pipeline(stage,next_action_at);
