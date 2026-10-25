use aevio_core::coach::models::{PlannedTask, TimeEntry};
use rusqlite::{Connection, params};

pub fn entries(
    conn: &Connection,
    start: i64,
    end: i64,
    tasks: &[PlannedTask],
) -> Result<Vec<TimeEntry>, String> {
    let mut stmt=conn.prepare("SELECT e.id,e.description,MAX(0,MIN(e.ended_at,?2)-MAX(e.started_at,?1)),l.daily_task_id,e.task_id FROM time_entries e LEFT JOIN task_time_links l ON l.entry_id=e.id LEFT JOIN categories c ON c.id=e.category_id WHERE e.deleted_at IS NULL AND e.started_at<?2 AND e.ended_at>?1 AND COALESCE(c.counts_as_work,1)=1 ORDER BY e.started_at").map_err(crate::err)?;
    stmt.query_map(params![start, end], |row| {
        let explicit: Option<String> = row.get(3)?;
        let task_id: Option<String> = row.get(4)?;
        let assigned_to = explicit.or_else(|| {
            tasks
                .iter()
                .find(|item| {
                    item.task.source == "project" && Some(&item.task.id) == task_id.as_ref()
                })
                .map(|item| item.id.clone())
        });
        Ok(TimeEntry {
            id: row.get(0)?,
            description: row.get(1)?,
            duration_ms: row.get::<_, i64>(2)?.max(0) as u64,
            assigned_to,
        })
    })
    .map_err(crate::err)?
    .collect::<rusqlite::Result<Vec<_>>>()
    .map_err(crate::err)
}
pub fn link(
    conn: &Connection,
    id: &str,
    entry_id: &str,
    start: i64,
    end: i64,
) -> Result<(), String> {
    let count=conn.execute("INSERT INTO task_time_links(entry_id,daily_task_id) SELECT id,?2 FROM time_entries WHERE id=?1 AND deleted_at IS NULL AND started_at<?4 AND ended_at>?3 ON CONFLICT(entry_id) DO UPDATE SET daily_task_id=excluded.daily_task_id",params![entry_id,id,start,end]).map_err(crate::err)?;
    if count != 1 {
        return Err("Select a time entry from this day".into());
    }
    Ok(())
}
pub fn unlink(conn: &Connection, entry_id: &str, day: &str) -> Result<(), String> {
    conn.execute("DELETE FROM task_time_links WHERE entry_id=?1 AND daily_task_id IN (SELECT id FROM daily_tasks WHERE day=?2)",params![entry_id,day]).map_err(crate::err)?;
    Ok(())
}

/// A builder entry may span multiple focus and activity segments. Attribute
/// only each recorded focus slice rather than assigning its entire entry.
pub fn focus_allocations(
    conn: &Connection,
    day: &str,
    start: i64,
    end: i64,
) -> Result<Vec<(String, String, u64)>, String> {
    let mut stmt=conn.prepare("SELECT f.daily_task_id,e.id,MAX(0,MIN(COALESCE(s.ended_at,?2),e.ended_at,?2)-MAX(s.started_at,e.started_at,?1)) FROM coach_focus f JOIN daily_tasks d ON d.id=f.daily_task_id JOIN segments s ON s.id=f.segment_id JOIN time_entries e ON e.id=s.entry_id LEFT JOIN categories c ON c.id=e.category_id WHERE d.day=?3 AND e.deleted_at IS NULL AND COALESCE(c.counts_as_work,1)=1 AND s.started_at<?2 AND COALESCE(s.ended_at,?2)>?1").map_err(crate::err)?;
    stmt.query_map(params![start, end, day], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)?.max(0) as u64))
    })
    .map_err(crate::err)?
    .collect::<rusqlite::Result<Vec<_>>>()
    .map_err(crate::err)
}
