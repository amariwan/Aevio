//! Bounded question builders used by Coach integrations.
//!
//! These functions only construct contracts. Coach remains responsible for
//! eligibility, priority, scheduling, snooze and execution.

use super::{
    DecisionContext, DecisionId, DecisionQuestion, DecisionRequest, DecisionType, QuestionId,
};

pub fn focus_recovery(
    session_status: &str,
    title: &str,
    next_step: &str,
    snoozed: bool,
) -> DecisionRequest {
    DecisionRequest {
        id: DecisionId::FocusRecovery,
        context: DecisionContext::bounded(
            format!("Interrupted focus session: {title}. Next step: {next_step}"),
            vec![
                ("sessionStatus".into(), session_status.into()),
                ("snoozed".into(), snoozed.to_string()),
            ],
        ),
        questions: vec![DecisionQuestion {
            id: QuestionId::from_static("should_resume"),
            decision_type: DecisionType::Noul,
            options: vec![],
            score_range: None,
            instructions: "Should the interrupted focus session be offered for resumption?".into(),
        }],
    }
}

pub fn daily_review(review_outcome: &str, plan_available: bool) -> DecisionRequest {
    DecisionRequest {
        id: DecisionId::DailyReview,
        context: DecisionContext::bounded(
            "Daily review eligibility",
            vec![
                ("reviewOutcome".into(), review_outcome.into()),
                ("planAvailable".into(), plan_available.to_string()),
                ("reviewRecommendation".into(), "review".into()),
            ],
        ),
        questions: vec![DecisionQuestion {
            id: QuestionId::from_static("review_action"),
            decision_type: DecisionType::Choice,
            options: vec!["review".into(), "skip".into()],
            score_range: None,
            instructions: "Which existing review action should be highlighted?".into(),
        }],
    }
}

pub fn task_prioritization(options: Vec<String>, recommended: Option<String>) -> DecisionRequest {
    DecisionRequest {
        id: DecisionId::TaskPrioritization,
        context: DecisionContext::bounded(
            "Choose among the explicitly supplied existing tasks",
            vec![("topTask".into(), recommended.unwrap_or_default())],
        ),
        questions: vec![DecisionQuestion {
            id: QuestionId::from_static("selected_task"),
            decision_type: DecisionType::Choice,
            options,
            score_range: None,
            instructions:
                "Which existing task is most suitable in the current context? Do not invent a task."
                    .into(),
        }],
    }
}

fn choice(
    id: DecisionId,
    question_id: &'static str,
    instructions: &'static str,
    summary: impl Into<String>,
    options: Vec<String>,
    recommended: Option<String>,
    facts: Vec<(String, String)>,
) -> DecisionRequest {
    let mut facts = facts;
    facts.push(("recommended".into(), recommended.unwrap_or_default()));
    DecisionRequest {
        id,
        context: DecisionContext::bounded(summary, facts),
        questions: vec![DecisionQuestion {
            id: QuestionId::from_static(question_id),
            decision_type: DecisionType::Choice,
            options,
            score_range: None,
            instructions: instructions.into(),
        }],
    }
}

fn score(
    id: DecisionId,
    question_id: &'static str,
    instructions: &'static str,
    summary: impl Into<String>,
    facts: Vec<(String, String)>,
) -> DecisionRequest {
    DecisionRequest {
        id,
        context: DecisionContext::bounded(summary, facts),
        questions: vec![DecisionQuestion {
            id: QuestionId::from_static(question_id),
            decision_type: DecisionType::Score,
            options: vec![],
            score_range: Some(super::ScoreRange { min: 0.0, max: 1.0 }),
            instructions: instructions.into(),
        }],
    }
}

pub fn interruption_risk(in_focus: bool, meeting_soon: bool, work_minutes: u32) -> DecisionRequest {
    score(
        DecisionId::InterruptionRisk,
        "interruption_risk",
        "Score the risk that an interruption would disrupt the current work context from 0 to 1.",
        "Bounded interruption risk context",
        vec![
            ("inFocus".into(), in_focus.to_string()),
            ("meetingSoon".into(), meeting_soon.to_string()),
            ("workMinutes".into(), work_minutes.to_string()),
        ],
    )
}

pub fn meeting_recovery(options: Vec<String>, recommended: Option<String>) -> DecisionRequest {
    choice(
        DecisionId::MeetingRecovery,
        "recovery_task",
        "Choose the best existing task for recovering after a meeting. Do not invent a task.",
        "Choose a bounded post-meeting recovery task",
        options,
        recommended,
        vec![],
    )
}

pub fn energy_task_selection(
    options: Vec<String>,
    recommended: Option<String>,
    energy_level: u8,
) -> DecisionRequest {
    choice(
        DecisionId::EnergyTaskSelection,
        "energy_task",
        "Choose an existing task that fits the current energy level. Do not invent a task.",
        "Choose a bounded energy-compatible task",
        options,
        recommended,
        vec![("energyLevel".into(), energy_level.min(100).to_string())],
    )
}

