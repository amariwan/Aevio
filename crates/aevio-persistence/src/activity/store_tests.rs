use super::*;

fn store() -> ActivityStore {
    let mut s = ActivityStore::from_conn(Connection::open_in_memory().expect("in-memory db"))
        .expect("schema");
    s.tracking_hours.enabled = false;
    s
}

fn sample(app: &str, title: &str) -> Option<WindowSample> {
    Some(WindowSample {
        app: app.to_string(),
        title: title.to_string(),
        bundle_id: None,
        url: None,
        domain: None,
        keeps_display_awake: false,
    })
}

#[test]
fn a_window_switch_closes_one_segment_and_opens_another() {
    let mut store = store();
    store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();
    assert!(!store.tick(sample("Code", "main.rs"), 0, 4_000).unwrap());
    // Slack is first seen at 7_000; 20-second grace period starts, so tick returns false
    assert!(!store.tick(sample("Slack", "#general"), 0, 7_000).unwrap());

    // While within the grace period, Code remains the open segment
    let intermediate = store.snapshot(0, 7_000).unwrap();
    assert_eq!(intermediate.segments.len(), 1);
    assert_eq!(intermediate.segments[0].app, "Code");
    assert_eq!(intermediate.segments[0].ended_at, None);

    // Sustained visit reaches 20 seconds (7_000 + 20_000 = 27_000)
    assert!(store.tick(sample("Slack", "#general"), 0, 27_000).unwrap());

    let snapshot = store.snapshot(0, 27_000).unwrap();
    assert_eq!(snapshot.segments.len(), 2);
    assert_eq!(snapshot.segments[0].app, "Code");
    assert_eq!(snapshot.segments[0].ended_at, Some(7_000));
    assert_eq!(snapshot.segments[1].app, "Slack");
    assert_eq!(snapshot.segments[1].ended_at, None);
    assert_eq!(snapshot.tracked_ms, 26_000);
}

