//! SQLite is confined to this boundary; callers own the transaction.
#![cfg(test)]
pub mod adaptive;
pub mod brain;
pub mod focus;
#[cfg(test)]
pub mod legacy;
pub mod plans;
pub mod reminders;
pub mod tasks;
pub mod time;
use aevio_core::coach::{models::Preferences, policies};
use rusqlite::{Connection, OptionalExtension, params};

pub fn preferences(conn: &Connection) -> Result<Preferences, String> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM coach_preferences WHERE id=1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(crate::err)?;
    value
        .map(|raw| {
            serde_json::from_str(&raw)
                .map_err(|error| format!("Coach preferences are invalid: {error}"))
        })
        .unwrap_or_else(|| Ok(policies::defaults()))
}
pub fn save_preferences(conn: &Connection, value: &Preferences) -> Result<(), String> {
    conn.execute("INSERT INTO coach_preferences(id,value) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET value=excluded.value", [serde_json::to_string(value).map_err(|e| e.to_string())?]).map_err(crate::err)?;
    Ok(())
}
pub fn last_intervention(conn: &Connection) -> Result<Option<u64>, String> {
    conn.query_row("SELECT MAX(at) FROM coach_interventions", [], |row| {
        row.get::<_, Option<i64>>(0)
    })
    .map(|value| value.map(|v| v as u64))
    .map_err(crate::err)
}
pub fn audit(conn: &Connection, day: &str, kind: &str, now: u64) -> Result<(), String> {
    conn.execute(
        "INSERT INTO coach_interventions(day,kind,at) VALUES(?1,?2,?3)",
        params![day, kind, now as i64],
    )
    .map_err(crate::err)?;
    // Event metadata only, bounded retention; never retain captured activity.
    conn.execute(
        "DELETE FROM coach_interventions WHERE at < ?1",
        [now.saturating_sub(90 * 86_400_000) as i64],
    )
    .map_err(crate::err)?;
    Ok(())
}
pub fn erase(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("DELETE FROM coach_routine_runs; DELETE FROM coach_routine_items; DELETE FROM coach_routines; DELETE FROM brain_dump_entries; DELETE FROM coach_memory; DELETE FROM focus_interruptions; DELETE FROM coach_focus; DELETE FROM task_time_links; DELETE FROM task_steps; DELETE FROM daily_tasks; DELETE FROM daily_reviews; DELETE FROM daily_plans; DELETE FROM coach_preferences; DELETE FROM coach_interventions; DELETE FROM coach_tasks;").map_err(crate::err)
}
