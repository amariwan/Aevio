//! Typed, policy-gated decision making for additive Aevio intelligence.
//!
//! The decision domain owns contracts, deterministic routing and validation.
//! It does not know about Tauri, Coach UI, or a concrete provider transport.

mod domain;
mod policy;
mod providers;
mod router;
pub mod use_cases;

pub use domain::{
    DecisionAbstention, DecisionContext, DecisionError, DecisionId, DecisionMetadata,
    DecisionOutcome, DecisionQuestion, DecisionRequest, DecisionResult, DecisionType,
    ProviderCapabilities, ProviderId, QuestionId, ScoreRange,
};
pub use policy::{DecisionPolicy, PolicyDecision};
pub use providers::{DecisionProvider, RemoteProtocol, RemoteProvider, RuleProvider};
pub use router::{DecisionRouter, ProviderMode, RoutingDecision};