#[test]
fn deleting_an_entry_also_deletes_its_activity_segments() {
    let mut store = store();
    store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();
    store.tick(sample("Slack", "#general"), 0, 5_000).unwrap();
    // Sustain Slack past the 20s grace threshold (5_000 + 20_000 = 25_000)
    store.tick(sample("Slack", "#general"), 0, 25_000).unwrap();
    let entries = store
        .rebuild_time_entries_in_range(0, 25_000, 25_000)
        .unwrap();
    // One live session covers both apps.
    assert_eq!(entries.len(), 1);
    let code_entry = entries.first().unwrap();

    store.delete_time_entry(&code_entry.id, 26_000).unwrap();

    // Its segments go with it, the one capture was writing included:
    // capture must not keep writing into a deleted row.
    let remaining = store.snapshot(0, 26_000).unwrap();
    assert!(remaining.segments.is_empty());
    store.tick(sample("Slack", "#general"), 0, 27_000).unwrap();
    let resumed = store.snapshot(0, 28_000).unwrap();
    assert_eq!(resumed.segments.len(), 1);
    assert_eq!(resumed.segments[0].app, "Slack");
    assert_eq!(resumed.segments[0].started_at, 27_000);
    let linked: i64 = store
        .conn
        .query_row(
            "SELECT COUNT(*) FROM segments WHERE entry_id = ?1;",
            params![code_entry.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(linked, 0);
}

#[test]
fn idle_crosses_into_an_automatic_break_and_back() {
    let mut store = store();
    store.set_idle_threshold_ms(60_000).unwrap();
    store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();

    store
        .tick(sample("Code", "main.rs"), 90_000, 100_000)
        .unwrap();
    assert_eq!(
        store.snapshot(0, 100_000).unwrap().current.unwrap().kind,
        KIND_BREAK
    );

    store.tick(sample("Code", "main.rs"), 0, 130_000).unwrap();
    let snapshot = store.snapshot(0, 130_000).unwrap();
    let current = snapshot.current.unwrap();
    assert_eq!(current.kind, KIND_ACTIVITY);
    // The break is backdated to the last input, 90s before it was noticed.
    assert_eq!(snapshot.segments[0].ended_at, Some(10_000));
    assert_eq!(snapshot.break_ms, 120_000);
}

#[test]
fn going_away_ends_activity_at_once_and_holds_a_break_until_input() {
    let mut store = store();
    store.set_idle_threshold_ms(60_000).unwrap();
    store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();

    // Locked 5s after the last input: the segment ends at the lock, not
    // backdated to the input, and a background wake keeps it a break.
    assert!(store.tick_away(10_000).unwrap());
    assert!(!store.tick_away(11_000).unwrap());
    let snapshot = store.snapshot(0, 11_000).unwrap();
    assert_eq!(snapshot.segments[0].ended_at, Some(10_000));
    assert_eq!(snapshot.current.unwrap().kind, KIND_BREAK);

    store.tick(sample("Code", "main.rs"), 0, 20_000).unwrap();
    let current = store.snapshot(0, 20_000).unwrap().current.unwrap();
    assert_eq!(current.kind, KIND_ACTIVITY);
    assert_eq!(current.started_at, 20_000);
}

/// Ticks once a second through `[from, to)` with Zen in front. `inputs`
/// are the moments the user touched the keyboard or mouse, and
/// `watching` says whether Zen keeps the display awake at a moment.
fn watch_zen(
    store: &mut ActivityStore,
    (from, to): (u64, u64),
    inputs: &[u64],
    watching: impl Fn(u64) -> bool,
) {
    for now in (from..to).step_by(1_000) {
        let last_input = inputs.iter().rev().find(|&&at| at <= now).unwrap();
        let mut zen = sample("Zen", "").unwrap();
        zen.keeps_display_awake = watching(now);
        store.tick(Some(zen), now - last_input, now).unwrap();
    }
}

/// Three hours of a show in Zen, touched only now and then, the way the
/// evening of 2026-09-25 was captured: the player drops its display
/// assertion for 40 seconds every 10 minutes (between episodes, ads) and
/// the show is paused at 3h, after which nobody touches the Mac.
const SHOW_END: u64 = 180 * MIN;
const SHOW_INPUTS: [u64; 6] = [0, 21 * MIN, 47 * MIN, 88 * MIN, 131 * MIN, 170 * MIN];

fn show_playing(at: u64) -> bool {
    at < SHOW_END && (at < 10 * MIN || at % (10 * MIN) >= 40_000)
}

#[test]
fn a_show_watched_in_the_foreground_is_one_continuous_session() {
    let mut store = store();
    watch_zen(
        &mut store,
        (0, SHOW_END + 10 * MIN),
        &SHOW_INPUTS,
        show_playing,
    );

    let segments = store.snapshot(0, SHOW_END + 10 * MIN).unwrap().segments;
    let spans: Vec<_> = segments
        .iter()
        .map(|s| (s.kind.as_str(), s.started_at, s.ended_at))
        .collect();
    // The break starts when the show stopped playing, not at the last
    // input 10 minutes before it.
    let paused = SHOW_END - 1_000;
    assert_eq!(
        spans,
        vec![(KIND_ACTIVITY, 0, Some(paused)), (KIND_BREAK, paused, None)]
    );

    let entries = store
        .rebuild_time_entries_in_range(0, SHOW_END + 10 * MIN, SHOW_END + 10 * MIN)
        .unwrap();
    let entries: Vec<_> = entries
        .iter()
        .map(|e| (e.started_at, e.ended_at, e.status.as_str()))
        .collect();
    // Closed, so it is queued for classification.
    assert_eq!(entries, vec![(0, paused, "processing")]);
}

#[test]
fn the_same_evening_without_a_display_assertion_is_idle() {
    // The counterfactual: identical input, but Zen never keeps the
    // display awake, so every stretch without input is a break and each
    // input leaves only an instant of activity, too short to be an entry.
    let mut store = store();
    watch_zen(&mut store, (0, SHOW_END + 10 * MIN), &SHOW_INPUTS, |_| {
        false
    });

    let snapshot = store.snapshot(0, SHOW_END + 10 * MIN).unwrap();
    assert_eq!(snapshot.tracked_ms, 0);
    let entries = store
        .rebuild_time_entries_in_range(0, SHOW_END + 10 * MIN, SHOW_END + 10 * MIN)
        .unwrap();
    assert!(entries.is_empty());
}

#[test]
fn a_video_does_not_end_an_idle_break() {
    // Autoplay after the user left must not bring them back: only input
    // ends a break.
    let mut store = store();
    watch_zen(&mut store, (0, 10 * MIN), &[0], |_| false);
    watch_zen(&mut store, (10 * MIN, 20 * MIN), &[0], |_| true);

    let snapshot = store.snapshot(0, 20 * MIN).unwrap();
    assert_eq!(snapshot.current.unwrap().kind, KIND_BREAK);
    assert_eq!(snapshot.segments.len(), 2);
}

#[test]
fn a_manual_focus_is_not_interrupted_by_capture() {
    let mut store = store();
    store
        .start_session(KIND_FOCUS, Some("Deep work"), 1_000)
        .unwrap();
    store.tick(sample("Slack", "#random"), 0, 2_000).unwrap();

    let snapshot = store.snapshot(0, 2_000).unwrap();
    assert_eq!(snapshot.segments.len(), 1);
    assert_eq!(snapshot.current.unwrap().kind, KIND_FOCUS);
    assert_eq!(snapshot.focus_ms, 1_000);
}

#[test]
fn pausing_capture_closes_the_open_segment() {
    let mut store = store();
    store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();
    store.set_capture_enabled(false, 5_000).unwrap();
    store.tick(sample("Code", "main.rs"), 0, 5_000).unwrap();

    let snapshot = store.snapshot(0, 5_000).unwrap();
    assert_eq!(snapshot.current, None);
    assert_eq!(snapshot.segments[0].ended_at, Some(5_000));
}

#[test]
fn an_unclean_shutdown_does_not_leave_a_segment_running_forever() {
    let dir = std::env::temp_dir().join(format!("aevio-activity-{}", now_epoch_ms()));
    fs::create_dir_all(&dir).unwrap();
    {
        let mut store = ActivityStore::load(&dir).unwrap();
        // Epoch 0 is midnight UTC; the default tracking hours would drop
        // the tick depending on the machine's timezone. The test is about
        // the unclean shutdown, not the window.
        store.tracking_hours.enabled = false;
        store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();
    }
    let reloaded = ActivityStore::load(&dir).unwrap();
    let snapshot = reloaded.snapshot(0, 9_999_999).unwrap();
    assert_eq!(snapshot.current, None);
    assert_eq!(snapshot.segments[0].ended_at, Some(1_000));
}

#[test]
fn idle_does_not_interrupt_a_manual_focus() {
    let mut store = store();
    store.set_idle_threshold_ms(60_000).unwrap();
    store
        .start_session(KIND_FOCUS, Some("Deep work"), 1_000)
        .unwrap();

    store
        .tick(sample("Code", "main.rs"), 120_000, 121_000)
        .unwrap();
    let snapshot = store.snapshot(0, 121_000).unwrap();
    assert_eq!(snapshot.current.unwrap().kind, KIND_FOCUS);
    assert_eq!(snapshot.break_ms, 0);
}

#[test]
fn a_meeting_session_is_recorded_as_tracked_time() {
    let mut store = store();
    store
        .start_session(KIND_MEETING, Some("Meeting"), 1_000)
        .unwrap();
    assert_eq!(
        store.snapshot(0, 1_000).unwrap().current.unwrap().kind,
        KIND_MEETING
    );
}

#[test]
fn tick_summary_matches_snapshot_totals_without_a_segment_list() {
    let mut store = store();
    store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();
    store.tick(sample("Code", "main.rs"), 0, 5_000).unwrap();

    let snapshot = store.snapshot(0, 5_000).unwrap();
    let tick = store.tick_summary(5_000).unwrap();

    assert_eq!(tick.tracked_ms, snapshot.tracked_ms);
    assert_eq!(tick.focus_ms, snapshot.focus_ms);
    assert_eq!(tick.break_ms, snapshot.break_ms);
    assert_eq!(tick.unreviewed, snapshot.unreviewed);
    assert_eq!(
        tick.current.map(|segment| segment.app),
        snapshot.current.map(|segment| segment.app)
    );
}

#[test]
fn mark_reviewed_updates_the_open_segment_in_memory_too() {
    let mut store = store();
    store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();
    let id = store.snapshot(0, 1_000).unwrap().current.unwrap().id;

    store.mark_reviewed(id).unwrap();
    let tick = store.tick_summary(2_000).unwrap();
    assert!(tick.current.unwrap().reviewed);
    assert_eq!(tick.unreviewed, 0);
}

#[test]
fn categories_crud_works() {
    let mut store = store();
    let cats = store.list_categories().unwrap();
    assert_eq!(cats.len(), 11); // Seeded default 11 categories

    let created = store
        .create_category(
            NewCategory {
                name: "Custom Category".to_string(),
                color: "#123456".to_string(),
                description: Some("Custom desc".to_string()),
                ai_prompt: None,
                billable_default: Some(true),
                counts_as_work: Some(true),
                sort: Some(13),
            },
            1_000,
        )
        .unwrap();
    assert_eq!(created.name, "Custom Category");

    let updated = store
        .update_category(
            &created.id,
            UpdateCategory {
                name: Some("Renamed Category".to_string()),
                color: None,
                description: None,
                ai_prompt: None,
                billable_default: None,
                counts_as_work: None,
                archived: None,
                sort: None,
            },
            2_000,
        )
        .unwrap();
    assert_eq!(updated.name, "Renamed Category");

    store.delete_category(&created.id, 3_000).unwrap();
    let remaining = store.list_categories().unwrap();
    assert_eq!(remaining.len(), 11);
}

#[test]
fn projects_and_clients_crud_works() {
    let mut store = store();
    let client = store
        .create_client(
            NewClient {
                name: "Acme Corp".to_string(),
                email: Some("contact@acme.com".to_string()),
                address: None,
                default_rate: Some(120.0),
                currency: Some("USD".to_string()),
                notes: None,
            },
            1_000,
        )
        .unwrap();
    assert_eq!(client.name, "Acme Corp");

    let proj = store
        .create_project(
            NewProject {
                client_id: Some(client.id.clone()),
                name: "Acme Web".to_string(),
                color: "#75a4e5".to_string(),
                description: Some("Web platform".to_string()),
                ai_hints: Some("acme, web".to_string()),
                status: Some("active".to_string()),
                due_date: None,
                budget_kind: Some("hours".to_string()),
                budget_value: Some(50.0),
                budget_period: Some("total".to_string()),
                billable_default: Some(true),
                hourly_rate: Some(120.0),
            },
            2_000,
        )
        .unwrap();
    assert_eq!(proj.name, "Acme Web");

    let projects = store.list_projects().unwrap();
    assert_eq!(projects.len(), 1);
}

fn new_client(name: &str) -> NewClient {
    NewClient {
        name: name.to_string(),
        email: Some("billing@acme.test".to_string()),
        address: Some("1 Main St.".to_string()),
        default_rate: Some(120.0),
        currency: Some("EUR".to_string()),
        notes: Some("Net 15".to_string()),
    }
}

fn new_project_for(client_id: Option<&str>, name: &str) -> NewProject {
    NewProject {
        client_id: client_id.map(str::to_string),
        name: name.to_string(),
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
    }
}

#[test]
fn tasks_keep_project_labels_keywords_and_ai_instructions_separate() {
    let mut store = store();
    let client = store.create_client(new_client("Acme"), 1).unwrap();
    let project = store
        .create_project(new_project_for(Some(&client.id), "Acme Web"), 2)
        .unwrap();
    let label = store
        .create_label(
            NewLabel {
                name: "Urgent".to_string(),
                color: "#e97e7b".to_string(),
            },
            3,
        )
        .unwrap();

    let task = store
        .create_task(
            NewTask {
                project_id: project.id.clone(),
                title: "Prepare launch notes".to_string(),
                description: None,
                status: None,
                priority: Some("high".to_string()),
                due_date: Some(10),
                keywords: Some(vec![" Launch ".to_string(), "launch".to_string()]),
                ai_instructions: Some("Prefer release notes context".to_string()),
                label_ids: Some(vec![label.id.clone()]),
            },
            4,
        )
        .unwrap();

    assert_eq!(task.project_id, project.id);
    assert_eq!(task.keywords, vec!["launch"]);
    assert_eq!(task.labels, vec![label]);
    assert_eq!(
        task.ai_instructions.as_deref(),
        Some("Prefer release notes context")
    );

    let listed = store.list_tasks(Some(&project.id)).unwrap();
    assert_eq!(listed, vec![task]);
}

#[test]
fn a_client_patch_edits_clears_and_leaves_fields_alone() {
    let mut store = store();
    let client = store.create_client(new_client("Acme"), 1).unwrap();
    assert_eq!(client.notes.as_deref(), Some("Net 15"));

    let renamed = store
        .update_client(
            &client.id,
            serde_json::from_str(r#"{"name": " Acme Inc ", "email": null, "defaultRate": 90}"#)
                .unwrap(),
            2,
        )
        .unwrap();
    assert_eq!(renamed.name, "Acme Inc");
    assert_eq!(renamed.email, None);
    assert_eq!(renamed.default_rate, Some(90.0));
    // Untouched fields stay.
    assert_eq!(renamed.address.as_deref(), Some("1 Main St."));
    assert_eq!(renamed.currency.as_deref(), Some("EUR"));
    assert_eq!(renamed.notes.as_deref(), Some("Net 15"));
    assert_eq!(store.list_clients().unwrap(), vec![renamed]);

    assert!(
        store
            .update_client(
                &client.id,
                serde_json::from_str(r#"{"name": "  "}"#).unwrap(),
                3
            )
            .is_err()
    );
    assert!(store.create_client(new_client(" "), 4).is_err());
}

#[test]
fn archiving_a_client_keeps_it_and_its_projects_and_can_be_undone() {
    let mut store = store();
    let client = store.create_client(new_client("Acme"), 1).unwrap();
    let project = store
        .create_project(new_project_for(Some(&client.id), "Acme Web"), 2)
        .unwrap();

    let archive: UpdateClient = serde_json::from_str(r#"{"archived": true}"#).unwrap();
    let archived = store.update_client(&client.id, archive, 10).unwrap();
    assert_eq!(archived.archived_at, Some(10));
    // Archiving again keeps the first date.
    let again: UpdateClient = serde_json::from_str(r#"{"archived": true}"#).unwrap();
    let again = store.update_client(&client.id, again, 20).unwrap();
    assert_eq!(again.archived_at, Some(10));

    // Still listed (so its projects resolve its name) and still linked.
    let listed = store.list_clients().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].archived_at, Some(10));
    let projects = store.list_projects().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, project.id);
    assert_eq!(projects[0].client_id.as_deref(), Some(client.id.as_str()));
    assert_eq!(projects[0].status, "active");

    let restore: UpdateClient = serde_json::from_str(r#"{"archived": false}"#).unwrap();
    let restored = store.update_client(&client.id, restore, 30).unwrap();
    assert_eq!(restored.archived_at, None);
}

#[test]
fn deleting_a_client_detaches_its_projects_and_keeps_them_and_their_time() {
    let mut store = store();
    let client = store.create_client(new_client("Acme"), 1).unwrap();
    let other = store.create_client(new_client("Other"), 1).unwrap();
    let project = store
        .create_project(new_project_for(Some(&client.id), "Acme Web"), 2)
        .unwrap();
    let kept = store
        .create_project(new_project_for(Some(&other.id), "Other Web"), 2)
        .unwrap();
    store
            .conn()
            .execute(
                "INSERT INTO time_entries (id, started_at, ended_at, description, project_id, status, source, billable, created_at, updated_at)
                 VALUES ('e', 0, 3600000, 'x', ?1, 'approved', 'auto', 1, 0, 0);",
                [&project.id],
            )
            .unwrap();

    store.delete_client(&client.id, 50).unwrap();

    let clients = store.list_clients().unwrap();
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].id, other.id);
    let projects = store.list_projects().unwrap();
    assert_eq!(projects.len(), 2, "no project is deleted or hidden");
    let detached = projects.iter().find(|p| p.id == project.id).unwrap();
    assert_eq!(detached.client_id, None);
    assert_eq!(detached.status, "active");
    let untouched = projects.iter().find(|p| p.id == kept.id).unwrap();
    assert_eq!(untouched.client_id.as_deref(), Some(other.id.as_str()));
    let entry_project: Option<String> = store
        .conn()
        .query_row(
            "SELECT project_id FROM time_entries WHERE id = 'e'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(entry_project.as_deref(), Some(project.id.as_str()));
}

#[test]
fn a_client_with_invoices_cannot_be_deleted() {
    let mut store = store();
    let client = store.create_client(new_client("Acme"), 1).unwrap();
    let project = store
        .create_project(new_project_for(Some(&client.id), "Acme Web"), 2)
        .unwrap();
    store
            .conn()
            .execute(
                "INSERT INTO invoices (id, client_id, client_name, currency, status, created_at, updated_at)
                 VALUES ('inv', ?1, 'Acme', 'USD', 'draft', 0, 0);",
                [&client.id],
            )
            .unwrap();

    let refused = store.delete_client(&client.id, 50).unwrap_err();
    assert!(refused.contains("Archive it instead"), "{refused}");
    // Nothing changed: still listed, project still attached.
    assert_eq!(store.list_clients().unwrap().len(), 1);
    let projects = store.list_projects().unwrap();
    assert_eq!(projects[0].id, project.id);
    assert_eq!(projects[0].client_id.as_deref(), Some(client.id.as_str()));
}

const MIN: u64 = 60_000;

/// Code then Slack for 20 minutes, 10 minutes away, then back to Code:
/// the first session closes, the second is still building.
fn tracked_morning(store: &mut ActivityStore) {
    store.set_idle_threshold_ms(5 * MIN).unwrap();
    store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();
    store
        .tick(sample("Slack", "#general"), 0, 12 * MIN)
        .unwrap();
    store
        .tick(sample("Slack", "#general"), 5 * MIN, 25 * MIN)
        .unwrap();
    store.tick(sample("Code", "main.rs"), 0, 35 * MIN).unwrap();
}

fn live_entries(store: &ActivityStore) -> Vec<TimeEntry> {
    store.list_time_entries(0, u64::MAX / 2).unwrap()
}

#[test]
fn rebuilding_the_same_day_never_duplicates_entries() {
    let mut store = store();
    tracked_morning(&mut store);
    let first = store
        .rebuild_time_entries_in_range(0, 60 * MIN, 36 * MIN)
        .unwrap();
    let second = store
        .rebuild_time_entries_in_range(0, 60 * MIN, 37 * MIN)
        .unwrap();
    let third = store
        .rebuild_time_entries_in_range(0, 60 * MIN, 38 * MIN)
        .unwrap();
    assert_eq!(first.len(), 2);
    let ids = |list: &[TimeEntry]| list.iter().map(|e| e.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(&first), ids(&second));
    assert_eq!(ids(&second), ids(&third));
    // The open block kept its id while it grew.
    assert_eq!(third[1].status, "building");
    assert_eq!(third[1].ended_at, 38 * MIN);
}

#[test]
fn live_classification_runs_at_thirty_then_every_fifteen_minutes() {
    let mut store = store();
    store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();
    let start = 1_000;
    for minute in [29, 30, 44, 45, 59, 60] {
        let now = start + minute * MIN;
        store.rebuild_range(0, now, now).unwrap();
        let count = crate::ai::store::enqueue_live_due(store.conn(), now).unwrap();
        assert_eq!(count, usize::from(matches!(minute, 30 | 45 | 60)));
        if count > 0 {
            let entry = live_entries(&store).remove(0);
            let job = crate::ai::store::next_due(store.conn(), now)
                .unwrap()
                .unwrap();
            assert_eq!(entry.id, job.entry_id);
            crate::ai::store::set_job_state(store.conn(), &job.id, "done", now).unwrap();
        }
    }
    store.stop_session(start + 61 * MIN).unwrap();
    store
        .rebuild_range(0, start + 70 * MIN, start + 70 * MIN)
        .unwrap();
    assert_eq!(
        crate::ai::store::enqueue_live_due(store.conn(), start + 75 * MIN).unwrap(),
        0
    );
}

#[test]
fn overnight_session_stays_in_previous_five_am_day() {
    let mut store = store();
    let hour = 60 * MIN;
    store
        .tick(sample("Code", "night.rs"), 0, 23 * hour + 30 * MIN)
        .unwrap();
    store.stop_session(28 * hour).unwrap();
    let entries = store
        .rebuild_time_entries_in_range(5 * hour, 29 * hour, 29 * hour)
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].started_at, 23 * hour + 30 * MIN);
    assert_eq!(entries[0].ended_at, 28 * hour);
    assert_eq!(entries[0].status, "processing");
}

#[test]
fn blank_manual_entry_is_classified_only_after_its_end() {
    let mut store = store();
    let entry = store
        .create_manual_entry(
            NewTimeEntry {
                started_at: 1_000,
                ended_at: 16 * MIN,
                description: String::new(),
                category_id: None,
                project_id: None,
                billable: None,
                review: false,
            },
            2_000,
        )
        .unwrap();
    assert_eq!(entry.status, "pending");
    store.rebuild_range(0, 20 * MIN, 15 * MIN).unwrap();
    assert!(
        crate::ai::store::next_due(store.conn(), 15 * MIN)
            .unwrap()
            .is_none()
    );
    store.rebuild_range(0, 20 * MIN, 16 * MIN).unwrap();
    assert_eq!(live_entries(&store)[0].status, "processing");
    assert!(
        crate::ai::store::next_due(store.conn(), 16 * MIN)
            .unwrap()
            .is_some()
    );
}

#[test]
fn a_manual_session_dragged_over_the_live_entry_leaves_it_running() {
    let mut store = store();
    let start = 13 * 60 * MIN;
    // Tracking since 1:00 PM, still going at 2:00 PM.
    store.tick(sample("Code", "main.rs"), 0, start).unwrap();
    let now = start + 60 * MIN;
    store.tick(sample("Code", "main.rs"), 0, now).unwrap();
    store.rebuild_range(0, now + MIN, now).unwrap();
    let live = live_entries(&store);
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].status, "building");

    // Drag a session from 1:30 PM to 2:30 PM.
    let dragged = store
        .create_manual_entry(
            NewTimeEntry {
                started_at: start + 30 * MIN,
                ended_at: now + 30 * MIN,
                description: "Untitled session".to_string(),
                category_id: None,
                project_id: None,
                billable: None,
                review: true,
            },
            now,
        )
        .unwrap();
    store.rebuild_range(0, now + 40 * MIN, now).unwrap();

    let entries = live_entries(&store);
    assert_eq!(entries.len(), 2);
    let open = entries.iter().find(|e| e.id == live[0].id).unwrap();
    assert_eq!(open.status, "building");
    assert_eq!(open.started_at, start);
    assert_eq!(open.ended_at, now);
    assert!(entries.iter().any(|e| e.id == dragged.id));
    // The live segments stay owned by the open entry (no untinted blocks).
    let owned: i64 = store
        .conn
        .query_row(
            "SELECT COUNT(*) FROM segments WHERE entry_id = ?1;",
            params![open.id],
            |row| row.get(0),
        )
        .unwrap();
    assert!(owned > 0);
    let orphaned: i64 = store
        .conn
        .query_row(
            "SELECT COUNT(*) FROM segments WHERE entry_id IS NULL;",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(orphaned, 0);
}

