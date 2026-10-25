//! Local and remote decision-provider adapters.

use std::future::Future;
use std::pin::Pin;
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{Value, json};

use super::{
    DecisionAbstention, DecisionError, DecisionId, DecisionMetadata, DecisionOutcome,
    DecisionRequest, DecisionResult, DecisionType, ProviderCapabilities, ProviderId,
};

pub type DecisionFuture<'a> =
    Pin<Box<dyn Future<Output = Result<DecisionResult, DecisionError>> + Send + 'a>>;

pub trait DecisionProvider: Send + Sync {
    fn capabilities(&self) -> ProviderCapabilities;
    fn decide<'a>(&'a self, request: &'a DecisionRequest, reason: &'a str) -> DecisionFuture<'a>;
}

#[derive(Debug, Default, Clone)]
pub struct RuleProvider;

impl RuleProvider {
    fn outcome(
        request: &DecisionRequest,
        question: &super::DecisionQuestion,
    ) -> Option<DecisionOutcome> {
        let fact = |key: &str| {
            request
                .context
                .facts
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value)
        };
        match question.decision_type {
            DecisionType::Choice => {
                let value = match request.id {
                    DecisionId::FocusRecovery => {
                        if fact("sessionStatus") == Some(&"interrupted".to_string())
                            && fact("snoozed") != Some(&"true".to_string())
                        {
                            "resume_focus"
                        } else {
                            return None;
                        }
                    }
                    DecisionId::TaskPrioritization => fact("topTask")?.as_str(),
                    DecisionId::DailyReview => fact("reviewRecommendation")?.as_str(),
                    DecisionId::MeetingRecovery
                    | DecisionId::EnergyTaskSelection
                    | DecisionId::FocusStart => fact("recommended")?.as_str(),
                    _ => return None,
                };
                if question.options.iter().any(|option| option == value) {
                    Some(DecisionOutcome::Choice {
                        question: question.id.clone(),
                        value: value.to_string(),
                        confidence: Some(1.0),
                    })
                } else {
                    None
                }
            }
            DecisionType::Noul => {
                let value = match request.id {
                    DecisionId::FocusRecovery => {
                        fact("sessionStatus") == Some(&"interrupted".to_string())
                    }
                    DecisionId::ScopeCreep => fact("offPlan") == Some(&"true".to_string()),
                    DecisionId::BreakTiming => {
                        let focus_state = fact("focusState")?.as_str();
                        let meeting_soon = fact("meetingSoon")?.parse::<bool>().ok()?;
                        let work_minutes = fact("workMinutes")?.parse::<u32>().ok()?;
                        !meeting_soon && focus_state != "deep" && work_minutes >= 50
                    }
                    _ => return None,
                };
                Some(DecisionOutcome::Noul {
                    question: question.id.clone(),
                    value: Some(value),
                    probability: Some(1.0),
                })
            }
            DecisionType::Score => {
                let score = match request.id {
                    DecisionId::InterruptionRisk => {
                        let in_focus = fact("inFocus")?.parse::<bool>().ok()?;
                        let meeting_soon = fact("meetingSoon")?.parse::<bool>().ok()?;
                        let work_minutes = fact("workMinutes")?.parse::<u32>().ok()?;
                        if meeting_soon {
                            0.95
                        } else if in_focus && work_minutes >= 45 {
                            0.75
                        } else if work_minutes >= 45 {
                            0.55
                        } else {
                            0.2
                        }
                    }
                    DecisionId::DeadlineRisk => {
                        let remaining = fact("remainingMinutes")?.parse::<f64>().ok()?;
                        let estimate = fact("estimateMinutes")?.parse::<f64>().ok()?;
                        let due_soon = fact("dueSoon")?.parse::<bool>().ok()?;
                        if estimate <= 0.0 {
                            0.0
                        } else if remaining <= 0.0 {
                            1.0
                        } else {
                            let ratio = (estimate / remaining).min(2.0) / 2.0;
                            if due_soon { ratio.max(0.65) } else { ratio }
                        }
                    }
                    DecisionId::InvoiceAnomaly => fact("anomalyScore")?.parse::<f64>().ok()?,
                    DecisionId::ProjectHealth => fact("healthScore")?.parse::<f64>().ok()?,
                    _ => return None,
                };
                if !(0.0..=1.0).contains(&score) || !score.is_finite() {
                    return None;
                }
                Some(DecisionOutcome::Score {
                    question: question.id.clone(),
                    value: score,
                    confidence: None,
                })
            }
        }
    }
}

