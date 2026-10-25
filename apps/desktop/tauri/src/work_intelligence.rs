use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use tauri::AppHandle;
use tauri::Manager;

use aevio_core::entries::models::{Project, Task, TimeEntry};
use aevio_persistence::database::connect;
use aevio_persistence::database::repositories::work_intelligence::{
    NewRelationship, WorkIntelligenceRepository,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRef {
    pub kind: String,
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkContext {
    pub pending_entries: u32,
    pub active_projects: u32,
    pub ai_engine: String,
    pub project_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkAnswer {
    pub answer: String,
    pub mode: String,
    pub range_start: u64,
    pub range_end: u64,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub total_ms: u64,
    pub entries: Vec<TimeEntry>,
    pub tasks: Vec<Task>,
    pub evidence: Vec<String>,
    pub sources: Vec<SourceRef>,
    pub context: WorkContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkRelationship {
    pub id: String,
    pub from_type: String,
    pub from_id: String,
    pub relation: String,
    pub to_type: String,
    pub to_id: String,
    pub provenance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub kind: String,
    pub id: String,
    pub label: String,
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphTraversal {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSuggestion {
    pub key: String,
    pub name: String,
    pub ms: u64,
    pub evidence: Vec<String>,
}

fn day_bounds(now_ms: u64, offset_days: i64) -> (u64, u64) {
    let day_ms = 86_400_000u64;
    let day_index = (now_ms / day_ms) as i64 + offset_days;
    let start = (day_index * day_ms as i64) as u64;
    (start, start + day_ms)
}

pub fn ask(app: &AppHandle, question: &str) -> Result<WorkAnswer, String> {
    let lower = question.to_lowercase();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let (range_start, range_end) = if lower.contains("yesterday") || lower.contains("gestern") {
        day_bounds(now_ms, -1)
    } else if lower.contains("today") || lower.contains("heute") {
        day_bounds(now_ms, 0)
    } else if lower.contains("this week") || lower.contains("diese woche") {
        (now_ms.saturating_sub(7 * 86_400_000), now_ms)
    } else if lower.contains("last week") || lower.contains("letzte woche") {
        let end = now_ms.saturating_sub(7 * 86_400_000);
        (end.saturating_sub(7 * 86_400_000), end)
    } else {
        (now_ms.saturating_sub(7 * 86_400_000), now_ms)
    };

    let state = app.state::<crate::AppState>();
    let database = connect(&state.database_path).map_err(|e| e.to_string())?;
    let store = state.activity.lock().map_err(|e| e.to_string())?;

    let projects = store.list_projects()?;
    let project_names: Vec<String> = projects
        .iter()
        .filter(|p| p.deleted_at.is_none())
        .map(|p| p.name.clone())
        .take(20)
        .collect();
    let active_projects = project_names.len() as u32;
    let pending_entries = WorkIntelligenceRepository::new(&database)
        .pending_entries()
        .map_err(|e| e.to_string())?;

    let mut resolved: Option<Project> = None;
    for p in &projects {
        if p.deleted_at.is_some() || p.status == "archived" {
            continue;
        }
        if lower.contains(&p.name.to_lowercase()) {
            match &resolved {
                Some(r) if r.name.len() >= p.name.len() => {}
                _ => resolved = Some(p.clone()),
            }
        }
    }

    let mut entries = store.list_time_entries(range_start, range_end)?;
    if let Some(p) = &resolved {
        entries.retain(|e| e.project_id.as_deref() == Some(p.id.as_str()));
    }

    let tasks = if let Some(p) = &resolved {
        store.list_tasks(Some(&p.id))?
    } else {
        store
            .list_tasks(None)?
            .into_iter()
            .filter(|t| t.status != "done")
            .collect()
    };

    let total_ms: u64 = entries
        .iter()
        .map(|e| e.ended_at.saturating_sub(e.started_at))
        .sum();

    let mut evidence: Vec<String> = Vec::new();
    let mut sources: Vec<SourceRef> = Vec::new();
    for e in &entries {
        evidence.push(format!(
            "time_entry:{}:{}ms",
            e.id,
            e.ended_at.saturating_sub(e.started_at)
        ));
        sources.push(SourceRef {
            kind: "time_entry".into(),
            id: e.id.clone(),
            label: e.description.clone(),
        });
    }
    for t in &tasks {
        evidence.push(format!("task:{}:{}", t.id, t.title));
        sources.push(SourceRef {
            kind: "task".into(),
            id: t.id.clone(),
            label: t.title.clone(),
        });
    }
    if let Some(p) = &resolved {
        evidence.push(format!("project:{}", p.id));
        sources.push(SourceRef {
            kind: "project".into(),
            id: p.id.clone(),
            label: p.name.clone(),
        });
    }

    let project_id = resolved.as_ref().map(|p| p.id.clone());
    let project_name = resolved.as_ref().map(|p| p.name.clone());

    drop(store);

    let ai_engine = app.state::<crate::ai::AiRuntime>().status().engine.clone();

    let answer = if let Some(p) = project_name.as_deref() {
        format!(
            "Im gewählten Zeitraum wurden {} Zeitbuchungen für Projekt \"{}\" mit insgesamt {} ms erfasst. {} Aufgaben zugeordnet.",
            entries.len(),
            p,
            total_ms,
            tasks.len()
        )
    } else {
        format!(
            "Im gewählten Zeitraum wurden {} Zeitbuchungen mit insgesamt {} ms erfasst. {} offene Aufgaben.",
            entries.len(),
            total_ms,
            tasks.len()
        )
    };

    let settings = app.state::<crate::AppState>().settings_snapshot();
    let (final_answer, mode) = {
        let ctx = serde_json::json!({
            "question": question,
            "rangeStart": range_start,
            "rangeEnd": range_end,
            "projectId": project_id,
            "projectName": project_name,
            "totalMs": total_ms,
            "entryCount": entries.len(),
            "taskCount": tasks.len(),
        });
        if !settings.ai_provider.is_empty() {
            match crate::ai::provider::answer(
                question,
                &ctx,
                &settings.ai_provider,
                &settings.ai_base_url,
                &settings.ai_model,
                &settings.ai_api_key,
            ) {
                Ok(r) => (r.answer, "external-ai".to_string()),
                Err(_) => match crate::ai::agent::answer(question, &ctx) {
                    Ok(r) => (r.answer, "local-ai".to_string()),
                    Err(_) => (answer, "local-read-only".to_string()),
                },
            }
        } else {
            match crate::ai::agent::answer(question, &ctx) {
                Ok(r) => (r.answer, "local-ai".to_string()),
                Err(_) => (answer, "local-read-only".to_string()),
            }
        }
    };

    Ok(WorkAnswer {
        answer: final_answer,
        mode,
        range_start,
        range_end,
        project_id,
        project_name,
        total_ms,
        entries,
        tasks,
        evidence,
        sources,
        context: WorkContext {
            pending_entries,
            active_projects,
            ai_engine,
            project_names,
        },
    })
}

pub fn list_relationships(app: &AppHandle) -> Result<Vec<WorkRelationship>, String> {
    let state = app.state::<crate::AppState>();
    let database = connect(&state.database_path).map_err(|e| e.to_string())?;
    WorkIntelligenceRepository::new(&database)
        .relationships()
        .map(|rows| {
            rows.into_iter()
                .map(|row| WorkRelationship {
                    id: row.id,
                    from_type: row.from_type,
                    from_id: row.from_id,
                    relation: row.relation,
                    to_type: row.to_type,
                    to_id: row.to_id,
                    provenance: row.provenance,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

pub fn add_relationship(
    app: &AppHandle,
    from_type: &str,
    from_id: &str,
    relation: &str,
    to_type: &str,
    to_id: &str,
    provenance: &str,
) -> Result<WorkRelationship, String> {
    // validation of entity existence
    let state = app.state::<crate::AppState>();
    let database = connect(&state.database_path).map_err(|e| e.to_string())?;
    let repository = WorkIntelligenceRepository::new(&database);
    if !repository
        .entity_exists(from_type, from_id)
        .map_err(|e| e.to_string())?
        || !repository
            .entity_exists(to_type, to_id)
            .map_err(|e| e.to_string())?
    {
        return Err("invalid or deleted entity reference".to_string());
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    repository
        .add_relationship(NewRelationship {
            from_type,
            from_id,
            relation,
            to_type,
            to_id,
            provenance,
            created_at: now,
        })
        .map(|row| WorkRelationship {
            id: row.id,
            from_type: row.from_type,
            from_id: row.from_id,
            relation: row.relation,
            to_type: row.to_type,
            to_id: row.to_id,
            provenance: row.provenance,
        })
        .map_err(|e| e.to_string())
}

pub fn traverse(
    app: &AppHandle,
    start_type: &str,
    start_id: &str,
    max_depth: u32,
    max_results: u32,
) -> Result<GraphTraversal, String> {
    let state = app.state::<crate::AppState>();
    let database = connect(&state.database_path).map_err(|e| e.to_string())?;
    let repository = WorkIntelligenceRepository::new(&database);
    let max_depth = max_depth.clamp(1, 5);
    let max_results = max_results.clamp(1, 500);

    let mut visited: HashSet<String> = HashSet::new();
    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut queue: VecDeque<(String, String, u32)> = VecDeque::new();
    queue.push_back((start_type.to_string(), start_id.to_string(), 0));
    visited.insert(format!("{}:{}", start_type, start_id));

    while let Some((t, id, depth)) = queue.pop_front() {
        if nodes.len() >= max_results as usize {
            return Ok(GraphTraversal {
                nodes,
                edges,
                truncated: true,
            });
        }
        nodes.push(GraphNode {
            kind: t.clone(),
            id: id.clone(),
            label: repository.label(&t, &id).map_err(|e| e.to_string())?,
            depth,
        });
        if depth >= max_depth {
            continue;
        }
        // neighbors from work_relationships
        for neighbor in repository.neighbors(&t, &id).map_err(|e| e.to_string())? {
            let (ft, fid, rel, tt, tid) = (
                neighbor.from_type,
                neighbor.from_id,
                neighbor.relation,
                neighbor.to_type,
                neighbor.to_id,
            );
            let (nt, nid) = if ft == t && fid == id {
                (tt, tid)
            } else {
                (ft, fid)
            };
            let key = format!("{}:{}", nt, nid);
            edges.push(GraphEdge {
                from: format!("{}:{}", t, id),
                to: key.clone(),
                relation: rel,
            });
            if !visited.contains(&key) {
                visited.insert(key.clone());
                queue.push_back((nt, nid, depth + 1));
            }
        }
        // FK-based neighbors
        let mut fk_neighbors: Vec<(String, String, &str)> = Vec::new();
        match t.as_str() {
            "project" => {
                for (kind, child_id) in repository
                    .project_children(&id)
                    .map_err(|e| e.to_string())?
                {
                    let relation = if kind == "task" {
                        "has_task"
                    } else {
                        "has_time_entry"
                    };
                    fk_neighbors.push((kind, child_id, relation));
                }
            }
            "task" => {
                if let Some((kind, parent_id)) =
                    repository.parent(&t, &id).map_err(|e| e.to_string())?
                {
                    fk_neighbors.push((kind, parent_id, "belongs_to"));
                }
            }
            "time_entry" => {
                if let Some((kind, parent_id)) =
                    repository.parent(&t, &id).map_err(|e| e.to_string())?
                {
                    fk_neighbors.push((kind, parent_id, "belongs_to"));
                }
            }
            _ => {}
        }
        for (nt, nid, rel) in fk_neighbors {
            let key = format!("{}:{}", nt, nid);
            edges.push(GraphEdge {
                from: format!("{}:{}", t, id),
                to: key.clone(),
                relation: rel.to_string(),
            });
            if !visited.contains(&key) {
                visited.insert(key);
                queue.push_back((nt, nid, depth + 1));
            }
        }
    }

    let truncated = nodes.len() >= max_results as usize;
    Ok(GraphTraversal {
        nodes,
        edges,
        truncated,
    })
}

pub fn project_suggestions(
    app: &AppHandle,
    since_ms: u64,
) -> Result<Vec<ProjectSuggestion>, String> {
    let state = app.state::<crate::AppState>();
    let database = connect(&state.database_path).map_err(|e| e.to_string())?;
    let discovered = aevio_persistence::projects::discover_with_database(&database, since_ms)?;
    Ok(discovered
        .into_iter()
        .map(|d| ProjectSuggestion {
            key: d.key,
            name: d.name,
            ms: d.ms,
            evidence: d.evidence,
        })
        .collect())
}
