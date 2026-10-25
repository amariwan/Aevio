use aevio_core::coach::models::{CatalogTask, PlannedTask, Step, TaskRef};
use rusqlite::{Connection, params};

pub fn catalog(conn: &Connection) -> Result<Vec<CatalogTask>, String> {
    let mut stmt = conn.prepare("SELECT 'personal',id,title,COALESCE(notes,''),status='done',priority,estimate_ms/60000,reminder_at FROM coach_tasks UNION ALL SELECT 'project',t.id,t.title,COALESCE(t.description,''),t.status='done',CASE t.priority WHEN 'urgent' THEN 5 WHEN 'high' THEN 4 WHEN 'low' THEN 2 ELSE 3 END,NULL,NULL FROM tasks t JOIN projects p ON p.id=t.project_id WHERE t.deleted_at IS NULL AND p.deleted_at IS NULL ORDER BY 6 DESC,3 COLLATE NOCASE").map_err(crate::err)?;
    stmt.query_map([], |row| {
        Ok(CatalogTask {
            task: TaskRef {
                source: row.get(0)?,
                id: row.get(1)?,
            },
            title: row.get(2)?,
            notes: row.get(3)?,
            done: row.get(4)?,
            priority: row.get(5)?,
            estimate_minutes: row.get(6)?,
            reminder_at: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
        })
    })
    .map_err(crate::err)?
    .collect::<rusqlite::Result<Vec<_>>>()
    .map_err(crate::err)
}

pub fn list(conn: &Connection, day: &str, carryover: bool) -> Result<Vec<PlannedTask>, String> {
    let mut stmt = conn.prepare("SELECT d.id,CASE WHEN d.personal_task_id IS NULL THEN 'project' ELSE 'personal' END,COALESCE(d.personal_task_id,d.project_task_id),COALESCE(c.title,t.title),COALESCE(c.notes,t.description,''),COALESCE(c.status,t.status)='done',COALESCE(c.priority,CASE t.priority WHEN 'urgent' THEN 5 WHEN 'high' THEN 4 WHEN 'low' THEN 2 ELSE 3 END),d.estimate_minutes,d.important,d.position,d.postponements,c.reminder_at FROM daily_tasks d LEFT JOIN coach_tasks c ON c.id=d.personal_task_id LEFT JOIN tasks t ON t.id=d.project_task_id WHERE (t.id IS NULL OR t.deleted_at IS NULL) AND ((?2=0 AND d.day=?1) OR (?2=1 AND d.day<?1 AND COALESCE(c.status,t.status)!='done' AND NOT EXISTS(SELECT 1 FROM daily_tasks today WHERE today.day=?1 AND (today.personal_task_id=d.personal_task_id OR today.project_task_id=d.project_task_id)))) ORDER BY d.day DESC,d.position,d.id").map_err(crate::err)?;
    let mut items = stmt
        .query_map(params![day, carryover], |row| {
            Ok(PlannedTask {
                id: row.get(0)?,
                task: TaskRef {
                    source: row.get(1)?,
                    id: row.get(2)?,
                },
                title: row.get(3)?,
                notes: row.get(4)?,
                done: row.get(5)?,
                priority: row.get(6)?,
                estimate_minutes: row.get(7)?,
                important: row.get(8)?,
                position: row.get(9)?,
                postponements: row.get(10)?,
                reminder_at: row.get::<_, Option<i64>>(11)?.map(|v| v as u64),
                steps: Vec::new(),
                actual_ms: 0,
            })
        })
        .map_err(crate::err)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(crate::err)?;
    let mut steps = conn
        .prepare("SELECT title,done FROM task_steps WHERE daily_task_id=?1 ORDER BY position")
        .map_err(crate::err)?;
    for item in &mut items {
        item.steps = steps
            .query_map([&item.id], |row| {
                Ok(Step {
                    title: row.get(0)?,
                    done: row.get(1)?,
                })
            })
            .map_err(crate::err)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(crate::err)?;
    }
    Ok(items)
}

pub fn add(
    conn: &Connection,
    day: &str,
    task: &TaskRef,
    minutes: u32,
    important: bool,
) -> Result<(), String> {
    let exists = catalog(conn)?
        .iter()
        .any(|item| item.task.source == task.source && item.task.id == task.id);
    if !exists {
        return Err("Task no longer exists in this local catalog".into());
    }
    conn.execute("INSERT INTO daily_tasks(id,day,personal_task_id,project_task_id,estimate_minutes,important,position) VALUES(?1,?2,?3,?4,?5,?6,COALESCE((SELECT MAX(position)+1 FROM daily_tasks WHERE day=?2),0)) ON CONFLICT DO NOTHING",params![uuid::Uuid::now_v7().to_string(),day,(task.source=="personal").then_some(&task.id),(task.source=="project").then_some(&task.id),minutes,important]).map_err(crate::err)?;
    Ok(())
}

