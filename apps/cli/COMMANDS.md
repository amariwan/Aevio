# aevio command reference

Every command `aevio` has or is planned to have, the app command it maps to, and whether it
works while the app is closed. Keep this file current: a PR that adds, changes or removes a
command, or changes the app command behind one, updates its row here.

## How aevio reaches the data

- **App running:** aevio sends the request to the app's local endpoint (a Unix socket on macOS,
  a named pipe on Windows). The app stays the only writer while it runs, and open windows and the
  tray update at once.
- **App closed, or not installed at all:** aevio opens the stored data itself and answers with
  the same code the app uses (`aevio_core::rpc`). Everything that reads or edits stored data
  works; nothing new is tracked until the app runs again, and it picks the edits up at launch. A
  lock file keeps the app and aevio from ever holding the stores at once.
- **aevio never creates the data.** Only the app makes and migrates the database. Without one,
  aevio exits with "This laptop doesn't have any aevio data. Are you sure you've installed the app
  before?"; with one from another app version, it fails with `INCOMPATIBLE` and leaves it alone.

There are two ways to get aevio: it comes bundled with the app (`aevio install-path` puts it on
PATH), or the curl installer (`install.sh`) installs it on its own. A standalone aevio finds the
installed app only to launch it (`aevio app start`, `aevio track start`).

`aevio app start` only launches the app (in the background, no window). Tracking is its own
command: `aevio track start`, or `aevio app start --track` to do both.

## Status

- **Shipped:** in the current binary.
- **M2:** next.
- **Later:** planned, not scheduled.

## Commands

In the "App closed" column, "Yes" means the command works with the app closed or not installed
at all, as long as the app has run once on this machine. "Needs app" means the command depends on
live app state, so aevio says the app is not running and suggests `aevio app start`. "Starts app"
means the command launches the app itself, because that is what it asks for, so it needs the app
installed.

