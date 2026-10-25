//! Decision acceptance and review policy.

use super::{DecisionError, DecisionOutcome, DecisionRequest, DecisionResult};

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionPolicy {
    pub minimum_confidence: f64,
    pub remote_allowed: bool,
    pub human_review_below_threshold: bool,
}

impl Default for DecisionPolicy {
    fn default() -> Self {
        Self {
            minimum_confidence: 0.80,
            remote_allowed: false,
            human_review_below_threshold: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    Approved,
    HumanReview,
    Abstained,
}

impl DecisionPolicy {
    pub fn evaluate(
        &self,
        request: &DecisionRequest,
        result: &DecisionResult,
    ) -> Result<PolicyDecision, DecisionError> {
        if result.metadata.provider != super::ProviderId::Rules && !self.remote_allowed {
            return Err(DecisionError::Policy("remote inference is disabled".into()));
        }
        if result.abstention.is_some() {
            return Ok(PolicyDecision::Abstained);
        }
        let mut review = false;
        for question in &request.questions {
            let Some(outcome) = result
                .outcomes
                .iter()
                .find(|outcome| outcome.question() == &question.id)
            else {
                return Err(DecisionError::InvalidResponse);
            };
            let confidence = match outcome {
                DecisionOutcome::Choice { confidence, .. }
                | DecisionOutcome::Score { confidence, .. } => *confidence,
                DecisionOutcome::Noul { probability, .. } => *probability,
            };
            if confidence.is_none_or(|value| {
                !value.is_finite()
                    || !(0.0..=1.0).contains(&value)
                    || value < self.minimum_confidence
            }) {
                review = true;
            }
        }
        Ok(if review && self.human_review_below_threshold {
            PolicyDecision::HumanReview
        } else {
            PolicyDecision::Approved
        })
    }
}
