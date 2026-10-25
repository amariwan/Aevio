# Workflows

1. Open Coach. It is initially off; manual features remain available. Enable preferred options under Settings.
2. Choose a civil date; edit a main goal, available working minutes, optional energy and notes. Save or skip. Saved planning can be reopened later.
3. Add a personal task or reference an existing project task. Edit priority, estimated minutes, importance and notes. Reorder with keyboard-accessible up/down controls. Over-capacity plans explain the difference without blocking decisions.
4. Open Just Start / Steps, choose the current difficulty and granularity. Generate local guidance or explicitly request enabled on-device AI. Edit one step per line, regenerate or remove steps, then explicitly accept. Step completion does not complete the parent task.
5. Assign time entries to daily work items. Project task links are recognized automatically; captured duration does not imply completion.
6. Mark tasks complete deliberately. Move unfinished tasks to another date, or explicitly bring earlier unfinished assignments into the selected day.
7. Open Daily Review, assess the goal yourself, record what worked/obstacles/tomorrow and save. Select the date again to edit it later. The scheduler can offer one gentle native review reminder after the configured time.
8. Disable the coach/reminders/inference independently. Delete coach data through the explicit destructive-action dialog.

All navigation reuses the existing Coach route and shadcn tabs. Loading, retry, mutation errors and save feedback use real IPC results. There are no simulated accounts, time entries or tasks.

9. In Focus, choose a current-day unfinished task and a 2–180-minute interval. Store a next step; optionally choose quiet Body Doubling. Pause/end through the Coach or existing tracking controls. An interrupted segment keeps the goal and next step for an explicit return. Focus time is never proof of task completion.
10. Under Week & Memory, compare daily aggregates with the previous week. Toggle optional personalization, write/edit/delete a memory note or explicitly confirm a duration observation. No aggregate observation becomes memory automatically.
11. In Brain Dump, capture an entry, inspect a suggested category before applying it, and explicitly convert selected thoughts into the chosen day's tasks. Routine templates show every proposed task; applying one again on the same day is idempotent.
12. In Coach settings, optionally show context inside the existing floating widget or hide the widget entirely. The existing timer/capture window remains the native implementation; no second Coach window is created.
