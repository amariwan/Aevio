use aevio_core::settings::{Language, Settings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedLanguage {
    De,
    En,
}

pub fn resolve(language: Language) -> ResolvedLanguage {
    match language {
        Language::De => ResolvedLanguage::De,
        Language::System | Language::En => ResolvedLanguage::En,
    }
}

pub fn from_settings(settings: &Settings) -> ResolvedLanguage {
    if settings.language != Language::System {
        return resolve(settings.language);
    }
    let mut system = String::new();
    for key in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
        if let Ok(value) = std::env::var(key) {
            if !value.trim().is_empty() {
                system = value;
                break;
            }
        }
    }
    let system = system.to_lowercase();
    if system.starts_with("de")
        || system.contains("_de")
        || system.contains("-de")
        || system.contains("de.")
    {
        ResolvedLanguage::De
    } else {
        ResolvedLanguage::En
    }
}

pub fn nothing_running(language: ResolvedLanguage) -> &'static str {
    match language {
        ResolvedLanguage::De => "Nichts aktiv",
        ResolvedLanguage::En => "Nothing running",
    }
}

pub fn open(language: ResolvedLanguage) -> &'static str {
    match language {
        ResolvedLanguage::De => "Aevio öffnen",
        ResolvedLanguage::En => "Open Aevio",
    }
}

pub fn quit(language: ResolvedLanguage) -> &'static str {
    match language {
        ResolvedLanguage::De => "Aevio beenden",
        ResolvedLanguage::En => "Quit Aevio",
    }
}

pub fn pause(language: ResolvedLanguage, label: &str) -> String {
    match language {
        ResolvedLanguage::De => format!("{} pausieren", label),
        ResolvedLanguage::En => format!("Pause {label}"),
    }
}

pub fn coach_title(language: ResolvedLanguage) -> &'static str {
    match language {
        ResolvedLanguage::De => "Aevio Coach",
        ResolvedLanguage::En => "Aevio Coach",
    }
}

pub fn focus_due(language: ResolvedLanguage) -> &'static str {
    match language {
        ResolvedLanguage::De => {
            "Deine Fokuszeit ist vorbei. Möchtest du pausieren oder bewusst weiterarbeiten?"
        }
        ResolvedLanguage::En => {
            "Your focus session is over. Would you like to pause or continue deliberately?"
        }
    }
}

pub fn guardian(language: ResolvedLanguage) -> &'static str {
    match language {
        ResolvedLanguage::De => "In den letzten zehn Minuten gab es mehrere Appwechsel. Du kannst deinen nächsten Schritt im Coach wieder aufnehmen.",
        ResolvedLanguage::En => "You switched apps several times in the last ten minutes. You can resume your next step in Coach.",
    }
}

pub fn task_reminder(language: ResolvedLanguage, title: &str) -> String {
    match language {
        ResolvedLanguage::De => format!("Du hast eine Erinnerung für „{title}“ vorgemerkt."),
        ResolvedLanguage::En => format!("You saved a reminder for “{title}”."),
    }
}

pub fn review(language: ResolvedLanguage) -> &'static str {
    match language {
        ResolvedLanguage::De => "Wenn es gerade passt, kannst du deinen Tag im Coach reflektieren.",
        ResolvedLanguage::En => "When it suits you, you can reflect on your day in Coach.",
    }
}

pub fn tooltip(language: ResolvedLanguage, running: usize, paused: usize) -> String {
    match (language, running, paused) {
        (ResolvedLanguage::De, 0, 0) => "Aevio — keine aktiven Timer".to_string(),
        (ResolvedLanguage::De, 0, paused) => {
            format!("Aevio — keine aktiven Timer, {paused} pausiert")
        }
        (ResolvedLanguage::De, running, 0) => format!("Aevio — {running} aktiv"),
        (ResolvedLanguage::De, running, paused) => {
            format!("Aevio — {running} aktiv, {paused} pausiert")
        }
        (ResolvedLanguage::En, 0, 0) => "Aevio — no trackers".to_string(),
        (ResolvedLanguage::En, 0, paused) => format!("Aevio — nothing running, {paused} paused"),
        (ResolvedLanguage::En, running, 0) => format!("Aevio — {running} running"),
        (ResolvedLanguage::En, running, paused) => {
            format!("Aevio — {running} running, {paused} paused")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_defaults_to_english_until_the_webview_resolves_the_host_locale() {
        assert_eq!(resolve(Language::System), ResolvedLanguage::En);
    }

    #[test]
    fn german_tray_copy_is_localized() {
        assert_eq!(nothing_running(ResolvedLanguage::De), "Nichts aktiv");
        assert_eq!(pause(ResolvedLanguage::De, "Timer"), "Timer pausieren");
    }

    #[test]
    fn resolve_maps_explicit_choices() {
        assert_eq!(resolve(Language::De), ResolvedLanguage::De);
        assert_eq!(resolve(Language::En), ResolvedLanguage::En);
    }

    #[test]
    fn tooltip_and_task_reminder_are_localized() {
        assert_eq!(
            tooltip(ResolvedLanguage::De, 2, 1),
            "Aevio — 2 aktiv, 1 pausiert"
        );
        assert_eq!(
            task_reminder(ResolvedLanguage::En, "Standup"),
            "You saved a reminder for “Standup”."
        );
    }

    #[test]
    fn from_settings_honours_explicit_language() {
        let settings = Settings {
            language: Language::De,
            ..Settings::default()
        };
        assert_eq!(from_settings(&settings), ResolvedLanguage::De);
        let settings = Settings {
            language: Language::En,
            ..Settings::default()
        };
        assert_eq!(from_settings(&settings), ResolvedLanguage::En);
    }
}