#[test]
fn manual_drag_session_stays_pending_without_ai_categorization() {
    let mut store = store();
    let cat = store
        .create_category(
            aevio_core::models::NewCategory {
                name: "Break".to_string(),
                color: "#64748b".to_string(),
                description: None,
                ai_prompt: None,
                billable_default: Some(false),
                counts_as_work: Some(false),
                sort: None,
            },
            1_000,
        )
        .unwrap();
    let entry = store
        .create_manual_entry(
            NewTimeEntry {
                started_at: 1_000,
                ended_at: 10 * MIN,
                description: "Untitled session".to_string(),
                category_id: Some(cat.id.clone()),
                project_id: None,
                billable: None,
                review: true,
            },
            11 * MIN,
        )
        .unwrap();
    assert_eq!(entry.status, "pending");
    assert_eq!(entry.category_id, Some(cat.id.clone()));
    assert_eq!(entry.description, "Untitled session");
    assert_eq!(entry.project_id, None);
    let has_classify_job: bool = store
            .conn()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM classify_jobs WHERE entry_id = ?1 AND kind IN ('classify', 'embed'))",
                rusqlite::params![entry.id],
                |row| row.get(0),
            )
            .unwrap();
    assert!(!has_classify_job);

    store
        .approve_time_entries(std::slice::from_ref(&entry.id), "user", 12 * MIN)
        .unwrap();
    store
        .unapprove_time_entries(std::slice::from_ref(&entry.id), 13 * MIN)
        .unwrap();
    let reopened = store.time_entry(&entry.id).unwrap();
    assert_eq!(reopened.status, "pending");
    assert_eq!(reopened.approved_by, None);
    assert_eq!(reopened.category_id, Some(cat.id));
}

