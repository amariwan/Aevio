//! Read-only proposals, independent of persistence and provider availability.
use super::{
    models::{AssistRequest, Proposal},
    policies,
};
pub const PROMPT_VERSION: &str = "coach-1";

pub fn validate(request: &AssistRequest) -> Result<(), String> {
    policies::title(&request.title)?;
    if !(1..=3).contains(&request.granularity)
        || request.estimate_minutes > 1440
        || request.previous_steps.len() > 30
        || request
            .previous_steps
            .iter()
            .map(|step| step.chars().count())
            .sum::<usize>()
            > 6000
    {
        return Err("Invalid assistance limits".into());
    }
    if !matches!(
        request.reason.as_str(),
        "cannot_start" | "unsure" | "too_large" | "low_energy" | "distracted" | "first_step"
    ) {
        return Err("Choose a supported starting difficulty".into());
    }
    for step in &request.previous_steps {
        policies::title(step)?;
    }
    Ok(())
}
pub fn fallback(request: &AssistRequest, note: Option<String>) -> Proposal {
    let task_title = request.title.chars().take(300).collect::<String>();
    let first = match request.reason.as_str() {
        "low_energy" => format!(
            "Öffne nur das Material zu „{}“. Du kannst danach aufhören.",
            task_title
        ),
        "distracted" => format!(
            "Notiere in einem Satz, was du bei „{}“ als Nächstes sehen möchtest.",
            task_title
        ),
        "unsure" => format!(
            "Lies „{}“ und notiere eine offene Frage, die du klären möchtest.",
            task_title
        ),
        _ => format!(
            "Öffne das Arbeitsmaterial zu „{}“ und sieh dir den aktuellen Stand an.",
            task_title
        ),
    };
    let mut steps = vec![
        first.clone(),
        "Beschreibe ein kleines, überprüfbares Ergebnis in einem Satz.".into(),
        "Bearbeite nur den ersten Teil, der zu diesem Ergebnis beiträgt.".into(),
        "Prüfe das Ergebnis und notiere den nächsten möglichen Schritt.".into(),
    ];
    if request.granularity == 1 {
        steps = vec![first.clone(), steps[2].clone(), steps[3].clone()];
    }
    if request.granularity == 3 {
        steps.insert(
            2,
            "Lege die benötigten Informationen bereit und notiere, was noch fehlt.".into(),
        );
        steps.insert(
            4,
            "Mache eine kurze Pause und entscheide frei, ob du weitermachen möchtest.".into(),
        );
    }
    Proposal {
        steps,
        next_step: first,
        source: "rules".into(),
        note,
        prompt_version: PROMPT_VERSION.into(),
    }
}
pub fn validate_output(value: &Proposal) -> Result<(), String> {
    if value.steps.is_empty() || value.steps.len() > 12 {
        return Err("Provider returned an invalid number of steps".into());
    }
    policies::title(&value.next_step)?;
    for step in &value.steps {
        policies::title(step)?;
    }
    Ok(())
}

pub fn apply_style(mut proposal: Proposal, style: &super::models::Style) -> Proposal {
    use super::models::Style;
    match style {
        Style::Minimal => {
            proposal.steps.truncate(3);
        }
        Style::Supportive => {
            proposal
                .next_step
                .push_str(" Ein kleiner Anfang darf für jetzt genug sein.");
        }
        Style::Structured => {
            proposal.next_step = format!(
                "Erster Schritt: {} Danach kannst du den nächsten Schritt auswählen.",
                proposal.next_step
            );
        }
        Style::Direct => {
            proposal.next_step = format!("Beginne hier: {}", proposal.next_step);
        }
        Style::Calm => {}
    }
    proposal
}
