use super::{plans, tasks};
use crate::coach::brain::{self, BrainEntry, Command, Routine, RoutineTask};
use aevio_core::coach::{dates, policies};
use rusqlite::{Connection, OptionalExtension, params};

pub fn entries(conn: &Connection) -> Result<Vec<BrainEntry>, String> {
    let mut stmt=conn.prepare("SELECT id,content,category,converted_task_id,updated_at FROM brain_dump_entries ORDER BY updated_at DESC,id LIMIT 500").map_err(crate::err)?;
    stmt.query_map([], |r| {
        Ok(BrainEntry {
            id: r.get(0)?,
            content: r.get(1)?,
            category: r.get(2)?,
            converted_task_id: r.get(3)?,
            updated_at: r.get::<_, i64>(4)? as u64,
        })
    })
    .map_err(crate::err)?
    .collect::<rusqlite::Result<Vec<_>>>()
    .map_err(crate::err)
}
pub fn routines(conn: &Connection) -> Result<Vec<Routine>, String> {
    let mut stmt = conn
        .prepare("SELECT id,title FROM coach_routines ORDER BY title COLLATE NOCASE LIMIT 100")
        .map_err(crate::err)?;
    let mut rows = stmt
        .query_map([], |r| {
            Ok(Routine {
                id: r.get(0)?,
                title: r.get(1)?,
                tasks: Vec::new(),
            })
        })
        .map_err(crate::err)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(crate::err)?;
    let mut items=conn.prepare("SELECT title,estimate_minutes FROM coach_routine_items WHERE routine_id=?1 ORDER BY position").map_err(crate::err)?;
    for routine in &mut rows {
        routine.tasks = items
            .query_map([&routine.id], |r| {
                Ok(RoutineTask {
                    title: r.get(0)?,
                    estimate_minutes: r.get(1)?,
                })
            })
            .map_err(crate::err)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(crate::err)?;
    }
    Ok(rows)
}
pub fn apply(
    conn: &mut Connection,
    day: &str,
    zone: &str,
    command: Command,
    now: u64,
) -> Result<(), String> {
    dates::bounds(day, zone)?;
    let tx = conn.transaction().map_err(crate::err)?;
    match command {
        Command::SaveEntry { entry } => {
            policies::text(&entry.content, 10000)?;
            if entry.content.trim().is_empty() || !brain::valid_category(&entry.category) {
                return Err("Enter a thought and select a supported category".into());
            }
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM brain_dump_entries WHERE id=?1)",
                    [&entry.id],
                    |r| r.get(0),
                )
                .map_err(crate::err)?;
            if !exists && entries(&tx)?.len() >= 500 {
                return Err("Keep at most 500 thoughts; remove an older entry first".into());
            }
            tx.execute("INSERT INTO brain_dump_entries(id,content,category,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET content=excluded.content,category=excluded.category,updated_at=excluded.updated_at",params![if entry.id.is_empty(){uuid::Uuid::now_v7().to_string()}else{entry.id},entry.content.trim(),entry.category,now as i64]).map_err(crate::err)?;
        }
        Command::RemoveEntry { id } => {
            tx.execute("DELETE FROM brain_dump_entries WHERE id=?1", [id])
                .map_err(crate::err)?;
        }
        Command::Convert { id } => {
            let (content, converted): (String, Option<String>) = tx
                .query_row(
                    "SELECT content,converted_task_id FROM brain_dump_entries WHERE id=?1",
                    [&id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(crate::err)?
                .ok_or("Thought not found")?;
            if converted.is_none() {
                plans::ensure(&tx, day, zone, now)?;
                let title = content
                    .lines()
                    .next()
                    .unwrap_or(&content)
                    .chars()
                    .take(300)
                    .collect::<String>();
                let task = tasks::create_personal(&tx, &title, &content, 3, 0, None, now)?;
                tasks::add(&tx, day, &task, 0, false)?;
                tx.execute(
                    "UPDATE brain_dump_entries SET converted_task_id=?2,updated_at=?3 WHERE id=?1",
                    params![id, task.id, now as i64],
                )
                .map_err(crate::err)?;
            }
        }
        Command::SaveRoutine { routine } => {
            policies::title(&routine.title)?;
            if routine.tasks.is_empty() || routine.tasks.len() > 30 {
                return Err("A routine needs between 1 and 30 tasks".into());
            }
            let id = if routine.id.is_empty() {
                uuid::Uuid::now_v7().to_string()
            } else {
                routine.id
            };
            if !routines(&tx)?.iter().any(|routine| routine.id == id) && routines(&tx)?.len() >= 100
            {
                return Err("Keep at most 100 routines".into());
            }
            tx.execute("INSERT INTO coach_routines(id,title) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET title=excluded.title",params![id,routine.title.trim()]).map_err(crate::err)?;
            tx.execute("DELETE FROM coach_routine_items WHERE routine_id=?1", [&id])
                .map_err(crate::err)?;
            for (index, item) in routine.tasks.iter().enumerate() {
                policies::title(&item.title)?;
                if item.estimate_minutes > 1440 {
                    return Err("Routine task estimate exceeds one day".into());
                }
                tx.execute("INSERT INTO coach_routine_items(routine_id,position,title,estimate_minutes) VALUES(?1,?2,?3,?4)",params![id,index as i64,item.title.trim(),item.estimate_minutes]).map_err(crate::err)?;
            }
        }
        Command::RemoveRoutine { id } => {
            tx.execute("DELETE FROM coach_routines WHERE id=?1", [id])
                .map_err(crate::err)?;
        }
        Command::ApplyRoutine { id } => {
            let routine = routines(&tx)?
                .into_iter()
                .find(|routine| routine.id == id)
                .ok_or("Routine no longer exists")?;
            plans::ensure(&tx, day, zone, now)?;
            if tx
                .execute(
                    "INSERT OR IGNORE INTO coach_routine_runs(routine_id,day) VALUES(?1,?2)",
                    params![id, day],
                )
                .map_err(crate::err)?
                == 1
            {
                for item in routine.tasks {
                    let task = tasks::create_personal(
                        &tx,
                        &item.title,
                        "",
                        3,
                        item.estimate_minutes,
                        None,
                        now,
                    )?;
                    tasks::add(&tx, day, &task, item.estimate_minutes, false)?;
                }
            }
        }
    }
    super::audit(&tx, day, "brain.changed", now)?;
    tx.commit().map_err(crate::err)
}