#[test]
fn a_closed_entry_is_queued_for_classification() {
    let mut store = store();
    tracked_morning(&mut store);
    store.rebuild_range(0, 60 * MIN, 36 * MIN).unwrap();
    let entries = live_entries(&store);
    assert_eq!(entries[0].status, "processing");
    assert_eq!(
        entries[0].ai.as_ref().and_then(|ai| ai.state.as_deref()),
        Some("queued")
    );
    assert_eq!(entries[1].status, "building");
    assert_eq!(crate::ai::store::queued_count(&store.conn).unwrap(), 1);
}

#[test]
fn an_edited_description_survives_a_rebuild() {
    let mut store = store();
    store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();
    store.rebuild_range(0, 60 * MIN, 5 * MIN).unwrap();
    let id = live_entries(&store)[0].id.clone();
    store
        .update_time_entry(
            &id,
            UpdateTimeEntry {
                description: Some("Fixed the rebuild".into()),
                category_id: None,
                project_id: None,
                started_at: None,
                ended_at: None,
                status: None,
                billable: None,
            },
            6 * MIN,
        )
        .unwrap();
    store.rebuild_range(0, 60 * MIN, 7 * MIN).unwrap();
    let entry = &live_entries(&store)[0];
    assert_eq!(entry.id, id);
    assert_eq!(entry.description, "Fixed the rebuild");
    assert_eq!(entry.ended_at, 7 * MIN);
}

