//! Existing personal-task IPC persistence, shared by the desktop facade.
use aevio_core::coach::legacy::{CoachTask, NewCoachTask, UpdateCoachTask};
use rusqlite::{Connection, params};
fn task_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CoachTask> {
    Ok(CoachTask {
        id: row.get(0)?,
        title: row.get(1)?,
        notes: row.get(2)?,
        priority: row.get(3)?,
        status: row.get(4)?,
        due_at: row.get::<_, Option<i64>>(5)?.map(|v| v as u64),
        reminder_at: row.get::<_, Option<i64>>(6)?.map(|v| v as u64),
        estimate_ms: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
        created_at: row.get::<_, i64>(8)? as u64,
        updated_at: row.get::<_, i64>(9)? as u64,
        completed_at: row.get::<_, Option<i64>>(10)?.map(|v| v as u64),
    })
}

pub fn list_conn(conn: &Connection) -> Result<Vec<CoachTask>, String> {
    let mut statement = conn
        .prepare("SELECT id,title,notes,priority,status,due_at,reminder_at,estimate_ms,created_at,updated_at,completed_at FROM coach_tasks ORDER BY status ASC, priority DESC, due_at IS NULL, due_at ASC, created_at ASC")
        .map_err(|e| e.to_string())?;

    statement
        .query_map([], task_from_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn create(conn: &Connection, input: NewCoachTask, now: u64) -> Result<CoachTask, String> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err("A task needs a title".to_string());
    }
    let id = uuid::Uuid::now_v7().to_string();
    conn.execute("INSERT INTO coach_tasks (id,title,notes,priority,due_at,reminder_at,estimate_ms,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8)", params![id, title, input.notes, input.priority.unwrap_or(3).clamp(1, 5), input.due_at.map(|v| v as i64), input.reminder_at.map(|v| v as i64), input.estimate_ms.map(|v| v as i64), now as i64]).map_err(|e| e.to_string())?;
    get_by_id(conn, &id)
}

pub fn update(
    conn: &Connection,
    id: &str,
    patch: UpdateCoachTask,
    now: u64,
) -> Result<CoachTask, String> {
    let current = get_by_id(conn, id)?;
    let title = patch.title.unwrap_or(current.title);
    let title = title.trim();
    if title.is_empty() {
        return Err("A task needs a title".to_string());
    }
    let status = patch.status.unwrap_or(current.status);
    if !matches!(status.as_str(), "open" | "done") {
        return Err(format!("unsupported task status: {status}"));
    }
    let completed_at = if status == "done" {
        Some(current.completed_at.unwrap_or(now))
    } else {
        None
    };
    let reminder_changed = patch.reminder_at.is_some();
    conn.execute("UPDATE coach_tasks SET title=?1, notes=?2, priority=?3, status=?4, due_at=?5, reminder_at=?6, estimate_ms=?7, updated_at=?8, completed_at=?9, notified_at=CASE WHEN ?10 THEN NULL ELSE notified_at END WHERE id=?11", params![title, patch.notes.unwrap_or(current.notes), patch.priority.unwrap_or(current.priority).clamp(1, 5), status, patch.due_at.unwrap_or(current.due_at).map(|v| v as i64), patch.reminder_at.unwrap_or(current.reminder_at).map(|v| v as i64), patch.estimate_ms.unwrap_or(current.estimate_ms).map(|v| v as i64), now as i64, completed_at.map(|v| v as i64), reminder_changed, id]).map_err(|e| e.to_string())?;
    get_by_id(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), String> {
    let changed = conn
        .execute("DELETE FROM coach_tasks WHERE id=?1", [id])
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Task not found".to_string());
    }
    Ok(())
}

fn get_by_id(conn: &Connection, id: &str) -> Result<CoachTask, String> {
    conn.query_row("SELECT id,title,notes,priority,status,due_at,reminder_at,estimate_ms,created_at,updated_at,completed_at FROM coach_tasks WHERE id=?1", [id], task_from_row).map_err(|e| e.to_string())
}
