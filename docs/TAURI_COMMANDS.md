---
id: ref.tauri-commands-legacy
title: Tauri Commands (legacy)
category: reference
domain: platform
status: implemented
source_paths: []
related_docs: []
platforms: [macos, windows]
verification_commit: working-tree
---


# Aevio – Tauri Command Reference

> Vollständige Liste aller Tauri-Commands (`pub fn` in `apps/desktop/tauri/src/commands.rs`), gruppiert nach Domain. Stand: Working-Tree (v0.9.29).

## Documents, Notes, Comments, Attachments, Mentions
- `watch_pairing`
- `list_documents`, `create_document`, `update_document`, `delete_document`
- `list_notes`, `create_note`, `update_note`, `delete_note`
- `list_comments`, `create_comment`
- `list_attachments`, `create_attachment`
- `list_mentions`

## Cycles & Initiatives
- `list_cycles`, `create_cycle`
- `list_initiatives`, `create_initiative`

## Governance
- `governance_snapshot`
- `save_governance_team`, `save_governance_department`, `save_governance_member`, `save_governance_defaults`
- `authorize_governance`

## Coach
- `coach_snapshot`
- `list_coach_tasks`, `create_coach_task`, `update_coach_task`, `complete_coach_task`, `delete_coach_task`

## Timers
- `list_timers`, `create_timer`, `start_timer`, `pause_timer`, `pause_timer_by_id`, `reset_timer`, `rename_timer`
- `add_timer_segment`, `update_timer_segment`, `delete_timer_segment`, `delete_timer`

## Activity, Usage, Capture
- `activity_snapshot`, `usage_snapshot`
- `set_capture_enabled`, `set_idle_threshold`
- `start_session`, `stop_session`
- `mark_segment_reviewed`

## Categories
- `list_categories`, `create_category`, `update_category`, `delete_category`

## Projects
- `list_projects`, `create_project`, `update_project`, `delete_project`
- `project_rules`, `preview_project_hints`, `discover_projects`, `dismiss_project_suggestion`
- `import_projects_csv`, `project_stats`

## Tasks & Labels
- `list_tasks`, `create_task`, `update_task`, `delete_task`
- `list_labels`, `create_label`, `update_label`, `delete_label`

## Integrations
- `list_integrations`, `save_integration`

## Clients
- `list_clients`, `create_client`, `update_client`, `delete_client`

## Invoices
- `list_invoices`, `get_invoice`, `list_billable_entries`, `quote_invoice`
- `render_invoice_preview`, `get_invoice_pdf`
- `save_invoice_draft`, `finalize_invoice`, `set_invoice_paid`, `void_invoice`
- `delete_draft_invoice`, `delete_void_invoice`
- `get_invoice_profile`, `update_invoice_profile`
- `get_invoice_logo`, `set_invoice_logo`, `clear_invoice_logo`

## Time Entries
- `list_time_entries`, `get_entry_detail`, `update_time_entry`
- `approve_time_entries`, `unapprove_time_entries`, `update_time_entries`
- `reject_time_entry`, `split_time_entry`, `merge_time_entries`
- `delete_time_entry`, `delete_time_entries`, `undo`
- `create_time_entry`, `rebuild_time_entries`

## Apps
- `list_apps`, `update_app`

## AI, Agent, Work
- `work_ask`, `work_relationships_list`, `work_relationship_add`
- `ai_status`, `agent_chat`
- `retry_classification`, `resolve_rule_suggestion`
- `ai_metrics`, `ai_retrain`, `ai_reset_learned`

## Automation, Workflow, Reports, Dashboards
- `automation_snapshot`, `daily_workflow_snapshot`, `run_daily_workflow`
- `create_routine`, `create_report`, `create_dashboard`
- `run_routine`, `run_report`

## Finance
- `finance_snapshot`
- `list_finance_rate_cards`, `save_finance_rate_card`
- `list_finance_contracts`, `save_finance_contract`
- `share_finance_dashboard`, `list_finance_capacity_plans`, `save_finance_capacity_plan`
- `list_finance_dashboard_shares`, `revoke_finance_dashboard_share`

## Query, Rollup, Export
- `query_time_entries`, `entry_rollup`, `export_time_entries`

## Widget, Pulse, Summary, Window
- `resize_widget`, `resize_pulse_panel`, `hide_pulse_panel`
- `open_productivity_summary`, `hide_productivity_summary`, `open_main_window`

## Settings & Decision
- `get_settings`, `update_settings`
- `get_decision_settings`, `update_decision_settings`, `decision_health`

## Maintenance & OS
- `sweep_retention`
- `launch_at_login`, `set_launch_at_login`
- `check_permissions`, `storage_paths`

## Energy
- `get_energy_summary`, `query_energy_history`, `reset_energy_history`

## Status & Breaks
- `update_status`, `break_state`, `dismiss_stopwatch_reminder`
- `start_break`, `end_break`, `snooze_break`, `skip_break`
- `expand_break_reminder`, `extend_break`, `pause_break_reminders`
- `list_breaks`, `resize_reminder_panel`, `open_break_settings`
- `dev_sample_break_reminder`, `preview_break_chime`

---

**Gesamt: 174 Commands.**

> Hinweis: Signaturen (Parameter/Rückgabetypen) sind in `commands.rs` nachzulesen. Bei Änderungen `apps/cli/COMMANDS.md` synchron halten.