/// Seeds a pending entry with a suggestion, as the worker would.
fn suggested_entry(
    store: &mut ActivityStore,
    started: u64,
    value: &str,
    dominant_key: &str,
) -> String {
    let id = uuid::Uuid::now_v7().to_string();
    store
            .conn
            .execute(
                "INSERT INTO time_entries (id, started_at, ended_at, description, category_id, status, source, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'Figma', ?4, 'pending', 'auto', 0, 0);",
                params![id, started as i64, (started + 30 * MIN) as i64, value],
            )
            .unwrap();
    let decision = aevio_core::ai::arbiter::Decision {
        value: Some(value.to_string()),
        confidence: 0.7,
        raw_confidence: 0.7,
        alternatives: vec![],
        signals: vec![],
        engine: "full",
        tier: "model",
    };
    let dominant = aevio_core::models::Dominant {
        kind: "domain".into(),
        key: dominant_key.into(),
        label: dominant_key.into(),
        share: 0.9,
    };
    crate::ai::store::insert_suggestion(
        &store.conn,
        &id,
        aevio_core::ai::Field::Category,
        &decision,
        Some(&dominant),
        "test",
        None,
        started,
    )
    .unwrap();
    id
}

fn set_category(store: &mut ActivityStore, id: &str, category: &str) {
    store
        .update_time_entry(
            id,
            UpdateTimeEntry {
                description: None,
                category_id: Some(category.into()),
                project_id: None,
                started_at: None,
                ended_at: None,
                status: None,
                billable: None,
            },
            1,
        )
        .unwrap();
}

#[test]
fn approval_records_whether_the_suggestion_was_kept() {
    let mut store = store();
    let kept = suggested_entry(&mut store, 0, "coding", "github.com");
    let changed = suggested_entry(&mut store, 40 * MIN, "coding", "github.com");
    set_category(&mut store, &changed, "review");
    store
        .approve_time_entries(&[kept.clone(), changed.clone()], "user", 2)
        .unwrap();

    let outcome = |id: &str| {
        crate::ai::store::latest_suggestions(&store.conn, id).unwrap()[0]
            .outcome
            .clone()
    };
    assert_eq!(outcome(&kept).as_deref(), Some("accepted"));
    assert_eq!(outcome(&changed).as_deref(), Some("changed"));
    // Both now wait for a vector so they can join the kNN store.
    assert_eq!(crate::ai::store::queued_count(&store.conn).unwrap(), 2);
}

#[test]
fn reject_clears_the_prefilled_category_and_records_it() {
    let mut store = store();
    let id = suggested_entry(&mut store, 0, "coding", "github.com");
    store.reject_time_entry(&id, 5).unwrap();
    let detail = store.get_entry_detail(&id).unwrap();
    assert_eq!(detail.entry.category_id, None);
    assert_eq!(detail.entry.status, "pending");
    assert_eq!(detail.suggestions[0].outcome.as_deref(), Some("rejected"));
    let ai = live_entries(&store)[0].ai.clone();
    assert_eq!(ai.and_then(|ai| ai.category_confidence), None);
}

#[test]
fn three_consistent_corrections_offer_a_rule_once() {
    let mut store = store();
    for i in 0..2 {
        let id = suggested_entry(&mut store, i * 40 * MIN, "coding", "figma.com");
        set_category(&mut store, &id, "design");
        store.approve_time_entries(&[id], "user", 2).unwrap();
    }
    let third = suggested_entry(&mut store, 200 * MIN, "coding", "figma.com");
    assert!(
        store
            .get_entry_detail(&third)
            .unwrap()
            .rule_suggestion
            .is_none()
    );

    set_category(&mut store, &third, "design");
    let offer = store
        .get_entry_detail(&third)
        .unwrap()
        .rule_suggestion
        .expect("a rule suggestion after the third correction");
    assert_eq!(offer.pattern, "figma.com");
    assert_eq!(offer.value_id.as_deref(), Some("design"));
    assert_eq!(offer.corrections, 3);

    store.resolve_rule_suggestion(&offer, false, 3).unwrap();
    assert!(
        store
            .get_entry_detail(&third)
            .unwrap()
            .rule_suggestion
            .is_none()
    );
    // A dismissed suggestion is stored disabled, so T0 never applies it.
    let rules = crate::ai::store::load_rules(&store.conn).unwrap();
    assert!(rules.is_empty());
}

#[test]
fn rejections_count_toward_a_rule_suggestion() {
    let mut store = store();
    for i in 0..2 {
        let id = suggested_entry(&mut store, i * 40 * MIN, "coding", "figma.com");
        store.reject_time_entry(&id, 2).unwrap();
        set_category(&mut store, &id, "design");
        store.approve_time_entries(&[id], "user", 3).unwrap();
    }
    let third = suggested_entry(&mut store, 200 * MIN, "coding", "figma.com");
    store.reject_time_entry(&third, 4).unwrap();
    set_category(&mut store, &third, "design");
    let offer = store
        .get_entry_detail(&third)
        .unwrap()
        .rule_suggestion
        .expect("rejections followed by a pick are corrections");
    assert_eq!(offer.corrections, 3);
    assert_eq!(offer.value_id.as_deref(), Some("design"));
}

#[test]
fn recategorizing_an_unapproved_entry_is_a_correction() {
    let mut store = store();
    let auto = suggested_entry(&mut store, 0, "coding", "github.com");
    store
        .conn
        .execute(
            "UPDATE suggestions SET outcome = 'auto' WHERE entry_id = ?1;",
            params![auto],
        )
        .unwrap();
    let accepted = suggested_entry(&mut store, 40 * MIN, "coding", "github.com");
    store
        .approve_time_entries(&[auto.clone(), accepted.clone()], "user", 2)
        .unwrap();
    store
        .unapprove_time_entries(&[auto.clone(), accepted.clone()], 3)
        .unwrap();
    set_category(&mut store, &auto, "review");
    set_category(&mut store, &accepted, "coding");

    let outcome = |id: &str| {
        crate::ai::store::latest_suggestions(&store.conn, id).unwrap()[0]
            .outcome
            .clone()
    };
    assert_eq!(outcome(&auto).as_deref(), Some("changed"));
    // Saving the same value again is not a correction.
    assert_eq!(outcome(&accepted).as_deref(), Some("accepted"));
}

