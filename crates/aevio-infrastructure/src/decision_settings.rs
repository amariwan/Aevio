use std::fs;
use std::path::{Path, PathBuf};

use aevio_core::decision::ProviderMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DecisionSettings {
    pub enabled: bool,
    pub mode: ProviderMode,
    pub laya_endpoint: String,
    pub laya_checkpoint: String,
    pub jev_model: String,
    pub allow_cloud: bool,
    pub timeout_ms: u32,
    pub minimum_confidence_percent: u8,
    pub human_review: bool,
}

impl Default for DecisionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: ProviderMode::Auto,
            laya_endpoint: "http://127.0.0.1:8000/predict".into(),
            laya_checkpoint: "typed-decisions".into(),
            jev_model: "typesafe/jev-1.14".into(),
            allow_cloud: false,
            timeout_ms: 2_000,
            minimum_confidence_percent: 80,
            human_review: true,
        }
    }
}

impl DecisionSettings {
    pub fn normalized(mut self) -> Self {
        self.laya_endpoint = self.laya_endpoint.trim().chars().take(512).collect();
        self.laya_checkpoint = self.laya_checkpoint.trim().chars().take(96).collect();
        self.jev_model = self.jev_model.trim().chars().take(96).collect();
        self.timeout_ms = self.timeout_ms.clamp(100, 30_000);
        self.minimum_confidence_percent = self.minimum_confidence_percent.clamp(1, 100);
        if !self.allow_cloud {
            self.mode = match self.mode {
                ProviderMode::Cloud => ProviderMode::Auto,
                mode => mode,
            };
        }
        self
    }
}

pub struct DecisionSettingsStore {
    path: PathBuf,
    settings: DecisionSettings,
}

impl DecisionSettingsStore {
    pub fn load(dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|error| error.to_string())?;
        let path = dir.join("decision-settings.json");
        let settings: DecisionSettings = fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Ok(Self {
            path,
            settings: settings.normalized(),
        })
    }

    pub fn snapshot(&self) -> DecisionSettings {
        self.settings.clone()
    }

    pub fn set(&mut self, settings: DecisionSettings) -> Result<DecisionSettings, String> {
        let settings = settings.normalized();
        let bytes = serde_json::to_vec_pretty(&settings).map_err(|error| error.to_string())?;
        let temp = self.path.with_extension("json.tmp");
        fs::write(&temp, bytes).map_err(|error| error.to_string())?;
        fs::rename(&temp, &self.path).map_err(|error| error.to_string())?;
        self.settings = settings;
        Ok(self.snapshot())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_mode_is_not_enabled_by_normalization() {
        let settings = DecisionSettings {
            mode: ProviderMode::Cloud,
            ..Default::default()
        }
        .normalized();
        assert_eq!(settings.mode, ProviderMode::Auto);
    }
}
