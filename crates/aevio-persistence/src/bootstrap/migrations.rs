use std::path::Path;

use rusqlite::{Connection, Result};

/// The `user_version` `run_migrations` leaves a database at.
pub const SCHEMA_VERSION: i32 = 39;

/// Opens and migrates the database before a SeaORM connection is created.
/// Historical migration SQL and SQLite PRAGMAs are infrastructure exceptions.
#[allow(dead_code)]
pub(crate) fn initialize_database(path: &Path) -> Result<(), String> {
    let mut connection = Connection::open(path).map_err(|error| error.to_string())?;
    Connection::execute_batch(
        &connection,
        "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA optimize;",
    )
    .map_err(|error| error.to_string())?;
    run_migrations(&mut connection).map_err(|error| error.to_string())
}

pub fn run_migrations(conn: &mut Connection) -> Result<()> {
    let current_version: i32 = conn.query_row("PRAGMA user_version;", [], |row| row.get(0))?;

    if current_version < 1 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS segments (
              id         INTEGER PRIMARY KEY AUTOINCREMENT,
              app        TEXT    NOT NULL,
              title      TEXT    NOT NULL,
              kind       TEXT    NOT NULL,
              label      TEXT,
              started_at INTEGER NOT NULL,
              ended_at   INTEGER,
              reviewed   INTEGER NOT NULL DEFAULT 0,
              app_id     TEXT,
              bundle_id  TEXT,
              url        TEXT,
              domain     TEXT,
              entry_id   TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_segments_started_at ON segments (started_at);
            CREATE INDEX IF NOT EXISTS idx_segments_ended_at ON segments (ended_at);
            CREATE INDEX IF NOT EXISTS idx_segments_entry_id ON segments (entry_id);

            CREATE TABLE IF NOT EXISTS settings (
              key   TEXT PRIMARY KEY,
              value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS daily_rollups (
              day_epoch INTEGER NOT NULL,
              kind      TEXT    NOT NULL,
              total_ms  INTEGER NOT NULL,
              PRIMARY KEY (day_epoch, kind)
            );

            CREATE TABLE IF NOT EXISTS apps (
              id                  TEXT PRIMARY KEY,
              kind                TEXT NOT NULL,
              identifier          TEXT NOT NULL UNIQUE,
              display_name        TEXT NOT NULL,
              icon_png            BLOB,
              default_category_id TEXT,
              default_project_id  TEXT,
              excluded            INTEGER NOT NULL DEFAULT 0,
              first_seen          INTEGER NOT NULL,
              last_seen           INTEGER NOT NULL,
              created_at          INTEGER NOT NULL,
              updated_at          INTEGER NOT NULL,
              deleted_at          INTEGER
            );

            CREATE TABLE IF NOT EXISTS categories (
              id               TEXT PRIMARY KEY,
              name             TEXT NOT NULL,
              color            TEXT NOT NULL,
              description      TEXT,
              ai_prompt        TEXT,
              billable_default INTEGER NOT NULL DEFAULT 0,
              counts_as_work   INTEGER NOT NULL DEFAULT 1,
              archived         INTEGER NOT NULL DEFAULT 0,
              sort             INTEGER NOT NULL DEFAULT 0,
              created_at       INTEGER NOT NULL,
              updated_at       INTEGER NOT NULL,
              deleted_at       INTEGER
            );

            CREATE TABLE IF NOT EXISTS projects (
              id               TEXT PRIMARY KEY,
              client_id        TEXT,
              name             TEXT NOT NULL,
              color            TEXT NOT NULL,
              description      TEXT,
              ai_hints         TEXT,
              status           TEXT NOT NULL DEFAULT 'active',
              due_date         INTEGER,
              budget_kind      TEXT NOT NULL DEFAULT 'none',
              budget_value     REAL,
              budget_period    TEXT NOT NULL DEFAULT 'total',
              billable_default INTEGER NOT NULL DEFAULT 0,
              hourly_rate      REAL,
              created_at       INTEGER NOT NULL,
              updated_at       INTEGER NOT NULL,
              deleted_at       INTEGER
            );

            CREATE TABLE IF NOT EXISTS clients (
              id           TEXT PRIMARY KEY,
              name         TEXT NOT NULL,
              email        TEXT,
              address      TEXT,
              default_rate REAL,
              currency     TEXT,
              created_at   INTEGER NOT NULL,
              updated_at   INTEGER NOT NULL,
              deleted_at   INTEGER
            );

            CREATE TABLE IF NOT EXISTS rules (
              id          TEXT PRIMARY KEY,
              match_kind  TEXT NOT NULL,
              pattern     TEXT NOT NULL,
              category_id TEXT,
              project_id  TEXT,
              priority    INTEGER NOT NULL DEFAULT 0,
              enabled     INTEGER NOT NULL DEFAULT 1,
              origin      TEXT NOT NULL DEFAULT 'manual',
              created_at  INTEGER NOT NULL,
              updated_at  INTEGER NOT NULL,
              deleted_at  INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_rules_project ON rules (project_id);

            CREATE TABLE IF NOT EXISTS time_entries (
              id                 TEXT PRIMARY KEY,
              started_at         INTEGER NOT NULL,
              ended_at           INTEGER NOT NULL,
              description        TEXT NOT NULL,
              description_origin TEXT NOT NULL DEFAULT 'template',
              category_id        TEXT,
              project_id         TEXT,
              status             TEXT NOT NULL DEFAULT 'pending',
              approved_by        TEXT,
              source             TEXT NOT NULL DEFAULT 'auto',
              billable           INTEGER NOT NULL DEFAULT 0,
              invoice_id         TEXT,
              created_at         INTEGER NOT NULL,
              updated_at         INTEGER NOT NULL,
              deleted_at         INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_time_entries_started_at ON time_entries (started_at);
            CREATE INDEX IF NOT EXISTS idx_time_entries_status ON time_entries (status);
            CREATE INDEX IF NOT EXISTS idx_time_entries_project ON time_entries (project_id, started_at);

            CREATE TABLE IF NOT EXISTS suggestions (
              id            TEXT PRIMARY KEY,
              entry_id      TEXT NOT NULL,
              field         TEXT NOT NULL,
              value_id      TEXT,
              confidence    REAL NOT NULL,
              raw_confidence REAL,
              tier          TEXT,
              signals       TEXT,
              rationale     TEXT,
              alternatives  TEXT,
              engine        TEXT NOT NULL DEFAULT 'full',
              model_version TEXT,
              outcome       TEXT,
              created_at    INTEGER NOT NULL,
              updated_at    INTEGER NOT NULL,
              deleted_at    INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_suggestions_entry ON suggestions (entry_id, field, created_at);
            CREATE INDEX IF NOT EXISTS idx_suggestions_created_at ON suggestions (created_at);

            CREATE TABLE IF NOT EXISTS entry_events (
              id       TEXT PRIMARY KEY,
              entry_id TEXT NOT NULL,
              kind     TEXT NOT NULL,
              actor    TEXT NOT NULL,
              payload  TEXT,
              at       INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_entry_events_entry_id ON entry_events (entry_id);

            -- One NLEmbedding vector per entry (512 x f32, little-endian) plus the
            -- feature text it was computed from, reused as few-shot examples.
            CREATE TABLE IF NOT EXISTS entry_embeddings (
              entry_id   TEXT PRIMARY KEY,
              vec        BLOB NOT NULL,
              text_hash  TEXT NOT NULL,
              features   TEXT NOT NULL,
              model      TEXT NOT NULL,
              created_at INTEGER NOT NULL
            );

            -- Durable work queue for the classification worker. `kind` is
            -- classify (full pipeline) or embed (vector only, for entries that
            -- were approved without ever being classified).
            CREATE TABLE IF NOT EXISTS classify_jobs (
              id         TEXT PRIMARY KEY,
              entry_id   TEXT NOT NULL,
              kind       TEXT NOT NULL DEFAULT 'classify',
              state      TEXT NOT NULL DEFAULT 'queued',
              attempts   INTEGER NOT NULL DEFAULT 0,
              next_at    INTEGER NOT NULL,
              last_error TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              UNIQUE (entry_id, kind)
            );
            CREATE INDEX IF NOT EXISTS idx_classify_jobs_state ON classify_jobs (state, next_at);

            -- Trained personal models (Create ML). Only one per kind is active.
            CREATE TABLE IF NOT EXISTS model_artifacts (
              id          TEXT PRIMARY KEY,
              kind        TEXT NOT NULL,
              path        TEXT NOT NULL,
              trained_at  INTEGER NOT NULL,
              n_examples  INTEGER NOT NULL,
              holdout_acc REAL,
              calibration TEXT,
              active      INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_model_artifacts_kind ON model_artifacts (kind, active, trained_at);

            CREATE TABLE IF NOT EXISTS energy_samples (
              id            INTEGER PRIMARY KEY AUTOINCREMENT,
              sampled_at    INTEGER NOT NULL,
              duration_ms   INTEGER NOT NULL,
              cpu_time_ms   INTEGER NOT NULL,
              energy_nj     INTEGER NOT NULL,
              power_watts   REAL NOT NULL,
              impact_level  TEXT NOT NULL,
              ai_active     INTEGER NOT NULL DEFAULT 0,
              on_battery    INTEGER NOT NULL DEFAULT 0,
              battery_level REAL
            );
            CREATE INDEX IF NOT EXISTS idx_energy_samples_sampled_at ON energy_samples (sampled_at);

            -- Full-text search over window titles for Time Entries (\"where did I
            -- work on invoices.rs?\"). A trigram index answers substring `LIKE`
            -- queries, so \"Aevio\" finds \"Aevio\". It mirrors `segments`
            -- through triggers; `segments.id` is an INTEGER PRIMARY KEY, so the
            -- external-content rowid is stable across VACUUM.
            CREATE VIRTUAL TABLE IF NOT EXISTS segments_fts USING fts5(
              title, content='segments', content_rowid='id', tokenize='trigram'
            );
            CREATE TRIGGER IF NOT EXISTS segments_fts_insert AFTER INSERT ON segments BEGIN
              INSERT INTO segments_fts (rowid, title) VALUES (new.id, new.title);
            END;
            CREATE TRIGGER IF NOT EXISTS segments_fts_delete AFTER DELETE ON segments BEGIN
              INSERT INTO segments_fts (segments_fts, rowid, title) VALUES ('delete', old.id, old.title);
            END;
            CREATE TRIGGER IF NOT EXISTS segments_fts_update AFTER UPDATE OF title ON segments BEGIN
              INSERT INTO segments_fts (segments_fts, rowid, title) VALUES ('delete', old.id, old.title);
              INSERT INTO segments_fts (rowid, title) VALUES (new.id, new.title);
            END;
            ",
        )?;

        seed_default_categories(&tx)?;

        tx.execute("PRAGMA user_version = 1;", [])?;
        tx.commit()?;
    }

    if current_version < 2 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "CREATE TABLE invoices (
               id TEXT PRIMARY KEY,
               client_id TEXT NOT NULL,
               client_name TEXT NOT NULL,
               client_email TEXT,
               client_address TEXT,
               currency TEXT NOT NULL,
               status TEXT NOT NULL CHECK (status IN ('draft', 'sent', 'paid')),
               created_at INTEGER NOT NULL,
               updated_at INTEGER NOT NULL
             );
             CREATE TABLE invoice_lines (
               entry_id TEXT PRIMARY KEY,
               invoice_id TEXT NOT NULL REFERENCES invoices(id),
               project_name TEXT NOT NULL,
               description TEXT NOT NULL,
               started_at INTEGER NOT NULL,
               ended_at INTEGER NOT NULL,
               rate REAL NOT NULL,
               amount_cents INTEGER NOT NULL
             );
             CREATE INDEX idx_invoice_lines_invoice ON invoice_lines(invoice_id);
             CREATE TRIGGER protect_invoiced_time BEFORE UPDATE OF started_at, ended_at, description, project_id, status, billable, deleted_at ON time_entries
             WHEN OLD.invoice_id IS NOT NULL AND (
               NEW.started_at IS NOT OLD.started_at OR NEW.ended_at IS NOT OLD.ended_at OR
               NEW.description IS NOT OLD.description OR NEW.project_id IS NOT OLD.project_id OR
               NEW.status IS NOT OLD.status OR NEW.billable IS NOT OLD.billable OR NEW.deleted_at IS NOT OLD.deleted_at
             ) BEGIN SELECT RAISE(ABORT, 'Invoiced time cannot be edited; delete its draft invoice first'); END;
             PRAGMA user_version = 2;",
        )?;
        tx.commit()?;
    }

    if current_version < 3 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "DROP TRIGGER protect_invoiced_time;
             CREATE TRIGGER protect_invoiced_time BEFORE UPDATE OF started_at, ended_at, description, project_id, status, billable, deleted_at ON time_entries
             WHEN OLD.invoice_id IS NOT NULL AND (
               NEW.started_at IS NOT OLD.started_at OR NEW.ended_at IS NOT OLD.ended_at OR
               NEW.description IS NOT OLD.description OR
               (NEW.project_id IS NOT OLD.project_id AND NOT (
                 NEW.project_id IS NULL AND EXISTS (
                   SELECT 1 FROM projects WHERE id = OLD.project_id AND deleted_at IS NOT NULL
                 )
               )) OR
               NEW.status IS NOT OLD.status OR NEW.billable IS NOT OLD.billable OR NEW.deleted_at IS NOT OLD.deleted_at
             ) BEGIN SELECT RAISE(ABORT, 'Invoiced time cannot be edited; delete its draft invoice first'); END;
             PRAGMA user_version = 3;",
        )?;
        tx.commit()?;
    }

    if current_version < 4 {
        let tx = conn.transaction()?;
        tx.execute(
            "UPDATE categories SET color = '#52b788' WHERE color = '#e5995c';",
            [],
        )?;
        tx.execute(
            "UPDATE categories SET color = '#707bf0' WHERE color = '#e7b447';",
            [],
        )?;
        tx.execute(
            "UPDATE projects SET color = '#52b788' WHERE color = '#e5995c';",
            [],
        )?;
        tx.execute(
            "UPDATE projects SET color = '#707bf0' WHERE color = '#e7b447';",
            [],
        )?;
        tx.execute("PRAGMA user_version = 4;", [])?;
        tx.commit()?;
    }

    if current_version < 5 {
        let tx = conn.transaction()?;
        tx.execute_batch(INVOICE_DOCUMENTS_V5)?;
        tx.execute("PRAGMA user_version = 5;", [])?;
        tx.commit()?;
    }

    if current_version < 6 {
        let tx = conn.transaction()?;
        tx.execute_batch(INVOICE_FROM_V6)?;
        tx.execute("PRAGMA user_version = 6;", [])?;
        tx.commit()?;
    }

    if current_version < 7 {
        let tx = conn.transaction()?;
        tx.execute_batch(CLIENT_ARCHIVE_V7)?;
        tx.execute("PRAGMA user_version = 7;", [])?;
        tx.commit()?;
    }

    if current_version < 8 {
        let tx = conn.transaction()?;
        tx.execute_batch(BREAKS_V8)?;
        tx.execute("PRAGMA user_version = 8;", [])?;
        tx.commit()?;
    }

    if current_version < 9 {
        let tx = conn.transaction()?;
        tx.execute_batch(AGENTS_V9)?;
        tx.execute("PRAGMA user_version = 9;", [])?;
        tx.commit()?;
    }

    if current_version < 10 {
        let tx = conn.transaction()?;
        tx.execute_batch(REMOVE_AGENTS_V10)?;
        tx.execute("PRAGMA user_version = 10;", [])?;
        tx.commit()?;
    }

    if current_version < 11 {
        let tx = conn.transaction()?;
        tx.execute_batch(AUTOMATIONS_V11)?;
        tx.execute("PRAGMA user_version = 11;", [])?;
        tx.commit()?;
    }

    if current_version < 12 {
        let tx = conn.transaction()?;
        tx.execute_batch(TASKS_V12)?;
        let has_task_id = {
            let mut columns = tx.prepare("PRAGMA table_info(time_entries);")?;
            let names = columns.query_map([], |row| row.get::<_, String>(1))?;
            names
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .any(|name| name == "task_id")
        };
        if !has_task_id {
            tx.execute(
                "ALTER TABLE time_entries ADD COLUMN task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL;",
                [],
            )?;
        }
        tx.execute(
            "CREATE INDEX IF NOT EXISTS idx_time_entries_task ON time_entries(task_id);",
            [],
        )?;
        tx.execute("PRAGMA user_version = 12;", [])?;
        tx.commit()?;
    }

    if current_version < 13 {
        let tx = conn.transaction()?;
        tx.execute_batch(INTEGRATIONS_V13)?;
        tx.execute("PRAGMA user_version = 13;", [])?;
        tx.commit()?;
    }

    if current_version < 14 {
        let tx = conn.transaction()?;
        tx.execute_batch(FINANCE_V14)?;
        tx.execute("PRAGMA user_version = 14;", [])?;
        tx.commit()?;
    }

    if current_version < 15 {
        let tx = conn.transaction()?;
        tx.execute_batch(GOVERNANCE_V15)?;
        tx.execute("PRAGMA user_version = 15;", [])?;
        tx.commit()?;
    }

    if current_version < 16 {
        let tx = conn.transaction()?;
        let has_effective_defaults = {
            let mut columns = tx.prepare("PRAGMA table_info(members);")?;
            let names = columns.query_map([], |row| row.get::<_, String>(1))?;
            names
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .any(|name| name == "effective_defaults_json")
        };
        if !has_effective_defaults {
            tx.execute(
                "ALTER TABLE members ADD COLUMN effective_defaults_json TEXT NOT NULL DEFAULT '{}'",
                [],
            )?;
        }
        tx.execute("UPDATE members SET effective_defaults_json = (SELECT defaults_json FROM workspaces WHERE id = members.workspace_id)", [])?;
        tx.execute("PRAGMA user_version = 16", [])?;
        tx.commit()?;
    }

    if current_version < 17 {
        let tx = conn.transaction()?;
        tx.execute_batch(WORKSPACE_CONTENT_V17)?;
        tx.execute("PRAGMA user_version = 17;", [])?;
        tx.commit()?;
    }

    if current_version < 18 {
        let tx = conn.transaction()?;
        tx.execute_batch(WORK_TRACKING_V18)?;
        tx.execute("PRAGMA user_version = 18", [])?;
        tx.commit()?;
    }

    if current_version < 19 {
        let tx = conn.transaction()?;
        tx.execute_batch(include_str!("../coach/schema.sql"))?;
        // Early coach versions created this table outside the migration ladder.
        let has_notified_at: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('coach_tasks') WHERE name='notified_at')", [], |row| row.get(0))?;
        if !has_notified_at {
            tx.execute("ALTER TABLE coach_tasks ADD COLUMN notified_at INTEGER", [])?;
        }
        tx.execute("PRAGMA user_version = 19", [])?;
        tx.commit()?;
    }
    if current_version < 20 {
        let tx = conn.transaction()?;
        tx.execute_batch(include_str!("../coach/focus-schema.sql"))?;
        tx.execute("PRAGMA user_version=20", [])?;
        tx.commit()?;
    }
    if current_version < 21 {
        let tx = conn.transaction()?;
        tx.execute_batch(include_str!("../coach/adaptive-schema.sql"))?;
        tx.execute("PRAGMA user_version=21", [])?;
        tx.commit()?;
    }
    if current_version < 22 {
        let tx = conn.transaction()?;
        tx.execute_batch(include_str!("../coach/brain-schema.sql"))?;
        tx.execute("PRAGMA user_version=22", [])?;
        tx.commit()?;
    }
    if current_version < 23 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS work_relationships (
              id         TEXT PRIMARY KEY,
              from_type  TEXT NOT NULL,
              from_id    TEXT NOT NULL,
              relation   TEXT NOT NULL,
              to_type    TEXT NOT NULL,
              to_id      TEXT NOT NULL,
              provenance TEXT NOT NULL DEFAULT 'system',
              created_at INTEGER NOT NULL,
              deleted_at INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_work_rel_from ON work_relationships (from_type, from_id);
            CREATE INDEX IF NOT EXISTS idx_work_rel_to ON work_relationships (to_type, to_id);
            CREATE UNIQUE INDEX IF NOT EXISTS ux_work_rel_unique
              ON work_relationships (from_type, from_id, relation, to_type, to_id)
              WHERE deleted_at IS NULL;
            ",
        )?;
        tx.commit()?;
        conn.execute("PRAGMA user_version = 23;", [])?;
    }

    if current_version < 24 {
        let tx = conn.transaction()?;
        tx.execute_batch(include_str!("../staffing/schema.sql"))?;
        tx.execute("PRAGMA user_version = 24", [])?;
        tx.commit()?;
    }

    if current_version < 26 {
        let tx = conn.transaction()?;
        tx.execute_batch(TIMELINE_V26)?;
        tx.execute("PRAGMA user_version = 26", [])?;
        tx.commit()?;
    }

    if current_version < 27 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS timeline_sync_cursors (
              workspace_id TEXT PRIMARY KEY,
              cursor TEXT NOT NULL DEFAULT '0',
              last_success_at INTEGER,
              last_error TEXT
            );
            ",
        )?;
        tx.execute("PRAGMA user_version = 27", [])?;
        tx.commit()?;
    }

    if current_version < 28 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS timers (
              id TEXT PRIMARY KEY,
              label TEXT NOT NULL,
              accumulated_ms INTEGER NOT NULL,
              started_at INTEGER,
              created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS timer_segments (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              timer_id TEXT NOT NULL,
              duration_ms INTEGER NOT NULL,
              recorded_at INTEGER NOT NULL
            );
            ",
        )?;
        tx.execute("PRAGMA user_version = 28", [])?;
        tx.commit()?;
    }

    if current_version < 29 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS music_tracks (
              id          TEXT PRIMARY KEY,
              title       TEXT NOT NULL,
              description TEXT NOT NULL,
              kind        TEXT NOT NULL,
              sort_order  INTEGER NOT NULL,
              created_at  INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_music_tracks_sort ON music_tracks(sort_order);
            ",
        )?;
        tx.execute("PRAGMA user_version = 29", [])?;
        tx.commit()?;
    }

    if current_version < 30 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS plugin_installations (
              id TEXT PRIMARY KEY,
              plugin_id TEXT NOT NULL UNIQUE,
              publisher_id TEXT NOT NULL,
              version TEXT NOT NULL,
              scope TEXT NOT NULL CHECK (scope IN ('personal', 'workspace')),
              state TEXT NOT NULL,
              manifest_json TEXT NOT NULL,
              granted_permissions_json TEXT NOT NULL DEFAULT '[]',
              artifact_digest TEXT NOT NULL,
              artifact_path TEXT,
              revision INTEGER NOT NULL DEFAULT 0,
              last_error TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              last_activated_at INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_plugin_installations_state
              ON plugin_installations(state, updated_at);

            CREATE TABLE IF NOT EXISTS plugin_operations (
              id TEXT PRIMARY KEY,
              installation_id TEXT NOT NULL REFERENCES plugin_installations(id) ON DELETE CASCADE,
              kind TEXT NOT NULL,
              state TEXT NOT NULL,
              revision INTEGER NOT NULL,
              error TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_plugin_operations_installation
              ON plugin_operations(installation_id, updated_at);

            CREATE TABLE IF NOT EXISTS plugin_grants (
              id TEXT PRIMARY KEY,
              installation_id TEXT NOT NULL REFERENCES plugin_installations(id) ON DELETE CASCADE,
              capability TEXT NOT NULL,
              scope TEXT,
              expires_at INTEGER,
              revoked_at INTEGER,
              created_at INTEGER NOT NULL,
              UNIQUE (installation_id, capability, scope)
            );
            CREATE INDEX IF NOT EXISTS idx_plugin_grants_active
              ON plugin_grants(installation_id, revoked_at, expires_at);

            CREATE TABLE IF NOT EXISTS plugin_audit_events (
              id TEXT PRIMARY KEY,
              installation_id TEXT,
              event_type TEXT NOT NULL,
              actor TEXT NOT NULL,
              payload_json TEXT NOT NULL,
              created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_plugin_audit_events_installation
              ON plugin_audit_events(installation_id, created_at);

            CREATE TABLE IF NOT EXISTS plugin_storage_entries (
              id TEXT PRIMARY KEY,
              installation_id TEXT NOT NULL REFERENCES plugin_installations(id) ON DELETE CASCADE,
              user_ref TEXT NOT NULL,
              workspace_ref TEXT NOT NULL,
              device_ref TEXT NOT NULL DEFAULT 'legacy-device',
              key TEXT NOT NULL,
              value_json TEXT NOT NULL,
              schema_version INTEGER NOT NULL,
              size_bytes INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              UNIQUE (installation_id, user_ref, workspace_ref, device_ref, key)
            );
            CREATE INDEX IF NOT EXISTS idx_plugin_storage_namespace
              ON plugin_storage_entries(installation_id, user_ref, workspace_ref, device_ref);

            CREATE TABLE IF NOT EXISTS plugin_runtime_health (
              installation_id TEXT PRIMARY KEY REFERENCES plugin_installations(id) ON DELETE CASCADE,
              state TEXT NOT NULL,
              last_success_at INTEGER,
              last_failure_at INTEGER,
              crash_count INTEGER NOT NULL DEFAULT 0,
              permission_denials INTEGER NOT NULL DEFAULT 0,
              resource_violations INTEGER NOT NULL DEFAULT 0,
              last_error TEXT
            );
            ",
        )?;
        tx.execute("PRAGMA user_version = 30", [])?;
        tx.commit()?;
    }

    if current_version < 31 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS plugin_installation_history (
              id TEXT PRIMARY KEY,
              plugin_id TEXT NOT NULL,
              publisher_id TEXT NOT NULL,
              version TEXT NOT NULL,
              scope TEXT NOT NULL,
              state TEXT NOT NULL,
              manifest_json TEXT NOT NULL,
              artifact_digest TEXT NOT NULL,
              artifact_path TEXT,
              recorded_at INTEGER NOT NULL,
              reason TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_plugin_history_plugin
              ON plugin_installation_history(plugin_id, recorded_at DESC);
            ",
        )?;
        tx.execute("PRAGMA user_version = 31", [])?;
        tx.commit()?;
    }

    if current_version < 32 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS plugin_jobs (
              id TEXT PRIMARY KEY,
              installation_id TEXT NOT NULL REFERENCES plugin_installations(id) ON DELETE CASCADE,
              job_id TEXT NOT NULL,
              interval_ms INTEGER NOT NULL,
              next_run_at INTEGER NOT NULL,
              last_run_at INTEGER,
              run_count INTEGER NOT NULL DEFAULT 0,
              enabled INTEGER NOT NULL DEFAULT 1,
              payload_json TEXT NOT NULL DEFAULT '{}',
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              UNIQUE (installation_id, job_id)
            );
            CREATE INDEX IF NOT EXISTS idx_plugin_jobs_due
              ON plugin_jobs(enabled, next_run_at);
            ",
        )?;
        tx.execute("PRAGMA user_version = 32", [])?;
        tx.commit()?;
    }

    if current_version < 33 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS plugin_layout_preferences (
              id TEXT PRIMARY KEY,
              installation_id TEXT NOT NULL REFERENCES plugin_installations(id) ON DELETE CASCADE,
              contribution_id TEXT NOT NULL,
              visible INTEGER NOT NULL DEFAULT 1,
              pinned INTEGER NOT NULL DEFAULT 0,
              order_index INTEGER NOT NULL DEFAULT 0,
              updated_at INTEGER NOT NULL,
              UNIQUE (installation_id, contribution_id)
            );
            CREATE INDEX IF NOT EXISTS idx_plugin_layout_order
              ON plugin_layout_preferences(order_index, pinned, updated_at);
            ",
        )?;
        tx.execute("PRAGMA user_version = 33", [])?;
        tx.commit()?;
    }

    if current_version < 34 {
        let tx = conn.transaction()?;
        for (name, definition) in [
            (
                "attempt_count",
                "ALTER TABLE plugin_jobs ADD COLUMN attempt_count INTEGER NOT NULL DEFAULT 0",
            ),
            (
                "max_attempts",
                "ALTER TABLE plugin_jobs ADD COLUMN max_attempts INTEGER NOT NULL DEFAULT 3",
            ),
            (
                "lease_until",
                "ALTER TABLE plugin_jobs ADD COLUMN lease_until INTEGER",
            ),
            (
                "last_error",
                "ALTER TABLE plugin_jobs ADD COLUMN last_error TEXT",
            ),
            (
                "state",
                "ALTER TABLE plugin_jobs ADD COLUMN state TEXT NOT NULL DEFAULT 'scheduled'",
            ),
        ] {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('plugin_jobs') WHERE name = ?1)",
                [name],
                |row| row.get(0),
            )?;
            if !exists {
                tx.execute(definition, [])?;
            }
        }
        tx.execute_batch(
            "
            CREATE INDEX IF NOT EXISTS idx_plugin_jobs_claim
              ON plugin_jobs(state, enabled, next_run_at, lease_until);
            ",
        )?;
        tx.execute("PRAGMA user_version = 34", [])?;
        tx.commit()?;
    }

    if current_version < 35 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE plugin_storage_entries_v35 (
              id TEXT PRIMARY KEY,
              installation_id TEXT NOT NULL REFERENCES plugin_installations(id) ON DELETE CASCADE,
              user_ref TEXT NOT NULL,
              workspace_ref TEXT NOT NULL,
              device_ref TEXT NOT NULL,
              key TEXT NOT NULL,
              value_json TEXT NOT NULL,
              schema_version INTEGER NOT NULL,
              size_bytes INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              UNIQUE (installation_id, user_ref, workspace_ref, device_ref, key)
            );
            INSERT INTO plugin_storage_entries_v35
              (id, installation_id, user_ref, workspace_ref, device_ref, key,
               value_json, schema_version, size_bytes, updated_at)
            SELECT id, installation_id, user_ref, workspace_ref, 'legacy-device', key,
                   value_json, schema_version, size_bytes, updated_at
            FROM plugin_storage_entries;
            DROP TABLE plugin_storage_entries;
            ALTER TABLE plugin_storage_entries_v35 RENAME TO plugin_storage_entries;
            CREATE INDEX IF NOT EXISTS idx_plugin_storage_namespace
              ON plugin_storage_entries(installation_id, user_ref, workspace_ref, device_ref);
            ",
        )?;
        tx.execute("PRAGMA user_version = 35", [])?;
        tx.commit()?;
    }

    if current_version < 36 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS plugin_workflow_runs (
              id TEXT PRIMARY KEY,
              installation_id TEXT NOT NULL REFERENCES plugin_installations(id) ON DELETE CASCADE,
              workflow_id TEXT NOT NULL,
              trigger TEXT NOT NULL,
              idempotency_key TEXT NOT NULL,
              status TEXT NOT NULL,
              attempt_count INTEGER NOT NULL DEFAULT 0,
              max_attempts INTEGER NOT NULL DEFAULT 3,
              input_json TEXT NOT NULL,
              output_json TEXT,
              last_error TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              completed_at INTEGER,
              UNIQUE (installation_id, workflow_id, idempotency_key)
            );
            CREATE INDEX IF NOT EXISTS idx_plugin_workflow_runs_status
              ON plugin_workflow_runs(status, updated_at);
            ",
        )?;
        tx.execute("PRAGMA user_version = 36", [])?;
        tx.commit()?;
    }

    if current_version < 37 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS plugin_events (
              id TEXT PRIMARY KEY,
              publisher_installation_id TEXT NOT NULL REFERENCES plugin_installations(id) ON DELETE CASCADE,
              publisher_plugin_id TEXT NOT NULL,
              event_type TEXT NOT NULL,
              payload_json TEXT NOT NULL,
              occurred_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_plugin_events_type
              ON plugin_events(event_type, occurred_at);

            CREATE TABLE IF NOT EXISTS plugin_event_deliveries (
              id TEXT PRIMARY KEY,
              event_id TEXT NOT NULL REFERENCES plugin_events(id) ON DELETE CASCADE,
              subscriber_installation_id TEXT NOT NULL REFERENCES plugin_installations(id) ON DELETE CASCADE,
              workflow_id TEXT NOT NULL,
              status TEXT NOT NULL DEFAULT 'pending',
              attempt_count INTEGER NOT NULL DEFAULT 0,
              max_attempts INTEGER NOT NULL DEFAULT 3,
              lease_until INTEGER,
              last_error TEXT,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              completed_at INTEGER,
              UNIQUE (event_id, subscriber_installation_id, workflow_id)
            );
            CREATE INDEX IF NOT EXISTS idx_plugin_event_deliveries_due
              ON plugin_event_deliveries(status, lease_until, created_at);
            ",
        )?;
        tx.execute("PRAGMA user_version = 37", [])?;
        tx.commit()?;
    }

    if current_version < 38 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS plugin_update_policies (
              id TEXT PRIMARY KEY,
              channel TEXT NOT NULL DEFAULT 'stable',
              pinned_version TEXT,
              automatic_updates INTEGER NOT NULL DEFAULT 0,
              updated_at INTEGER NOT NULL
            );
            ",
        )?;
        tx.execute("PRAGMA user_version = 38", [])?;
        tx.commit()?;
    }

    if current_version < 39 {
        let tx = conn.transaction()?;
        let has_file_path: bool = tx.query_row(
            "SELECT EXISTS(
               SELECT 1 FROM pragma_table_info('music_tracks') WHERE name = 'file_path'
             )",
            [],
            |row| row.get(0),
        )?;
        if !has_file_path {
            tx.execute("ALTER TABLE music_tracks ADD COLUMN file_path TEXT", [])?;
        }
        tx.execute("PRAGMA user_version = 39", [])?;
        tx.commit()?;
    }

    Ok(())
}