| Command | What it does | App command | App closed | Status |
| --- | --- | --- | --- | --- |
| `aevio` / `aevio status [--full]` | Whether the app is running, tracking state, current activity, today's totals, entries to review and running timers (break state in `--json`). Titles and URLs only with `--full`. | `activity_snapshot`, `list_timers`, `break_state` | Stored totals and timers; no live activity | Shipped |
| `aevio app start [--track] [--show]` | Launch the app in the background (menu bar only, no window or Dock icon). `--track` also turns tracking on; `--show` opens the window. Waits until the app answers. | launches the app with `--background` | Starts app | Shipped |
| `aevio app status` | Running or not, app version and data directory. | `Hello` handshake | Yes | Shipped |
| `aevio app open [--review]` | Show the window, on today's review queue with `--review`. | `open_main_window` | Starts app | Shipped |
| `aevio app quit` | Close the open segment and exit, as the tray's Quit does. Returns once the app has released the data. | `AppHandle::exit` | Nothing to do | Shipped |
| `aevio track start` | Turn tracking on, launching the app if needed. | `set_capture_enabled` | Starts app | Shipped |
| `aevio track stop` | Turn tracking off. | `set_capture_enabled` | Yes (saved for next launch) | Shipped |
| `aevio track idle <minutes>` | Set the idle threshold. | `set_idle_threshold` | Yes | Shipped |
| `aevio focus start [label]` / `aevio focus stop` | Start or end a manual focus session. | `start_session`, `stop_session` | Needs app | Shipped |
| `aevio timers list` | All timers with elapsed time and state. | `list_timers` | Yes | Shipped |
| `aevio timers new <label>` | Create a timer. | `create_timer` | Yes | Shipped |
| `aevio timers start` / `pause` / `reset` / `rm <timer>` | Control or delete a timer by id or label; reset and rm are confirmed. | `start_timer`, `pause_timer`, `reset_timer`, `delete_timer` | Yes | Shipped |
| `aevio timers rename <timer> <label>` | Rename a timer. | `rename_timer` | Yes | Shipped |
| `aevio timers segments <timer>` / `segments add` / `edit` / `rm` | List, add, edit or remove the time segments that make up a timer's total. | `add_timer_segment`, `update_timer_segment`, `delete_timer_segment` | Yes | Planned |
| `aevio entries list` | Filter by range, project, client, category, app, status, billable and search. Newest first, `--limit` 1 to 5000. | `query_time_entries` | Yes | Shipped |
| `aevio entries show <id>` | One entry with its apps, events and AI suggestion; window titles with `--full`. | `get_entry_detail` | Yes | Shipped |
| `aevio entries add --from --to [fields]` | Create a manual entry. Refused when its time range overlaps approved entries: unapprove those entries first. | `create_time_entry` | Yes | Shipped |
| `aevio entries edit <id...> [fields]` | Change description, project, category, billable, start or end on one or many entries; `none` clears a project or category. Refused on approved entries (the whole batch): unapprove first. | `update_time_entry`, `update_time_entries` | Yes | Shipped |
| `aevio entries approve <id...>` / `--all-pending [range]` | Approve entries, or every pending entry in a range. | `approve_time_entries` | Yes | Shipped |
| `aevio entries unapprove <id...>` | Send entries back to review. | `unapprove_time_entries` | Yes | Shipped |
| `aevio entries reject <id>` | Reject an AI suggestion. | `reject_time_entry` | Yes | Shipped |
| `aevio entries split <id> --at <time>` | Split one entry in two. Refused on an approved entry. | `split_time_entry` | Yes | Shipped |
| `aevio entries merge <id> <other-id>` | Merge two adjacent entries into one. Refused when either is approved. | `merge_time_entries` | Yes | Shipped |
| `aevio entries rm <id...>` | Delete entries (confirmed). Refused when any is approved. | `delete_time_entries` | Yes | Shipped |
| `aevio entries rebuild [range]` | Rebuild entries from recorded activity (confirmed). | `rebuild_time_entries` | Yes | Shipped |
| `aevio entries export --format csv\|json [--out file]` | Export filtered entries to stdout or a file. | `export_time_entries` (`reports::export_body`) | Yes | Shipped |
| `aevio review [range]` | Pending entries, today by default. | `query_time_entries` (pending) | Yes | Shipped |
| `aevio report --by project\|client\|category\|app\|status\|none --per day\|week\|month\|total` | Totals for a range, with the filters of `entries list`. | `entry_rollup` | Yes | Shipped |
| `aevio projects list` / `show <project>` | Projects; one with this month's time, budget and rules. | `list_projects`, `project_stats`, `project_rules` | Yes | Shipped |
| `aevio projects add` / `edit` / `rm` | Manage projects; rm is confirmed and refused when it would change approved entries. | `create_project`, `update_project`, `delete_project` | Yes | Shipped |
| `aevio clients list` / `show` / `add` / `edit` / `rm` | Manage clients; show lists their projects; rm is confirmed. | `list_clients`, `create_client`, `update_client`, `delete_client` | Yes | Shipped |
| `aevio categories list` / `add` / `edit` / `rm` | Manage categories; rm is confirmed and refused when it would change approved entries. | `list_categories`, `create_category`, `update_category`, `delete_category` | Yes | Shipped |
| `aevio settings list` / `get <key>` / `set <key> <value>` | Every preference by dotted key, such as `tracking-hours.start` or `breaks.enabled`. Writes one field at a time. | `get_settings`, `SettingsStore::patch` | Yes (applies on next launch) | Shipped |
| `aevio plugins list` | List the local first-party catalog and installed lifecycle state. | `PluginsList` | Yes | Shipped |
| `aevio plugins plan <id>` | Show release, digest and requested capabilities before installation. | `PluginPlanInstall` | Yes | Shipped |
| `aevio plugins install <id> [--grant <capability>] [--enable]` | Install a signed first-party module and explicitly grant only requested capabilities. | `PluginInstall` | Yes | Shipped |
| `aevio plugins enable|disable <id>` | Change plugin activation without deleting its data. | `PluginSetEnabled` | Yes | Shipped |
| `aevio plugins rm <id>` | Uninstall a disabled plugin after confirmation. | `PluginUninstall` | Yes | Shipped |
| `aevio plugins purge-data <id>` | Explicitly delete all stored data for a disabled or uninstalled plugin after confirmation. | `PluginDeleteData` | Yes | Shipped |
| `aevio plugins rollback <id>` | Restore the last staged version after an update failure. | `PluginRollback` | Yes | Shipped |
| `aevio plugins recover <id>` | Clear a failed/quarantined state when its staged artifact is intact. | `PluginRecover` | Yes | Shipped |
| `aevio plugins search <id> <query>` | Search an active host-owned plugin contribution. | `PluginSearch` | Yes | Shipped |
| `aevio plugins execute <id> <command-id>` | Execute a host-owned First-Party plugin command. | `PluginExecuteCommand` | Yes | Shipped |
| `aevio plugins schedule <id> <job-id> --interval-ms <n>` | Schedule a declared offline plugin job. | `PluginScheduleJob` | Yes | Shipped |
| `aevio plugins cancel <id> <job-id>` | Disable a scheduled plugin job. | `PluginCancelJob` | Yes | Shipped |
| `aevio plugins validate <manifest>` | Validate a local plugin manifest. | local contract validator | Yes | Shipped |
| `aevio plugins pack <source> <output>` | Pack a deterministic unsigned local plugin package. | local package writer | Yes | Shipped |
| `aevio plugins inspect <package>` | Inspect a package without installing it. | local package reader | Yes | Shipped |
| `aevio plugins verify <package> --signature <base64>` | Verify package digest and publisher signature. | local artifact verifier | Yes | Shipped |
| `aevio paths` | Settings file, data directory and database path. | `storage_paths` | Yes | Shipped |
| `aevio install-path` | Put `aevio` on PATH: a symlink on macOS, the user PATH on Windows. | client only | Yes | Shipped |
| `aevio uninstall-path` | Remove the installed command from `~/.local/bin`, even if the app is gone. | client only | Yes | Shipped |
| `aevio completions bash\|zsh\|fish\|powershell\|elvish` | Print a shell completion script. | client only | Yes | Shipped |
| `aevio mcp serve` | Serve read-only status, project, category and bounded time-entry tools over MCP stdio. | local CLI + shared IPC | Yes, when Aevio has run once | Shipped |
| `aevio projects discover` / `dismiss <key>` | Suggested projects from recent activity. | `discover_projects`, `dismiss_project_suggestion` | Yes | M2 |
| `aevio projects import <file.csv>` | Bulk import projects. | `import_projects_csv` | Yes | M2 |
| `aevio projects hints <project> --preview` | Preview which activity a hint list would match. | `preview_project_hints` | Yes | M2 |
| `aevio apps list` / `edit <app>` | Default category or project per app; exclude or include an app. | `list_apps`, `update_app` | Yes | M2 |
| `aevio invoices list` / `show <invoice>` | Invoices with status and totals. | `list_invoices`, `get_invoice` | Yes | M2 |
| `aevio invoices billable --client --from --to` | Uninvoiced billable time for a client. | `list_billable_entries` | Yes | M2 |
| `aevio invoices draft` / `quote` | Create or update a draft; `quote` prints totals without saving. | `save_invoice_draft`, `quote_invoice` | Yes | M2 |
| `aevio invoices finalize` / `paid` / `unpaid` / `void` / `rm` | Invoice lifecycle; finalize and void are confirmed. | `finalize_invoice`, `set_invoice_paid`, `void_invoice`, `delete_draft_invoice` | Yes | M2 |
| `aevio invoices pdf <invoice> --out file` | Write the PDF, the same bytes the app renders. | `get_invoice_pdf` | Yes | M2 |
| `aevio invoices profile show` / `edit`, `logo set <file>` / `clear` | Sender profile and logo. | `get_invoice_profile`, `update_invoice_profile`, `set_invoice_logo`, `clear_invoice_logo` | Yes | M2 |
| `aevio breaks status` / `start` / `end` / `snooze <min>` / `skip` / `extend` / `pause [--until]` | Control break reminders. | `break_state`, `start_break`, `end_break`, `snooze_break`, `skip_break`, `extend_break`, `pause_break_reminders` | Needs app | M2 |
| `aevio breaks list --from --to` | Past breaks. | `list_breaks` | Yes | M2 |
| `aevio ai status` / `rules` / `rules accept` / `reject <rule>` | Classifier state, queue and suggested rules. | `ai_status`, `resolve_rule_suggestion` | Needs app | M2 |
| `aevio ai metrics [--days]` | Classification accuracy. | `ai_metrics` | Yes | M2 |
| `aevio ai retrain` / `reset-learned --days` / `retry <entry>` | Retrain, forget recent corrections, or reclassify one entry (confirmed where destructive). | `ai_retrain`, `ai_reset_learned`, `retry_classification` | Needs app | M2 |
| `aevio energy summary [--days]` / `history` / `reset` | Battery and energy history. | `get_energy_summary`, `query_energy_history`, `reset_energy_history` | Yes | M2 |
| `aevio login-item status` / `enable` / `disable` | Launch at login. | `launch_at_login`, `set_launch_at_login` | Needs app on macOS | M2 |
| `aevio update status` / `check` / `install` | App updates. | `update_status`, `check_for_updates`, `install_update` | Needs app | M2 |
| `aevio activity --from --to [--full]` | Raw activity segments; titles only with `--full`. | `activity_snapshot` | Yes | Later |
| `aevio status --watch` | Live status that redraws on every change. | new streaming subscription | Needs app | Later |
| `aevio ai classify` / `models` | Classification and model downloads for a non-Apple model backend. | new | Later | Later |

