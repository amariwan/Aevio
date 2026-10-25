//! Staffing and freelancer operations.
//!
//! This module owns the durable staffing model.  Reporting is intentionally
//! returned as an aggregate snapshot so the UI never reconstructs business
//! rules from raw SQLite rows.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use aevio_core::timers::now_epoch_ms;
use aevio_persistence::database::entities::staffing::{
    assignments, availability, contracts, freelancers, pipeline, requests, skills,
};
use aevio_persistence::database::repositories::staffing::StaffingRepository;
use sea_orm::{ActiveValue::Set, DatabaseConnection};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Freelancer {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub status: String,
    pub location: Option<String>,
    pub timezone: String,
    pub seniority: Option<String>,
    pub bio: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreelancerInput {
    pub id: Option<String>,
    pub name: String,
    pub email: Option<String>,
    pub status: String,
    pub location: Option<String>,
    pub timezone: String,
    pub seniority: Option<String>,
    pub bio: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffingSkill {
    pub id: String,
    pub name: String,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Availability {
    pub id: String,
    pub freelancer_id: String,
    pub start_date: String,
    pub end_date: String,
    pub available_minutes: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffingRequest {
    pub id: String,
    pub project_id: Option<String>,
    pub role: String,
    pub skills: Vec<String>,
    pub start_date: String,
    pub end_date: String,
    pub requested_minutes: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Assignment {
    pub id: String,
    pub freelancer_id: String,
    pub project_id: Option<String>,
    pub request_id: Option<String>,
    pub role: String,
    pub start_date: String,
    pub end_date: String,
    pub planned_minutes: i64,
    pub buy_rate_cents: i64,
    pub sell_rate_cents: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FreelancerContract {
    pub id: String,
    pub freelancer_id: String,
    pub kind: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub notice_days: i64,
    pub status: String,
    pub document_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineEntry {
    pub id: String,
    pub freelancer_id: String,
    pub stage: String,
    pub next_action_at: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffingReport {
    pub active_freelancers: i64,
    pub available_minutes: i64,
    pub open_requests: i64,
    pub active_assignments: i64,
    pub planned_minutes: i64,
    pub forecast_revenue_cents: i64,
    pub forecast_cost_cents: i64,
    pub forecast_margin_cents: i64,
    pub pipeline_by_stage: Vec<StageCount>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StageCount {
    pub stage: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffingSnapshot {
    pub freelancers: Vec<Freelancer>,
    pub skills: Vec<StaffingSkill>,
    pub availability: Vec<Availability>,
    pub requests: Vec<StaffingRequest>,
    pub assignments: Vec<Assignment>,
    pub contracts: Vec<FreelancerContract>,
    pub pipeline: Vec<PipelineEntry>,
    pub report: StaffingReport,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffingMatch {
    pub freelancer_id: String,
    pub score: f64,
    pub matched_skills: Vec<String>,
    pub available_minutes: i64,
}

fn freelancer_from_model(row: freelancers::Model) -> Freelancer {
    Freelancer {
        id: row.id,
        name: row.name,
        email: row.email,
        status: row.status,
        location: row.location,
        timezone: row.timezone,
        seniority: row.seniority,
        bio: row.bio,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

pub fn snapshot(db: &DatabaseConnection) -> Result<StaffingSnapshot, String> {
    let freelancers = list_freelancers(db)?;
    let skills = list_skills(db)?;
    let availability = list_availability(db)?;
    let requests = list_requests(db)?;
    let assignments = list_assignments(db)?;
    let contracts = list_contracts(db)?;
    let pipeline = list_pipeline(db)?;
    let report = report(db)?;
    Ok(StaffingSnapshot {
        freelancers,
        skills,
        availability,
        requests,
        assignments,
        contracts,
        pipeline,
        report,
    })
}

pub fn save_freelancer(
    db: &DatabaseConnection,
    input: FreelancerInput,
) -> Result<Freelancer, String> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err("Freelancer name is required".into());
    }
    let now = now_epoch_ms() as i64;
    let id = input.id.unwrap_or_else(|| Uuid::now_v7().to_string());
    StaffingRepository::new(db)
        .upsert_freelancer(freelancers::ActiveModel {
            id: Set(id.clone()),
            name: Set(name.to_owned()),
            email: Set(input.email),
            status: Set(input.status),
            location: Set(input.location),
            timezone: Set(input.timezone),
            seniority: Set(input.seniority),
            bio: Set(input.bio),
            created_at: Set(now),
            updated_at: Set(now),
        })
        .map_err(|e| e.to_string())?;
    get_freelancer(db, &id)
}

pub fn list_freelancers(db: &DatabaseConnection) -> Result<Vec<Freelancer>, String> {
    StaffingRepository::new(db)
        .freelancers()
        .map(|rows| rows.into_iter().map(freelancer_from_model).collect())
        .map_err(|e| e.to_string())
}

fn get_freelancer(db: &DatabaseConnection, id: &str) -> Result<Freelancer, String> {
    StaffingRepository::new(db)
        .freelancer(id)
        .map_err(|e| e.to_string())?
        .map(freelancer_from_model)
        .ok_or_else(|| "freelancer not found".into())
}

pub fn list_skills(db: &DatabaseConnection) -> Result<Vec<StaffingSkill>, String> {
    StaffingRepository::new(db)
        .skills()
        .map(|rows| {
            rows.into_iter()
                .map(|row| StaffingSkill {
                    id: row.id,
                    name: row.name,
                    category: row.category,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

pub fn set_freelancer_skills(
    db: &DatabaseConnection,
    freelancer_id: &str,
    skills: &[String],
) -> Result<(), String> {
    StaffingRepository::new(db)
        .replace_skills(freelancer_id, skills)
        .map_err(|e| e.to_string())
}

pub fn match_request(
    db: &DatabaseConnection,
    request_id: &str,
) -> Result<Vec<StaffingMatch>, String> {
    let (request, people, skills, links, availability) = StaffingRepository::new(db)
        .match_data(request_id)
        .map_err(|e| e.to_string())?;
    let required: Vec<String> = serde_json::from_str(&request.skills_json).unwrap_or_default();
    let mut matches = people
        .into_iter()
        .map(|person| {
            let owned: Vec<&str> = links
                .iter()
                .filter(|link| link.freelancer_id == person.id)
                .filter_map(|link| {
                    skills
                        .iter()
                        .find(|skill| skill.id == link.skill_id)
                        .map(|skill| skill.name.as_str())
                })
                .collect();
            let matched_skills = required
                .iter()
                .filter(|required| {
                    owned
                        .iter()
                        .any(|owned| owned.eq_ignore_ascii_case(required))
                })
                .cloned()
                .collect::<Vec<_>>();
            let score = if required.is_empty() {
                0.0
            } else {
                matched_skills.len() as f64 / required.len() as f64
            };
            let available_minutes = availability
                .iter()
                .filter(|row| row.freelancer_id == person.id)
                .map(|row| row.available_minutes)
                .sum();
            StaffingMatch {
                freelancer_id: person.id,
                score,
                matched_skills,
                available_minutes,
            }
        })
        .collect::<Vec<_>>();
    matches.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.available_minutes.cmp(&a.available_minutes))
    });
    Ok(matches)
}

pub fn list_availability(db: &DatabaseConnection) -> Result<Vec<Availability>, String> {
    StaffingRepository::new(db)
        .availability()
        .map(|rows| {
            rows.into_iter()
                .map(|row| Availability {
                    id: row.id,
                    freelancer_id: row.freelancer_id,
                    start_date: row.start_date,
                    end_date: row.end_date,
                    available_minutes: row.available_minutes,
                    status: row.status,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

pub fn list_requests(db: &DatabaseConnection) -> Result<Vec<StaffingRequest>, String> {
    StaffingRepository::new(db)
        .requests()
        .map(|rows| {
            rows.into_iter()
                .map(|row| StaffingRequest {
                    id: row.id,
                    project_id: row.project_id,
                    role: row.role,
                    skills: serde_json::from_str(&row.skills_json).unwrap_or_default(),
                    start_date: row.start_date,
                    end_date: row.end_date,
                    requested_minutes: row.requested_minutes,
                    status: row.status,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

pub fn list_assignments(db: &DatabaseConnection) -> Result<Vec<Assignment>, String> {
    StaffingRepository::new(db)
        .assignments()
        .map(|rows| {
            rows.into_iter()
                .map(|row| Assignment {
                    id: row.id,
                    freelancer_id: row.freelancer_id,
                    project_id: row.project_id,
                    request_id: row.request_id,
                    role: row.role,
                    start_date: row.start_date,
                    end_date: row.end_date,
                    planned_minutes: row.planned_minutes,
                    buy_rate_cents: row.buy_rate_cents,
                    sell_rate_cents: row.sell_rate_cents,
                    status: row.status,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

pub fn list_contracts(db: &DatabaseConnection) -> Result<Vec<FreelancerContract>, String> {
    StaffingRepository::new(db)
        .contracts()
        .map(|rows| {
            rows.into_iter()
                .map(|row| FreelancerContract {
                    id: row.id,
                    freelancer_id: row.freelancer_id,
                    kind: row.kind,
                    start_date: row.start_date,
                    end_date: row.end_date,
                    notice_days: row.notice_days,
                    status: row.status,
                    document_ref: row.document_ref,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

pub fn list_pipeline(db: &DatabaseConnection) -> Result<Vec<PipelineEntry>, String> {
    StaffingRepository::new(db)
        .pipeline()
        .map(|rows| {
            rows.into_iter()
                .map(|row| PipelineEntry {
                    id: row.id,
                    freelancer_id: row.freelancer_id,
                    stage: row.stage,
                    next_action_at: row.next_action_at,
                    notes: row.notes,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

pub fn report(db: &DatabaseConnection) -> Result<StaffingReport, String> {
    let repository = StaffingRepository::new(db);
    let freelancers = repository.freelancers().map_err(|e| e.to_string())?;
    let availability = repository.availability().map_err(|e| e.to_string())?;
    let requests = repository.requests().map_err(|e| e.to_string())?;
    let assignments = repository.assignments().map_err(|e| e.to_string())?;
    let pipeline = repository.pipeline().map_err(|e| e.to_string())?;
    let available_minutes = availability
        .iter()
        .filter(|row| row.status == "available")
        .map(|row| row.available_minutes)
        .sum();
    let active_assignments: Vec<_> = assignments
        .iter()
        .filter(|row| matches!(row.status.as_str(), "planned" | "active"))
        .collect();
    let planned_minutes = active_assignments
        .iter()
        .map(|row| row.planned_minutes)
        .sum();
    let revenue: i64 = active_assignments
        .iter()
        .map(|row| row.sell_rate_cents * row.planned_minutes / 60)
        .sum();
    let cost: i64 = active_assignments
        .iter()
        .map(|row| row.buy_rate_cents * row.planned_minutes / 60)
        .sum();
    let mut stage_counts = std::collections::BTreeMap::<String, i64>::new();
    for row in pipeline {
        *stage_counts.entry(row.stage).or_default() += 1;
    }
    let pipeline_by_stage = stage_counts
        .into_iter()
        .map(|(stage, count)| StageCount { stage, count })
        .collect();
    Ok(StaffingReport {
        active_freelancers: freelancers
            .iter()
            .filter(|row| row.status == "active")
            .count() as i64,
        available_minutes,
        open_requests: requests
            .iter()
            .filter(|row| matches!(row.status.as_str(), "open" | "matching"))
            .count() as i64,
        active_assignments: active_assignments.len() as i64,
        planned_minutes,
        forecast_revenue_cents: revenue,
        forecast_cost_cents: cost,
        forecast_margin_cents: revenue - cost,
        pipeline_by_stage,
    })
}

pub fn save_json_record(
    db: &DatabaseConnection,
    kind: &str,
    payload: &serde_json::Value,
) -> Result<(), String> {
    let id = payload
        .get("id")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .unwrap_or("");
    let id = if id.is_empty() {
        Uuid::now_v7().to_string()
    } else {
        id.to_string()
    };
    let now = now_epoch_ms() as i64;
    match kind {
        "skill" => StaffingRepository::new(db)
            .upsert_skill(skills::ActiveModel {
                id: Set(id),
                name: Set(payload
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .into()),
                category: Set(payload
                    .get("category")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)),
            })
            .map_err(|e| e.to_string())?,
        "availability" => StaffingRepository::new(db)
            .upsert_availability(availability::ActiveModel {
                id: Set(id),
                freelancer_id: Set(payload["freelancerId"].as_str().unwrap_or("").into()),
                start_date: Set(payload["startDate"].as_str().unwrap_or("").into()),
                end_date: Set(payload["endDate"].as_str().unwrap_or("").into()),
                available_minutes: Set(payload["availableMinutes"].as_i64().unwrap_or(0)),
                status: Set(payload["status"].as_str().unwrap_or("available").into()),
            })
            .map_err(|e| e.to_string())?,
        "request" => StaffingRepository::new(db)
            .upsert_request(requests::ActiveModel {
                id: Set(id),
                project_id: Set(payload["projectId"].as_str().map(str::to_owned)),
                role: Set(payload["role"].as_str().unwrap_or("").into()),
                skills_json: Set(payload
                    .get("skills")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "[]".into())),
                start_date: Set(payload["startDate"].as_str().unwrap_or("").into()),
                end_date: Set(payload["endDate"].as_str().unwrap_or("").into()),
                requested_minutes: Set(payload["requestedMinutes"].as_i64().unwrap_or(0)),
                status: Set(payload["status"].as_str().unwrap_or("open").into()),
                created_at: Set(now),
                updated_at: Set(now),
            })
            .map_err(|e| e.to_string())?,
        "pipeline" => StaffingRepository::new(db)
            .upsert_pipeline(pipeline::ActiveModel {
                id: Set(id),
                freelancer_id: Set(payload["freelancerId"].as_str().unwrap_or("").into()),
                stage: Set(payload["stage"].as_str().unwrap_or("lead").into()),
                next_action_at: Set(payload["nextActionAt"].as_i64()),
                notes: Set(payload["notes"].as_str().map(str::to_owned)),
            })
            .map_err(|e| e.to_string())?,
        "assignment" => StaffingRepository::new(db)
            .upsert_assignment(assignments::ActiveModel {
                id: Set(id),
                freelancer_id: Set(payload["freelancerId"].as_str().unwrap_or("").into()),
                project_id: Set(payload["projectId"].as_str().map(str::to_owned)),
                request_id: Set(payload["requestId"].as_str().map(str::to_owned)),
                role: Set(payload["role"].as_str().unwrap_or("").into()),
                start_date: Set(payload["startDate"].as_str().unwrap_or("").into()),
                end_date: Set(payload["endDate"].as_str().unwrap_or("").into()),
                planned_minutes: Set(payload["plannedMinutes"].as_i64().unwrap_or(0)),
                buy_rate_cents: Set(payload["buyRateCents"].as_i64().unwrap_or(0)),
                sell_rate_cents: Set(payload["sellRateCents"].as_i64().unwrap_or(0)),
                status: Set(payload["status"].as_str().unwrap_or("planned").into()),
            })
            .map_err(|e| e.to_string())?,
        "contract" => StaffingRepository::new(db)
            .upsert_contract(contracts::ActiveModel {
                id: Set(id),
                freelancer_id: Set(payload["freelancerId"].as_str().unwrap_or("").into()),
                kind: Set(payload["kind"].as_str().unwrap_or("contractor").into()),
                start_date: Set(payload["startDate"].as_str().unwrap_or("").into()),
                end_date: Set(payload["endDate"].as_str().map(str::to_owned)),
                notice_days: Set(payload["noticeDays"].as_i64().unwrap_or(0)),
                status: Set(payload["status"].as_str().unwrap_or("draft").into()),
                document_ref: Set(payload["documentRef"].as_str().map(str::to_owned)),
            })
            .map_err(|e| e.to_string())?,
        "freelancerSkills" => {
            let skills: Vec<String> = payload
                .get("skills")
                .and_then(|value| serde_json::from_value::<Vec<String>>(value.clone()).ok())
                .unwrap_or_default();
            set_freelancer_skills(db, payload["freelancerId"].as_str().unwrap_or(""), &skills)?;
        }
        _ => return Err(format!("Unsupported staffing record: {kind}")),
    };
    Ok(())
}

#[cfg(test)]
#[path = "staffing/tests.rs"]
mod tests;
