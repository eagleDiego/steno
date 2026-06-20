/// Minimal configuration management for steno-core.
///
/// Uses serde + TOML for user-facing settings,
/// keyring crate for secure API key storage.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::detection::DetectionMode;

/// Top-level application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub capture: CaptureSettings,
    pub detection: DetectionSettings,
    pub inference: InferenceSettings,
    pub storage: StorageSettings,
    pub consent: ConsentSettings,
    pub ui: UiSettings,
}

// ── Audio capture settings ────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSettings {
    pub mic_enabled: bool,
    pub system_audio_enabled: bool,
    pub sample_rate: u32,
    pub channels: u16,
    pub channel_mode: String, // "separate" | "mixed" | "both"
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            mic_enabled: true,
            system_audio_enabled: false,
            sample_rate: 16000,
            channels: 1,
            channel_mode: "separate".into(),
        }
    }
}

/// Meeting detection settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionSettings {
    pub mode: DetectionMode,
    pub allowlist: Vec<String>,
    pub poll_interval_ms: u64,
    pub candidate_poll_interval_ms: u64,
}

impl Default for DetectionSettings {
    fn default() -> Self {
        Self {
            mode: DetectionMode::Armed,
            allowlist: crate::detection::default_allowlist(),
            poll_interval_ms: 1000,
            candidate_poll_interval_ms: 500,
        }
    }
}

/// Inference endpoint settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceSettings {
    pub endpoint_url: String,
    pub model: String,
    pub api_key_stored: bool,
    pub chunk_duration_secs: u64,
    pub overlap_secs: u64,
}

impl Default for InferenceSettings {
    fn default() -> Self {
        Self {
            endpoint_url: "https://openrouter.ai/api/v1".into(),
            model: "openai/gpt-4o-mini-transcribe".into(),
            api_key_stored: false,
            chunk_duration_secs: 55,
            overlap_secs: 1,
        }
    }
}

/// Storage and retention settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSettings {
    pub output_dir: PathBuf,
    pub retention_policy: RetentionPolicy,
    pub retention_days: u32,
}

impl Default for StorageSettings {
    fn default() -> Self {
        let data_dir = directories::ProjectDirs::from("com", "nerd-aero", "steno-core")
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("./steno-output"));

        Self {
            output_dir: data_dir.join("sessions"),
            retention_policy: RetentionPolicy::Keep,
            retention_days: 30,
        }
    }
}

/// Retention policy options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetentionPolicy {
    Keep,
    DeleteAfterTranscription,
    DeleteAfterDays(u32),
}

/// Consent settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentSettings {
    pub require_ack: bool,
    pub banner_text: String,
    pub play_chime: bool,
}

impl Default for ConsentSettings {
    fn default() -> Self {
        Self {
            require_ack: true,
            banner_text: "This meeting may be recorded for transcription purposes.".into(),
            play_chime: true,
        }
    }
}

/// UI settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    pub notifications_enabled: bool,
    pub minimize_to_tray: bool,
    pub start_minimized: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            notifications_enabled: true,
            minimize_to_tray: true,
            start_minimized: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert!(config.capture.mic_enabled);
        assert!(!config.capture.system_audio_enabled);
        assert_eq!(config.capture.sample_rate, 16000);
        assert_eq!(config.detection.mode, DetectionMode::Armed);
        assert_eq!(
            config.inference.endpoint_url,
            "https://openrouter.ai/api/v1"
        );
        assert!(config.consent.require_ack);
    }

    #[test]
    fn test_config_serde_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(deserialized.capture.mic_enabled, config.capture.mic_enabled);
        assert_eq!(deserialized.detection.mode, config.detection.mode);
        assert_eq!(
            deserialized.inference.endpoint_url,
            config.inference.endpoint_url
        );
    }

    #[test]
    fn test_retention_policy_serde() {
        let policies = vec![
            RetentionPolicy::Keep,
            RetentionPolicy::DeleteAfterTranscription,
        ];
        for policy in policies {
            let json_str = serde_json::to_string(&policy).unwrap();
            let deserialized: RetentionPolicy = serde_json::from_str(&json_str).unwrap();
            assert_eq!(deserialized, policy);
        }
    }

    #[test]
    fn test_allowlist_default() {
        let config = Config::default();
        assert!(config.detection.allowlist.contains(&"zoom".to_string()));
        assert!(config.detection.allowlist.contains(&"Teams".to_string()));
    }
}
