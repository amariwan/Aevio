use super::models::{Plan, Preferences, Review};

pub fn text(value: &str, max: usize) -> Result<(), String> {
    if value.chars().count() > max || value.contains('\0') {
        Err(format!(
            "Text must contain at most {max} characters and no NUL"
        ))
    } else {
        Ok(())
    }
}
pub fn title(value: &str) -> Result<(), String> {
    text(value, 500)?;
    if value.trim().is_empty() {
        Err("Please enter a task title".into())
    } else {
        Ok(())
    }
}
pub fn plan(value: &Plan) -> Result<(), String> {
    super::dates::bounds(&value.day, &value.time_zone)?;
    text(&value.goal, 2000)?;
    text(&value.notes, 10000)?;
    if value.available_minutes > 1440
        || value
            .energy
            .is_some_and(|energy| !(1..=5).contains(&energy))
        || !matches!(value.state.as_str(), "planned" | "skipped")
    {
        return Err("Invalid planning values".into());
    }
    Ok(())
}
pub fn review(value: &Review) -> Result<(), String> {
    if !matches!(
        value.outcome.as_str(),
        "achieved" | "partially_achieved" | "not_achieved" | "changed" | "not_reviewed"
    ) {
        return Err("Choose a valid goal outcome".into());
    }
    for value in [&value.worked, &value.obstacles, &value.tomorrow] {
        text(value, 10000)?;
    }
    Ok(())
}
pub fn preferences(value: &Preferences) -> Result<(), String> {
    if value.quiet_start > 1439
        || value.quiet_end > 1439
        || value.review_minute > 1439
        || !(15..=1440).contains(&value.cooldown_minutes)
        || value.planning_buffer_minutes > 1440
    {
        return Err("Invalid reminder time or frequency".into());
    }
    Ok(())
}
pub fn quiet(preferences: &Preferences, minute: u16) -> bool {
    let (start, end) = (preferences.quiet_start, preferences.quiet_end);
    if start == end {
        return false;
    }
    if start < end {
        minute >= start && minute < end
    } else {
        minute >= start || minute < end
    }
}
pub fn defaults() -> Preferences {
    Preferences {
        quiet_start: 20 * 60,
        quiet_end: 8 * 60,
        cooldown_minutes: 60,
        review_minute: 17 * 60,
        visual_timer: true,
        widget_visible: true,
        ..Preferences::default()
    }
}
