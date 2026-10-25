//! Stable decision-provider selection shared by settings and services.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderMode {
    Auto,
    LocalOnly,
    Cloud,
    RulesOnly,
    Disabled,
}
