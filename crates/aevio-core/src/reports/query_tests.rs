use super::*;
use crate::NONE;
use crate::activity::ActivityStore;

const MIN: u64 = 60_000;
const HOUR: u64 = 60 * MIN;

fn store() -> ActivityStore {
    ActivityStore::from_conn(Connection::open_in_memory().unwrap()).unwrap()
}

fn category_id(conn: &Connection, name: &str) -> String {
    conn.query_row(
        "SELECT id FROM categories WHERE name = ?1;",
        [name],
        |row| row.get(0),
    )
    .unwrap()
}

/// Inserts an entry with one segment per `(app, domain, title, minutes)`.
#[allow(clippy::too_many_arguments)]
fn entry(
    conn: &Connection,
    id: &str,
    start: u64,
    minutes: u64,
    description: &str,
    category: Option<&str>,
    project: Option<&str>,
    status: &str,
    billable: bool,
    segments: &[(&str, Option<&str>, &str, u64)],
) {
    conn.execute(
            "INSERT INTO time_entries (id, started_at, ended_at, description, category_id, project_id, status, source, billable, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'auto', ?8, 0, 0);",
            params![
                id,
                start as i64,
                (start + minutes * MIN) as i64,
                description,
                category,
                project,
                status,
                billable as i64
            ],
        )
        .unwrap();
    let mut at = start;
    for (app, domain, title, mins) in segments {
        conn.execute(
            "INSERT INTO segments (app, title, kind, started_at, ended_at, domain, entry_id)
                 VALUES (?1, ?2, 'activity', ?3, ?4, ?5, ?6);",
            params![app, title, at as i64, (at + mins * MIN) as i64, domain, id],
        )
        .unwrap();
        at += mins * MIN;
    }
}

fn range(start: u64, end: u64) -> EntryFilter {
    EntryFilter {
        start_ms: start,
        end_ms: end,
        ..EntryFilter::default()
    }
}

/// Two days: day 0 has coding (Aevio, approved, billable) and a
/// pending Slack block; day 1 has an uncategoaeviod browsing block.
fn seeded() -> (ActivityStore, String, String) {
    let store = store();
    let conn = store.conn();
    let coding = category_id(conn, "Coding");
    let comms = category_id(conn, "Communication");
    conn.execute(
        "INSERT INTO clients (id, name, created_at, updated_at) VALUES ('acme', 'Acme', 0, 0);",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO projects (id, client_id, name, color, created_at, updated_at)
             VALUES ('aevio', 'acme', 'Aevio', '#75a4e5', 0, 0);",
        [],
    )
    .unwrap();
    entry(
        conn,
        "e1",
        HOUR,
        30,
        "Wired tray pause into the timer store",
        Some(&coding),
        Some("aevio"),
        "approved",
        true,
        &[
            ("Zed", None, "invoices.rs - Aevio", 20),
            ("Safari", Some("docs.rs"), "serde - Rust", 10),
        ],
    );
    entry(
        conn,
        "e2",
        2 * HOUR,
        15,
        "Slack triage",
        Some(&comms),
        None,
        "pending",
        false,
        &[("Slack", None, "general", 15)],
    );
    entry(
        conn,
        "e3",
        25 * HOUR,
        45,
        "Mixed browsing",
        None,
        None,
        "pending",
        false,
        &[("Zen", Some("youtube.com"), "Music", 45)],
    );
    (store, coding, comms)
}

#[test]
fn rollup_groups_by_category_per_day() {
    let (store, coding, comms) = seeded();
    let cells = rollup(
        store.conn(),
        &range(0, 48 * HOUR),
        &[0, 24 * HOUR, 48 * HOUR],
        GroupBy::Category,
    )
    .unwrap();
    let find = |key: Option<&str>, bucket: u32| {
        cells
            .iter()
            .find(|c| c.key.as_deref() == key && c.bucket == bucket)
            .cloned()
    };
    let code = find(Some(&coding), 0).unwrap();
    assert_eq!(code.ms, 30 * MIN);
    assert_eq!(code.approved_ms, 30 * MIN);
    assert_eq!(code.billable_ms, 30 * MIN);
    let slack = find(Some(&comms), 0).unwrap();
    assert_eq!(
        (slack.ms, slack.pending, slack.approved_ms),
        (15 * MIN, 1, 0)
    );
    let uncategoaeviod = find(None, 1).unwrap();
    assert_eq!(uncategoaeviod.ms, 45 * MIN);
    assert_eq!(cells.len(), 3);
}

#[test]
fn rollup_by_app_splits_an_entry_across_its_apps() {
    let (store, _, _) = seeded();
    let cells = rollup(
        store.conn(),
        &range(0, 24 * HOUR),
        &[0, 24 * HOUR],
        GroupBy::App,
    )
    .unwrap();
    let ms = |key: &str| {
        cells
            .iter()
            .find(|c| c.key.as_deref() == Some(key))
            .unwrap()
            .ms
    };
    assert_eq!(ms("Zed"), 20 * MIN);
    // Websites are keyed by domain, not by the browser.
    assert_eq!(ms("docs.rs"), 10 * MIN);
    assert_eq!(ms("Slack"), 15 * MIN);
}

