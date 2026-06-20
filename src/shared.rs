/// Shared consent and storage types.

pub mod consent {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    /// State of the consent gate.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum ConsentState {
        Pending,
        Acknowledged,
        BypassedNoAckRequired,
    }

    /// An entry in the consent log.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ConsentLogEntry {
        pub session_id: Uuid,
        pub timestamp: std::time::SystemTime,
        pub banner_text: String,
        pub acknowledgment_method: String,
        pub app_version: String,
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_consent_state_transitions() {
            let state = ConsentState::Pending;
            assert_eq!(state, ConsentState::Pending);
            assert_ne!(state, ConsentState::Acknowledged);
        }

        #[test]
        fn test_consent_log_entry_creation() {
            let entry = ConsentLogEntry {
                session_id: Uuid::new_v4(),
                timestamp: std::time::SystemTime::now(),
                banner_text: "This meeting may be recorded".into(),
                acknowledgment_method: "click".into(),
                app_version: "0.1.0".into(),
            };
            assert_eq!(entry.app_version, "0.1.0");
            assert_eq!(entry.acknowledgment_method, "click");
        }
    }
}

pub mod storage {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    /// A recorded meeting session.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Session {
        pub id: Uuid,
        pub started_at: std::time::SystemTime,
        pub ended_at: Option<std::time::SystemTime>,
        pub detection_mode: crate::detection::DetectionMode,
        pub detection_app: Option<String>,
        pub mic_wav_path: Option<std::path::PathBuf>,
        pub system_wav_path: Option<std::path::PathBuf>,
        pub mixed_wav_path: Option<std::path::PathBuf>,
        pub md_transcript_path: Option<std::path::PathBuf>,
        pub json_transcript_path: Option<std::path::PathBuf>,
        pub consent_log: Option<crate::shared::consent::ConsentLogEntry>,
        pub retention_policy: crate::config::RetentionPolicy,
        pub app_version: String,
    }

    /// Output paths for a finalized session.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SessionOutputPaths {
        pub mic_wav: Option<std::path::PathBuf>,
        pub system_wav: Option<std::path::PathBuf>,
        pub mixed_wav: Option<std::path::PathBuf>,
        pub md_transcript: Option<std::path::PathBuf>,
        pub json_transcript: Option<std::path::PathBuf>,
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::config::RetentionPolicy;
        use crate::detection::DetectionMode;

        #[test]
        fn test_session_creation() {
            let session = Session {
                id: Uuid::new_v4(),
                started_at: std::time::SystemTime::now(),
                ended_at: None,
                detection_mode: DetectionMode::Armed,
                detection_app: Some("zoom.us".into()),
                mic_wav_path: None,
                system_wav_path: None,
                mixed_wav_path: None,
                md_transcript_path: None,
                json_transcript_path: None,
                consent_log: None,
                retention_policy: RetentionPolicy::Keep,
                app_version: "0.1.0".into(),
            };
            assert!(session.ended_at.is_none());
            assert_eq!(session.detection_mode, DetectionMode::Armed);
        }

        #[test]
        fn test_session_output_paths_defaults() {
            let paths = SessionOutputPaths {
                mic_wav: Some(std::path::PathBuf::from("output/mic.wav")),
                system_wav: None,
                mixed_wav: None,
                md_transcript: None,
                json_transcript: None,
            };
            assert!(paths.mic_wav.is_some());
            assert!(paths.system_wav.is_none());
        }
    }
}
