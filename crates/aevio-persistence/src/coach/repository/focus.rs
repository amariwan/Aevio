//! Recovery context references ActivityStore segments; it never tracks time itself.
use rusqlite::{Connection, OptionalExtension, params};

use super as repository;
use aevio_core::coach::{
    focus::{Focus, Start},
    models::PlannedTask,
};

pub fn current(conn: &Connection) -> Result<Option<Focus>, String> {
    conn.query_row("SELECT f.id,f.daily_task_id,f.segment_id,COALESCE(c.title,t.title),f.goal,f.next_step,f.planned_minutes,f.body_doubling,f.state,f.started_at,f.ended_at,MAX(0,COALESCE(s.ended_at,?1)-s.started_at) FROM coach_focus f JOIN daily_tasks d ON d.id=f.daily_task_id LEFT JOIN coach_tasks c ON c.id=d.personal_task_id LEFT JOIN tasks t ON t.id=d.project_task_id JOIN segments s ON s.id=f.segment_id WHERE f.state!='ended' ORDER BY f.started_at DESC,f.id DESC LIMIT 1",[aevio_core::timers::now_epoch_ms() as i64],|row|Ok(Focus{id:row.get(0)?,task_id:row.get(1)?,segment_id:row.get(2)?,title:row.get(3)?,goal:row.get(4)?,next_step:row.get(5)?,planned_minutes:row.get(6)?,body_doubling:row.get(7)?,state:row.get(8)?,started_at:row.get::<_,i64>(9)? as u64,ended_at:row.get::<_,Option<i64>>(10)?.map(|v|v as u64),elapsed_ms:row.get::<_,i64>(11)?.max(0) as u64})).optional().map_err(crate::err)
}
pub fn begin(
    conn: &mut Connection,
    task: &PlannedTask,
    start: Start<'_>,
    now: u64,
) -> Result<(), String> {
    let Start {
        goal,
        next_step,
        minutes,
        body,
        segment_id,
    } = start;
    aevio_core::coach::policies::text(next_step, 1000)?;
    if !(2..=180).contains(&minutes) {
        return Err("Choose between 2 and 180 focus minutes".into());
    }
    let tx = conn.transaction().map_err(crate::err)?;
    tx.execute(
        "UPDATE coach_focus SET state='ended',ended_at=COALESCE(ended_at,?1) WHERE state!='ended'",
        [now as i64],
    )
    .map_err(crate::err)?;
    tx.execute("INSERT INTO coach_focus(id,daily_task_id,segment_id,goal,next_step,planned_minutes,body_doubling,state,started_at) VALUES(?1,?2,?3,?4,?5,?6,?7,'running',?8)",params![uuid::Uuid::now_v7().to_string(),task.id,segment_id,goal,next_step,minutes,body,now as i64]).map_err(crate::err)?;
    repository::audit(&tx, "", "focus.started", now)?;
    tx.commit().map_err(crate::err)
}
/// A segment ending for any reason interrupts focus. No behavioral diagnosis.
pub fn reconcile(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("INSERT INTO focus_interruptions(focus_id,at,reason) SELECT f.id,s.ended_at,'Activity session ended' FROM coach_focus f JOIN segments s ON s.id=f.segment_id WHERE f.state='running' AND s.ended_at IS NOT NULL;UPDATE coach_focus SET state='interrupted',ended_at=(SELECT ended_at FROM segments WHERE id=segment_id) WHERE state='running' AND EXISTS(SELECT 1 FROM segments s WHERE s.id=segment_id AND s.ended_at IS NOT NULL);").map_err(crate::err)
}
pub fn finish(conn: &Connection, id: &str, now: u64) -> Result<(), String> {
    conn.execute(
        "UPDATE coach_focus SET state='ended',ended_at=COALESCE(ended_at,?2) WHERE id=?1",
        params![id, now as i64],
    )
    .map_err(crate::err)?;
    Ok(())
}
pub fn due(conn: &Connection, now: u64) -> Result<Option<String>, String> {
    conn.query_row("SELECT id FROM coach_focus WHERE state='running' AND reminded_at IS NULL AND started_at+planned_minutes*60000<=?1 ORDER BY started_at LIMIT 1",[now as i64],|row|row.get(0)).optional().map_err(crate::err)
}
pub fn mark_reminded(conn: &Connection, id: &str, now: u64) -> Result<(), String> {
    conn.execute(
        "UPDATE coach_focus SET reminded_at=?2 WHERE id=?1",
        params![id, now as i64],
    )
    .map_err(crate::err)?;
    Ok(())
}
pub fn attribute_entries(conn: &Connection) -> Result<(), String> {
    // Only unambiguously contained focus entries are linked. Never overwrite an explicit link.
    conn.execute_batch("INSERT OR IGNORE INTO task_time_links(entry_id,daily_task_id) SELECT e.id,f.daily_task_id FROM coach_focus f JOIN segments s ON s.id=f.segment_id JOIN time_entries e ON e.id=s.entry_id AND e.deleted_at IS NULL AND e.started_at>=s.started_at AND e.ended_at<=COALESCE(s.ended_at,e.ended_at) AND e.started_at<e.ended_at;").map_err(crate::err)
}
