use super::*;

impl ActivityStore {
    pub fn list_categories(&self) -> Result<Vec<Category>, String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            return crate::database::repositories::catalog::CatalogRepository::new(&database)
                .categories()
                .map(|rows| rows.into_iter().map(category_from_model).collect())
                .map_err(|error| error.to_string());
        }
        #[cfg(test)]
        return self.list_categories_legacy();
        #[cfg(not(test))]
        Err("database path unavailable for catalog read".into())
    }

    #[cfg(test)]
    fn list_categories_legacy(&self) -> Result<Vec<Category>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, color, description, ai_prompt, billable_default, counts_as_work, archived, sort, created_at, updated_at, deleted_at
                 FROM categories
                 WHERE deleted_at IS NULL
                 ORDER BY sort ASC, name ASC;",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Category {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    description: row.get(3)?,
                    ai_prompt: row.get(4)?,
                    billable_default: row.get::<_, i64>(5)? != 0,
                    counts_as_work: row.get::<_, i64>(6)? != 0,
                    archived: row.get::<_, i64>(7)? != 0,
                    sort: row.get(8)?,
                    created_at: row.get::<_, i64>(9)? as u64,
                    updated_at: row.get::<_, i64>(10)? as u64,
                    deleted_at: row.get::<_, Option<i64>>(11)?.map(|v| v as u64),
                })
            })
            .map_err(|e| e.to_string())?;

        let mut list = Vec::new();
        for cat in rows {
            list.push(cat.map_err(|e| e.to_string())?);
        }
        Ok(list)
    }

    pub fn create_category(&mut self, cat: NewCategory, now: u64) -> Result<Category, String> {
        let id = uuid::Uuid::now_v7().to_string();
        let billable = cat.billable_default.unwrap_or(false) as i64;
        let counts = cat.counts_as_work.unwrap_or(true) as i64;
        let sort = cat.sort.unwrap_or(0);

        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let model = crate::database::entities::catalog::categories::ActiveModel {
                id: sea_orm::Set(id),
                name: sea_orm::Set(cat.name),
                color: sea_orm::Set(cat.color),
                description: sea_orm::Set(cat.description),
                ai_prompt: sea_orm::Set(cat.ai_prompt),
                billable_default: sea_orm::Set(billable),
                counts_as_work: sea_orm::Set(counts),
                archived: sea_orm::Set(0),
                sort: sea_orm::Set(sort),
                created_at: sea_orm::Set(now as i64),
                updated_at: sea_orm::Set(now as i64),
                deleted_at: sea_orm::NotSet,
            };
            let row = crate::database::repositories::catalog::CatalogRepository::new(&database)
                .create_category(model)
                .map_err(|error| error.to_string())?;
            return Ok(category_from_model(row));
        }

        #[cfg(not(test))]
        return Err("database path unavailable for catalog write".into());
        #[cfg(test)]
        {
            self.conn
            .execute(
                "INSERT INTO categories (id, name, color, description, ai_prompt, billable_default, counts_as_work, archived, sort, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, ?9);",
                params![id, cat.name, cat.color, cat.description, cat.ai_prompt, billable, counts, sort, now as i64],
            )
            .map_err(|e| e.to_string())?;

            Ok(Category {
                id,
                name: cat.name,
                color: cat.color,
                description: cat.description,
                ai_prompt: cat.ai_prompt,
                billable_default: billable != 0,
                counts_as_work: counts != 0,
                archived: false,
                sort,
                created_at: now,
                updated_at: now,
                deleted_at: None,
            })
        }
    }

    pub fn update_category(
        &mut self,
        id: &str,
        patch: UpdateCategory,
        now: u64,
    ) -> Result<Category, String> {
        let mut cat = self.get_category(id)?;
        if let Some(name) = patch.name {
            cat.name = name;
        }
        if let Some(color) = patch.color {
            cat.color = color;
        }
        if let Some(desc) = patch.description {
            cat.description = Some(desc);
        }
        if let Some(prompt) = patch.ai_prompt {
            cat.ai_prompt = Some(prompt);
        }
        if let Some(b) = patch.billable_default {
            cat.billable_default = b;
        }
        if let Some(c) = patch.counts_as_work {
            cat.counts_as_work = c;
        }
        if let Some(a) = patch.archived {
            cat.archived = a;
        }
        if let Some(s) = patch.sort {
            cat.sort = s;
        }
        cat.updated_at = now;

        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let model = crate::database::entities::catalog::categories::ActiveModel {
                id: sea_orm::Set(cat.id.clone()),
                name: sea_orm::Set(cat.name),
                color: sea_orm::Set(cat.color),
                description: sea_orm::Set(cat.description),
                ai_prompt: sea_orm::Set(cat.ai_prompt),
                billable_default: sea_orm::Set(cat.billable_default as i64),
                counts_as_work: sea_orm::Set(cat.counts_as_work as i64),
                archived: sea_orm::Set(cat.archived as i64),
                sort: sea_orm::Set(cat.sort),
                created_at: sea_orm::NotSet,
                updated_at: sea_orm::Set(now as i64),
                deleted_at: sea_orm::NotSet,
            };
            let row = crate::database::repositories::catalog::CatalogRepository::new(&database)
                .update_category(model)
                .map_err(|error| error.to_string())?;
            return Ok(category_from_model(row));
        }

        #[cfg(not(test))]
        return Err("database path unavailable for catalog write".into());
        #[cfg(test)]
        {
            self.conn
            .execute(
                "UPDATE categories SET name = ?1, color = ?2, description = ?3, ai_prompt = ?4, billable_default = ?5, counts_as_work = ?6, archived = ?7, sort = ?8, updated_at = ?9
                 WHERE id = ?10;",
                params![
                    cat.name,
                    cat.color,
                    cat.description,
                    cat.ai_prompt,
                    cat.billable_default as i64,
                    cat.counts_as_work as i64,
                    cat.archived as i64,
                    cat.sort,
                    now as i64,
                    id,
                ],
            )
            .map_err(|e| e.to_string())?;

            Ok(cat)
        }
    }

    pub fn delete_category(&mut self, id: &str, now: u64) -> Result<(), String> {
        self.delete_undoable(&[], Some(id), None, now)
    }

    fn get_category(&self, id: &str) -> Result<Category, String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let row = crate::database::repositories::catalog::CatalogRepository::new(&database)
                .category(id)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "category not found".to_string())?;
            return Ok(category_from_model(row));
        }
        #[cfg(not(test))]
        return Err("database path unavailable for catalog read".into());
        #[cfg(test)]
        self.conn
            .query_row(
                "SELECT id, name, color, description, ai_prompt, billable_default, counts_as_work, archived, sort, created_at, updated_at, deleted_at
                 FROM categories WHERE id = ?1;",
                params![id],
                |row| {
                    Ok(Category {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        color: row.get(2)?,
                        description: row.get(3)?,
                        ai_prompt: row.get(4)?,
                        billable_default: row.get::<_, i64>(5)? != 0,
                        counts_as_work: row.get::<_, i64>(6)? != 0,
                        archived: row.get::<_, i64>(7)? != 0,
                        sort: row.get(8)?,
                        created_at: row.get::<_, i64>(9)? as u64,
                        updated_at: row.get::<_, i64>(10)? as u64,
                        deleted_at: row.get::<_, Option<i64>>(11)?.map(|v| v as u64),
                    })
                },
            )
            .map_err(|e| e.to_string())
    }

    // --- P1: Projects CRUD ----------------------------------------------

    pub fn list_projects(&self) -> Result<Vec<Project>, String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            return crate::database::repositories::catalog::CatalogRepository::new(&database)
                .projects()
                .map(|rows| rows.into_iter().map(project_from_model).collect())
                .map_err(|error| error.to_string());
        }
        #[cfg(test)]
        return self.list_projects_legacy();
        #[cfg(not(test))]
        Err("database path unavailable for catalog read".into())
    }

    #[cfg(test)]
    fn list_projects_legacy(&self) -> Result<Vec<Project>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, client_id, name, color, description, ai_hints, status, due_date, budget_kind, budget_value, budget_period, billable_default, hourly_rate, created_at, updated_at, deleted_at
                 FROM projects
                 WHERE deleted_at IS NULL
                 ORDER BY name ASC;",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    client_id: row.get(1)?,
                    name: row.get(2)?,
                    color: row.get(3)?,
                    description: row.get(4)?,
                    ai_hints: row.get(5)?,
                    status: row.get(6)?,
                    due_date: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
                    budget_kind: row.get(8)?,
                    budget_value: row.get(9)?,
                    budget_period: row.get(10)?,
                    billable_default: row.get::<_, i64>(11)? != 0,
                    hourly_rate: row.get(12)?,
                    created_at: row.get::<_, i64>(13)? as u64,
                    updated_at: row.get::<_, i64>(14)? as u64,
                    deleted_at: row.get::<_, Option<i64>>(15)?.map(|v| v as u64),
                })
            })
            .map_err(|e| e.to_string())?;

        let mut list = Vec::new();
        for p in rows {
            list.push(p.map_err(|e| e.to_string())?);
        }
        Ok(list)
    }

    pub fn create_project(&mut self, proj: NewProject, now: u64) -> Result<Project, String> {
        let id = uuid::Uuid::now_v7().to_string();
        let status = proj.status.unwrap_or_else(|| "active".to_string());
        let budget_kind = proj.budget_kind.unwrap_or_else(|| "none".to_string());
        let budget_period = proj.budget_period.unwrap_or_else(|| "total".to_string());
        let billable = proj.billable_default.unwrap_or(false) as i64;

        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let model = crate::database::entities::catalog::projects::ActiveModel {
                id: sea_orm::Set(id),
                client_id: sea_orm::Set(proj.client_id.clone()),
                name: sea_orm::Set(proj.name.clone()),
                color: sea_orm::Set(proj.color.clone()),
                description: sea_orm::Set(proj.description.clone()),
                ai_hints: sea_orm::Set(proj.ai_hints.clone()),
                status: sea_orm::Set(status.clone()),
                due_date: sea_orm::Set(proj.due_date.map(|value| value as i64)),
                budget_kind: sea_orm::Set(budget_kind.clone()),
                budget_value: sea_orm::Set(proj.budget_value),
                budget_period: sea_orm::Set(budget_period.clone()),
                billable_default: sea_orm::Set(billable),
                hourly_rate: sea_orm::Set(proj.hourly_rate),
                created_at: sea_orm::Set(now as i64),
                updated_at: sea_orm::Set(now as i64),
                deleted_at: sea_orm::NotSet,
            };
            let row = crate::database::repositories::catalog::CatalogRepository::new(&database)
                .create_project(model)
                .map_err(|error| error.to_string())?;
            return Ok(project_from_model(row));
        }

        #[cfg(not(test))]
        return Err("database path unavailable for catalog write".into());
        #[cfg(test)]
        {
            self.conn.execute(
            "INSERT INTO projects (id, client_id, name, color, description, ai_hints, status, due_date, budget_kind, budget_value, budget_period, billable_default, hourly_rate, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14);",
            params![
                id,
                proj.client_id,
                proj.name,
                proj.color,
                proj.description,
                proj.ai_hints,
                status,
                proj.due_date.map(|v| v as i64),
                budget_kind,
                proj.budget_value,
                budget_period,
                billable,
                proj.hourly_rate,
                now as i64,
            ],
        ).map_err(|e| e.to_string())?;
            if let Some(path) = self.database_path.as_ref() {
                let _database =
                    crate::database::connect(path).map_err(|error| error.to_string())?;
            } else {
                #[cfg(test)]
                crate::projects::sync_hint_rules(&self.conn, &id, proj.ai_hints.as_deref(), now)?;
                #[cfg(not(test))]
                return Err("database path unavailable for project hint synchronization".into());
            }

            Ok(Project {
                id,
                client_id: proj.client_id,
                name: proj.name,
                color: proj.color,
                description: proj.description,
                ai_hints: proj.ai_hints,
                status,
                due_date: proj.due_date,
                budget_kind,
                budget_value: proj.budget_value,
                budget_period,
                billable_default: billable != 0,
                hourly_rate: proj.hourly_rate,
                created_at: now,
                updated_at: now,
                deleted_at: None,
            })
        }
    }

    pub fn update_project(
        &mut self,
        id: &str,
        patch: UpdateProject,
        now: u64,
    ) -> Result<Project, String> {
        let mut proj = self.get_project(id)?;
        if let Some(client_id) = patch.client_id {
            let client_id = client_id.filter(|value| !value.is_empty());
            if client_id != proj.client_id {
                // An invoice bills a client; moving invoiced time to another
                // one would change what was already sent.
                let invoiced = if let Some(path) = self.database_path.as_ref() {
                    let database =
                        crate::database::connect(path).map_err(|error| error.to_string())?;
                    crate::database::repositories::catalog::CatalogRepository::new(&database)
                        .project_has_invoiced_time(id)
                        .map_err(|error| error.to_string())?
                } else {
                    #[cfg(test)]
                    {
                        self.conn
                            .query_row(
                                "SELECT EXISTS (SELECT 1 FROM time_entries
                                 WHERE project_id = ?1 AND deleted_at IS NULL AND invoice_id IS NOT NULL);",
                                params![id],
                                |row| row.get(0),
                            )
                            .map_err(|e| e.to_string())?
                    }
                    #[cfg(not(test))]
                    return Err("database path unavailable for catalog read".into());
                };
                if invoiced {
                    return Err(
                        "This project has invoiced time, so its client can't change. Delete its draft invoice first."
                            .to_string(),
                    );
                }
            }
            proj.client_id = client_id;
        }
        if let Some(name) = patch.name {
            proj.name = name;
        }
        if let Some(color) = patch.color {
            proj.color = color;
        }
        if let Some(desc) = patch.description {
            proj.description = desc;
        }
        let hints_changed = patch
            .ai_hints
            .as_ref()
            .is_some_and(|hints| *hints != proj.ai_hints);
        if let Some(hints) = patch.ai_hints {
            proj.ai_hints = hints;
        }
        if let Some(status) = patch.status {
            proj.status = status;
        }
        if let Some(due) = patch.due_date {
            proj.due_date = due;
        }
        if let Some(bk) = patch.budget_kind {
            proj.budget_kind = bk;
        }
        if let Some(bv) = patch.budget_value {
            proj.budget_value = bv;
        }
        if let Some(bp) = patch.budget_period {
            proj.budget_period = bp;
        }
        if let Some(b) = patch.billable_default {
            proj.billable_default = b;
        }
        if let Some(rate) = patch.hourly_rate {
            proj.hourly_rate = rate;
        }
        proj.updated_at = now;

        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let model = crate::database::entities::catalog::projects::ActiveModel {
                id: sea_orm::Set(proj.id.clone()),
                client_id: sea_orm::Set(proj.client_id.clone()),
                name: sea_orm::Set(proj.name.clone()),
                color: sea_orm::Set(proj.color.clone()),
                description: sea_orm::Set(proj.description.clone()),
                ai_hints: sea_orm::Set(proj.ai_hints.clone()),
                status: sea_orm::Set(proj.status.clone()),
                due_date: sea_orm::Set(proj.due_date.map(|value| value as i64)),
                budget_kind: sea_orm::Set(proj.budget_kind.clone()),
                budget_value: sea_orm::Set(proj.budget_value),
                budget_period: sea_orm::Set(proj.budget_period.clone()),
                billable_default: sea_orm::Set(proj.billable_default as i64),
                hourly_rate: sea_orm::Set(proj.hourly_rate),
                created_at: sea_orm::NotSet,
                updated_at: sea_orm::Set(now as i64),
                deleted_at: sea_orm::NotSet,
            };
            let row = crate::database::repositories::catalog::CatalogRepository::new(&database)
                .update_project(model)
                .map_err(|error| error.to_string())?;
            let _ = hints_changed;
            return Ok(project_from_model(row));
        }

        #[cfg(not(test))]
        return Err("database path unavailable for catalog write".into());
        #[cfg(test)]
        {
            self.conn.execute(
            "UPDATE projects SET client_id = ?1, name = ?2, color = ?3, description = ?4, ai_hints = ?5, status = ?6, due_date = ?7, budget_kind = ?8, budget_value = ?9, budget_period = ?10, billable_default = ?11, hourly_rate = ?12, updated_at = ?13
             WHERE id = ?14;",
            params![
                proj.client_id,
                proj.name,
                proj.color,
                proj.description,
                proj.ai_hints,
                proj.status,
                proj.due_date.map(|v| v as i64),
                proj.budget_kind,
                proj.budget_value,
                proj.budget_period,
                proj.billable_default as i64,
                proj.hourly_rate,
                now as i64,
                id,
            ],
        ).map_err(|e| e.to_string())?;

            if hints_changed {
                if let Some(path) = self.database_path.as_ref() {
                    let _database =
                        crate::database::connect(path).map_err(|error| error.to_string())?;
                } else {
                    #[cfg(test)]
                    crate::projects::sync_hint_rules(
                        &self.conn,
                        id,
                        proj.ai_hints.as_deref(),
                        now,
                    )?;
                    #[cfg(not(test))]
                    return Err("database path unavailable for project hint synchronization".into());
                }
            }
            Ok(proj)
        }
    }

    /// Soft-deletes a project and unlinks its time entries and rules.
    /// Historical time remains intact, but is no longer assigned to a project.
    pub fn delete_project(&mut self, id: &str, now: u64) -> Result<(), String> {
        self.delete_undoable(&[], None, Some(id), now)
    }

    fn get_project(&self, id: &str) -> Result<Project, String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let row = crate::database::repositories::catalog::CatalogRepository::new(&database)
                .project(id)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "project not found".to_string())?;
            return Ok(project_from_model(row));
        }
        #[cfg(not(test))]
        return Err("database path unavailable for catalog read".into());
        #[cfg(test)]
        self.conn.query_row(
            "SELECT id, client_id, name, color, description, ai_hints, status, due_date, budget_kind, budget_value, budget_period, billable_default, hourly_rate, created_at, updated_at, deleted_at
             FROM projects WHERE id = ?1;",
            params![id],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    client_id: row.get(1)?,
                    name: row.get(2)?,
                    color: row.get(3)?,
                    description: row.get(4)?,
                    ai_hints: row.get(5)?,
                    status: row.get(6)?,
                    due_date: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
                    budget_kind: row.get(8)?,
                    budget_value: row.get(9)?,
                    budget_period: row.get(10)?,
                    billable_default: row.get::<_, i64>(11)? != 0,
                    hourly_rate: row.get(12)?,
                    created_at: row.get::<_, i64>(13)? as u64,
                    updated_at: row.get::<_, i64>(14)? as u64,
                    deleted_at: row.get::<_, Option<i64>>(15)?.map(|v| v as u64),
                })
            },
        ).map_err(|e| e.to_string())
    }

    // --- P1: Clients CRUD -----------------------------------------------

    /// Every client that is not deleted, archived ones included: projects
    /// still resolve their client's name, and the views split active from
    /// archived themselves.
    pub fn list_clients(&self) -> Result<Vec<Client>, String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            return crate::database::repositories::catalog::CatalogRepository::new(&database)
                .clients()
                .map(|rows| rows.into_iter().map(client_from_model).collect())
                .map_err(|error| error.to_string());
        }
        #[cfg(test)]
        return self.list_clients_legacy();
        #[cfg(not(test))]
        Err("database path unavailable for catalog read".into())
    }

    #[cfg(test)]
    fn list_clients_legacy(&self) -> Result<Vec<Client>, String> {
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT {CLIENT_COLUMNS} FROM clients WHERE deleted_at IS NULL ORDER BY name ASC;"
            ))
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], client_from_row)
            .map_err(|e| e.to_string())?;

        let mut list = Vec::new();
        for c in rows {
            list.push(c.map_err(|e| e.to_string())?);
        }
        Ok(list)
    }

    pub fn create_client(&mut self, client: NewClient, now: u64) -> Result<Client, String> {
        let name = client.name.trim().to_string();
        if name.is_empty() {
            return Err("A client needs a name".to_string());
        }
        let id = uuid::Uuid::now_v7().to_string();

        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let model = crate::database::entities::catalog::clients::ActiveModel {
                id: sea_orm::Set(id),
                name: sea_orm::Set(name),
                email: sea_orm::Set(client.email),
                address: sea_orm::Set(client.address),
                default_rate: sea_orm::Set(client.default_rate),
                currency: sea_orm::Set(client.currency),
                notes: sea_orm::Set(client.notes),
                archived_at: sea_orm::NotSet,
                created_at: sea_orm::Set(now as i64),
                updated_at: sea_orm::Set(now as i64),
                deleted_at: sea_orm::NotSet,
            };
            let row = crate::database::repositories::catalog::CatalogRepository::new(&database)
                .create_client(model)
                .map_err(|error| error.to_string())?;
            return Ok(client_from_model(row));
        }

        #[cfg(not(test))]
        return Err("database path unavailable for catalog write".into());
        #[cfg(test)]
        {
            self.conn.execute(
            "INSERT INTO clients (id, name, email, address, default_rate, currency, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8);",
            params![id, name, client.email, client.address, client.default_rate, client.currency, client.notes, now as i64],
        ).map_err(|e| e.to_string())?;

            Ok(Client {
                id,
                name,
                email: client.email,
                address: client.address,
                default_rate: client.default_rate,
                currency: client.currency,
                notes: client.notes,
                archived_at: None,
                created_at: now,
                updated_at: now,
                deleted_at: None,
            })
        }
    }

    pub fn update_client(
        &mut self,
        id: &str,
        patch: UpdateClient,
        now: u64,
    ) -> Result<Client, String> {
        let mut client = self.get_client(id)?;
        if let Some(name) = patch.name {
            let name = name.trim().to_string();
            if name.is_empty() {
                return Err("A client needs a name".to_string());
            }
            client.name = name;
        }
        if let Some(email) = patch.email {
            client.email = email;
        }
        if let Some(addr) = patch.address {
            client.address = addr;
        }
        if let Some(rate) = patch.default_rate {
            client.default_rate = rate;
        }
        if let Some(curr) = patch.currency {
            client.currency = curr;
        }
        if let Some(notes) = patch.notes {
            client.notes = notes;
        }
        if let Some(archived) = patch.archived {
            // Archiving twice keeps the first date.
            client.archived_at = match (archived, client.archived_at) {
                (true, Some(at)) => Some(at),
                (true, None) => Some(now),
                (false, _) => None,
            };
        }
        client.updated_at = now;

        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let model = crate::database::entities::catalog::clients::ActiveModel {
                id: sea_orm::Set(client.id.clone()),
                name: sea_orm::Set(client.name),
                email: sea_orm::Set(client.email),
                address: sea_orm::Set(client.address),
                default_rate: sea_orm::Set(client.default_rate),
                currency: sea_orm::Set(client.currency),
                notes: sea_orm::Set(client.notes),
                archived_at: sea_orm::Set(client.archived_at.map(|value| value as i64)),
                created_at: sea_orm::NotSet,
                updated_at: sea_orm::Set(now as i64),
                deleted_at: sea_orm::NotSet,
            };
            let row = crate::database::repositories::catalog::CatalogRepository::new(&database)
                .update_client(model)
                .map_err(|error| error.to_string())?;
            return Ok(client_from_model(row));
        }

        #[cfg(not(test))]
        return Err("database path unavailable for catalog write".into());
        #[cfg(test)]
        {
            self.conn.execute(
            "UPDATE clients SET name = ?1, email = ?2, address = ?3, default_rate = ?4, currency = ?5, notes = ?6, archived_at = ?7, updated_at = ?8
             WHERE id = ?9;",
            params![
                client.name,
                client.email,
                client.address,
                client.default_rate,
                client.currency,
                client.notes,
                client.archived_at.map(|v| v as i64),
                now as i64,
                id,
            ],
        ).map_err(|e| e.to_string())?;

            Ok(client)
        }
    }

    /// Soft-deletes a client and detaches its projects, which keep their
    /// status, time, and rules but lose the client link (so none is left
    /// pointing at a client that no longer lists). A client with invoices
    /// cannot be deleted: those documents bill it, so it can only be archived.
    pub fn delete_client(&mut self, id: &str, now: u64) -> Result<(), String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            crate::database::repositories::catalog::CatalogRepository::new(&database)
                .delete_client(id, now as i64)
                .map(|_| ())
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
        #[cfg(not(test))]
        return Err("database path unavailable for catalog write".into());
        #[cfg(test)]
        {
            let client = self.get_client(id)?;
            let tx = self.conn.transaction().map_err(|e| e.to_string())?;
            let invoiced: bool = tx
                .query_row(
                    "SELECT EXISTS (SELECT 1 FROM invoices WHERE client_id = ?1);",
                    params![id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            if invoiced {
                return Err(format!(
                    "{} has invoices, so it can't be deleted. Archive it instead.",
                    client.name
                ));
            }
            tx.execute(
                "UPDATE projects SET client_id = NULL, updated_at = ?1 WHERE client_id = ?2;",
                params![now as i64, id],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "UPDATE clients SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2;",
                params![now as i64, id],
            )
            .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())
        }
    }

    fn get_client(&self, id: &str) -> Result<Client, String> {
        if let Some(path) = self.database_path.as_ref() {
            let database = crate::database::connect(path).map_err(|error| error.to_string())?;
            let row = crate::database::repositories::catalog::CatalogRepository::new(&database)
                .client(id)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "client not found".to_string())?;
            return Ok(client_from_model(row));
        }
        #[cfg(not(test))]
        return Err("database path unavailable for catalog read".into());
        #[cfg(test)]
        self.conn
            .query_row(
                &format!("SELECT {CLIENT_COLUMNS} FROM clients WHERE id = ?1;"),
                params![id],
                client_from_row,
            )
            .map_err(|e| e.to_string())
    }

    // --- P1: Time Entries -----------------------------------------------
}

