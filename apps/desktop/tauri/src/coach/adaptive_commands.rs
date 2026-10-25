use crate::{timers::now_epoch_ms, AppState};
use aevio_core::coach::{
    adaptive::{self, Estimate, Memory, PlanningProposal, Weekly},
    intelligence::{self, Observation, Recommendation},
    interventions::{self, Intervention, InterventionAction},
    policies,
};
use aevio_persistence::coach::service;
use aevio_services::decision::{DecisionOutcome, DecisionRequest, QuestionId};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Insights {
    weekly: Weekly,
    memory: Vec<Memory>,
    estimates: Vec<Estimate>,
    planning: PlanningProposal,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MemoryCommand {
    Save { memory: Memory },
    Remove { id: String },
    Clear,
}
#[tauri::command]
pub fn coach_insights(app: AppHandle, day: String, time_zone: String) -> Result<Insights, String> {
    let state = app.state::<AppState>();
    let _store = state.activity.lock().map_err(|e| e.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    let metadata =
        aevio_persistence::database::repositories::coach::CoachMetadataRepository::new(&database);
    let snapshot = service::day_with_database(&database, &day, &time_zone)?;
    Ok(Insights {
        weekly: aevio_persistence::coach_adaptive::CoachAdaptiveRepository::new(&database)
            .weekly(&day, &time_zone)?,
        memory: metadata.memory()?,
        estimates: aevio_persistence::coach_adaptive::CoachAdaptiveRepository::new(&database)
            .estimates()?,
        planning: adaptive::planning(&snapshot),
    })
}
#[tauri::command]
pub fn coach_memory(app: AppHandle, command: MemoryCommand) -> Result<(), String> {
    {
        let state = app.state::<AppState>();
        let _writer = state.activity.lock().map_err(|e| e.to_string())?;
        let database = aevio_persistence::database::connect(&state.database_path)
            .map_err(|error| error.to_string())?;
        let metadata =
            aevio_persistence::database::repositories::coach::CoachMetadataRepository::new(
                &database,
            );
        match command {
            MemoryCommand::Save { memory } => metadata.save_memory(&memory, now_epoch_ms())?,
            MemoryCommand::Remove { id } => metadata.delete_memory(Some(&id))?,
            MemoryCommand::Clear => metadata.delete_memory(None)?,
        };
    }
    if let Err(error) = app.emit(super::EVENT_COACH_DAY_CHANGED, ()) {
        eprintln!("Coach memory invalidation: {error}");
    }
    Ok(())
}

#[tauri::command]
pub async fn coach_plan_proposal(
    app: AppHandle,
    day: String,
    time_zone: String,
    use_ai: bool,
) -> Result<PlanningProposal, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let snapshot = {
            let state = app.state::<AppState>();
            let _store = state.activity.lock().map_err(|e| e.to_string())?;
            let database = aevio_persistence::database::connect(&state.database_path)
                .map_err(|error| error.to_string())?;
            service::day_with_database(&database, &day, &time_zone)?
        };
        app.state::<super::provider::CoachAi>()
            .plan(&snapshot, use_ai)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn coach_intervention(
    app: AppHandle,
    day: String,
    time_zone: String,
) -> Result<Option<Intervention>, String> {
    let (snapshot, last, focus) = {
        let state = app.state::<AppState>();
        let _store = state.activity.lock().map_err(|e| e.to_string())?;
        let database = aevio_persistence::database::connect(&state.database_path)
            .map_err(|error| error.to_string())?;
        let metadata =
            aevio_persistence::database::repositories::coach::CoachMetadataRepository::new(
                &database,
            );
        (
            service::day_with_database(&database, &day, &time_zone)?,
            metadata.last_intervention()?,
            aevio_persistence::database::repositories::coach::CoachTimeRepository::new(&database)
                .current_focus(now_epoch_ms() as i64)?,
        )
    };
    use chrono::Timelike;
    let now = chrono::Local::now();
    let minute = (now.hour() as u16) * 60 + now.minute() as u16;
    let now_ms = crate::timers::now_epoch_ms();
    if !snapshot.preferences.enabled || policies::quiet(&snapshot.preferences, minute) {
        return Ok(None);
    }
    if let Some(last) = last {
        let cooldown = snapshot.preferences.cooldown_minutes as u64 * 60_000;
        if now_ms.saturating_sub(last) < cooldown {
            return Ok(None);
        }
    }
    // Focus recovery (highest priority among proactive suggestions).
    if let Some(f) = focus {
        if f.state == "interrupted" {
            let request = aevio_services::decision::use_cases::focus_recovery(
                &f.state,
                &f.title,
                &f.next_step,
                false,
            );
            if let Some(false) = decision_gate(
                &app,
                request,
                &QuestionId::from_static("should_resume"),
                None,
            )
            .await
            {
                return Ok(None);
            }
            return Ok(Some(Intervention {
                id: format!("focus_recovery:{}", day),
                kind: "focus_recovery".into(),
                priority: 2,
                title: "Rückkehr zum Fokus".into(),
                message: format!(
                    "Du hast {} unterbrochen. Letzter Schritt: {}",
                    f.title,
                    if f.next_step.is_empty() {
                        "noch kein Schritt festgelegt"
                    } else {
                        &f.next_step
                    }
                ),
                actions: vec![
                    InterventionAction {
                        id: "resume".into(),
                        label: "Fortsetzen".into(),
                        kind: "navigate".into(),
                    },
                    InterventionAction {
                        id: "dismiss".into(),
                        label: "Ignorieren".into(),
                        kind: "dismiss".into(),
                    },
                ],
            }));
        }
    }
    // Daily review eligibility.
    if snapshot.review.outcome == "not_reviewed"
        && minute >= snapshot.preferences.review_minute
        && snapshot.plan.is_some()
    {
        let request = aevio_services::decision::use_cases::daily_review(
            snapshot.review.outcome.as_str(),
            snapshot.plan.is_some(),
        );
        if let Some(false) = decision_gate(
            &app,
            request,
            &QuestionId::from_static("review_action"),
            Some("review"),
        )
        .await
        {
            return Ok(None);
        }
        return Ok(Some(Intervention {
            id: format!("daily_review:{}", day),
            kind: "daily_review".into(),
            priority: 3,
            title: "Tag abschließen".into(),
            message: "Möchtest du deinen Tag kurz abschließen?".into(),
            actions: vec![
                InterventionAction {
                    id: "review".into(),
                    label: "Tagesrückblick".into(),
                    kind: "navigate".into(),
                },
                InterventionAction {
                    id: "dismiss".into(),
                    label: "Später".into(),
                    kind: "dismiss".into(),
                },
            ],
        }));
    }
    // Fall through to existing suggest logic (overcommitment / morning planning).
    Ok(interventions::suggest_content(&snapshot))
}

/// Decision inference is an additive gate. If it is disabled, unavailable, or abstains,
/// Coach keeps its existing deterministic intervention policy and remains usable offline.
async fn decision_gate(
    app: &AppHandle,
    request: DecisionRequest,
    question: &QuestionId,
    expected_choice: Option<&str>,
) -> Option<bool> {
    let router = match crate::commands::configured_decision_router(app) {
        Ok(router) => router,
        Err(error) => {
            eprintln!("Decision gate unavailable; keeping Coach policy: {error}");
            return None;
        }
    };
    let result = match router.decide(&request).await {
        Ok(result) => result,
        Err(error) => {
            eprintln!("Decision gate failed; keeping Coach policy: {error}");
            return None;
        }
    };
    if result.abstention.is_some() {
        return None;
    }
    result.outcomes.iter().find_map(|outcome| {
        if outcome.question() != question {
            return None;
        }
        match outcome {
            DecisionOutcome::Noul { value, .. } => value.as_ref().copied(),
            DecisionOutcome::Choice { value, .. } => {
                expected_choice.map(|expected| value == expected)
            }
            DecisionOutcome::Score { .. } => None,
        }
    })
}

#[tauri::command]
pub fn coach_dismiss_intervention(app: AppHandle, day: String, kind: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let _writer = state.activity.lock().map_err(|e| e.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    aevio_persistence::database::repositories::coach::CoachMetadataRepository::new(&database)
        .audit(&day, &kind, crate::timers::now_epoch_ms())?;
    Ok(())
}

#[tauri::command]
pub fn coach_snooze_intervention(
    app: AppHandle,
    day: String,
    kind: String,
    minutes: u32,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let _writer = state.activity.lock().map_err(|e| e.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    let until = crate::timers::now_epoch_ms() + (minutes as u64 * 60_000);
    aevio_persistence::database::repositories::coach::CoachMetadataRepository::new(&database)
        .audit(&day, &kind, until)?;
    Ok(())
}

#[tauri::command]
pub fn coach_feedback(
    app: AppHandle,
    day: String,
    recommendation: String,
    outcome: String,
) -> Result<(), String> {
    if !matches!(
        outcome.as_str(),
        "helpful" | "not_helpful" | "incorrect" | "dismissed"
    ) {
        return Err("Invalid feedback outcome".into());
    }
    let state = app.state::<AppState>();
    let _writer = state.activity.lock().map_err(|e| e.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    let metadata =
        aevio_persistence::database::repositories::coach::CoachMetadataRepository::new(&database);
    let prefs = metadata.preferences()?;
    if !prefs.personal_intelligence_enabled {
        return Err("Personal Intelligence is disabled".into());
    }
    metadata.audit(
        &day,
        &format!("feedback:{}:{}", outcome, recommendation),
        crate::timers::now_epoch_ms(),
    )?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalIntelligence {
    pub observations: Vec<Observation>,
    pub recommendations: Vec<Recommendation>,
}

#[tauri::command]
pub fn coach_intelligence(
    app: AppHandle,
    day: String,
    time_zone: String,
) -> Result<PersonalIntelligence, String> {
    let state = app.state::<AppState>();
    let _store = state.activity.lock().map_err(|e| e.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    let metadata =
        aevio_persistence::database::repositories::coach::CoachMetadataRepository::new(&database);
    let prefs = metadata.preferences()?;
    if !prefs.personal_intelligence_enabled {
        return Ok(PersonalIntelligence {
            observations: vec![],
            recommendations: vec![],
        });
    }
    let suppressed = metadata.suppressed("suppress:")?;
    let snapshot = service::day_with_database(&database, &day, &time_zone)?;
    let suppressed_obs = metadata.suppressed("suppress_observation:")?;
    let (observations, mut recommendations) = intelligence::summarize(&snapshot);
    let observations: Vec<_> = observations
        .into_iter()
        .filter(|o| !suppressed_obs.contains(&o.id))
        .collect();
    recommendations.retain(|r| !suppressed.contains(&r.id));
    Ok(PersonalIntelligence {
        observations,
        recommendations,
    })
}

#[tauri::command]
pub fn coach_apply_recommendation(
    app: AppHandle,
    day: String,
    recommendation_id: String,
    kind: String,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let _store = state.activity.lock().map_err(|e| e.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    let metadata =
        aevio_persistence::database::repositories::coach::CoachMetadataRepository::new(&database);
    let prefs = metadata.preferences()?;
    if !prefs.personal_intelligence_enabled {
        return Err("Personal Intelligence is disabled".into());
    }
    match kind.as_str() {
        "planning_buffer" => {
            let mut next = prefs.clone();
            next.planning_buffer_minutes =
                next.planning_buffer_minutes.saturating_add(15).min(1440);
            metadata.save_preferences(&next)?;
        }
        _ => return Err("Unsupported recommendation".into()),
    }
    metadata.audit(
        &day,
        &format!("applied:{}", recommendation_id),
        crate::timers::now_epoch_ms(),
    )?;
    Ok(())
}

#[tauri::command]
pub fn coach_suppress_recommendation(
    app: AppHandle,
    day: String,
    recommendation_id: String,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let _store = state.activity.lock().map_err(|e| e.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    let metadata =
        aevio_persistence::database::repositories::coach::CoachMetadataRepository::new(&database);
    metadata.audit(
        &day,
        &format!("suppress:{}", recommendation_id),
        crate::timers::now_epoch_ms(),
    )?;
    Ok(())
}

#[tauri::command]
pub fn coach_suppress_observation(
    app: AppHandle,
    day: String,
    observation_id: String,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let _store = state.activity.lock().map_err(|e| e.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    let metadata =
        aevio_persistence::database::repositories::coach::CoachMetadataRepository::new(&database);
    metadata.audit(
        &day,
        &format!("suppress_observation:{}", observation_id),
        crate::timers::now_epoch_ms(),
    )?;
    Ok(())
}

#[tauri::command]
pub fn coach_reset_personal_intelligence(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let _store = state.activity.lock().map_err(|e| e.to_string())?;
    let database = aevio_persistence::database::connect(&state.database_path)
        .map_err(|error| error.to_string())?;
    let metadata =
        aevio_persistence::database::repositories::coach::CoachMetadataRepository::new(&database);
    metadata.reset_interventions()
}
