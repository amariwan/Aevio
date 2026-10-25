//! Deterministic decision-provider routing.

use std::sync::Arc;

use super::{
    DecisionError, DecisionPolicy, DecisionProvider, DecisionRequest, DecisionResult, ProviderId,
};
pub use aevio_core::decision::ProviderMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingDecision {
    pub provider: Option<ProviderId>,
    pub reason: String,
}

pub struct DecisionRouter {
    pub mode: ProviderMode,
    pub policy: DecisionPolicy,
    pub rules: Arc<dyn DecisionProvider>,
    pub local: Option<Arc<dyn DecisionProvider>>,
    pub cloud: Option<Arc<dyn DecisionProvider>>,
}

impl DecisionRouter {
    pub fn route(&self) -> RoutingDecision {
        match self.mode {
            ProviderMode::Disabled => RoutingDecision {
                provider: None,
                reason: "decision engine disabled".into(),
            },
            ProviderMode::RulesOnly => RoutingDecision {
                provider: Some(ProviderId::Rules),
                reason: "rules_only mode".into(),
            },
            ProviderMode::LocalOnly => RoutingDecision {
                provider: self
                    .local
                    .as_ref()
                    .map(|provider| provider.capabilities().provider)
                    .or(Some(ProviderId::Rules)),
                reason: if self.local.is_some() {
                    "local provider configured"
                } else {
                    "local provider unavailable; rules only"
                }
                .into(),
            },
            ProviderMode::Cloud => RoutingDecision {
                provider: self
                    .cloud
                    .as_ref()
                    .map(|provider| provider.capabilities().provider),
                reason: if self.cloud.is_some() {
                    "cloud provider explicitly enabled"
                } else {
                    "cloud provider unavailable"
                }
                .into(),
            },
            ProviderMode::Auto => RoutingDecision {
                provider: self
                    .local
                    .as_ref()
                    .map(|provider| provider.capabilities().provider)
                    .or(Some(ProviderId::Rules)),
                reason: if self.local.is_some() {
                    "local provider preferred by auto mode"
                } else {
                    "no local provider; rules only"
                }
                .into(),
            },
        }
    }

    pub async fn decide(&self, request: &DecisionRequest) -> Result<DecisionResult, DecisionError> {
        request.validate()?;
        let selected = match self.mode {
            ProviderMode::Disabled => {
                return Err(DecisionError::Policy("decision engine disabled".into()));
            }
            ProviderMode::RulesOnly => &self.rules,
            ProviderMode::LocalOnly | ProviderMode::Auto => {
                self.local.as_ref().map_or(&self.rules, |provider| provider)
            }
            ProviderMode::Cloud => self
                .cloud
                .as_ref()
                .ok_or_else(|| DecisionError::Provider("cloud provider unavailable".into()))?,
        };
        let route = self.route();
        let mut result = match selected.decide(request, &route.reason).await {
            Ok(result) => result,
            Err(DecisionError::Provider(error)) if self.mode == ProviderMode::Auto => {
                self.rules
                    .decide(
                        request,
                        &format!("local provider unavailable; rules fallback: {error}"),
                    )
                    .await?
            }
            Err(error) => return Err(error),
        };
        request.validate_result(&result)?;
        let policy = self.policy.evaluate(request, &result)?;
        if matches!(policy, super::PolicyDecision::HumanReview) {
            result.metadata.human_review_required = true;
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::super::providers::DecisionFuture;
    use super::*;
    use crate::decision::{
        DecisionMetadata, DecisionOutcome, DecisionType, ProviderCapabilities, RuleProvider,
        use_cases,
    };

    struct FailingProvider;

    impl DecisionProvider for FailingProvider {
        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities {
                provider: ProviderId::Laya,
                question_types: vec![DecisionType::Choice],
                remote: true,
                model: Some("test".into()),
            }
        }

        fn decide<'a>(
            &'a self,
            _request: &'a DecisionRequest,
            _reason: &'a str,
        ) -> DecisionFuture<'a> {
            Box::pin(async { Err(DecisionError::Provider("offline".into())) })
        }
    }

    struct LowConfidenceProvider;

    impl DecisionProvider for LowConfidenceProvider {
        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities {
                provider: ProviderId::Laya,
                question_types: vec![DecisionType::Choice],
                remote: true,
                model: Some("test".into()),
            }
        }

        fn decide<'a>(
            &'a self,
            request: &'a DecisionRequest,
            reason: &'a str,
        ) -> DecisionFuture<'a> {
            Box::pin(async move {
                Ok(DecisionResult {
                    outcomes: vec![DecisionOutcome::Choice {
                        question: request.questions[0].id.clone(),
                        value: "task-b".into(),
                        confidence: Some(0.2),
                    }],
                    abstention: None,
                    metadata: DecisionMetadata {
                        provider: ProviderId::Laya,
                        model: Some("test".into()),
                        routing_reason: reason.into(),
                        latency_ms: 1,
                        human_review_required: false,
                    },
                })
            })
        }
    }

    fn task_request() -> DecisionRequest {
        use_cases::task_prioritization(
            vec!["task-a".into(), "task-b".into()],
            Some("task-b".into()),
        )
    }

    #[test]
    fn auto_mode_falls_back_to_rules_on_transport_failure() {
        let router = DecisionRouter {
            mode: ProviderMode::Auto,
            policy: DecisionPolicy::default(),
            rules: Arc::new(RuleProvider),
            local: Some(Arc::new(FailingProvider)),
            cloud: None,
        };
        let result = futures_lite::future::block_on(router.decide(&task_request())).unwrap();
        assert_eq!(result.metadata.provider, ProviderId::Rules);
        assert!(result.metadata.routing_reason.contains("rules fallback"));
    }

    #[test]
    fn low_confidence_results_are_marked_for_human_review() {
        let router = DecisionRouter {
            mode: ProviderMode::LocalOnly,
            policy: DecisionPolicy {
                remote_allowed: true,
                ..DecisionPolicy::default()
            },
            rules: Arc::new(RuleProvider),
            local: Some(Arc::new(LowConfidenceProvider)),
            cloud: None,
        };
        let result = futures_lite::future::block_on(router.decide(&task_request())).unwrap();
        assert!(result.metadata.human_review_required);
    }
}
