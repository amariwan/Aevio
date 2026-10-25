use super::*;

#[cfg(not(test))]
impl ActivityStore {
    pub fn list_apps(&self) -> Result<Vec<AppRecord>, String> {
        let database = self.database().map_err(|error| error.to_string())?;
        crate::database::repositories::apps::AppRepository::new(&database)
            .list()
            .map_err(|error| error.to_string())
    }

    pub fn update_app(
        &mut self,
        id: &str,
        default_category_id: Option<String>,
        default_project_id: Option<String>,
        excluded: Option<bool>,
        now: u64,
    ) -> Result<AppRecord, String> {
        let should_exclude = excluded == Some(true);
        let database = self.database().map_err(|error| error.to_string())?;
        let result = crate::database::repositories::apps::AppRepository::new(&database)
            .update(id, default_category_id, default_project_id, excluded, now)
            .map_err(|error| error.to_string())?;
        drop(database);
        if should_exclude {
            if let Some(ref pending) = self.pending_switch
                && self.is_sample_excluded(&pending.sample)
            {
                self.pending_switch = None;
            }
            if let Some(ref cur) = self.current {
                let cur_sample = WindowSample {
                    app: cur.app.clone(),
                    title: cur.title.clone(),
                    bundle_id: cur.bundle_id.clone(),
                    url: cur.url.clone(),
                    domain: cur.domain.clone(),
                    keeps_display_awake: false,
                };
                if self.is_sample_excluded(&cur_sample) {
                    self.pending_switch = None;
                    let _ = self.close_current(now);
                }
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
impl ActivityStore {
    pub fn list_apps(&self) -> Result<Vec<AppRecord>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, identifier, display_name, default_category_id, default_project_id, excluded, first_seen, last_seen, created_at, updated_at, deleted_at
             FROM apps
             WHERE deleted_at IS NULL
             ORDER BY last_seen DESC;",
        ).map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(AppRecord {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    identifier: row.get(2)?,
                    display_name: row.get(3)?,
                    default_category_id: row.get(4)?,
                    default_project_id: row.get(5)?,
                    excluded: row.get::<_, i64>(6)? != 0,
                    first_seen: row.get::<_, i64>(7)? as u64,
                    last_seen: row.get::<_, i64>(8)? as u64,
                    created_at: row.get::<_, i64>(9)? as u64,
                    updated_at: row.get::<_, i64>(10)? as u64,
                    deleted_at: row.get::<_, Option<i64>>(11)?.map(|v| v as u64),
                })
            })
            .map_err(|e| e.to_string())?;

        let mut list = Vec::new();
        for a in rows {
            list.push(a.map_err(|e| e.to_string())?);
        }
        Ok(list)
    }

    pub fn update_app(
        &mut self,
        id: &str,
        default_category_id: Option<String>,
        default_project_id: Option<String>,
        excluded: Option<bool>,
        now: u64,
    ) -> Result<AppRecord, String> {
        if let Some(cat) = default_category_id.as_deref() {
            self.conn
                .execute(
                    "UPDATE apps SET default_category_id = ?1, updated_at = ?2 WHERE id = ?3;",
                    params![cat, now as i64, id],
                )
                .map_err(|e| e.to_string())?;
        }
        if let Some(proj) = default_project_id.as_deref() {
            self.conn
                .execute(
                    "UPDATE apps SET default_project_id = ?1, updated_at = ?2 WHERE id = ?3;",
                    params![proj, now as i64, id],
                )
                .map_err(|e| e.to_string())?;
        }
        if let Some(exc) = excluded {
            self.conn
                .execute(
                    "UPDATE apps SET excluded = ?1, updated_at = ?2 WHERE id = ?3;",
                    params![exc as i64, now as i64, id],
                )
                .map_err(|e| e.to_string())?;

            if exc {
                if let Some(ref pending) = self.pending_switch
                    && self.is_sample_excluded(&pending.sample)
                {
                    self.pending_switch = None;
                }
                if let Some(ref cur) = self.current {
                    let cur_sample = WindowSample {
                        app: cur.app.clone(),
                        title: cur.title.clone(),
                        bundle_id: cur.bundle_id.clone(),
                        url: cur.url.clone(),
                        domain: cur.domain.clone(),
                        keeps_display_awake: false,
                    };
                    if self.is_sample_excluded(&cur_sample) {
                        self.pending_switch = None;
                        let _ = self.close_current(now);
                    }
                }
            }
        }

        self.conn.query_row(
            "SELECT id, kind, identifier, display_name, default_category_id, default_project_id, excluded, first_seen, last_seen, created_at, updated_at, deleted_at
             FROM apps WHERE id = ?1;",
            params![id],
            |row| {
                Ok(AppRecord {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    identifier: row.get(2)?,
                    display_name: row.get(3)?,
                    default_category_id: row.get(4)?,
                    default_project_id: row.get(5)?,
                    excluded: row.get::<_, i64>(6)? != 0,
                    first_seen: row.get::<_, i64>(7)? as u64,
                    last_seen: row.get::<_, i64>(8)? as u64,
                    created_at: row.get::<_, i64>(9)? as u64,
                    updated_at: row.get::<_, i64>(10)? as u64,
                    deleted_at: row.get::<_, Option<i64>>(11)?.map(|v| v as u64),
                })
            },
        ).map_err(|e| e.to_string())
    }
}