fn category_from_model(row: crate::database::entities::catalog::categories::Model) -> Category {
    Category {
        id: row.id,
        name: row.name,
        color: row.color,
        description: row.description,
        ai_prompt: row.ai_prompt,
        billable_default: row.billable_default != 0,
        counts_as_work: row.counts_as_work != 0,
        archived: row.archived != 0,
        sort: row.sort,
        created_at: row.created_at.max(0) as u64,
        updated_at: row.updated_at.max(0) as u64,
        deleted_at: row.deleted_at.map(|value| value.max(0) as u64),
    }
}

fn project_from_model(row: crate::database::entities::catalog::projects::Model) -> Project {
    Project {
        id: row.id,
        client_id: row.client_id,
        name: row.name,
        color: row.color,
        description: row.description,
        ai_hints: row.ai_hints,
        status: row.status,
        due_date: row.due_date.map(|value| value.max(0) as u64),
        budget_kind: row.budget_kind,
        budget_value: row.budget_value,
        budget_period: row.budget_period,
        billable_default: row.billable_default != 0,
        hourly_rate: row.hourly_rate,
        created_at: row.created_at.max(0) as u64,
        updated_at: row.updated_at.max(0) as u64,
        deleted_at: row.deleted_at.map(|value| value.max(0) as u64),
    }
}

fn client_from_model(row: crate::database::entities::catalog::clients::Model) -> Client {
    Client {
        id: row.id,
        name: row.name,
        email: row.email,
        address: row.address,
        default_rate: row.default_rate,
        currency: row.currency,
        notes: row.notes,
        archived_at: row.archived_at.map(|value| value.max(0) as u64),
        created_at: row.created_at.max(0) as u64,
        updated_at: row.updated_at.max(0) as u64,
        deleted_at: row.deleted_at.map(|value| value.max(0) as u64),
    }
}