pub fn deadline_risk(
    task_id: &str,
    remaining_minutes: u32,
    estimate_minutes: u32,
    due_soon: bool,
) -> DecisionRequest {
    score(
        DecisionId::DeadlineRisk,
        "deadline_risk",
        "Score the risk that the existing task misses its deadline from 0 to 1.",
        format!("Deadline risk for task {task_id}"),
        vec![
            ("taskId".into(), task_id.into()),
            ("remainingMinutes".into(), remaining_minutes.to_string()),
            ("estimateMinutes".into(), estimate_minutes.to_string()),
            ("dueSoon".into(), due_soon.to_string()),
        ],
    )
}

pub fn scope_creep(
    planned_task: &str,
    observed_context: &str,
    minutes_off_task: u32,
) -> DecisionRequest {
    DecisionRequest {
        id: DecisionId::ScopeCreep,
        context: DecisionContext::bounded(
            format!("Scope check for planned task {planned_task}"),
            vec![
                ("plannedTask".into(), planned_task.into()),
                ("observedContext".into(), observed_context.into()),
                ("minutesOffTask".into(), minutes_off_task.to_string()),
                ("offPlan".into(), (minutes_off_task >= 10).to_string()),
            ],
        ),
        questions: vec![DecisionQuestion {
            id: QuestionId::from_static("should_flag_scope_creep"),
            decision_type: DecisionType::Noul,
            options: vec![],
            score_range: None,
            instructions: "Should a possible scope deviation be shown for human review?".into(),
        }],
    }
}

pub fn focus_start(options: Vec<String>, recommended: Option<String>) -> DecisionRequest {
    choice(
        DecisionId::FocusStart,
        "focus_task",
        "Choose the best existing task to start a focus session. Do not invent or mutate a task.",
        "Choose a bounded focus-start task",
        options,
        recommended,
        vec![],
    )
}

pub fn invoice_anomaly(invoice_id: &str, anomaly_score: Option<f64>) -> DecisionRequest {
    score(
        DecisionId::InvoiceAnomaly,
        "invoice_anomaly",
        "Score whether the existing invoice needs human review from 0 to 1.",
        format!("Invoice review for {invoice_id}"),
        vec![
            ("invoiceId".into(), invoice_id.into()),
            (
                "anomalyScore".into(),
                anomaly_score.map_or_else(String::new, |score| score.to_string()),
            ),
        ],
    )
}

pub fn project_health(project_id: &str, health_score: Option<f64>) -> DecisionRequest {
    score(
        DecisionId::ProjectHealth,
        "project_health",
        "Score the current health risk of the existing project from 0 to 1.",
        format!("Project health for {project_id}"),
        vec![
            ("projectId".into(), project_id.into()),
            (
                "healthScore".into(),
                health_score.map_or_else(String::new, |score| score.to_string()),
            ),
        ],
    )
}

pub fn break_timing(focus_state: &str, work_minutes: u32, meeting_soon: bool) -> DecisionRequest {
    DecisionRequest {
        id: DecisionId::BreakTiming,
        context: DecisionContext::bounded(
            "Bounded break timing context",
            vec![
                ("focusState".into(), focus_state.into()),
                ("workMinutes".into(), work_minutes.to_string()),
                ("meetingSoon".into(), meeting_soon.to_string()),
            ],
        ),
        questions: vec![DecisionQuestion {
            id: QuestionId::from_static("should_suggest_break"),
            decision_type: DecisionType::Noul,
            options: vec![],
            score_range: None,
            instructions: "Should a break be suggested without interrupting a protected focus or meeting window?".into(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_question_contains_only_supplied_options() {
        let request = task_prioritization(
            vec!["task-1".into(), "task-2".into()],
            Some("task-2".into()),
        );
        assert_eq!(request.questions[0].options, ["task-1", "task-2"]);
    }

    #[test]
    fn all_use_cases_have_valid_typed_contracts() {
        let requests = vec![
            interruption_risk(true, false, 45),
            meeting_recovery(vec!["task-1".into()], Some("task-1".into())),
            energy_task_selection(vec!["task-1".into()], Some("task-1".into()), 40),
            deadline_risk("task-1", 30, 60, true),
            scope_creep("task-1", "mail", 15),
            focus_start(vec!["task-1".into()], Some("task-1".into())),
            daily_review("not_reviewed", true),
            invoice_anomaly("invoice-1", Some(0.4)),
            project_health("project-1", Some(0.3)),
            break_timing("working", 55, false),
        ];
        assert!(requests.iter().all(|request| request.validate().is_ok()));
        assert_eq!(requests.len(), 10);
    }
}
