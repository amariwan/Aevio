//! Runtime-independent decision contracts.

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DecisionId {
    FocusRecovery,
    TaskPrioritization,
    DailyReview,
    InterruptionRisk,
    MeetingRecovery,
    EnergyTaskSelection,
    DeadlineRisk,
    ScopeCreep,
    FocusStart,
    InvoiceAnomaly,
    ProjectHealth,
    BreakTiming,
}

impl DecisionId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FocusRecovery => "focus_recovery",
            Self::TaskPrioritization => "task_prioritization",
            Self::DailyReview => "daily_review",
            Self::InterruptionRisk => "interruption_risk",
            Self::MeetingRecovery => "meeting_recovery",
            Self::EnergyTaskSelection => "energy_task_selection",
            Self::DeadlineRisk => "deadline_risk",
            Self::ScopeCreep => "scope_creep",
            Self::FocusStart => "focus_start",
            Self::InvoiceAnomaly => "invoice_anomaly",
            Self::ProjectHealth => "project_health",
            Self::BreakTiming => "break_timing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct QuestionId(String);

impl QuestionId {
    pub fn from_static(value: &'static str) -> Self {
        debug_assert!(!value.is_empty());
        debug_assert!(value.len() <= 96);
        debug_assert!(
            value
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        );
        Self(value.to_string())
    }