#[test]
fn calibration_learns_from_verdicts_on_model_suggestions() {
    let mut store = store();
    let kept = suggested_entry(&mut store, 0, "coding", "github.com");
    let fixed = suggested_entry(&mut store, 40 * MIN, "coding", "github.com");
    let open = suggested_entry(&mut store, 80 * MIN, "coding", "github.com");
    set_category(&mut store, &fixed, "review");
    store
        .approve_time_entries(&[kept, fixed], "user", 2)
        .unwrap();
    let _ = open;

    let now = 10 * MIN;
    let mut samples = crate::ai::store::calibration_samples(&store.conn, now).unwrap();
    samples.sort_by_key(|a| a.1);
    assert_eq!(samples, vec![(0.7, false), (0.7, true)]);
    assert_eq!(
        crate::ai::store::outcome_count(&store.conn, now).unwrap(),
        2
    );
    assert_eq!(
        crate::ai::store::verdicts_since(&store.conn, 1, now).unwrap(),
        2
    );
    assert_eq!(
        crate::ai::store::verdicts_since(&store.conn, 2, now).unwrap(),
        0
    );
}

#[test]
fn reset_forgets_what_was_learned_but_keeps_the_entries() {
    let mut store = store();
    let id = suggested_entry(&mut store, 0, "coding", "github.com");
    store
        .approve_time_entries(std::slice::from_ref(&id), "user", 2)
        .unwrap();
    crate::ai::store::upsert_embedding(&store.conn, &id, &[1.0, 0.0], "h", "Xcode", "m", 3)
        .unwrap();
    crate::ai::store::insert_artifact(
        &store.conn,
        "category",
        "/tmp/m.mlmodelc",
        20,
        Some(0.8),
        true,
        4,
    )
    .unwrap();
    let curve = aevio_core::ai::calibration::Calibrator {
        x: vec![0.0, 1.0],
        y: vec![0.0, 1.0],
        samples: 50,
    };
    crate::ai::store::insert_calibration(&store.conn, &curve, 5).unwrap();
    assert_eq!(crate::ai::store::labeled_count(&store.conn).unwrap(), 1);
    assert!(
        crate::ai::store::active_calibration(&store.conn)
            .unwrap()
            .is_some()
    );

    let paths = crate::ai::store::reset_learned(&store.conn, 10).unwrap();
    assert_eq!(paths, vec!["/tmp/m.mlmodelc".to_string()]);
    assert!(
        crate::ai::store::active_calibration(&store.conn)
            .unwrap()
            .is_none()
    );
    assert!(
        crate::ai::store::active_artifact(&store.conn, "category")
            .unwrap()
            .is_none()
    );
    assert_eq!(crate::ai::store::labeled_count(&store.conn).unwrap(), 0);
    assert_eq!(crate::ai::store::outcome_count(&store.conn, 20).unwrap(), 0);
    assert_eq!(crate::ai::store::learning_since(&store.conn).unwrap(), 10);
    let entry = store.get_entry_detail(&id).unwrap().entry;
    assert_eq!(entry.status, "approved");
    assert_eq!(entry.category_id.as_deref(), Some("coding"));
}

#[test]
fn outside_tracking_hours_suppresses_new_capture() {
    use chrono::TimeZone;
    let mut store = ActivityStore::from_conn(Connection::open_in_memory().expect("in-memory db"))
        .expect("schema");
    store.set_tracking_hours(aevio_core::settings::TrackingHours {
        enabled: true,
        per_day: false,
        default_start: "07:00".to_string(),
        default_end: "19:00".to_string(),
        ..aevio_core::settings::TrackingHours::default()
    });

    let dt = chrono::Local
        .with_ymd_and_hms(2026, 9, 25, 20, 0, 0)
        .single()
        .expect("local dt");
    let now = dt.timestamp_millis() as u64;

    let changed = store.tick(sample("Code", "main.rs"), 0, now).unwrap();
    assert!(!changed);
    let snapshot = store.snapshot(0, now).unwrap();
    assert!(snapshot.current.is_none());
    assert!(snapshot.segments.is_empty());
    assert!(!snapshot.in_tracking_hours);
    assert!(!snapshot.tracking_active);
}

#[test]
fn session_in_progress_finishes_naturally_outside_window() {
    use chrono::TimeZone;
    let mut store = ActivityStore::from_conn(Connection::open_in_memory().expect("in-memory db"))
        .expect("schema");
    store.set_idle_threshold_ms(300_000).unwrap();
    store.set_tracking_hours(aevio_core::settings::TrackingHours {
        enabled: true,
        per_day: false,
        default_start: "07:00".to_string(),
        default_end: "19:00".to_string(),
        ..aevio_core::settings::TrackingHours::default()
    });

    // Start session at 18:58 (inside window)
    let dt_start = chrono::Local
        .with_ymd_and_hms(2026, 9, 25, 18, 58, 0)
        .single()
        .expect("local dt");
    let start_ms = dt_start.timestamp_millis() as u64;
    store.tick(sample("Code", "main.rs"), 0, start_ms).unwrap();
    assert!(store.snapshot(0, start_ms).unwrap().current.is_some());

    // At 19:02 (outside window): switch to Slack. Session continues!
    let dt_outside = chrono::Local
        .with_ymd_and_hms(2026, 9, 25, 19, 2, 0)
        .single()
        .expect("local dt");
    let outside_ms = dt_outside.timestamp_millis() as u64;
    store
        .tick(sample("Slack", "#general"), 0, outside_ms)
        .unwrap();
    store
        .tick(sample("Slack", "#general"), 0, outside_ms + 20_000)
        .unwrap();

    let live = store.live_state(outside_ms + 20_000);
    assert!(!live.in_tracking_hours);
    assert!(live.tracking_active);
    assert_eq!(live.current.as_ref().unwrap().app, "Slack");

    // At 19:10: idle detected (idle for 5 min, since 19:05). Idle ends it naturally.
    let dt_idle = chrono::Local
        .with_ymd_and_hms(2026, 9, 25, 19, 10, 0)
        .single()
        .expect("local dt");
    let idle_ms = dt_idle.timestamp_millis() as u64;
    let changed = store
        .tick(sample("Slack", "#general"), 300_000, idle_ms)
        .unwrap();
    assert!(changed);

    let snapshot = store.snapshot(0, idle_ms).unwrap();
    assert!(snapshot.current.is_none());
    assert!(!snapshot.tracking_active);

    let dt_cutoff = chrono::Local
        .with_ymd_and_hms(2026, 9, 25, 19, 5, 0)
        .single()
        .expect("local dt");
    assert_eq!(
        snapshot.segments.last().unwrap().ended_at,
        Some(dt_cutoff.timestamp_millis() as u64)
    );

    // Further activity outside window does not start new capture
    let dt_later = chrono::Local
        .with_ymd_and_hms(2026, 9, 25, 19, 15, 0)
        .single()
        .expect("local dt");
    let later_ms = dt_later.timestamp_millis() as u64;
    store.tick(sample("Code", "main.rs"), 0, later_ms).unwrap();
    assert!(store.snapshot(0, later_ms).unwrap().current.is_none());
}

