use aevio_core::coach::{
    assistance,
    models::{AssistRequest, Preferences, Proposal},
};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

#[cfg_attr(target_os = "macos", path = "provider/macos.rs")]
#[cfg_attr(windows, path = "provider/unsupported.rs")]
#[cfg_attr(
    not(any(target_os = "macos", windows)),
    path = "provider/unsupported.rs"
)]
mod platform;

#[derive(Default)]
pub struct CoachAi {
    last_request: Mutex<Option<Instant>>,
}
impl CoachAi {
    pub fn plan(
        &self,
        day: &aevio_core::coach::models::Day,
        use_ai: bool,
    ) -> Result<aevio_core::coach::adaptive::PlanningProposal, String> {
        use aevio_core::coach::adaptive;
        if !use_ai {
            return Ok(adaptive::planning(day));
        }
        if !day.preferences.enabled || !day.preferences.ai_enabled {
            return Err("Enable on-device coach AI first".into());
        }
        let mut last = self
            .last_request
            .try_lock()
            .map_err(|_| "A coach proposal is already running")?;
        if last.is_some_and(|at| at.elapsed() < Duration::from_secs(10)) {
            return Err("Please wait before generating another proposal".into());
        }
        *last = Some(Instant::now());
        Ok(
            match platform::generate_plan(day)
                .and_then(|selection| adaptive::validate_selection(day, selection))
            {
                Ok(proposal) => proposal,
                Err(error) => {
                    let mut proposal = adaptive::planning(day);
                    proposal.reason = format!(
                        "On-device planning unavailable ({error}). {}",
                        proposal.reason
                    );
                    proposal
                }
            },
        )
    }

    pub fn propose(
        &self,
        request: &AssistRequest,
        preferences: &Preferences,
        memory: &[String],
    ) -> Result<Proposal, String> {
        assistance::validate(request)?;
        if !request.use_ai {
            return Ok(assistance::apply_style(
                assistance::fallback(request, None),
                &preferences.style,
            ));
        }
        if !preferences.enabled || !preferences.ai_enabled {
            return Err("Enable on-device coach AI in Coach settings first".into());
        }
        // try_lock bounds concurrency: no queued unbounded generation work.
        let mut last = self
            .last_request
            .try_lock()
            .map_err(|_| "A coach proposal is already being generated")?;
        if last.is_some_and(|at| at.elapsed() < Duration::from_secs(10)) {
            return Err("Please wait a few seconds before requesting another proposal".into());
        }
        *last = Some(Instant::now());
        let result = platform::generate(request, preferences, memory).and_then(|value| {
            assistance::validate_output(&value)?;
            Ok(value)
        });
        Ok(assistance::apply_style(
            match result {
                Ok(value) => value,
                Err(error) => assistance::fallback(
                    request,
                    Some(format!(
                        "On-device AI unavailable; using editable local guidance. {error}"
                    )),
                ),
            },
            &preferences.style,
        ))
    }
}