const TIMELINE_V26: &str = "
CREATE TABLE IF NOT EXISTS timeline_devices (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  platform TEXT NOT NULL,
  last_seen_at INTEGER NOT NULL,
  tracking_enabled INTEGER NOT NULL DEFAULT 1,
  created_at INTEGER NOT NULL,
  revoked_at INTEGER
);
CREATE TABLE IF NOT EXISTS timeline_publications (
  segment_id INTEGER PRIMARY KEY REFERENCES segments(id) ON DELETE CASCADE,
  visibility TEXT NOT NULL CHECK (visibility IN ('private', 'shared')) DEFAULT 'private',
  workspace_id TEXT,
  title_override TEXT,
  published_at INTEGER,
  updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_timeline_publications_workspace
  ON timeline_publications(workspace_id, updated_at);
CREATE TABLE IF NOT EXISTS timeline_sync_state (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  mode TEXT NOT NULL CHECK (mode IN ('standalone', 'server')) DEFAULT 'standalone',
  server_url TEXT,
  status TEXT NOT NULL DEFAULT 'local',
  cursor TEXT,
  last_success_at INTEGER,
  last_error TEXT
);
INSERT OR IGNORE INTO timeline_sync_state (id, mode, status) VALUES (1, 'standalone', 'local');
CREATE TABLE IF NOT EXISTS timeline_sync_outbox (
  id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  operation TEXT NOT NULL,
  payload TEXT NOT NULL,
  state TEXT NOT NULL CHECK (state IN ('pending', 'sent', 'failed')) DEFAULT 'pending',
  attempts INTEGER NOT NULL DEFAULT 0,
  last_error TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_timeline_outbox_state ON timeline_sync_outbox(state, created_at);
CREATE TABLE IF NOT EXISTS timeline_remote_events (
  id TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL,
  payload TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  deleted_at INTEGER
);
CREATE INDEX IF NOT EXISTS idx_timeline_remote_events_workspace
  ON timeline_remote_events(workspace_id, updated_at);
";

const WORKSPACE_CONTENT_V17: &str = "
CREATE TABLE IF NOT EXISTS documents (
  id TEXT PRIMARY KEY, title TEXT NOT NULL, content TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_documents_updated ON documents(updated_at DESC);
CREATE TABLE IF NOT EXISTS notes (
  id TEXT PRIMARY KEY, title TEXT NOT NULL, body TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_notes_updated ON notes(updated_at DESC);
CREATE TABLE IF NOT EXISTS comments (
  id TEXT PRIMARY KEY, target_type TEXT NOT NULL, target_id TEXT NOT NULL,
  body TEXT NOT NULL, author TEXT NOT NULL, created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_comments_target ON comments(target_type, target_id, created_at);
CREATE TABLE IF NOT EXISTS attachments (
  id TEXT PRIMARY KEY, target_type TEXT NOT NULL, target_id TEXT NOT NULL,
  name TEXT NOT NULL, path TEXT NOT NULL, mime_type TEXT, size INTEGER,
  created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_attachments_target ON attachments(target_type, target_id, created_at);
CREATE TABLE IF NOT EXISTS mentions (
  id TEXT PRIMARY KEY, target_type TEXT NOT NULL, target_id TEXT NOT NULL,
  handle TEXT NOT NULL, created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_mentions_target ON mentions(target_type, target_id, created_at);
";

const WORK_TRACKING_V18: &str = "
CREATE TABLE IF NOT EXISTS cycles (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, goal TEXT NOT NULL DEFAULT '',
  starts_at INTEGER NOT NULL, ends_at INTEGER NOT NULL, status TEXT NOT NULL DEFAULT 'planned',
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_cycles_status_dates ON cycles(status, starts_at, ends_at);
CREATE TABLE IF NOT EXISTS initiatives (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'active', target_date INTEGER, created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_initiatives_status ON initiatives(status, updated_at DESC);
CREATE TABLE IF NOT EXISTS initiative_projects (
  initiative_id TEXT NOT NULL REFERENCES initiatives(id) ON DELETE CASCADE,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  PRIMARY KEY(initiative_id, project_id)
);
";

/// Workspace governance is local-first but deliberately relational: every
/// mutation can be attributed and every permission is evaluated in the core.
const GOVERNANCE_V15: &str = "
CREATE TABLE IF NOT EXISTS workspaces (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, currency TEXT NOT NULL DEFAULT 'USD',
  timezone TEXT NOT NULL DEFAULT 'UTC', defaults_json TEXT NOT NULL DEFAULT '{}',
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS teams (
  id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  name TEXT NOT NULL, currency TEXT NOT NULL DEFAULT 'USD', timezone TEXT NOT NULL DEFAULT 'UTC',
  rate_strategy TEXT NOT NULL DEFAULT 'member', auto_approve INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS departments (
  id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  name TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS members (
  id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  email TEXT NOT NULL, display_name TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'active',
  org_role TEXT NOT NULL DEFAULT 'member', job_title TEXT, manager_id TEXT REFERENCES members(id),
  default_team_id TEXT REFERENCES teams(id), bill_rate_cents INTEGER, cost_rate_cents INTEGER,
  billable_default INTEGER NOT NULL DEFAULT 1, department_id TEXT REFERENCES departments(id),
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS team_members (
  team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
  member_id TEXT NOT NULL REFERENCES members(id) ON DELETE CASCADE,
  team_role TEXT NOT NULL DEFAULT 'member', PRIMARY KEY(team_id, member_id)
);
CREATE TABLE IF NOT EXISTS member_permissions (
  member_id TEXT NOT NULL REFERENCES members(id) ON DELETE CASCADE,
  permission TEXT NOT NULL, PRIMARY KEY(member_id, permission)
);
CREATE TABLE IF NOT EXISTS governance_audit (
  id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  actor_id TEXT NOT NULL, action TEXT NOT NULL, entity_type TEXT NOT NULL, entity_id TEXT NOT NULL,
  before_json TEXT, after_json TEXT, at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_governance_members_workspace ON members(workspace_id, status);
CREATE INDEX IF NOT EXISTS idx_governance_audit_workspace ON governance_audit(workspace_id, at DESC);
INSERT OR IGNORE INTO workspaces(id,name,currency,timezone,defaults_json,created_at,updated_at)
VALUES ('workspace-local','My Workspace','USD','UTC','{}',0,0);
INSERT OR IGNORE INTO teams(id,workspace_id,name,currency,timezone,rate_strategy,created_at,updated_at)
VALUES ('team-default','workspace-local','Default Team','USD','UTC','member',0,0);
INSERT OR IGNORE INTO members(id,workspace_id,email,display_name,org_role,default_team_id,created_at,updated_at)
VALUES ('member-local','workspace-local','local@aevio','Local Owner','admin','team-default',0,0);
INSERT OR IGNORE INTO team_members(team_id,member_id,team_role) VALUES ('team-default','member-local','admin');
";

/// Financial planning inputs are additive and remain separate from the
/// activity and invoice tables.  This keeps historical invoice snapshots
/// immutable while allowing rates and plans to change going forward.
const FINANCE_V14: &str = "
CREATE TABLE IF NOT EXISTS finance_rate_cards (
  project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
  bill_rate_cents INTEGER NOT NULL DEFAULT 0,
  cost_rate_cents INTEGER NOT NULL DEFAULT 0,
  currency TEXT NOT NULL DEFAULT 'USD',
  effective_from INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS finance_contracts (
  id TEXT PRIMARY KEY,
  client_id TEXT NOT NULL REFERENCES clients(id),
  project_id TEXT REFERENCES projects(id),
  kind TEXT NOT NULL CHECK (kind IN ('fixed', 'retainer', 'hybrid')),
  currency TEXT NOT NULL DEFAULT 'USD',
  start_date TEXT NOT NULL,
  end_date TEXT,
  recurring_cents INTEGER NOT NULL DEFAULT 0,
  included_hours_hundredths INTEGER NOT NULL DEFAULT 0,
  overage_rate_cents INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'active'
);
CREATE INDEX IF NOT EXISTS idx_finance_contracts_client ON finance_contracts(client_id, status);
CREATE TABLE IF NOT EXISTS finance_capacity_plans (
  id TEXT PRIMARY KEY,
  project_id TEXT REFERENCES projects(id),
  person_key TEXT NOT NULL DEFAULT 'default',
  period_start INTEGER NOT NULL,
  period_end INTEGER NOT NULL,
  planned_minutes INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_finance_capacity_period ON finance_capacity_plans(period_start, period_end, project_id);
CREATE TABLE IF NOT EXISTS dashboard_shares (
  id TEXT PRIMARY KEY,
  dashboard_id TEXT NOT NULL REFERENCES saved_dashboards(id) ON DELETE CASCADE,
  token TEXT NOT NULL UNIQUE,
  permission TEXT NOT NULL DEFAULT 'view',
  expires_at INTEGER,
  created_at INTEGER NOT NULL
);
";

/// Connection metadata and idempotent local↔provider mappings. Secrets are
/// referenced by `secret_ref` and belong in the OS keychain, never SQLite.
const INTEGRATIONS_V13: &str = "
CREATE TABLE IF NOT EXISTS integrations (
  id TEXT PRIMARY KEY,
  provider TEXT NOT NULL,
  name TEXT NOT NULL,
  workspace_ref TEXT NOT NULL,
  secret_ref TEXT NOT NULL,
  direction TEXT NOT NULL DEFAULT 'bidirectional',
  enabled INTEGER NOT NULL DEFAULT 1,
  last_synced_at INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_integrations_provider_workspace ON integrations(provider, workspace_ref);

CREATE TABLE IF NOT EXISTS external_mappings (
  connection_id TEXT NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
  local_kind TEXT NOT NULL,
  local_id TEXT NOT NULL,
  external_kind TEXT NOT NULL,
  external_id TEXT NOT NULL,
  external_updated_at INTEGER,
  local_updated_at INTEGER NOT NULL,
  PRIMARY KEY(connection_id, local_kind, local_id),
  UNIQUE(connection_id, external_kind, external_id)
);

CREATE TABLE IF NOT EXISTS sync_runs (
  id TEXT PRIMARY KEY,
  connection_id TEXT NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
  direction TEXT NOT NULL,
  status TEXT NOT NULL,
  pulled INTEGER NOT NULL DEFAULT 0,
  pushed INTEGER NOT NULL DEFAULT 0,
  conflicts INTEGER NOT NULL DEFAULT 0,
  error TEXT,
  started_at INTEGER NOT NULL,
  finished_at INTEGER
);
CREATE INDEX IF NOT EXISTS idx_sync_runs_connection_started ON sync_runs(connection_id, started_at DESC);
";

/// First-class work-management tasks. Keywords stay as JSON so the domain can
/// evolve without another schema change, while labels remain relational for
/// filtering and stable external mappings.
const TASKS_V12: &str = "
CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id),
  title TEXT NOT NULL,
  description TEXT,
  status TEXT NOT NULL DEFAULT 'todo',
  priority TEXT NOT NULL DEFAULT 'normal',
  due_date INTEGER,
  keywords_json TEXT NOT NULL DEFAULT '[]',
  ai_instructions TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  completed_at INTEGER,
  deleted_at INTEGER
);
CREATE INDEX IF NOT EXISTS idx_tasks_project_status ON tasks(project_id, status, updated_at);
CREATE INDEX IF NOT EXISTS idx_tasks_due_date ON tasks(due_date);

CREATE TABLE IF NOT EXISTS labels (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  color TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  deleted_at INTEGER
);

CREATE TABLE IF NOT EXISTS task_labels (
  task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  label_id TEXT NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
  PRIMARY KEY(task_id, label_id)
);
CREATE INDEX IF NOT EXISTS idx_task_labels_label ON task_labels(label_id);

";

/// Persisted agent routines, report definitions, saved analysis filters and
/// immutable run snapshots. Delivery is intentionally data-driven: a later
/// connector can consume the channel status without changing the run history.
const AUTOMATIONS_V11: &str = "
CREATE TABLE IF NOT EXISTS agent_routines (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  prompt TEXT NOT NULL,
  scope TEXT NOT NULL DEFAULT 'personal',
  cadence TEXT NOT NULL DEFAULT 'on-demand',
  send_at TEXT,
  timezone TEXT NOT NULL DEFAULT 'local',
  delivery_json TEXT NOT NULL DEFAULT '[\"in-app\"]',
  active INTEGER NOT NULL DEFAULT 1,
  next_run_at INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS agent_routines_next_run ON agent_routines (active, next_run_at);

CREATE TABLE IF NOT EXISTS routine_runs (
  id TEXT PRIMARY KEY,
  routine_id TEXT NOT NULL REFERENCES agent_routines(id) ON DELETE CASCADE,
  status TEXT NOT NULL,
  period_start INTEGER,
  period_end INTEGER,
  output_json TEXT NOT NULL,
  delivery_json TEXT NOT NULL DEFAULT '[]',
  created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS routine_runs_routine ON routine_runs (routine_id, created_at DESC);

CREATE TABLE IF NOT EXISTS report_definitions (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  scope TEXT NOT NULL DEFAULT 'personal',
  format TEXT NOT NULL DEFAULT 'summary',
  filter_json TEXT NOT NULL,
  prompt TEXT,
  dashboard_id TEXT,
  cadence TEXT NOT NULL DEFAULT 'on-demand',
  send_at TEXT,
  timezone TEXT NOT NULL DEFAULT 'local',
  delivery_json TEXT NOT NULL DEFAULT '[\"in-app\"]',
  active INTEGER NOT NULL DEFAULT 1,
  next_run_at INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS report_definitions_next_run ON report_definitions (active, next_run_at);

CREATE TABLE IF NOT EXISTS report_runs (
  id TEXT PRIMARY KEY,
  report_id TEXT NOT NULL REFERENCES report_definitions(id) ON DELETE CASCADE,
  status TEXT NOT NULL,
  period_start INTEGER NOT NULL,
  period_end INTEGER NOT NULL,
  output_json TEXT NOT NULL,
  delivery_json TEXT NOT NULL DEFAULT '[]',
  created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS report_runs_report ON report_runs (report_id, created_at DESC);

CREATE TABLE IF NOT EXISTS saved_dashboards (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  filter_json TEXT NOT NULL,
  pinned INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
";

/// Agent jobs. A job is one agent turn (prompt to stop) on one project, seen
/// through an agent bridge (see `agents/mod.rs`): `agent_transitions` holds its
/// state changes with timestamps, and `agent_focus` the time the person had an
/// agent's pane in front of them. Counted agent time becomes ordinary
/// `time_entries` rows with `source = 'agent'`, linked back to their job by
/// `agent_job_id`. Only a working directory that belongs to a project is ever
/// stored. Purely additive, so existing installs upgrade in place untouched.
const AGENTS_V9: &str = "
CREATE TABLE agent_jobs (
  id             TEXT PRIMARY KEY,
  source         TEXT NOT NULL,
  agent          TEXT NOT NULL,
  pane_key       TEXT NOT NULL,
  project_id     TEXT NOT NULL,
  cwd            TEXT NOT NULL,
  started_at     INTEGER NOT NULL,
  stopped_at     INTEGER,
  reviewed_at    INTEGER,
  state          TEXT NOT NULL,
  state_since    INTEGER NOT NULL,
  output_known   INTEGER NOT NULL DEFAULT 0,
  last_output_at INTEGER,
  last_seen_at   INTEGER NOT NULL,
  confirmed      INTEGER NOT NULL DEFAULT 0,
  confidence     TEXT,
  created_at     INTEGER NOT NULL,
  updated_at     INTEGER NOT NULL
);
CREATE INDEX agent_jobs_started_at ON agent_jobs (started_at);
CREATE INDEX agent_jobs_pane ON agent_jobs (pane_key, started_at);
CREATE INDEX agent_jobs_project ON agent_jobs (project_id, started_at);

CREATE TABLE agent_transitions (
  job_id TEXT NOT NULL REFERENCES agent_jobs(id) ON DELETE CASCADE,
  at     INTEGER NOT NULL,
  state  TEXT NOT NULL,
  PRIMARY KEY (job_id, at, state)
);

CREATE TABLE agent_focus (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  pane_key   TEXT NOT NULL,
  project_id TEXT NOT NULL,
  agent      TEXT NOT NULL,
  started_at INTEGER NOT NULL,
  ended_at   INTEGER NOT NULL
);
CREATE INDEX agent_focus_started_at ON agent_focus (started_at);
CREATE INDEX agent_focus_pane ON agent_focus (pane_key, started_at);

ALTER TABLE time_entries ADD COLUMN agent_job_id TEXT;
CREATE INDEX idx_time_entries_agent_job ON time_entries (agent_job_id);
";

/// Removes the agent job tables and counted agent time that v9 added.
///
/// - Agent time (`source = 'agent'`) was derived from the job tables and was
///   never part of the person's own work, so it goes with them. It is deleted
///   outright, not soft-deleted: the entry builder treats every row in a
///   range, deleted ones included, as already accounting for its span, and
///   agent rows overlap the person's own time.
/// - Invoices keep their amounts. A draft line that billed agent time becomes
///   a fixed (`manual`) line with the same description, quantity, rate and
///   amount, so the draft total does not move and the draft stays editable.
///   Lines of issued invoices are left as they are: they are snapshots (the
///   issued PDF is stored) and never read the entry again.
/// - The person's own time that was carved out for an agent's folder keeps its
///   project and stays as ordinary entries (`source = 'auto'`).
const REMOVE_AGENTS_V10: &str = "
UPDATE invoice_lines
SET kind = 'manual', entry_id = NULL, started_at = NULL, ended_at = NULL
WHERE entry_id IN (SELECT id FROM time_entries WHERE source = 'agent')
  AND invoice_id IN (SELECT id FROM invoices WHERE status = 'draft');

UPDATE segments SET entry_id = NULL
WHERE entry_id IN (SELECT id FROM time_entries WHERE source = 'agent');
DELETE FROM suggestions
WHERE entry_id IN (SELECT id FROM time_entries WHERE source = 'agent');
DELETE FROM entry_events
WHERE entry_id IN (SELECT id FROM time_entries WHERE source = 'agent');
DELETE FROM entry_embeddings
WHERE entry_id IN (SELECT id FROM time_entries WHERE source = 'agent');
DELETE FROM classify_jobs
WHERE entry_id IN (SELECT id FROM time_entries WHERE source = 'agent');
DELETE FROM time_entries WHERE source = 'agent';

DROP INDEX idx_time_entries_agent_job;
ALTER TABLE time_entries DROP COLUMN agent_job_id;
DROP TABLE agent_transitions;
DROP TABLE agent_jobs;
DROP TABLE agent_focus;
";

/// Break reminders: one row per break decision (taken, skipped, missed, or
/// credited from idle time). The break itself stays a `break` segment; this
/// table records the choice around it and links to its segment. Purely
/// additive, so existing installs upgrade in place untouched.
const BREAKS_V8: &str = "
CREATE TABLE breaks (
  id          TEXT PRIMARY KEY,
  source      TEXT NOT NULL,
  schedule_id TEXT,
  due_at      INTEGER,
  planned_ms  INTEGER NOT NULL,
  status      TEXT NOT NULL,
  snoozes     INTEGER NOT NULL DEFAULT 0,
  started_at  INTEGER,
  ended_at    INTEGER,
  segment_id  INTEGER REFERENCES segments(id) ON DELETE SET NULL,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL
);
CREATE INDEX breaks_started_at ON breaks (started_at);
CREATE INDEX breaks_due_at ON breaks (due_at);
";

/// The Clients page: a client can be archived (kept, hidden from the active
/// list and from the pickers that assign one) and carries free-form notes.
/// Both columns are additive, so existing clients upgrade in place untouched.
const CLIENT_ARCHIVE_V7: &str = "
ALTER TABLE clients ADD COLUMN archived_at INTEGER;
ALTER TABLE clients ADD COLUMN notes TEXT;
";

/// Real invoice documents. Rebuilds `invoices` and `invoice_lines` (their
/// status CHECK and the one-row-per-entry primary key cannot be altered in
/// place) and adds the issuer profile and per-year number sequences.
///
/// A USD draft becomes an editable draft (issue date from its creation day,
/// Net 30); everything else is marked `legacy` here and deleted by v6, which
/// also covers databases that already ran this migration. `sent` maps to
/// `open`. Rates become cents and hours become hundredths.
const INVOICE_DOCUMENTS_V5: &str = "
ALTER TABLE invoices RENAME TO invoices_v4;

CREATE TABLE invoice_profile (
  id                   INTEGER PRIMARY KEY CHECK (id = 1),
  name                 TEXT NOT NULL DEFAULT '',
  address              TEXT NOT NULL DEFAULT '',
  email                TEXT,
  phone                TEXT,
  payment_instructions TEXT,
  default_notes        TEXT,
  default_terms_days   INTEGER NOT NULL DEFAULT 30,
  logo                 BLOB,
  logo_mime            TEXT,
  updated_at           INTEGER NOT NULL DEFAULT 0
);
INSERT INTO invoice_profile (id) VALUES (1);

CREATE TABLE invoice_sequences (
  year        INTEGER PRIMARY KEY,
  next_number INTEGER NOT NULL
);

CREATE TABLE invoices (
  id                   TEXT PRIMARY KEY,
  client_id            TEXT NOT NULL,
  client_name          TEXT NOT NULL,
  client_email         TEXT,
  client_address       TEXT,
  currency             TEXT NOT NULL,
  status               TEXT NOT NULL CHECK (status IN ('draft', 'open', 'paid', 'void')),
  legacy               INTEGER NOT NULL DEFAULT 0,
  number               TEXT UNIQUE,
  issue_date           TEXT,
  due_date             TEXT,
  terms_days           INTEGER,
  subject              TEXT,
  notes                TEXT,
  payment_instructions TEXT,
  issuer_json          TEXT,
  total_cents          INTEGER NOT NULL DEFAULT 0,
  pdf                  BLOB,
  issued_at            INTEGER,
  paid_at              INTEGER,
  created_at           INTEGER NOT NULL,
  updated_at           INTEGER NOT NULL
);

INSERT INTO invoices (
  id, client_id, client_name, client_email, client_address, currency, status, legacy,
  issue_date, due_date, terms_days, total_cents, created_at, updated_at
)
SELECT
  id, client_id, client_name, client_email, client_address, currency,
  CASE status WHEN 'sent' THEN 'open' ELSE status END,
  CASE WHEN status = 'draft' AND currency = 'USD' THEN 0 ELSE 1 END,
  CASE WHEN status = 'draft' AND currency = 'USD'
       THEN date(created_at / 1000, 'unixepoch', 'localtime') END,
  CASE WHEN status = 'draft' AND currency = 'USD'
       THEN date(created_at / 1000, 'unixepoch', 'localtime', '+30 days') END,
  CASE WHEN status = 'draft' AND currency = 'USD' THEN 30 END,
  COALESCE((SELECT SUM(amount_cents) FROM invoice_lines WHERE invoice_id = invoices_v4.id), 0),
  created_at, updated_at
FROM invoices_v4;

CREATE TABLE invoice_lines_v5 (
  id                  TEXT PRIMARY KEY,
  invoice_id          TEXT NOT NULL REFERENCES invoices(id),
  position            INTEGER NOT NULL,
  kind                TEXT NOT NULL CHECK (kind IN ('time', 'retainer', 'manual')),
  entry_id            TEXT UNIQUE,
  project_name        TEXT NOT NULL DEFAULT '',
  description         TEXT NOT NULL,
  started_at          INTEGER,
  ended_at            INTEGER,
  quantity_hundredths INTEGER NOT NULL,
  unit                TEXT NOT NULL DEFAULT '',
  rate_cents          INTEGER NOT NULL,
  amount_cents        INTEGER NOT NULL
);

INSERT INTO invoice_lines_v5 (
  id, invoice_id, position, kind, entry_id, project_name, description, started_at, ended_at,
  quantity_hundredths, unit, rate_cents, amount_cents
)
SELECT
  lower(hex(randomblob(16))),
  invoice_id,
  row_number() OVER (PARTITION BY invoice_id ORDER BY started_at, entry_id) - 1,
  'time', entry_id, project_name, description, started_at, ended_at,
  MAX(1, CAST(((ended_at - started_at) * 100 + 1800000) / 3600000 AS INTEGER)),
  'hrs',
  CAST(round(rate * 100) AS INTEGER),
  amount_cents
FROM invoice_lines;

-- Editable drafts must satisfy quantity x rate = amount on paper.
UPDATE invoice_lines_v5
SET amount_cents = (quantity_hundredths * rate_cents + 50) / 100
WHERE invoice_id IN (SELECT id FROM invoices WHERE status = 'draft' AND legacy = 0);
UPDATE invoices
SET total_cents = COALESCE((SELECT SUM(amount_cents) FROM invoice_lines_v5 WHERE invoice_id = invoices.id), 0)
WHERE status = 'draft' AND legacy = 0;

DROP TABLE invoice_lines;
DROP TABLE invoices_v4;
ALTER TABLE invoice_lines_v5 RENAME TO invoice_lines;
CREATE INDEX idx_invoice_lines_invoice ON invoice_lines(invoice_id, position);
CREATE INDEX idx_invoices_status ON invoices(status);
";

/// Drops the `legacy` concept and gives every invoice its own From block.
///
/// v5 preserved pre-document invoices as read-only `legacy` rows; they are not
/// supported, so they (and their lines) are deleted here, on fresh installs and
/// on databases already upgraded through v5 alike. Time they reserved is
/// released so no `invoice_id` dangles. The From columns are backfilled from
/// each finalized invoice's issuer snapshot, and drafts start from the current
/// Invoice settings.
const INVOICE_FROM_V6: &str = "
UPDATE time_entries SET invoice_id = NULL
WHERE invoice_id IN (SELECT id FROM invoices WHERE legacy = 1);
DELETE FROM invoice_lines WHERE invoice_id IN (SELECT id FROM invoices WHERE legacy = 1);
DELETE FROM invoices WHERE legacy = 1;
ALTER TABLE invoices DROP COLUMN legacy;

ALTER TABLE invoices ADD COLUMN from_name    TEXT NOT NULL DEFAULT '';
ALTER TABLE invoices ADD COLUMN from_address TEXT NOT NULL DEFAULT '';
ALTER TABLE invoices ADD COLUMN from_email   TEXT;
ALTER TABLE invoices ADD COLUMN from_phone   TEXT;

UPDATE invoices SET
  from_name    = COALESCE(json_extract(issuer_json, '$.name'), ''),
  from_address = COALESCE(json_extract(issuer_json, '$.address'), ''),
  from_email   = json_extract(issuer_json, '$.email'),
  from_phone   = json_extract(issuer_json, '$.phone')
WHERE issuer_json IS NOT NULL;
UPDATE invoices SET
  from_name    = (SELECT name FROM invoice_profile WHERE id = 1),
  from_address = (SELECT address FROM invoice_profile WHERE id = 1),
  from_email   = (SELECT email FROM invoice_profile WHERE id = 1),
  from_phone   = (SELECT phone FROM invoice_profile WHERE id = 1)
WHERE issuer_json IS NULL;
";

fn seed_default_categories(tx: &rusqlite::Transaction<'_>) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let defaults = [
        (
            "Coding",
            "#75a4e5",
            "Writing, reviewing, debugging code",
            "Apply when writing, reviewing, or debugging code, using an IDE, terminal, or code editor.",
            1,
            1,
            1,
        ),
        (
            "Research",
            "#56c2b1",
            "Documentation and technical research",
            "Reading documentation, investigating solutions, exploring tools and libraries.",
            1,
            1,
            2,
        ),
        (
            "Communication",
            "#52b788",
            "Email, chat, and messaging",
            "Email, Slack, Discord, async messaging, team coordination.",
            0,
            1,
            3,
        ),
        (
            "Design",
            "#df84b5",
            "UI/UX, visual design, assets",
            "Working in Figma, design tools, creating mockups, UI components, styling.",
            1,
            1,
            4,
        ),
        (
            "Writing",
            "#e4817d",
            "Docs, articles, specifications",
            "Writing prose, documentation, specifications, blog posts, proposals.",
            1,
            1,
            5,
        ),
        (
            "Planning",
            "#9aa6b4",
            "Roadmaps, tasks, issue tracking",
            "Project planning, issue trackers, linear, github issues, backlog management.",
            1,
            1,
            6,
        ),
        (
            "Administrative",
            "#bfa181",
            "Invoicing, accounts, setup",
            "Administrative tasks, billing, accounts, dev environment setup, file management.",
            0,
            1,
            7,
        ),
        (
            "Meetings",
            "#707bf0",
            "Calls, video conferences, syncs",
            "Zoom, Google Meet, video calls, live client meetings, team syncs.",
            1,
            1,
            8,
        ),
        (
            "Learning",
            "#66b1df",
            "Courses, tutorials, reading",
            "Tutorials, educational courses, learning new frameworks and concepts.",
            0,
            1,
            9,
        ),
        (
            "Review",
            "#9b87df",
            "Pull requests, code review",
            "Reviewing pull requests, inspecting diffs, giving feedback on changes.",
            1,
            1,
            10,
        ),
        (
            "Debugging",
            "#e97e7b",
            "Bug reproduction, profiling, fixing",
            "Tracking bugs, stepping through debuggers, profiling performance bottlenecks.",
            1,
            1,
            11,
        ),
    ];

    let mut insert_cat = tx.prepare(
        "INSERT INTO categories (id, name, color, description, ai_prompt, billable_default, counts_as_work, archived, sort, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, ?9);",
    )?;

    for (name, color, desc, prompt, billable, counts, sort) in defaults {
        let id = uuid::Uuid::now_v7().to_string();
        insert_cat.execute((
            id, name, color, desc, prompt, billable, counts, sort, now as i64,
        ))?;
    }

    Ok(())
}

#[cfg(test)]
#[path = "migrations_tests.rs"]
mod tests;
