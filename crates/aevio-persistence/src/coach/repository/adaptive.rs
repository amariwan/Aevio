use crate::coach::service;
use aevio_core::coach::{
    adaptive::{WeekDay, Weekly},
    dates,
};
use chrono::{Datelike, Duration};
use rusqlite::{Connection, params};

#[cfg(test)]
use aevio_core::coach::{
    adaptive::{Estimate, Memory},
    policies,
};

#[cfg(test)]
pub fn memory(conn: &Connection) -> Result<Vec<Memory>, String> {
    let mut stmt=conn.prepare("SELECT id,content,kind,updated_at FROM coach_memory ORDER BY updated_at DESC,id LIMIT 200").map_err(crate::err)?;
    stmt.query_map([], |r| {
        Ok(Memory {
            id: r.get(0)?,
            content: r.get(1)?,
            kind: r.get(2)?,
            updated_at: r.get::<_, i64>(3)? as u64,
        })
    })
    .map_err(crate::err)?
    .collect::<rusqlite::Result<Vec<_>>>()
    .map_err(crate::err)
}
#[cfg(test)]
pub fn save_memory(conn: &Connection, value: &Memory, now: u64) -> Result<(), String> {
    policies::title(&value.content)?;
    if !matches!(value.kind.as_str(), "explicit" | "confirmed") {
        return Err("Memory must be explicitly authored or confirmed".into());
    }
    let existing = memory(conn)?;
    if !existing.iter().any(|item| item.id == value.id) && existing.len() >= 200 {
        return Err("Keep at most 200 memory notes; remove an older note first".into());
    }
    conn.execute("INSERT INTO coach_memory(id,content,kind,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET content=excluded.content,kind=excluded.kind,updated_at=excluded.updated_at",params![if value.id.is_empty(){uuid::Uuid::now_v7().to_string()}else{value.id.clone()},value.content.trim(),value.kind,now as i64]).map_err(crate::err)?;
    Ok(())
}
#[cfg(test)]
pub fn delete_memory(conn: &Connection, id: Option<&str>) -> Result<(), String> {
    conn.execute("DELETE FROM coach_memory WHERE ?1 IS NULL OR id=?1", [id])
        .map_err(crate::err)?;
    Ok(())
}

pub fn weekly(conn: &Connection, day: &str, zone: &str) -> Result<Weekly, String> {
    let date = dates::parse(day)?;
    let monday = date - Duration::days(date.weekday().num_days_from_monday() as i64);
    let mut weeks = Vec::new();
    for offset in [-7, 0] {
        let mut rows = Vec::new();
        for index in 0..7 {
            let date = (monday + Duration::days(offset + index))
                .format("%Y-%m-%d")
                .to_string();
            let snapshot = service::day(conn, &date, zone)?;
            let (start, end) = dates::bounds(&date, &snapshot.time_zone)?;
            let focus_sessions = conn
                .query_row(
                    "SELECT COUNT(*) FROM coach_focus WHERE started_at>=?1 AND started_at<?2",
                    params![start, end],
                    |r| r.get(0),
                )
                .map_err(crate::err)?;
            rows.push(WeekDay {
                day: date,
                completed: conn.query_row("SELECT COUNT(DISTINCT CASE WHEN d.personal_task_id IS NULL THEN 'project:'||d.project_task_id ELSE 'personal:'||d.personal_task_id END) FROM daily_tasks d LEFT JOIN coach_tasks c ON c.id=d.personal_task_id LEFT JOIN tasks t ON t.id=d.project_task_id WHERE COALESCE(c.completed_at,t.completed_at)>=?1 AND COALESCE(c.completed_at,t.completed_at)<?2",params![start,end],|r|r.get(0)).map_err(crate::err)?,
                planned_minutes: snapshot.planned_minutes,
                actual_ms: snapshot.tracked_ms,
                outcome: snapshot.review.outcome,
                focus_sessions,
                postponed: snapshot
                    .tasks
                    .iter()
                    .filter(|item| item.postponements > 0)
                    .count() as u32,
            });
        }
        weeks.push(rows);
    }
    let days = weeks.pop().ok_or("Missing current week")?;
    let previous = weeks.pop().ok_or("Missing previous week")?;
    let mut recommendations=vec!["Erfasste Zeit und erledigte Aufgaben beschreiben unterschiedliche Dinge. Deine Zielbewertung bleibt maßgeblich.".into()];
    let planned: u32 = days.iter().map(|day| day.planned_minutes).sum();
    let actual: u64 = days.iter().map(|day| day.actual_ms).sum();
    if planned > 0 && actual > 0 {
        recommendations.push(format!("Diese Woche sind {planned} Minuten geplant und {} Minuten erfasst. Nutze die Abweichung als Gesprächsanlass für deine nächste Planung.",actual/60000));
    }
    if days.iter().any(|day| day.postponed > 0) {
        recommendations.push("Falls eine Aufgabe mehrfach weiterwandert, kannst du ihren ersten Schritt verkleinern oder ihre Relevanz neu entscheiden.".into());
    }
    Ok(Weekly {
        starts_on: monday.format("%Y-%m-%d").to_string(),
        days,
        previous,
        recommendations,
        source: "local_aggregates".into(),
    })
}

/// Exact task identity only; no behavioral or psychological profiling.
#[cfg(test)]
pub fn estimates(conn: &Connection) -> Result<Vec<Estimate>, String> {
    if !super::preferences(conn)?.memory_enabled {
        return Ok(Vec::new());
    }
    let mut stmt=conn.prepare("SELECT CASE WHEN d.personal_task_id IS NULL THEN 'project' ELSE 'personal' END,COALESCE(d.personal_task_id,d.project_task_id),CAST(ROUND(AVG(actual.ms)/60000.0) AS INTEGER),COUNT(*) FROM daily_tasks d JOIN (SELECT l.daily_task_id,SUM(e.ended_at-e.started_at) AS ms FROM task_time_links l JOIN time_entries e ON e.id=l.entry_id AND e.deleted_at IS NULL GROUP BY l.daily_task_id) actual ON actual.daily_task_id=d.id WHERE actual.ms>0 GROUP BY 1,2 HAVING COUNT(*)>=2 LIMIT 200").map_err(crate::err)?;
    stmt.query_map([], |r| {
        Ok(Estimate {
            source: r.get(0)?,
            task_id: r.get(1)?,
            minutes: r.get::<_, u32>(2)?.clamp(1, 1440),
            samples: r.get(3)?,
        })
    })
    .map_err(crate::err)?
    .collect::<rusqlite::Result<Vec<_>>>()
    .map_err(crate::err)
}
