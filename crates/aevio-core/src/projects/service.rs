//! The Projects page (P3): AI hints as T0 rules with a match preview,
//! per-project totals, project discovery from captured activity, and CSV
//! import.
//!
//! A project's `ai_hints` is free text: keywords, folders, repos, domains.
//! Each hint becomes one `origin = 'hint'` rule for the project, which is how
//! most project assignments resolve before the model is asked (design board
//! C§3b). Saving the hints replaces that project's hint rules.
//!
//! Everything here is pure: the hints become rules, the captured segments
//! become suggestions, and a CSV becomes rows. Reading the projects, the
//! segments, and the dismissed names is the persistence layer's job.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::ai::rules::Rule;
use crate::entries::models::ActivitySegment;

pub const ORIGIN_HINT: &str = "hint";
/// Keywords shorter than this would match nearly every window title.
const MIN_KEYWORD_CHARS: usize = 3;
/// Discovery ignores anything seen for less than this in its window.
pub const DISCOVERY_MIN_MS: u64 = 30 * 60 * 1000;
pub const DISCOVERY_LIMIT: usize = 5;
/// The settings key the dismissed suggestion names live under.
pub const DISMISSED_KEY: &str = "project_suggestions_dismissed";

/// The muted palette (design board, theme T), for projects created without a
/// colour picker: imports and discovery.
pub const PALETTE: [&str; 10] = [
    "#75a4e5", "#56c2b1", "#52b788", "#df84b5", "#e4817d", "#9aa6b4", "#bfa181", "#707bf0",
    "#66b1df", "#9b87df",
];

// --- Hints ------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    /// The text the user wrote.
    pub text: String,
    /// The rule it becomes, or `None` when it is too short to match on.
    pub rule: Option<(&'static str, String)>,
}

static DOMAIN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^[a-z0-9-]+(\.[a-z0-9-]+)*\.[a-z]{2,}$").expect("domain regex")
});

/// Classifies one hint: `~/Code/Aevio` is a path, `github.com/a/b` a URL
/// prefix, `figma.com` a domain, and anything else a title keyword.
fn classify(text: &str) -> Option<(&'static str, String)> {
    if text.starts_with("~/") || text.starts_with('/') {
        return Some(("path_prefix", text.trim_end_matches('/').to_string()));
    }
    let bare = text
        .strip_prefix("https://")
        .or_else(|| text.strip_prefix("http://"))
        .unwrap_or(text);
    if let Some((host, _)) = bare.split_once('/')
        && DOMAIN.is_match(host)
    {
        return Some(("url_prefix", bare.trim_end_matches('/').to_string()));
    }
    if DOMAIN.is_match(bare) {
        return Some(("domain", bare.to_ascii_lowercase()));
    }
    if text.chars().count() < MIN_KEYWORD_CHARS {
        return None;
    }
    Some(("title_contains", text.to_string()))
}

