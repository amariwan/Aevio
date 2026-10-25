use super::*;

#[cfg(not(test))]
impl ActivityStore {
    pub fn list_tasks(&self, project_id: Option<&str>) -> Result<Vec<Task>, String> {
        let database = self.database().map_err(|error| error.to_string())?;
        crate::database::repositories::tasks::TaskRepository::new(&database)
            .list(project_id)
            .map_err(|error| error.to_string())
    }

    pub fn create_task(&mut self, input: NewTask, now: u64) -> Result<Task, String> {
        let database = self.database().map_err(|error| error.to_string())?;
        crate::database::repositories::tasks::TaskRepository::new(&database)
            .create(input, now)
            .map_err(|error| error.to_string())
    }

    pub fn update_task(&mut self, id: &str, patch: UpdateTask, now: u64) -> Result<Task, String> {
        let database = self.database().map_err(|error| error.to_string())?;
        crate::database::repositories::tasks::TaskRepository::new(&database)
            .update(id, patch, now)
            .map_err(|error| error.to_string())
    }

    pub fn delete_task(&mut self, id: &str, now: u64) -> Result<(), String> {
        let database = self.database().map_err(|error| error.to_string())?;
        crate::database::repositories::tasks::TaskRepository::new(&database)
            .delete(id, now)
            .map_err(|error| error.to_string())
    }

    pub fn list_labels(&self) -> Result<Vec<Label>, String> {
        let database = self.database().map_err(|error| error.to_string())?;
        crate::database::repositories::tasks::TaskRepository::new(&database)
            .list_labels()
            .map_err(|error| error.to_string())
    }

    pub fn create_label(&mut self, input: NewLabel, now: u64) -> Result<Label, String> {
        let database = self.database().map_err(|error| error.to_string())?;
        crate::database::repositories::tasks::TaskRepository::new(&database)
            .create_label(input, now)
            .map_err(|error| error.to_string())
    }

    pub fn update_label(
        &mut self,
        id: &str,
        patch: UpdateLabel,
        now: u64,
    ) -> Result<Label, String> {
        let database = self.database().map_err(|error| error.to_string())?;
        crate::database::repositories::tasks::TaskRepository::new(&database)
            .update_label(id, patch, now)
            .map_err(|error| error.to_string())
    }

    pub fn delete_label(&mut self, id: &str, now: u64) -> Result<(), String> {
        let database = self.database().map_err(|error| error.to_string())?;
        crate::database::repositories::tasks::TaskRepository::new(&database)
            .delete_label(id, now)
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
impl ActivityStore {
    pub fn list_tasks(&self, project_id: Option<&str>) -> Result<Vec<Task>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, project_id, title, description, status, priority, due_date,
                        keywords_json, ai_instructions, created_at, updated_at, completed_at, deleted_at
                 FROM tasks
                 WHERE deleted_at IS NULL AND (?1 IS NULL OR project_id = ?1)
                 ORDER BY CASE status WHEN 'in_progress' THEN 0 WHEN 'todo' THEN 1 ELSE 2 END,
                          due_date IS NULL, due_date, title COLLATE NOCASE;",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![project_id], |row| self.task_from_row(row))
            .map_err(|e| e.to_string())?;
        rows.map(|row| {
            let mut task = row.map_err(|e| e.to_string())?;
            task.labels = self.labels_for_task(&task.id)?;
            Ok(task)
        })
        .collect()
    }

    pub fn create_task(&mut self, input: NewTask, now: u64) -> Result<Task, String> {
        self.ensure_project_exists(&input.project_id)?;
        let id = uuid::Uuid::now_v7().to_string();
        let status = input.status.unwrap_or_else(|| "todo".to_string());
        let priority = input.priority.unwrap_or_else(|| "normal".to_string());
        let keywords = normalize_keywords(input.keywords.unwrap_or_default());
        self.conn
            .execute(
                "INSERT INTO tasks (id, project_id, title, description, status, priority, due_date,
                                    keywords_json, ai_instructions, created_at, updated_at, completed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10, ?11);",
                params![
                    id,
                    input.project_id,
                    input.title.trim(),
                    input.description,
                    status,
                    priority,
                    input.due_date.map(|v| v as i64),
                    serde_json::to_string(&keywords).map_err(|e| e.to_string())?,
                    input.ai_instructions,
                    now as i64,
                    (status == "done").then_some(now as i64),
                ],
            )
            .map_err(|e| e.to_string())?;
        self.replace_task_labels(&id, input.label_ids.as_deref().unwrap_or_default())?;
        self.get_task(&id)
    }

    pub fn update_task(&mut self, id: &str, patch: UpdateTask, now: u64) -> Result<Task, String> {
        let mut task = self.get_task(id)?;
        if let Some(project_id) = patch.project_id {
            self.ensure_project_exists(&project_id)?;
            task.project_id = project_id;
        }
        if let Some(title) = patch.title {
            task.title = title.trim().to_string();
        }
        if let Some(description) = patch.description {
            task.description = description;
        }
        if let Some(status) = patch.status {
            task.status = status;
        }
        if let Some(priority) = patch.priority {
            task.priority = priority;
        }
        if let Some(due_date) = patch.due_date {
            task.due_date = due_date;
        }
        if let Some(keywords) = patch.keywords {
            task.keywords = normalize_keywords(keywords);
        }
        if let Some(instructions) = patch.ai_instructions {
            task.ai_instructions = instructions;
        }
        task.completed_at = (task.status == "done").then_some(task.completed_at.unwrap_or(now));
        task.updated_at = now;
        self.conn
            .execute(
                "UPDATE tasks SET project_id = ?1, title = ?2, description = ?3, status = ?4,
                 priority = ?5, due_date = ?6, keywords_json = ?7, ai_instructions = ?8,
                 updated_at = ?9, completed_at = ?10 WHERE id = ?11;",
                params![
                    task.project_id,
                    task.title,
                    task.description,
                    task.status,
                    task.priority,
                    task.due_date.map(|v| v as i64),
                    serde_json::to_string(&task.keywords).map_err(|e| e.to_string())?,
                    task.ai_instructions,
                    now as i64,
                    task.completed_at.map(|v| v as i64),
                    id,
                ],
            )
            .map_err(|e| e.to_string())?;
        if let Some(label_ids) = patch.label_ids {
            self.replace_task_labels(id, &label_ids)?;
        }
        self.get_task(id)
    }