impl DecisionProvider for RuleProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider: ProviderId::Rules,
            question_types: vec![DecisionType::Choice, DecisionType::Noul],
            remote: false,
            model: None,
        }
    }

    fn decide<'a>(&'a self, request: &'a DecisionRequest, reason: &'a str) -> DecisionFuture<'a> {
        Box::pin(async move {
            request.validate()?;
            let started = Instant::now();
            let outcomes: Vec<_> = request
                .questions
                .iter()
                .filter_map(|question| Self::outcome(request, question))
                .collect();
            let abstention = (outcomes.len() != request.questions.len())
                .then_some(DecisionAbstention::Unsupported);
            Ok(DecisionResult {
                outcomes,
                abstention,
                metadata: DecisionMetadata {
                    provider: ProviderId::Rules,
                    model: None,
                    routing_reason: reason.to_string(),
                    latency_ms: started.elapsed().as_millis() as u64,
                    human_review_required: false,
                },
            })
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteProtocol {
    Laya,
    Jev,
}

#[derive(Clone)]
pub struct RemoteProvider {
    client: reqwest::Client,
    endpoint: String,
    model: String,
    api_key: Option<String>,
    protocol: RemoteProtocol,
}

impl RemoteProvider {
    pub fn new(
        endpoint: String,
        model: String,
        api_key: Option<String>,
        protocol: RemoteProtocol,
        timeout: std::time::Duration,
    ) -> Result<Self, DecisionError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|error| DecisionError::Provider(format!("http client: {error}")))?;
        Ok(Self {
            client,
            endpoint,
            model,
            api_key,
            protocol,
        })
    }

    pub fn protocol(&self) -> RemoteProtocol {
        self.protocol
    }

    async fn send(&self, request: &DecisionRequest) -> Result<DecisionResult, DecisionError> {
        let questions = request.questions.iter().map(|question| {
            let mut value = json!({
                "type": match question.decision_type { DecisionType::Choice => "choice", DecisionType::Noul => "noul", DecisionType::Score => "score" },
                "instructions": question.instructions,
            });
            if !question.options.is_empty() { value["options"] = json!(question.options); }
            if let Some(range) = &question.score_range { value["min"] = json!(range.min); value["max"] = json!(range.max); }
            (question.id.as_str().to_string(), value)
        }).collect::<serde_json::Map<_, _>>();
        let body = match self.protocol {
            RemoteProtocol::Laya => {
                json!({ "state": request.context.summary, "model": self.model, "questions": questions })
            }
            RemoteProtocol::Jev => {
                json!({ "state": request.context.summary, "model": self.model, "questions": questions })
            }
        };
        for attempt in 0..3u32 {
            let mut call = self
                .client
                .post(&self.endpoint)
                .header(CONTENT_TYPE, "application/json")
                .json(&body);
            if let Some(key) = &self.api_key {
                call = call.header(AUTHORIZATION, format!("Bearer {key}"));
            }
            let response = match call.send().await {
                Ok(response) => response,
                Err(error) if attempt < 2 && error.is_timeout() => {
                    retry_delay(attempt).await;
                    continue;
                }
                Err(error) => {
                    return Err(DecisionError::Provider(format!("request failed: {error}")));
                }
            };
            let status = response.status();
            if status.is_success() {
                let payload: Value = response.json().await.map_err(|error| {
                    DecisionError::Provider(format!("invalid JSON response: {error}"))
                })?;
                let result = map_response(&payload, request, self.protocol, self.model.clone())?;
                request.validate_result(&result)?;
                return Ok(result);
            }
            if attempt < 2
                && (status == reqwest::StatusCode::REQUEST_TIMEOUT
                    || status == reqwest::StatusCode::TOO_MANY_REQUESTS
                    || status.is_server_error())
            {
                retry_delay(attempt).await;
                continue;
            }
            return Err(DecisionError::Provider(format!(
                "provider returned HTTP {status}"
            )));
        }
        Err(DecisionError::Provider("provider retries exhausted".into()))
    }
}