/// Splits hint text on commas, semicolons, and newlines, dropping blanks and
/// case-insensitive repeats.
pub fn parse_hints(text: &str) -> Vec<Hint> {
    let mut seen = HashSet::new();
    text.split([',', ';', '\n'])
        .map(str::trim)
        .filter(|hint| !hint.is_empty())
        .filter(|hint| seen.insert(hint.to_lowercase()))
        .map(|hint| Hint {
            text: hint.to_string(),
            rule: classify(hint),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HintMatch {
    pub hint: String,
    /// The rule kind the hint becomes; `None` when it is too short to use.
    pub match_kind: Option<String>,
    pub pattern: Option<String>,
    pub matched_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HintPreview {
    pub hints: Vec<HintMatch>,
    /// Time any hint matched, counted once ("would have matched 14h").
    pub total_ms: u64,
}

/// How much captured time each hint would have matched. A hint that is too
/// short to match on reports no kind and no time.
pub fn preview_hints_over(segments: &[ActivitySegment], hints: &str) -> HintPreview {
    let parsed = parse_hints(hints);
    let rules: Vec<Option<Rule>> = parsed
        .iter()
        .enumerate()
        .map(|(index, hint)| {
            hint.rule.as_ref().map(|(kind, pattern)| Rule {
                id: index.to_string(),
                match_kind: kind.to_string(),
                pattern: pattern.clone(),
                category_id: None,
                project_id: None,
                priority: 0,
                origin: ORIGIN_HINT.to_string(),
            })
        })
        .collect();

    let mut matched = vec![0_u64; parsed.len()];
    let mut total_ms = 0_u64;
    if rules.iter().any(Option::is_some) {
        for segment in segments {
            let ms = segment_ms(segment);
            let mut any = false;
            for (index, rule) in rules.iter().enumerate() {
                if rule.as_ref().is_some_and(|rule| rule.matches(segment)) {
                    matched[index] += ms;
                    any = true;
                }
            }
            if any {
                total_ms += ms;
            }
        }
    }

    HintPreview {
        hints: parsed
            .into_iter()
            .zip(matched)
            .map(|(hint, matched_ms)| HintMatch {
                hint: hint.text,
                match_kind: hint.rule.as_ref().map(|(kind, _)| kind.to_string()),
                pattern: hint.rule.map(|(_, pattern)| pattern),
                matched_ms,
            })
            .collect(),
        total_ms,
    }
}

// --- Stats and links ----------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStats {
    pub project_id: String,
    pub entries: u32,
    pub total_ms: u64,
    /// Time in the page's selected range (the Time column).
    pub range_ms: u64,
    /// Time since the start of this month, for monthly budgets.
    pub month_ms: u64,
    pub billable_ms: u64,
    pub billable_month_ms: u64,
    /// Approved, billable time not yet on an invoice: what is ready to bill.
    pub unbilled_ms: u64,
    pub unbilled_entries: u32,
    pub last_activity: Option<u64>,
}

/// A rule (or an Apps-page default) that assigns time to a project.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRule {
    pub id: String,
    pub match_kind: String,
    pub pattern: String,
    /// manual | suggested | hint | app
    pub origin: String,
}

// --- Discovery ------------------------------------------------------------------

/// "Create project pantry-tracker? (6h 10m, matched by ~/Code/pantry-tracker,
/// github.com/…/pantry-tracker)".
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSuggestion {
    /// Lowercased name, the identity a dismissal remembers.
    pub key: String,
    pub name: String,
    pub ms: u64,
    /// The strongest matches, ready to become the project's AI hints.
    pub evidence: Vec<String>,
}

/// Folders people keep repositories in.
const CODE_ROOTS: &str = "Code|code|Projects|projects|src|dev|Developer|developer|repos|git|workspace|Workspace|Sites|work";

static TITLE_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?:~|/Users/[^/\s]+|/home/[^/\s]+)/({CODE_ROOTS})/([A-Za-z0-9][A-Za-z0-9._-]*)"
    ))
    .expect("title path regex")
});

const FORGES: [&str; 4] = ["github.com", "gitlab.com", "bitbucket.org", "codeberg.org"];
/// First path segments on a forge that are pages, not owners.
const FORGE_PAGES: [&str; 18] = [
    "settings",
    "notifications",
    "pulls",
    "issues",
    "marketplace",
    "explore",
    "orgs",
    "topics",
    "login",
    "new",
    "search",
    "sponsors",
    "features",
    "about",
    "trending",
    "collections",
    "codespaces",
    "dashboard",
];

/// Editors whose window titles name the open folder.
const EDITORS: [&str; 12] = [
    "Code",
    "Visual Studio Code",
    "Cursor",
    "Zed",
    "Xcode",
    "Sublime Text",
    "Nova",
    "IntelliJ IDEA",
    "WebStorm",
    "RustRover",
    "PyCharm",
    "Android Studio",
];

