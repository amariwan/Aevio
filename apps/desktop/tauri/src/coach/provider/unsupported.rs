use aevio_core::coach::models::{AssistRequest, Preferences, Proposal};
pub fn generate(_: &AssistRequest, _: &Preferences, _: &[String]) -> Result<Proposal, String> {
    Err("On-device generative coaching is unavailable on this platform".into())
}

pub fn generate_plan(
    _: &aevio_core::coach::models::Day,
) -> Result<aevio_core::coach::adaptive::PlanSelection, String> {
    Err("On-device generative planning is unsupported on this platform".into())
}