#[test]
fn manual_tracking_works_outside_window() {
    use chrono::TimeZone;
    let mut store = ActivityStore::from_conn(Connection::open_in_memory().expect("in-memory db"))
        .expect("schema");
    store.set_tracking_hours(aevio_core::settings::TrackingHours {
        enabled: true,
        per_day: false,
        default_start: "07:00".to_string(),
        default_end: "19:00".to_string(),
        ..aevio_core::settings::TrackingHours::default()
    });

    let dt = chrono::Local
        .with_ymd_and_hms(2026, 9, 25, 20, 0, 0)
        .single()
        .expect("local dt");
    let now = dt.timestamp_millis() as u64;

    store.set_capture_enabled(true, now).unwrap();
    assert!(store.manual_tracking);

    store.tick(sample("Code", "main.rs"), 0, now).unwrap();
    let snapshot = store.snapshot(0, now).unwrap();
    assert!(snapshot.current.is_some());
    assert_eq!(snapshot.current.unwrap().app, "Code");
    assert!(snapshot.tracking_active);

    store.set_capture_enabled(false, now + 10_000).unwrap();
    assert!(!store.manual_tracking);
    assert!(store.capture_enabled);

    let snapshot = store.snapshot(0, now + 10_000).unwrap();
    assert!(snapshot.current.is_none());
    assert!(!snapshot.tracking_active);

    store
        .tick(sample("Code", "main.rs"), 0, now + 20_000)
        .unwrap();
    assert!(store.snapshot(0, now + 20_000).unwrap().current.is_none());
}