    pub fn delete_task(&mut self, id: &str, now: u64) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE tasks SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL;",
                params![now as i64, id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_labels(&self) -> Result<Vec<Label>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, color, created_at, updated_at, deleted_at FROM labels WHERE deleted_at IS NULL ORDER BY name COLLATE NOCASE;")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| self.label_from_row(row))
            .map_err(|e| e.to_string())?;
        rows.map(|row| row.map_err(|e| e.to_string())).collect()
    }

    pub fn create_label(&mut self, input: NewLabel, now: u64) -> Result<Label, String> {
        let id = uuid::Uuid::now_v7().to_string();
        self.conn
            .execute(
                "INSERT INTO labels (id, name, color, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4);",
                params![id, input.name.trim(), input.color, now as i64],
            )
            .map_err(|e| e.to_string())?;
        self.get_label(&id)
    }

    pub fn update_label(
        &mut self,
        id: &str,
        patch: UpdateLabel,
        now: u64,
    ) -> Result<Label, String> {
        let mut label = self.get_label(id)?;
        if let Some(name) = patch.name {
            label.name = name.trim().to_string();
        }
        if let Some(color) = patch.color {
            label.color = color;
        }
        self.conn
            .execute(
                "UPDATE labels SET name = ?1, color = ?2, updated_at = ?3 WHERE id = ?4;",
                params![label.name, label.color, now as i64, id],
            )
            .map_err(|e| e.to_string())?;
        self.get_label(id)
    }

    pub fn delete_label(&mut self, id: &str, now: u64) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE labels SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2;",
                params![now as i64, id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn get_task(&self, id: &str) -> Result<Task, String> {
        let mut task = self
            .conn
            .query_row(
                "SELECT id, project_id, title, description, status, priority, due_date, keywords_json,
                        ai_instructions, created_at, updated_at, completed_at, deleted_at FROM tasks WHERE id = ?1;",
                params![id],
                |row| self.task_from_row(row),
            )
            .map_err(|e| e.to_string())?;
        task.labels = self.labels_for_task(id)?;
        Ok(task)
    }

    fn task_from_row(&self, row: &Row<'_>) -> rusqlite::Result<Task> {
        let raw_keywords: String = row.get(7)?;
        Ok(Task {
            id: row.get(0)?,
            project_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            status: row.get(4)?,
            priority: row.get(5)?,
            due_date: row.get::<_, Option<i64>>(6)?.map(|v| v as u64),
            keywords: serde_json::from_str(&raw_keywords).unwrap_or_default(),
            ai_instructions: row.get(8)?,
            created_at: row.get::<_, i64>(9)? as u64,
            updated_at: row.get::<_, i64>(10)? as u64,
            completed_at: row.get::<_, Option<i64>>(11)?.map(|v| v as u64),
            deleted_at: row.get::<_, Option<i64>>(12)?.map(|v| v as u64),
            labels: Vec::new(),
        })
    }

    fn labels_for_task(&self, task_id: &str) -> Result<Vec<Label>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT l.id, l.name, l.color, l.created_at, l.updated_at, l.deleted_at
             FROM labels l JOIN task_labels tl ON tl.label_id = l.id
             WHERE tl.task_id = ?1 AND l.deleted_at IS NULL ORDER BY l.name COLLATE NOCASE;",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![task_id], |row| self.label_from_row(row))
            .map_err(|e| e.to_string())?;
        rows.map(|row| row.map_err(|e| e.to_string())).collect()
    }

    fn replace_task_labels(&mut self, task_id: &str, label_ids: &[String]) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM task_labels WHERE task_id = ?1;",
            params![task_id],
        )
        .map_err(|e| e.to_string())?;
        for label_id in label_ids {
            tx.execute("INSERT INTO task_labels (task_id, label_id) SELECT ?1, id FROM labels WHERE id = ?2 AND deleted_at IS NULL;", params![task_id, label_id]).map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    fn get_label(&self, id: &str) -> Result<Label, String> {
        self.conn.query_row("SELECT id, name, color, created_at, updated_at, deleted_at FROM labels WHERE id = ?1;", params![id], |row| self.label_from_row(row)).map_err(|e| e.to_string())
    }

    fn label_from_row(&self, row: &Row<'_>) -> rusqlite::Result<Label> {
        Ok(Label {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            created_at: row.get::<_, i64>(3)? as u64,
            updated_at: row.get::<_, i64>(4)? as u64,
            deleted_at: row.get::<_, Option<i64>>(5)?.map(|v| v as u64),
        })
    }

    fn ensure_project_exists(&self, id: &str) -> Result<(), String> {
        let exists: bool = self
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND deleted_at IS NULL);",
                params![id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        exists
            .then_some(())
            .ok_or_else(|| "The selected project does not exist or is archived.".to_string())
    }
}

#[cfg(test)]
fn normalize_keywords(keywords: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for keyword in keywords {
        let value = keyword.trim().to_lowercase();
        if !value.is_empty() && !normalized.contains(&value) {
            normalized.push(value);
        }
    }
    normalized
}