#[test]
fn rollup_by_client_and_status() {
    let (store, _, _) = seeded();
    let by_client = rollup(
        store.conn(),
        &range(0, 48 * HOUR),
        &[0, 48 * HOUR],
        GroupBy::Client,
    )
    .unwrap();
    let acme = by_client
        .iter()
        .find(|c| c.key.as_deref() == Some("acme"))
        .unwrap();
    assert_eq!(acme.ms, 30 * MIN);
    let none = by_client.iter().find(|c| c.key.is_none()).unwrap();
    assert_eq!(none.ms, 60 * MIN);

    let by_status = rollup(
        store.conn(),
        &range(0, 48 * HOUR),
        &[0, 48 * HOUR],
        GroupBy::Status,
    )
    .unwrap();
    let pending = by_status
        .iter()
        .find(|c| c.key.as_deref() == Some("pending"))
        .unwrap();
    assert_eq!((pending.ms, pending.entries), (60 * MIN, 2));
}

#[test]
fn filters_narrow_every_query() {
    let (store, coding, _) = seeded();
    let conn = store.conn();
    let ids = |filter: EntryFilter| -> Vec<String> {
        query_entries(conn, &filter, 100)
            .unwrap()
            .into_iter()
            .map(|e| e.id)
            .collect()
    };
    let all = range(0, 48 * HOUR);
    assert_eq!(ids(all.clone()), vec!["e3", "e2", "e1"]);
    assert_eq!(
        ids(EntryFilter {
            category_id: Some(coding),
            ..all.clone()
        }),
        vec!["e1"]
    );
    assert_eq!(
        ids(EntryFilter {
            category_id: Some(NONE.to_string()),
            ..all.clone()
        }),
        vec!["e3"]
    );
    assert_eq!(
        ids(EntryFilter {
            project_id: Some(NONE.to_string()),
            ..all.clone()
        }),
        vec!["e3", "e2"]
    );
    assert_eq!(
        ids(EntryFilter {
            client_id: Some("acme".to_string()),
            ..all.clone()
        }),
        vec!["e1"]
    );
    assert_eq!(
        ids(EntryFilter {
            app: Some("docs.rs".to_string()),
            ..all.clone()
        }),
        vec!["e1"]
    );
    assert_eq!(
        ids(EntryFilter {
            status: Some("pending".to_string()),
            ..all.clone()
        }),
        vec!["e3", "e2"]
    );
    assert_eq!(
        ids(EntryFilter {
            billable: Some(true),
            ..all.clone()
        }),
        vec!["e1"]
    );
    assert!(
        query_entries(
            conn,
            &EntryFilter {
                status: Some("bogus".to_string()),
                ..all
            },
            10
        )
        .is_err()
    );
}

#[test]
fn search_finds_window_titles_and_descriptions() {
    let (store, _, _) = seeded();
    let conn = store.conn();
    let search = |text: &str| -> Vec<String> {
        query_entries(
            conn,
            &EntryFilter {
                search: Some(text.to_string()),
                ..range(0, 48 * HOUR)
            },
            100,
        )
        .unwrap()
        .into_iter()
        .map(|e| e.id)
        .collect()
    };
    // "where did I work on invoices.rs?": only a window title says so.
    assert_eq!(search("invoices.rs"), vec!["e1"]);
    // Substring, case-insensitive.
    assert_eq!(search("aevio"), vec!["e1"]);
    assert_eq!(search("slack"), vec!["e2"]);
    // Every word must match somewhere.
    assert_eq!(search("tray serde"), vec!["e1"]);
    assert!(search("tray youtube").is_empty());
    // Short terms still work, through a scan.
    assert_eq!(search("rs"), vec!["e1"]);
    assert_eq!(search("100%").len(), 0);
}

#[test]
fn entries_carry_their_dominant_app() {
    let (store, _, _) = seeded();
    let entries = query_entries(store.conn(), &range(0, 48 * HOUR), 100).unwrap();
    let app = |id: &str| {
        entries
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.dominant_app.clone())
    };
    assert_eq!(app("e1").as_deref(), Some("Zed"));
    assert_eq!(app("e3").as_deref(), Some("youtube.com"));
}

#[test]
fn bad_buckets_are_rejected() {
    let (store, _, _) = seeded();
    let conn = store.conn();
    let all = range(0, 48 * HOUR);
    assert!(rollup(conn, &all, &[0], GroupBy::None).is_err());
    assert!(rollup(conn, &all, &[10, 5], GroupBy::None).is_err());
    assert!(rollup(conn, &range(5, 5), &[0, 1], GroupBy::None).is_err());
}

#[test]
fn csv_fields_are_quoted_and_defused() {
    assert_eq!(csv_field("plain"), "plain");
    assert_eq!(csv_field("a, b"), "\"a, b\"");
    assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    assert_eq!(csv_field("=SUM(A1)"), "'=SUM(A1)");
}

#[test]
fn export_writes_the_filtered_rows_without_titles() {
    let (store, _, _) = seeded();
    let dir = std::env::temp_dir().join(format!("aevio-export-{}", crate::timers::now_epoch_ms()));
    let filter = EntryFilter {
        billable: Some(true),
        ..range(0, 48 * HOUR)
    };
    let result = export_entries(store.conn(), &filter, ExportFormat::Csv, &dir).unwrap();
    assert_eq!(result.count, 1);
    let body = fs::read_to_string(&result.path).unwrap();
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[1].contains("Wired tray pause into the timer store"));
    assert!(lines[1].contains("Aevio"));
    assert!(lines[1].contains("Acme"));
    assert!(!body.contains("invoices.rs"));

    // A second export never overwrites the first.
    let again = export_entries(store.conn(), &filter, ExportFormat::Json, &dir).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&again.path).unwrap()).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_ne!(result.path, again.path);
    let _ = fs::remove_dir_all(&dir);
}
