use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    ClickUp,
    Linear,
    Asana,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ClickUp => "clickup",
            Self::Linear => "linear",
            Self::Asana => "asana",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SyncDirection {
    Pull,
    Push,
    Bidirectional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCapabilities {
    pub task_sync: bool,
    pub label_sync: bool,
    pub time_pull: bool,
    pub time_push: bool,
}

pub fn capabilities(provider: Provider) -> ProviderCapabilities {
    match provider {
        Provider::ClickUp => ProviderCapabilities {
            task_sync: true,
            label_sync: true,
            time_pull: true,
            time_push: true,
        },
        Provider::Linear => ProviderCapabilities {
            task_sync: true,
            label_sync: true,
            time_pull: false,
            time_push: false,
        },
        Provider::Asana => ProviderCapabilities {
            task_sync: true,
            label_sync: true,
            time_pull: true,
            time_push: true,
        },
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationConnection {
    pub id: String,
    pub provider: Provider,
    pub name: String,
    pub workspace_ref: String,
    pub secret_ref: String,
    pub direction: SyncDirection,
    pub enabled: bool,
    pub last_synced_at: Option<u64>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalMapping {
    pub connection_id: String,
    pub local_kind: String,
    pub local_id: String,
    pub external_kind: String,
    pub external_id: String,
    pub external_updated_at: Option<u64>,
    pub local_updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRun {
    pub id: String,
    pub connection_id: String,
    pub direction: SyncDirection,
    pub status: String,
    pub pulled: u64,
    pub pushed: u64,
    pub conflicts: u64,
    pub error: Option<String>,
    pub started_at: u64,
    pub finished_at: Option<u64>,
}