    pub fn new(value: impl Into<String>) -> Result<Self, DecisionError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 96
            || !value
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            return Err(DecisionError::InvalidQuestionId(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DecisionType {
    Choice,
    Noul,
    Score,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreRange {
    pub min: f64,
    pub max: f64,
}

impl ScoreRange {
    pub fn validate(&self) -> Result<(), DecisionError> {
        if !self.min.is_finite() || !self.max.is_finite() || self.min >= self.max {
            return Err(DecisionError::InvalidScoreRange {
                min: self.min,
                max: self.max,
            });
        }
        Ok(())
    }

    pub fn contains(&self, value: f64) -> bool {
        value.is_finite() && value >= self.min && value <= self.max
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionQuestion {
    pub id: QuestionId,
    pub decision_type: DecisionType,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub score_range: Option<ScoreRange>,
    pub instructions: String,
}

impl DecisionQuestion {
    pub fn validate(&self) -> Result<(), DecisionError> {
        if self.instructions.trim().is_empty() {
            return Err(DecisionError::MissingInstructions(
                self.id.as_str().to_string(),
            ));
        }
        match self.decision_type {
            DecisionType::Choice if self.options.is_empty() => Err(
                DecisionError::ChoiceNeedsOptions(self.id.as_str().to_string()),
            ),
            DecisionType::Choice if self.options.iter().any(|option| option.trim().is_empty()) => {
                Err(DecisionError::EmptyChoiceOption(
                    self.id.as_str().to_string(),
                ))
            }
            DecisionType::Choice if has_duplicate_options(&self.options) => Err(
                DecisionError::DuplicateChoiceOption(self.id.as_str().to_string()),
            ),
            DecisionType::Score => self
                .score_range
                .as_ref()
                .ok_or_else(|| DecisionError::ScoreNeedsRange(self.id.as_str().to_string()))?
                .validate(),
            DecisionType::Noul => Ok(()),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionContext {
    pub summary: String,
    #[serde(default)]
    pub facts: Vec<(String, String)>,
}

impl DecisionContext {
    pub fn bounded(summary: impl Into<String>, facts: Vec<(String, String)>) -> Self {
        Self {
            summary: summary.into().chars().take(4_000).collect(),
            facts: facts
                .into_iter()
                .take(32)
                .map(|(key, value)| {
                    (
                        key.chars().take(96).collect(),
                        value.chars().take(512).collect(),
                    )
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionRequest {
    pub id: DecisionId,
    pub context: DecisionContext,
    pub questions: Vec<DecisionQuestion>,
}

impl DecisionRequest {
    pub fn validate(&self) -> Result<(), DecisionError> {
        if self.questions.is_empty() || self.questions.len() > 16 {
            return Err(DecisionError::InvalidQuestionCount(self.questions.len()));
        }
        for question in &self.questions {
            question.validate()?;
        }
        Ok(())
    }

    pub fn validate_result(&self, result: &DecisionResult) -> Result<(), DecisionError> {
        if result.abstention.is_some() {
            return Ok(());
        }
        if result.outcomes.len() != self.questions.len() {
            return Err(DecisionError::InvalidResponse);
        }
        for question in &self.questions {
            let mut matches = result
                .outcomes
                .iter()
                .filter(|outcome| outcome.question() == &question.id);
            let outcome = matches.next().ok_or(DecisionError::InvalidResponse)?;
            if matches.next().is_some() {
                return Err(DecisionError::DuplicateOutcome(
                    question.id.as_str().to_string(),
                ));
            }
            match (question, outcome) {
                (
                    DecisionQuestion {
                        decision_type: DecisionType::Choice,
                        options,
                        ..
                    },
                    DecisionOutcome::Choice {
                        value, confidence, ..
                    },
                ) => {
                    if !options.iter().any(|option| option == value)
                        || !valid_confidence(*confidence)
                    {
                        return Err(DecisionError::InvalidChoice {
                            question: question.id.as_str().into(),
                            value: value.clone(),
                        });
                    }
                }
                (
                    DecisionQuestion {
                        decision_type: DecisionType::Noul,
                        ..
                    },
                    DecisionOutcome::Noul { probability, .. },
                ) if probability.is_none_or(|value| valid_confidence(Some(value))) => {}
                (
                    DecisionQuestion {
                        decision_type: DecisionType::Score,
                        score_range: Some(range),
                        ..
                    },
                    DecisionOutcome::Score {
                        value, confidence, ..
                    },
                ) => {
                    if !range.contains(*value) || !valid_confidence(*confidence) {
                        return Err(DecisionError::InvalidScore {
                            question: question.id.as_str().into(),
                            value: *value,
                        });
                    }
                }
                _ => return Err(DecisionError::InvalidResponse),
            }
        }
        Ok(())
    }
}

fn valid_confidence(value: Option<f64>) -> bool {
    value.is_none_or(|value| value.is_finite() && (0.0..=1.0).contains(&value))
}

fn has_duplicate_options(options: &[String]) -> bool {
    options
        .iter()
        .enumerate()
        .any(|(index, option)| options[..index].iter().any(|previous| previous == option))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DecisionOutcome {
    Choice {
        question: QuestionId,
        value: String,
        confidence: Option<f64>,
    },
    Noul {
        question: QuestionId,
        value: Option<bool>,
        probability: Option<f64>,
    },
    Score {
        question: QuestionId,
        value: f64,
        confidence: Option<f64>,
    },
}

impl DecisionOutcome {
    pub fn question(&self) -> &QuestionId {
        match self {
            Self::Choice { question, .. }
            | Self::Noul { question, .. }
            | Self::Score { question, .. } => question,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderId {
    Rules,
    Laya,
    Jev,
}

impl ProviderId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rules => "rules",
            Self::Laya => "laya",
            Self::Jev => "jev",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCapabilities {
    pub provider: ProviderId,
    pub question_types: Vec<DecisionType>,
    pub remote: bool,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionMetadata {
    pub provider: ProviderId,
    pub model: Option<String>,
    pub routing_reason: String,
    pub latency_ms: u64,
    pub human_review_required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionResult {
    pub outcomes: Vec<DecisionOutcome>,
    pub abstention: Option<DecisionAbstention>,
    pub metadata: DecisionMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DecisionAbstention {
    Unsupported,
    LowConfidence,
    PrivacyBlocked,
    ProviderUnavailable,
    InvalidResponse,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecisionError {
    InvalidQuestionId(String),
    InvalidQuestionCount(usize),
    MissingInstructions(String),
    ChoiceNeedsOptions(String),
    EmptyChoiceOption(String),
    DuplicateChoiceOption(String),
    ScoreNeedsRange(String),
    InvalidScoreRange { min: f64, max: f64 },
    InvalidScore { question: String, value: f64 },
    InvalidChoice { question: String, value: String },
    DuplicateOutcome(String),
    Provider(String),
    Policy(String),
    InvalidResponse,
}

impl fmt::Display for DecisionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for DecisionError {}
