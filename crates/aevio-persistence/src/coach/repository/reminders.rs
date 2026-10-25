use aevio_core::coach::{models::Preferences, policies};
use rusqlite::{Connection, params};

pub fn allowed(
    conn: &Connection,
    preferences: &Preferences,
    minute: u16,
    now: u64,
) -> Result<bool, String> {
    if !preferences.enabled || !preferences.reminders || policies::quiet(preferences, minute) {
        return Ok(false);
    }
    let last: Option<i64> = conn
        .query_row(
            "SELECT MAX(at) FROM coach_interventions WHERE kind='reminder.sent'",
            [],
            |row| row.get(0),
        )
        .map_err(crate::err)?;
    Ok(last.is_none_or(|last| {
        now.saturating_sub(last.max(0) as u64) >= preferences.cooldown_minutes as u64 * 60000
    }))
}
pub fn review_due(
    conn: &Connection,
    day: &str,
    minute: u16,
    preferences: &Preferences,
) -> Result<bool, String> {
    if minute < preferences.review_minute {
        return Ok(false);
    }
    conn.query_row("SELECT EXISTS(SELECT 1 FROM daily_plans p WHERE p.day=?1 AND p.state='planned' AND NOT EXISTS(SELECT 1 FROM daily_reviews r WHERE r.day=p.day AND r.outcome!='not_reviewed') AND NOT EXISTS(SELECT 1 FROM coach_interventions i WHERE i.day=p.day AND i.kind='review.offered'))",[day],|row|row.get(0)).map_err(crate::err)
}
pub fn mark_sent(conn: &Connection, day: &str, kind: &str, now: u64) -> Result<(), String> {
    super::audit(conn, day, "reminder.sent", now)?;
    super::audit(conn, day, kind, now)
}
pub fn due_task(conn: &Connection, now: u64) -> Result<Option<(String, String)>, String> {
    use rusqlite::OptionalExtension;
    conn.query_row("SELECT id,title FROM coach_tasks WHERE status='open' AND reminder_at IS NOT NULL AND reminder_at<=?1 AND notified_at IS NULL ORDER BY reminder_at LIMIT 1",[now as i64],|row|Ok((row.get(0)?,row.get(1)?))).optional().map_err(crate::err)
}
pub fn mark_task(conn: &Connection, id: &str, now: u64) -> Result<(), String> {
    conn.execute(
        "UPDATE coach_tasks SET notified_at=?2 WHERE id=?1",
        params![id, now as i64],
    )
    .map_err(crate::err)?;
    Ok(())
}

pub fn guardian_due(conn: &Connection, now: u64) -> Result<bool, String> {
    let switches: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM (SELECT app,LAG(app) OVER(ORDER BY started_at,id) AS previous FROM segments WHERE kind='activity' AND started_at>=?1) WHERE previous IS NOT NULL AND app<>previous",
            [now.saturating_sub(10 * 60_000) as i64],
            |row| row.get(0),
        )
        .map_err(crate::err)?;
    Ok(switches >= 8)
}