static FOLDER_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9_-]{1,59}$").expect("folder regex"));

/// Title parts an editor shows that are never a project.
const GENERIC_TITLES: [&str; 8] = [
    "welcome",
    "untitled",
    "settings",
    "preferences",
    "extensions",
    "terminal",
    "output",
    "search",
];

/// `(name, evidence)` pairs one segment points at.
fn candidates(segment: &ActivitySegment) -> Vec<(String, String)> {
    let mut out = Vec::new();

    if let Some(url) = segment.url.as_deref() {
        let bare = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(url);
        let bare = bare.strip_prefix("www.").unwrap_or(bare);
        let mut parts = bare.split(['/', '?', '#']);
        if let (Some(host), Some(owner), Some(repo)) = (parts.next(), parts.next(), parts.next()) {
            let repo = repo.trim_end_matches(".git");
            if FORGES.contains(&host)
                && !owner.is_empty()
                && !repo.is_empty()
                && !FORGE_PAGES.contains(&owner.to_ascii_lowercase().as_str())
            {
                out.push((repo.to_string(), format!("{host}/{owner}/{repo}")));
            }
        }
    }

    for capture in TITLE_PATH.captures_iter(&segment.title) {
        let name = capture[2].trim_end_matches(['.', '-', '_']);
        if !name.is_empty() {
            out.push((name.to_string(), format!("~/{}/{name}", &capture[1])));
        }
    }

    // Editors title windows `file — folder` or `folder — file`; the piece
    // with no file extension is the folder.
    if EDITORS.contains(&segment.app.as_str()) {
        let title = segment.title.replace(" — ", " - ").replace(" – ", " - ");
        for piece in title.split(" - ") {
            let piece = piece.trim();
            if FOLDER_NAME.is_match(piece)
                && !piece.eq_ignore_ascii_case(&segment.app)
                && !GENERIC_TITLES.contains(&piece.to_ascii_lowercase().as_str())
            {
                out.push((piece.to_string(), piece.to_string()));
            }
        }
    }
    out
}

#[derive(Default)]
struct Cluster {
    ms: u64,
    names: HashMap<String, u64>,
    evidence: HashMap<String, u64>,
}

/// How discovery compares names: "Acme Web", "acme-web", and "acme_web" are
/// one project.
pub fn name_key(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The names that already are a project or a project's hint, normalized.
pub fn known_names(project_names: &[String], rule_patterns: &[String]) -> HashSet<String> {
    let mut known = HashSet::new();
    for name in project_names {
        known.insert(name_key(name));
    }
    for pattern in rule_patterns {
        if let Some(last) = pattern.trim_end_matches('/').rsplit('/').next() {
            known.insert(name_key(last));
        }
        known.insert(name_key(pattern));
    }
    known
}

/// Proposes projects from recurring repo, folder, and forge names in the
/// capture, skipping names that already are a project (or a project's hint)
/// and ones the user dismissed. Runs over local data only.
pub fn suggest_projects(
    segments: &[ActivitySegment],
    known: &HashSet<String>,
    skipped: &HashSet<String>,
) -> Vec<ProjectSuggestion> {
    let mut clusters: HashMap<String, Cluster> = HashMap::new();
    for segment in segments {
        let ms = segment_ms(segment);
        if ms == 0 {
            continue;
        }
        let mut counted = HashSet::new();
        for (name, evidence) in candidates(segment) {
            let key = name.to_lowercase();
            let cluster = clusters.entry(key.clone()).or_default();
            if counted.insert(key) {
                cluster.ms += ms;
                *cluster.names.entry(name).or_default() += ms;
            }
            *cluster.evidence.entry(evidence).or_default() += ms;
        }
    }

    let mut out: Vec<ProjectSuggestion> = clusters
        .into_iter()
        .filter(|(key, cluster)| {
            cluster.ms >= DISCOVERY_MIN_MS
                && !known.contains(&name_key(key))
                && !skipped.contains(key)
        })
        .map(|(key, cluster)| {
            let name = cluster
                .names
                .into_iter()
                .max_by_key(|(name, ms)| (*ms, name.clone()))
                .map(|(name, _)| name)
                .unwrap_or_else(|| key.clone());
            let mut evidence: Vec<(String, u64)> = cluster.evidence.into_iter().collect();
            evidence.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            ProjectSuggestion {
                key,
                name,
                ms: cluster.ms,
                evidence: evidence.into_iter().take(3).map(|(e, _)| e).collect(),
            }
        })
        .collect();
    out.sort_by(|a, b| b.ms.cmp(&a.ms).then_with(|| a.key.cmp(&b.key)));
    out.truncate(DISCOVERY_LIMIT);
    out
}

/// A segment's own duration.
pub fn segment_ms(segment: &ActivitySegment) -> u64 {
    segment
        .ended_at
        .unwrap_or(segment.started_at)
        .saturating_sub(segment.started_at)
}

// --- CSV import -------------------------------------------------------------------

/// Minimal RFC 4180: quoted fields, doubled quotes, CRLF or LF line ends.
pub fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    field.push('"');
                    chars.next();
                }
                '"' => quoted = false,
                _ => field.push(c),
            }
            continue;
        }
        match c {
            '"' => quoted = true,
            ',' => row.push(std::mem::take(&mut field)),
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows.retain(|row| row.iter().any(|field| !field.trim().is_empty()));
    rows
}