#[test]
fn test_nineteen_second_detour_does_not_create_switch_fragment() {
    let mut store = store();
    store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();
    // Switch to Slack at 10_000
    assert!(!store.tick(sample("Slack", "#general"), 0, 10_000).unwrap());
    // Still in Slack at 29_000 (19s in Slack < 20s grace period)
    assert!(!store.tick(sample("Slack", "#general"), 0, 29_000).unwrap());

    let snap_during = store.snapshot(0, 29_000).unwrap();
    assert_eq!(snap_during.segments.len(), 1);
    assert_eq!(snap_during.segments[0].app, "Code");
    assert_eq!(snap_during.segments[0].ended_at, None);

    // Return to Code at 29_500: detour is discarded
    assert!(!store.tick(sample("Code", "main.rs"), 0, 29_500).unwrap());
    assert!(!store.tick(sample("Code", "main.rs"), 0, 40_000).unwrap());

    let snapshot = store.snapshot(0, 40_000).unwrap();
    assert_eq!(snapshot.segments.len(), 1);
    assert_eq!(snapshot.segments[0].app, "Code");
    assert_eq!(snapshot.segments[0].ended_at, None);
    assert_eq!(snapshot.tracked_ms, 39_000);

    let slack_segments: i64 = store
        .conn
        .query_row(
            "SELECT COUNT(*) FROM segments WHERE app = 'Slack';",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(slack_segments, 0, "Slack must not create a segment row");

    let slack_apps: i64 = store
        .conn
        .query_row(
            "SELECT COUNT(*) FROM apps WHERE display_name = 'Slack';",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        slack_apps, 0,
        "Detour app must not be registered in apps table"
    );
}

#[test]
fn test_exact_twenty_second_boundary_switches_app() {
    let mut store = store();
    store.tick(sample("Code", "main.rs"), 0, 10_000).unwrap();
    // Switch to Slack at 20_000
    assert!(!store.tick(sample("Slack", "#general"), 0, 20_000).unwrap());

    // At 39_999 (19_999 ms in Slack): still pending
    assert!(!store.tick(sample("Slack", "#general"), 0, 39_999).unwrap());
    assert_eq!(store.snapshot(0, 39_999).unwrap().segments.len(), 1);

    // At exactly 40_000 (20_000 ms in Slack): threshold reached!
    assert!(store.tick(sample("Slack", "#general"), 0, 40_000).unwrap());

    let snapshot = store.snapshot(0, 40_000).unwrap();
    assert_eq!(snapshot.segments.len(), 2);
    assert_eq!(snapshot.segments[0].app, "Code");
    assert_eq!(snapshot.segments[0].started_at, 10_000);
    assert_eq!(snapshot.segments[0].ended_at, Some(20_000));
    assert_eq!(snapshot.segments[1].app, "Slack");
    assert_eq!(snapshot.segments[1].started_at, 20_000);
    assert_eq!(snapshot.segments[1].ended_at, None);
    // Continuous total work time without double counting or gaps
    assert_eq!(snapshot.tracked_ms, 30_000);
}

#[test]
fn test_sustained_switch_retains_real_start_timestamp() {
    let mut store = store();
    store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();
    store.tick(sample("Slack", "#general"), 0, 10_000).unwrap();
    // Sustained for 25s
    assert!(store.tick(sample("Slack", "#general"), 0, 35_000).unwrap());
    // Continues further in Slack
    store.tick(sample("Slack", "#general"), 0, 60_000).unwrap();

    let snapshot = store.snapshot(0, 60_000).unwrap();
    assert_eq!(snapshot.segments.len(), 2);
    assert_eq!(snapshot.segments[0].app, "Code");
    assert_eq!(snapshot.segments[0].ended_at, Some(10_000));
    assert_eq!(snapshot.segments[1].app, "Slack");
    // Real start timestamp retained rather than starting at 30_000 or 35_000
    assert_eq!(snapshot.segments[1].started_at, 10_000);
    assert_eq!(snapshot.tracked_ms, 59_000);
}

#[test]
fn test_rapid_alternating_apps_detour_does_not_fragment() {
    let mut store = store();
    store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();

    // Rapid switches: Code -> Slack (5s) -> Chrome (7s) -> Terminal (8s) -> Slack (6s) -> Code
    store.tick(sample("Slack", "#general"), 0, 10_000).unwrap();
    store.tick(sample("Chrome", "Google"), 0, 15_000).unwrap();
    store.tick(sample("Terminal", "zsh"), 0, 22_000).unwrap();
    store.tick(sample("Slack", "#general"), 0, 30_000).unwrap();
    store.tick(sample("Code", "main.rs"), 0, 36_000).unwrap();
    store.tick(sample("Code", "main.rs"), 0, 50_000).unwrap();

    let snapshot = store.snapshot(0, 50_000).unwrap();
    assert_eq!(
        snapshot.segments.len(),
        1,
        "Detours must not produce standalone fragments"
    );
    assert_eq!(snapshot.segments[0].app, "Code");
    assert_eq!(snapshot.segments[0].ended_at, None);
    assert_eq!(snapshot.tracked_ms, 49_000);
}

#[test]
fn test_burst_switches_a_b_c_commits_sustained_candidate() {
    let mut store = store();
    store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();
    // Switch to Slack at 10_000
    store.tick(sample("Slack", "#general"), 0, 10_000).unwrap();
    // Switch to Chrome at 15_000 (Slack was 5s < 20s, aborted)
    store.tick(sample("Chrome", "Docs"), 0, 15_000).unwrap();
    // Chrome reaches 20 seconds at 35_000
    assert!(store.tick(sample("Chrome", "Docs"), 0, 35_000).unwrap());

    let snapshot = store.snapshot(0, 35_000).unwrap();
    assert_eq!(snapshot.segments.len(), 2);
    assert_eq!(snapshot.segments[0].app, "Code");
    assert_eq!(snapshot.segments[0].ended_at, Some(15_000));
    assert_eq!(snapshot.segments[1].app, "Chrome");
    assert_eq!(snapshot.segments[1].started_at, 15_000);
    assert_eq!(snapshot.segments[1].ended_at, None);
    assert_eq!(snapshot.tracked_ms, 34_000);

    let slack_count: i64 = store
        .conn
        .query_row(
            "SELECT COUNT(*) FROM segments WHERE app = 'Slack';",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(slack_count, 0);
}

#[test]
fn test_first_event_opens_immediately_without_grace_delay() {
    let mut store = store();
    // First sample opens immediately
    let changed = store.tick(sample("Code", "main.rs"), 0, 5_000).unwrap();
    assert!(changed, "First event must open immediately");

    let snapshot = store.snapshot(0, 5_000).unwrap();
    assert_eq!(snapshot.segments.len(), 1);
    assert_eq!(snapshot.segments[0].app, "Code");
    assert_eq!(snapshot.segments[0].started_at, 5_000);
    assert_eq!(snapshot.segments[0].ended_at, None);
}

#[test]
fn test_pending_switch_at_idle_terminates_grace_and_closes_cleanly() {
    let mut store = store();
    store.set_idle_threshold_ms(300_000).unwrap();
    store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();
    // Switch to Slack at 10_000
    store.tick(sample("Slack", "#general"), 0, 10_000).unwrap();
    // Last input at 15_000 (5s in Slack)
    // At 315_000, idle threshold (300s) crossed:
    let changed = store
        .tick(sample("Slack", "#general"), 300_000, 315_000)
        .unwrap();
    assert!(changed);

    let snapshot = store.snapshot(0, 315_000).unwrap();
    assert_eq!(snapshot.segments.len(), 2);
    // Code closed at idle_since (15_000)
    assert_eq!(snapshot.segments[0].app, "Code");
    assert_eq!(snapshot.segments[0].ended_at, Some(15_000));
    // Break opened at 15_000
    assert_eq!(snapshot.segments[1].kind, KIND_BREAK);
    assert_eq!(snapshot.segments[1].started_at, 15_000);

    let slack_count: i64 = store
        .conn
        .query_row(
            "SELECT COUNT(*) FROM segments WHERE app = 'Slack';",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        slack_count, 0,
        "Pending Slack must not create a segment on idle"
    );
}

#[test]
fn test_pending_switch_at_stop_and_sleep_terminates_grace_period() {
    let mut store = store();
    store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();
    store.tick(sample("Slack", "#general"), 0, 10_000).unwrap();
    // At 18_000 (8s in Slack), close active segment (e.g. system sleep or tracking stop)
    store.close_active_segment(18_000).unwrap();

    let snapshot = store.snapshot(0, 18_000).unwrap();
    assert_eq!(snapshot.segments.len(), 1);
    assert_eq!(snapshot.segments[0].app, "Code");
    assert_eq!(snapshot.segments[0].ended_at, Some(18_000));
    assert_eq!(snapshot.tracked_ms, 17_000);

    let slack_count: i64 = store
        .conn
        .query_row(
            "SELECT COUNT(*) FROM segments WHERE app = 'Slack';",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(slack_count, 0);
}

#[test]
fn test_ignored_excluded_apps_take_effect_immediately_without_grace_delay() {
    let mut store = store();
    // Insert an excluded app into apps table
    store
            .conn
            .execute(
                "INSERT INTO apps (id, kind, identifier, display_name, excluded, first_seen, last_seen, created_at, updated_at)
                 VALUES ('app-1', 'app', 'com.agilebits.onepassword7', '1Password', 1, 0, 0, 0, 0);",
                [],
            )
            .unwrap();

    let op_sample = Some(WindowSample {
        app: "1Password".to_string(),
        title: "Vault".to_string(),
        bundle_id: Some("com.agilebits.onepassword7".to_string()),
        url: None,
        domain: None,
        keeps_display_awake: false,
    });

    store.tick(sample("Code", "main.rs"), 0, 1_000).unwrap();
    // Switch to excluded app 1Password at 10_000
    let changed = store.tick(op_sample.clone(), 0, 10_000).unwrap();
    assert!(
        changed,
        "Switching to excluded app must close current work immediately"
    );

    let snap1 = store.snapshot(0, 10_000).unwrap();
    assert_eq!(snap1.segments.len(), 1);
    assert_eq!(snap1.segments[0].app, "Code");
    assert_eq!(snap1.segments[0].ended_at, Some(10_000));
    assert!(
        snap1.current.is_none(),
        "Excluded app must not open a segment"
    );

    // Stays in 1Password for 15 seconds (10_000 to 25_000)
    assert!(!store.tick(op_sample.clone(), 0, 25_000).unwrap());
    let snap2 = store.snapshot(0, 25_000).unwrap();
    assert_eq!(snap2.segments.len(), 1);
    assert_eq!(
        snap2.tracked_ms, 9_000,
        "1Password time must never be attributed to Code or tracked"
    );

    // Returns to Code at 25_000: opens immediately (first event attribution)
    assert!(store.tick(sample("Code", "main.rs"), 0, 25_000).unwrap());
    let snap3 = store.snapshot(0, 30_000).unwrap();
    assert_eq!(snap3.segments.len(), 2);
    assert_eq!(snap3.segments[1].app, "Code");
    assert_eq!(snap3.segments[1].started_at, 25_000);
    // Code ran 1_000..10_000 (9s) + 25_000..30_000 (5s) = 14s. 1Password was excluded for 15s.
    assert_eq!(snap3.tracked_ms, 14_000);
}

#[test]
fn test_constant_total_work_time_preserved_across_detours_and_switches() {
    let mut store = store();
    // 0..10_000: Code
    store.tick(sample("Code", "main.rs"), 0, 0).unwrap();
    // 10_000..15_000: Slack detour (5s)
    store.tick(sample("Slack", "#general"), 0, 10_000).unwrap();
    // 15_000..30_000: Back to Code
    store.tick(sample("Code", "main.rs"), 0, 15_000).unwrap();
    // 30_000: Sustained switch to Chrome
    store.tick(sample("Chrome", "GitHub"), 0, 30_000).unwrap();
    // 50_000: Chrome sustained (20s)
    store.tick(sample("Chrome", "GitHub"), 0, 50_000).unwrap();
    // 60_000: Stop tracking
    store.close_active_segment(60_000).unwrap();

    let snapshot = store.snapshot(0, 60_000).unwrap();
    // Total work time from 0 to 60_000 was 60_000 ms.
    // Segments: Code [0..30_000] (30s) + Chrome [30_000..60_000] (30s) = 60s total.
    assert_eq!(snapshot.segments.len(), 2);
    assert_eq!(snapshot.segments[0].app, "Code");
    assert_eq!(snapshot.segments[0].started_at, 0);
    assert_eq!(snapshot.segments[0].ended_at, Some(30_000));
    assert_eq!(snapshot.segments[1].app, "Chrome");
    assert_eq!(snapshot.segments[1].started_at, 30_000);
    assert_eq!(snapshot.segments[1].ended_at, Some(60_000));
    assert_eq!(snapshot.tracked_ms, 60_000);
}