## Left to the window

These app commands only serve what is on screen, so aevio leaves them out:
`resize_pulse_panel`, `hide_pulse_panel`, `resize_reminder_panel`, `expand_break_reminder`,
`open_break_settings`, `render_invoice_preview`, `get_invoice_logo`, `preview_break_chime`,
`dismiss_stopwatch_reminder`, `mark_segment_reviewed` and the debug-only `dev_sample_break_reminder`. Appearance preferences
stay reachable through `aevio settings set`.

## Conventions

| Area | Rule |
| --- | --- |
| Output | Plain aligned tables, no color. Durations as `3h 25m`, times local. |
| `--json` | One envelope for every command, errors included: `schemaVersion`, `ok`, `data`, `error { code, message }`. Adding a field is compatible; removing or renaming one bumps `schemaVersion`. |
| Exit codes | 0 success, 1 operation failed, 2 invalid arguments (including an unconfirmed deletion), 3 app needed but not running, 4 incompatible versions (the running app speaks another aevio protocol, or the stored data is from another app version). |
| Error codes | `INVALID_ARGUMENT`, `NOT_FOUND`, `AMBIGUOUS` (with `candidates`), `NO_DATA`, `APP_NOT_RUNNING`, `INCOMPATIBLE`, `FAILED`. `NO_DATA` means the app never ran on this machine. |
| Dates | A bare date covers the whole local day (`--to 2026-09-30` includes that day); offset-less times are local. `today`, `yesterday`, weekday names, `this-week`, `last-week`, `this-month` and `last-month` work for `--from`, `--to` and `--period`; `--last 7d` (or `90m`, `12h`, `2w`) runs up to now. Weeks start on Monday. Ranges default to today. |
| Names | Projects, clients, categories and timers take an id, an exact name or a unique case-insensitive prefix; two matches fail with `AMBIGUOUS` and list both. Entries take a full id or its last characters (lists show the last eight). |
| Confirmation | Deleting, voiding, finalizing, resetting and rebuilding prompt on a terminal and need `--yes` otherwise. |
| Paging | `entries list` takes `--limit` (default 50) and reports `truncated` when more match. |
| Privacy | Window titles and URLs only with `--full`. |
| Global flags | `--json`, `--full`, `--yes` (`-y`), `--data-dir` (another app data directory, such as a dev build's). `AEVIO_APP` points aevio at a specific app binary to launch, for dev builds. |

## Desktop Coach IPC

The following are desktop commands, not new CLI subcommands. All durable operations use
`aevio_core::coach`; no second store or local HTTP/auth service is introduced.

| Tauri command | Purpose |
| --- | --- |
| `coach_day` / `coach_apply` | Read a selected civil day; apply a typed explicit plan/task/review/preferences mutation |
| `coach_propose` | Read-only editable breakdown/Just Start proposal; optional on-device generation |
| `coach_plan_proposal` | Read-only validated priority/capacity suggestion |
| `coach_focus` / `coach_start_focus` / `coach_end_focus` | Existing ActivityStore focus sessions with durable recovery context |
| `coach_insights` / `coach_memory` | Weekly local aggregates and explicit/confirmed memory management |
| `coach_brain` / `coach_brain_apply` / `coach_classify_thought` | Brain Dump, local category proposals, explicit conversion and routines |

Existing `coach_snapshot` and personal-task CRUD command names remain compatible.
New Coach invalidations use `coach-day-changed`; existing project catalogs also receive
`entries-changed` after task changes. No Coach command is exposed as a remote multi-user API.
