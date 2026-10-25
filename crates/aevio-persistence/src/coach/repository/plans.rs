use aevio_core::coach::models::{Plan, Review};
use rusqlite::{Connection, OptionalExtension, params};

pub fn get(conn: &Connection, day: &str) -> Result<Option<Plan>, String> {
    conn.query_row("SELECT day,time_zone,goal,notes,available_minutes,energy,state,revision FROM daily_plans WHERE day=?1",[day],|row| Ok(Plan {day:row.get(0)?,time_zone:row.get(1)?,goal:row.get(2)?,notes:row.get(3)?,available_minutes:row.get(4)?,energy:row.get(5)?,state:row.get(6)?,revision:row.get(7)?})).optional().map_err(crate::err)
}
pub fn save(conn: &Connection, plan: &Plan, now: u64) -> Result<(), String> {
    let changed = conn.execute("INSERT INTO daily_plans(day,time_zone,goal,notes,available_minutes,energy,state,revision,created_at,updated_at) SELECT ?1,?2,?3,?4,?5,?6,?7,1,?8,?8 WHERE ?9=0 ON CONFLICT(day) DO NOTHING",params![plan.day,plan.time_zone,plan.goal.trim(),plan.notes,plan.available_minutes,plan.energy,plan.state,now as i64,plan.revision]).map_err(crate::err)?;
    if changed == 1 {
        return Ok(());
    }
    let changed = conn.execute("UPDATE daily_plans SET time_zone=?2,goal=?3,notes=?4,available_minutes=?5,energy=?6,state=?7,revision=revision+1,updated_at=?8 WHERE day=?1 AND revision=?9",params![plan.day,plan.time_zone,plan.goal.trim(),plan.notes,plan.available_minutes,plan.energy,plan.state,now as i64,plan.revision]).map_err(crate::err)?;
    if changed != 1 {
        return Err("The plan changed in another window. Reload before saving.".into());
    }
    Ok(())
}
pub fn ensure(conn: &Connection, day: &str, zone: &str, now: u64) -> Result<(), String> {
    conn.execute("INSERT OR IGNORE INTO daily_plans(day,time_zone,available_minutes,state,created_at,updated_at) VALUES(?1,?2,0,'planned',?3,?3)", params![day,zone,now as i64]).map_err(crate::err)?;
    Ok(())
}
pub fn review(conn: &Connection, day: &str) -> Result<Review, String> {
    Ok(conn
        .query_row(
            "SELECT outcome,worked,obstacles,tomorrow,revision FROM daily_reviews WHERE day=?1",
            [day],
            |row| {
                Ok(Review {
                    outcome: row.get(0)?,
                    worked: row.get(1)?,
                    obstacles: row.get(2)?,
                    tomorrow: row.get(3)?,
                    revision: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(crate::err)?
        .unwrap_or_default())
}
pub fn save_review(
    conn: &Connection,
    day: &str,
    review: &Review,
    revision: u32,
    now: u64,
) -> Result<(), String> {
    let current = self::review(conn, day)?;
    // A repeated save is idempotent, even when a response was lost.
    if current.outcome == review.outcome
        && current.worked == review.worked
        && current.obstacles == review.obstacles
        && current.tomorrow == review.tomorrow
    {
        return Ok(());
    }
    if current.revision != revision {
        return Err("The review changed in another window. Reload before saving.".into());
    }
    conn.execute("INSERT INTO daily_reviews(day,outcome,worked,obstacles,tomorrow,revision,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(day) DO UPDATE SET outcome=excluded.outcome,worked=excluded.worked,obstacles=excluded.obstacles,tomorrow=excluded.tomorrow,revision=excluded.revision,updated_at=excluded.updated_at", params![day,review.outcome,review.worked,review.obstacles,review.tomorrow,revision+1,now as i64]).map_err(crate::err)?;
    Ok(())
}
pub fn changes(conn: &Connection, day: &str) -> Result<u32, String> {
    conn.query_row("SELECT COUNT(*) FROM coach_interventions WHERE day=?1 AND kind IN ('plan.changed','task.moved','task.added','task.removed','tasks.reordered')",[day],|row|row.get(0)).map_err(crate::err)
}