pub fn create_personal(
    conn: &Connection,
    title: &str,
    notes: &str,
    priority: u8,
    minutes: u32,
    reminder_at: Option<u64>,
    now: u64,
) -> Result<TaskRef, String> {
    let id = uuid::Uuid::now_v7().to_string();
    conn.execute("INSERT INTO coach_tasks(id,title,notes,priority,estimate_ms,created_at,updated_at,reminder_at) VALUES(?1,?2,?3,?4,?5,?6,?6,?7)",params![id,title.trim(),notes,priority,minutes as i64*60000,now as i64,reminder_at.map(|v|v as i64)]).map_err(crate::err)?;
    Ok(TaskRef {
        source: "personal".into(),
        id,
    })
}

pub struct TaskEdit<'a> {
    pub title: &'a str,
    pub notes: &'a str,
    pub priority: u8,
    pub minutes: u32,
    pub important: bool,
    pub reminder_at: Option<u64>,
}
pub fn edit(
    conn: &Connection,
    item: &PlannedTask,
    edit: &TaskEdit<'_>,
    now: u64,
) -> Result<(), String> {
    let TaskEdit {
        title,
        notes,
        priority,
        minutes,
        important,
        reminder_at,
    } = *edit;
    if item.task.source == "personal" {
        conn.execute("UPDATE coach_tasks SET title=?2,notes=?3,priority=?4,estimate_ms=?5,updated_at=?6,notified_at=CASE WHEN reminder_at IS NOT ?7 THEN NULL ELSE notified_at END,reminder_at=?7 WHERE id=?1",params![item.task.id,title.trim(),notes,priority,minutes as i64*60000,now as i64,reminder_at.map(|v|v as i64)]).map_err(crate::err)?;
    } else {
        let priority = match priority {
            5 => "urgent",
            4 => "high",
            1 | 2 => "low",
            _ => "normal",
        };
        conn.execute("UPDATE tasks SET title=?2,description=?3,priority=?4,updated_at=?5 WHERE id=?1 AND deleted_at IS NULL",params![item.task.id,title.trim(),notes,priority,now as i64]).map_err(crate::err)?;
    }
    conn.execute(
        "UPDATE daily_tasks SET estimate_minutes=?2,important=?3 WHERE id=?1",
        params![item.id, minutes, important],
    )
    .map_err(crate::err)?;
    Ok(())
}
pub fn complete(conn: &Connection, item: &PlannedTask, done: bool, now: u64) -> Result<(), String> {
    if item.task.source == "personal" {
        conn.execute("UPDATE coach_tasks SET status=?2,completed_at=CASE WHEN ?3 THEN COALESCE(completed_at,?4) ELSE NULL END,updated_at=?4 WHERE id=?1",params![item.task.id,if done{"done"}else{"open"},done,now as i64]).map_err(crate::err)?;
    } else {
        conn.execute("UPDATE tasks SET status=?2,completed_at=CASE WHEN ?3 THEN COALESCE(completed_at,?4) ELSE NULL END,updated_at=?4 WHERE id=?1 AND deleted_at IS NULL",params![item.task.id,if done{"done"}else{"todo"},done,now as i64]).map_err(crate::err)?;
    }
    Ok(())
}
pub fn move_to(conn: &Connection, id: &str, day: &str) -> Result<(), String> {
    conn.execute("UPDATE daily_tasks SET day=?2,postponements=postponements+1,position=COALESCE((SELECT MAX(position)+1 FROM daily_tasks WHERE day=?2),0) WHERE id=?1",params![id,day]).map_err(crate::err)?;
    Ok(())
}
pub fn remove(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM daily_tasks WHERE id=?1", [id])
        .map_err(crate::err)?;
    Ok(())
}
pub fn reorder(conn: &Connection, ids: &[String]) -> Result<(), String> {
    for (position, id) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE daily_tasks SET position=?2 WHERE id=?1",
            params![id, position as i64],
        )
        .map_err(crate::err)?;
    }
    Ok(())
}
pub fn save_steps(conn: &Connection, id: &str, steps: &[Step]) -> Result<(), String> {
    conn.execute("DELETE FROM task_steps WHERE daily_task_id=?1", [id])
        .map_err(crate::err)?;
    for (position, step) in steps.iter().enumerate() {
        conn.execute(
            "INSERT INTO task_steps(daily_task_id,position,title,done) VALUES(?1,?2,?3,?4)",
            params![id, position as i64, step.title.trim(), step.done],
        )
        .map_err(crate::err)?;
    }
    Ok(())
}
