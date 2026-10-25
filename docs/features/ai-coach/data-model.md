# Persistence and ownership

Migration v19 adds `daily_plans`, `daily_tasks`, `task_steps`, `daily_reviews`, `coach_preferences`, `coach_interventions`, `task_time_links`. It adopts the existing `coach_tasks` table and repairs its oldest missing `notified_at` column. Plans have a civil-date key, IANA timezone, goal, notes, available minutes, optional energy, explicit planning/skip state and revision. The single goal is part of the plan aggregate, not an unnecessary one-to-one goals table.

Daily task rows reference exactly one existing personal or project task through foreign keys and a CHECK constraint. Only estimates, importance, order and postponement count belong to the assignment. Steps refer to the assignment. Moving an assignment preserves its identity, steps and time links. Completed tasks cannot be carried over. Removing from a plan does not delete the original task.

Reviews are unique per date and support all five outcomes. Updates check revisions; an identical retry is idempotent. Plan/review changes cannot silently overwrite another window's edits. Assignment mutations are explicit, atomic transactions. Reordering requires an exact permutation. Time-entry links are unique per entry; manual links take precedence over project task identity. Time is clipped to the selected civil day and excludes non-work categories. Deleted entries contribute nothing. Time never completes a task or goal.

The local data directory and process lock define the current ownership boundary. There is no authenticated multi-user coach API. Workspace member metadata must not be treated as an authentication principal. A future remote API must add real ownership enforcement before exposing these records.

Coach deletion removes its personal content and assignments while preserving project tasks and original time entries. User-authored review text stays until explicit deletion; technical intervention metadata is retained for at most 90 days. No activity titles/URLs are copied into intervention logs.

Migrations v20–v22 add `coach_focus` and `focus_interruptions`, `coach_memory`, `brain_dump_entries`, `coach_routines`, `coach_routine_items`, and `coach_routine_runs`. Each migration is independently transactional and repeatable. Recovery references the existing captured segment; no focus duration is independently accumulated. Closing the ActivityStore segment changes the recovery state to interrupted. Only an entry linked by `segments.entry_id` and fully contained in the focus segment is automatically associated with that assignment.

Memory contains only `explicit` or `confirmed` user-authored notes. Computed estimates are queried, not copied into memory. Brain conversion and routine application are transactional and idempotent. Brain category `reminder` is organizational metadata; a notification time must be explicitly set on a personal task. All these coach-specific records are removed by Coach deletion.

When the existing entry builder coalesces focus and ordinary activity into one entry, progress attributes only the intersecting captured focus segment duration to its assignment. It never assigns the entire mixed entry. An explicit whole-entry assignment takes precedence. Assigned and unassigned durations remain disjoint.