/// `50h` / `50 hours` / `50` are hours; `$5,000` or `5000 USD` an amount.
pub fn parse_budget(text: &str) -> Option<(&'static str, f64)> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let lower = text.to_lowercase();
    let is_amount = text.starts_with(['$', '€', '£', '¥'])
        || text.ends_with(['$', '€', '£', '¥'])
        || lower.ends_with("usd")
        || lower.ends_with("eur")
        || lower.ends_with("gbp");
    let number: String = text
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let value: f64 = number.parse().ok().filter(|v: &f64| *v > 0.0)?;
    Some((if is_amount { "amount" } else { "hours" }, value))
}

/// `YYYY-MM-DD` as local midnight.
pub fn parse_due(text: &str) -> Option<u64> {
    use chrono::{NaiveDate, TimeZone};
    let date = NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").ok()?;
    let midnight = date.and_hms_opt(0, 0, 0)?;
    let local = chrono::Local.from_local_datetime(&midnight).earliest()?;
    u64::try_from(local.timestamp_millis()).ok()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub created: u32,
    pub clients_created: u32,
    /// "Row 3: Aevio already exists", one line per row not imported.
    pub skipped: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_split_on_every_separator() {
        let hints = parse_hints("~/Code/Aevio, figma.com\nfigma.com ; rust");
        assert_eq!(hints.len(), 3);
        assert_eq!(
            hints[0].rule.as_ref().map(|(kind, _)| *kind),
            Some("path_prefix")
        );
        assert_eq!(
            hints[1].rule.as_ref().map(|(kind, _)| *kind),
            Some("domain")
        );
        assert_eq!(
            hints[2].rule.as_ref().map(|(kind, _)| *kind),
            Some("title_contains")
        );
    }

    #[test]
    fn a_short_keyword_is_not_a_rule() {
        assert!(parse_hints("ab").first().unwrap().rule.is_none());
    }

    #[test]
    fn names_normalize_to_one_project() {
        assert_eq!(name_key("Acme Web"), name_key("acme-web"));
        assert_eq!(name_key("acme_web"), name_key("Acme Web"));
    }

    #[test]
    fn known_names_covers_projects_and_hints() {
        let known = known_names(
            &["Acme Web".to_string()],
            &["github.com/acme/pantry-tracker".to_string()],
        );
        assert!(known.contains(&name_key("Acme Web")));
        assert!(known.contains(&name_key("pantry-tracker")));
    }
}