async fn retry_delay(attempt: u32) {
    let jitter = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.subsec_millis() as u64 % 40);
    tokio::time::sleep(std::time::Duration::from_millis(
        100 * 2u64.pow(attempt) + jitter,
    ))
    .await;
}

impl DecisionProvider for RemoteProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider: match self.protocol {
                RemoteProtocol::Laya => ProviderId::Laya,
                RemoteProtocol::Jev => ProviderId::Jev,
            },
            question_types: vec![
                DecisionType::Choice,
                DecisionType::Noul,
                DecisionType::Score,
            ],
            remote: true,
            model: Some(self.model.clone()),
        }
    }

    fn decide<'a>(&'a self, request: &'a DecisionRequest, reason: &'a str) -> DecisionFuture<'a> {
        Box::pin(async move {
            request.validate()?;
            let started = Instant::now();
            let mut result = self.send(request).await?;
            result.metadata.routing_reason = reason.to_string();
            result.metadata.latency_ms = started.elapsed().as_millis() as u64;
            Ok(result)
        })
    }
}

fn map_response(
    payload: &Value,
    request: &DecisionRequest,
    protocol: RemoteProtocol,
    model: String,
) -> Result<DecisionResult, DecisionError> {
    let answers = payload
        .get("answers")
        .and_then(Value::as_object)
        .ok_or(DecisionError::InvalidResponse)?;
    let mut outcomes = Vec::with_capacity(request.questions.len());
    for question in &request.questions {
        let answer = answers
            .get(question.id.as_str())
            .ok_or(DecisionError::InvalidResponse)?;
        let confidence = answer.get("confidence").and_then(Value::as_f64);
        let outcome = match question.decision_type {
            DecisionType::Choice => DecisionOutcome::Choice {
                question: question.id.clone(),
                value: answer
                    .get("choice")
                    .and_then(Value::as_str)
                    .ok_or(DecisionError::InvalidResponse)?
                    .to_string(),
                confidence,
            },
            DecisionType::Noul => DecisionOutcome::Noul {
                question: question.id.clone(),
                value: answer
                    .get("noul")
                    .and_then(Value::as_bool)
                    .or_else(|| answer.get("value").and_then(Value::as_bool)),
                probability: answer
                    .get("probability")
                    .and_then(Value::as_f64)
                    .or(confidence),
            },
            DecisionType::Score => DecisionOutcome::Score {
                question: question.id.clone(),
                value: answer
                    .get("score")
                    .and_then(Value::as_f64)
                    .or_else(|| answer.get("value").and_then(Value::as_f64))
                    .ok_or(DecisionError::InvalidResponse)?,
                confidence,
            },
        };
        outcomes.push(outcome);
    }
    Ok(DecisionResult {
        outcomes,
        abstention: None,
        metadata: DecisionMetadata {
            provider: match protocol {
                RemoteProtocol::Laya => ProviderId::Laya,
                RemoteProtocol::Jev => ProviderId::Jev,
            },
            model: Some(model),
            routing_reason: "provider response".into(),
            latency_ms: 0,
            human_review_required: false,
        },
    })
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use super::*;
    use crate::decision::{
        DecisionContext, DecisionId, DecisionOutcome, DecisionQuestion, QuestionId, use_cases,
    };

    fn choice_request() -> DecisionRequest {
        DecisionRequest {
            id: DecisionId::TaskPrioritization,
            context: DecisionContext::bounded(
                "choose a task",
                vec![("topTask".into(), "task_a".into())],
            ),
            questions: vec![DecisionQuestion {
                id: QuestionId::new("selected_task").unwrap(),
                decision_type: DecisionType::Choice,
                options: vec!["task_a".into(), "task_b".into()],
                score_range: None,
                instructions: "select one task".into(),
            }],
        }
    }

    fn serve_once(payload: &'static str, status: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                payload.len()
            );
            let _ = stream.write_all(response.as_bytes());
        });
        format!("http://{address}")
    }

    fn remote_result(
        protocol: RemoteProtocol,
        payload: &'static str,
    ) -> Result<DecisionResult, DecisionError> {
        let endpoint = serve_once(payload, "200 OK");
        let provider = RemoteProvider::new(
            endpoint,
            "test-model".into(),
            None,
            protocol,
            std::time::Duration::from_secs(2),
        )
        .unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(provider.decide(&choice_request(), "test"))
    }

    #[test]
    fn rule_provider_abstains_without_a_matching_rule() {
        let request = DecisionRequest {
            id: DecisionId::DailyReview,
            context: DecisionContext::bounded("review", vec![]),
            questions: vec![DecisionQuestion {
                id: QuestionId::new("review").unwrap(),
                decision_type: DecisionType::Choice,
                options: vec!["highlight".into()],
                score_range: None,
                instructions: "pick".into(),
            }],
        };
        let result = futures_lite::future::block_on(RuleProvider.decide(&request, "test")).unwrap();
        assert_eq!(result.abstention, Some(DecisionAbstention::Unsupported));
    }

    #[test]
    fn laya_adapter_maps_a_controlled_http_response() {
        let result = remote_result(
            RemoteProtocol::Laya,
            r#"{"answers":{"selected_task":{"choice":"task_a","confidence":0.91}}}"#,
        )
        .unwrap();
        assert_eq!(result.metadata.provider, ProviderId::Laya);
        assert!(
            matches!(result.outcomes.first(), Some(DecisionOutcome::Choice { value, .. }) if value == "task_a")
        );
    }

    #[test]
    fn jev_adapter_rejects_a_malformed_choice_response() {
        let error = remote_result(
            RemoteProtocol::Jev,
            r#"{"answers":{"selected_task":{"choice":"invented","confidence":0.99}}}"#,
        )
        .unwrap_err();
        assert!(matches!(error, DecisionError::InvalidChoice { .. }));
    }

    #[test]
    fn rules_cover_the_ten_initial_use_cases_without_inventing_tasks() {
        let requests = [
            use_cases::interruption_risk(true, false, 50),
            use_cases::meeting_recovery(vec!["task-a".into()], Some("task-a".into())),
            use_cases::energy_task_selection(vec!["task-a".into()], Some("task-a".into()), 30),
            use_cases::deadline_risk("task-a", 20, 60, true),
            use_cases::scope_creep("task-a", "mail", 15),
            use_cases::focus_start(vec!["task-a".into()], Some("task-a".into())),
            use_cases::daily_review("not_reviewed", true),
            use_cases::invoice_anomaly("invoice-a", Some(0.7)),
            use_cases::project_health("project-a", Some(0.4)),
            use_cases::break_timing("working", 55, false),
        ];
        for request in requests {
            let result =
                futures_lite::future::block_on(RuleProvider.decide(&request, "test")).unwrap();
            assert!(
                result.abstention.is_none(),
                "{} abstained",
                request.id.as_str()
            );
            assert_eq!(result.outcomes.len(), 1);
            assert!(matches!(
                result.outcomes[0],
                DecisionOutcome::Choice { .. }
                    | DecisionOutcome::Noul { .. }
                    | DecisionOutcome::Score { .. }
            ));
        }
    }
}
