use super::*;

const MIN: u64 = 60_000;

fn store() -> ActivityStore {
    ActivityStore::from_conn(Connection::open_in_memory().unwrap()).unwrap()
}

fn segment(conn: &Connection, app: &str, title: &str, url: Option<&str>, start: u64, minutes: u64) {
    conn.execute(
            "INSERT INTO segments (app, title, kind, started_at, ended_at, url) VALUES (?1, ?2, 'activity', ?3, ?4, ?5);",
            params![app, title, start as i64, (start + minutes * MIN) as i64, url],
        )
        .unwrap();
}

fn new_project(name: &str, hints: Option<&str>) -> NewProject {
    NewProject {
        client_id: None,
        name: name.to_string(),
        color: PALETTE[0].to_string(),
        description: None,
        ai_hints: hints.map(str::to_string),
        status: None,
        due_date: None,
        budget_kind: None,
        budget_value: None,
        budget_period: None,
        billable_default: None,
        hourly_rate: None,
    }
}

fn hint_rules(conn: &Connection, project: &str) -> Vec<(String, String)> {
    let mut stmt = conn
            .prepare(
                "SELECT match_kind, pattern FROM rules
                 WHERE project_id = ?1 AND origin = 'hint' AND deleted_at IS NULL ORDER BY pattern;",
            )
            .unwrap();
    stmt.query_map([project], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn hints_are_classified_into_rule_kinds() {
    let hints = parse_hints(
        "~/Code/Aevio/, https://github.com/amariwan/Aevio; figma.com\nAevio, ui, aevio",
    );
    let rules: Vec<_> = hints.iter().map(|h| h.rule.clone()).collect();
    assert_eq!(
        rules,
        vec![
            Some(("path_prefix", "~/Code/Aevio".to_string())),
            Some(("url_prefix", "github.com/amariwan/Aevio".to_string())),
            Some(("domain", "figma.com".to_string())),
            Some(("title_contains", "Aevio".to_string())),
            // Too short to match on; the case-insensitive repeat is gone.
            None,
        ]
    );
}

#[test]
fn saving_hints_replaces_the_projects_rules() {
    let mut store = store();
    let project = store
        .create_project(new_project("Aevio", Some("~/Code/Aevio, figma.com")), 1)
        .unwrap();
    assert_eq!(
        hint_rules(store.conn(), &project.id),
        vec![
            ("path_prefix".to_string(), "~/Code/Aevio".to_string()),
            ("domain".to_string(), "figma.com".to_string()),
        ]
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
    );

    store
        .update_project(
            &project.id,
            crate::models::UpdateProject {
                ai_hints: Some(Some("Aevio".to_string())),
                ..Default::default()
            },
            2,
        )
        .unwrap();
    assert_eq!(
        hint_rules(store.conn(), &project.id),
        vec![("title_contains".to_string(), "Aevio".to_string())]
    );

    // Hint rules feed T0 only while the project is active.
    let active = crate::ai::store::load_rules(store.conn()).unwrap();
    assert!(
        active
            .iter()
            .any(|r| r.project_id.as_deref() == Some(project.id.as_str()))
    );
    store
        .update_project(
            &project.id,
            crate::models::UpdateProject {
                status: Some("archived".to_string()),
                ..Default::default()
            },
            3,
        )
        .unwrap();
    let archived = crate::ai::store::load_rules(store.conn()).unwrap();
    assert!(
        !archived
            .iter()
            .any(|r| r.project_id.as_deref() == Some(project.id.as_str()))
    );
}

#[test]
fn preview_counts_matched_time_once() {
    let store = store();
    let conn = store.conn();
    segment(conn, "WezTerm", "~/Code/Aevio: cargo test", None, 0, 30);
    segment(conn, "Zed", "activity.rs - Aevio", None, 30 * MIN, 20);
    segment(
        conn,
        "Safari",
        "Figma",
        Some("https://www.figma.com/file/x"),
        50 * MIN,
        10,
    );

    let preview = preview_hints(conn, "Aevio, ~/Code/Aevio, ui", 0).unwrap();
    let ms: Vec<u64> = preview.hints.iter().map(|h| h.matched_ms).collect();
    assert_eq!(ms, vec![50 * MIN, 30 * MIN, 0]);
    assert_eq!(preview.hints[2].match_kind, None);
    // The terminal segment matches both hints but counts once.
    assert_eq!(preview.total_ms, 50 * MIN);
}

#[test]
fn discovery_clusters_repo_folder_and_forge_names() {
    let mut store = store();
    let conn = store.conn();
    segment(
        conn,
        "WezTerm",
        "kareem@mac: ~/Code/pantry-tracker",
        None,
        0,
        40,
    );
    segment(
        conn,
        "Safari",
        "pantry-tracker",
        Some("https://github.com/kareem/pantry-tracker/pulls"),
        40 * MIN,
        20,
    );
    segment(conn, "Zed", "main.rs — pantry-tracker", None, 60 * MIN, 10);
    // Too little time to suggest.
    segment(conn, "WezTerm", "~/Code/scratch", None, 70 * MIN, 5);
    // A forge page, not a repository.
    segment(
        conn,
        "Safari",
        "Notifications",
        Some("https://github.com/notifications"),
        75 * MIN,
        60,
    );

    let found = discover(conn, 0).unwrap();
    assert_eq!(found.len(), 1);
    let suggestion = &found[0];
    assert_eq!(suggestion.name, "pantry-tracker");
    assert_eq!(suggestion.ms, 70 * MIN);
    assert_eq!(
        suggestion.evidence,
        vec![
            "~/Code/pantry-tracker".to_string(),
            "github.com/kareem/pantry-tracker".to_string(),
            "pantry-tracker".to_string(),
        ]
    );

    // Accepting it (a project with those hints) or dismissing it ends it.
    dismiss_suggestion(conn, "Pantry-Tracker").unwrap();
    assert!(discover(conn, 0).unwrap().is_empty());
    conn.execute("DELETE FROM settings WHERE key = ?1;", [DISMISSED_KEY])
        .unwrap();
    store
        .create_project(new_project("Pantry", Some("~/Code/pantry-tracker")), 1)
        .unwrap();
    assert!(discover(store.conn(), 0).unwrap().is_empty());
}

#[test]
fn discovery_skips_a_project_that_exists_under_another_spelling() {
    let mut store = store();
    segment(store.conn(), "Zed", "main.rs - pantry-tracker", None, 0, 40);
    assert_eq!(discover(store.conn(), 0).unwrap().len(), 1);
    store
        .create_project(new_project("Pantry Tracker", None), 1)
        .unwrap();
    assert!(discover(store.conn(), 0).unwrap().is_empty());
}

#[test]
fn csv_parses_quotes_and_line_endings() {
    let rows = parse_csv("name,client\r\n\"Acme, Web\",\"The \"\"Big\"\" Co\"\n\nSolo,\n");
    assert_eq!(
        rows,
        vec![
            vec!["name".to_string(), "client".to_string()],
            vec!["Acme, Web".to_string(), "The \"Big\" Co".to_string()],
            vec!["Solo".to_string(), String::new()],
        ]
    );
}

#[test]
fn budgets_parse_as_hours_or_amounts() {
    assert_eq!(parse_budget("50h"), Some(("hours", 50.0)));
    assert_eq!(parse_budget("12.5 hours"), Some(("hours", 12.5)));
    assert_eq!(parse_budget("$5,000"), Some(("amount", 5000.0)));
    assert_eq!(parse_budget("800 EUR"), Some(("amount", 800.0)));
    assert_eq!(parse_budget("lots"), None);
}

#[test]
fn import_creates_projects_and_clients() {
    let mut store = store();
    store.create_project(new_project("Aevio", None), 1).unwrap();
    let summary = import_csv(
            &mut store,
            "Name,Client,Budget,Due date\nAcme Web,Acme Corp,50h,2026-10-15\nAcme Brand,acme corp,$900,soon\naevio,,,\n,Nobody,,\n",
            2,
        )
        .unwrap();
    assert_eq!(summary.created, 2);
    assert_eq!(summary.clients_created, 1);
    assert_eq!(summary.skipped.len(), 3);

    let projects = store.list_projects().unwrap();
    let web = projects.iter().find(|p| p.name == "Acme Web").unwrap();
    assert_eq!(web.budget_kind, "hours");
    assert_eq!(web.budget_value, Some(50.0));
    assert!(web.due_date.is_some());
    let brand = projects.iter().find(|p| p.name == "Acme Brand").unwrap();
    assert_eq!(brand.budget_kind, "amount");
    assert_eq!(brand.client_id, web.client_id);
    assert!(brand.due_date.is_none());

    assert!(import_csv(&mut store, "client\nAcme\n", 3).is_err());
}

#[test]
fn stats_sum_range_month_and_billable_time() {
    let store = store();
    let conn = store.conn();
    for (id, start, minutes, billable) in [("a", 0u64, 60u64, 1), ("b", 100 * MIN, 30, 0)] {
        conn.execute(
                "INSERT INTO time_entries (id, started_at, ended_at, description, project_id, status, source, billable, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'x', 'p1', 'approved', 'auto', ?4, 0, 0);",
                params![id, start as i64, (start + minutes * MIN) as i64, billable],
            )
            .unwrap();
    }
    let stats = project_stats(conn, 90 * MIN, 200 * MIN, 50 * MIN).unwrap();
    assert_eq!(stats.len(), 1);
    let s = &stats[0];
    assert_eq!(s.entries, 2);
    assert_eq!(s.total_ms, 90 * MIN);
    assert_eq!(s.range_ms, 30 * MIN);
    assert_eq!(s.month_ms, 30 * MIN);
    assert_eq!(s.billable_ms, 60 * MIN);
    assert_eq!(s.billable_month_ms, 0);
    assert_eq!(s.last_activity, Some(130 * MIN));
}

#[test]
fn deleting_a_project_with_invoiced_time_is_atomic() {
    let mut store = store();
    let project = store
        .create_project(new_project("Aevio", Some("figma.com")), 1)
        .unwrap();
    store
            .conn()
            .execute(
                "INSERT INTO time_entries (id, started_at, ended_at, description, project_id, status, source, billable, created_at, updated_at)
                 VALUES ('e', 0, 1, 'x', ?1, 'approved', 'auto', 0, 0, 0);",
                [&project.id],
            )
            .unwrap();
    store
            .conn()
            .execute(
                "INSERT INTO time_entries (id, started_at, ended_at, description, project_id, status, source, billable, invoice_id, created_at, updated_at)
                 VALUES ('invoiced', 0, 1, 'x', ?1, 'approved', 'auto', 1, 'inv', 0, 0);",
                [&project.id],
            )
            .unwrap();
    assert!(
        store
            .conn()
            .execute(
                "UPDATE time_entries SET project_id = NULL WHERE id = 'invoiced';",
                [],
            )
            .is_err()
    );
    assert!(store.delete_project(&project.id, 2).is_err());
    let linked_count: i64 = store
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM time_entries WHERE project_id = ?1;",
            [&project.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(linked_count, 2);
    let preserved_invoice: Option<String> = store
        .conn()
        .query_row(
            "SELECT invoice_id FROM time_entries WHERE id = 'invoiced';",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(preserved_invoice.as_deref(), Some("inv"));
    assert!(!hint_rules(store.conn(), &project.id).is_empty());
    let deleted_at: Option<i64> = store
        .conn()
        .query_row(
            "SELECT deleted_at FROM projects WHERE id = ?1;",
            [&project.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(deleted_at, None);
}

#[test]
fn invoiced_time_pins_the_client() {
    let mut store = store();
    let project = store.create_project(new_project("Aevio", None), 1).unwrap();
    store
            .conn()
            .execute(
                "INSERT INTO time_entries (id, started_at, ended_at, description, project_id, status, source, billable, invoice_id, created_at, updated_at)
                 VALUES ('e', 0, 1, 'x', ?1, 'approved', 'auto', 1, 'inv', 0, 0);",
                [&project.id],
            )
            .unwrap();
    let moved = store.update_project(
        &project.id,
        crate::models::UpdateProject {
            client_id: Some(Some("other".to_string())),
            ..Default::default()
        },
        2,
    );
    assert!(moved.is_err());
    // Other edits still go through.
    store
        .update_project(
            &project.id,
            crate::models::UpdateProject {
                name: Some("Aevio 2".to_string()),
                ..Default::default()
            },
            3,
        )
        .unwrap();
}

#[test]
fn a_null_patch_clears_optional_fields() {
    let patch: crate::models::UpdateProject =
        serde_json::from_str(r#"{"clientId": null, "hourlyRate": 90, "dueDate": null}"#).unwrap();
    assert_eq!(patch.client_id, Some(None));
    assert_eq!(patch.hourly_rate, Some(Some(90.0)));
    assert_eq!(patch.due_date, Some(None));
}

#[test]
fn optional_due_date_is_preserved_on_create_and_cleared_on_update() {
    let mut store = store();
    let proj = store
        .create_project(
            crate::models::NewProject {
                client_id: None,
                name: "No Due Date".to_string(),
                color: "#75a4e5".to_string(),
                description: None,
                ai_hints: None,
                status: None,
                due_date: None,
                budget_kind: None,
                budget_value: None,
                budget_period: None,
                billable_default: None,
                hourly_rate: None,
            },
            1_000,
        )
        .unwrap();
    assert_eq!(proj.due_date, None);

    let updated = store
        .update_project(
            &proj.id,
            crate::models::UpdateProject {
                due_date: Some(Some(50_000)),
                ..Default::default()
            },
            2_000,
        )
        .unwrap();
    assert_eq!(updated.due_date, Some(50_000));

    let cleared = store
        .update_project(
            &proj.id,
            crate::models::UpdateProject {
                due_date: Some(None),
                ..Default::default()
            },
            3_000,
        )
        .unwrap();
    assert_eq!(cleared.due_date, None);
}
