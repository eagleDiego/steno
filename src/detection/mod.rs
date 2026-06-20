pub mod audio_activity;
pub mod mode;
pub mod process_monitor;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use crate::error::DetectionError;

// ── Detection event ──────────────────────────────────────────────────

/// Events emitted by the detection engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionEvent {
    MeetingStarted {
        app_name: String,
        detected_at: SystemTime,
        detection_mode: DetectionMode,
    },
    MeetingEnded {
        app_name: String,
        ended_at: SystemTime,
    },
}

// ── Detection mode ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetectionMode {
    /// User starts/stops capture explicitly.
    Manual,
    /// Capture begins automatically when criteria are met.
    Auto,
    /// Auto-detect but require one click to confirm.
    Armed,
}

impl std::fmt::Display for DetectionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectionMode::Manual => write!(f, "manual"),
            DetectionMode::Auto => write!(f, "auto"),
            DetectionMode::Armed => write!(f, "armed"),
        }
    }
}

// ── DetectionEngine trait ────────────────────────────────────────────

/// Platform-specific process/foreground-window detection.
///
/// Implementations:
/// - macOS: NSWorkspace.shared.frontmostApplication / AX API
/// - Windows: GetForegroundWindow + GetWindowText
/// - Linux: _NET_ACTIVE_WINDOW via X11 or PipeWire session info
#[async_trait]
pub trait DetectionEngine: Send + Sync + 'static {
    /// Check if a meeting is currently active.
    /// Returns None if uncertain, Some(true/false) if confident.
    async fn is_meeting_active(&self, allowlist: &[String])
        -> Result<Option<bool>, DetectionError>;

    /// Get the name of the foreground application, if identifiable.
    async fn foreground_app(&self) -> Result<Option<String>, DetectionError>;

    /// Set the detection mode.
    fn set_mode(&mut self, mode: DetectionMode);

    /// Current detection mode.
    fn mode(&self) -> DetectionMode;
}

// ── AudioActivitySensor trait ────────────────────────────────────────

/// Checks whether any audio input device is currently producing signal.
#[async_trait]
pub trait AudioActivitySensor: Send + Sync + 'static {
    /// Returns true if an audio input device appears to be active.
    async fn is_audio_input_active(&self) -> Result<bool, DetectionError>;
}

// ── DetectionManager ─────────────────────────────────────────────────

/// Orchestrates the detection lifecycle: polls the engine,
/// checks audio activity, and emits events.
pub struct DetectionManager {
    engine: Box<dyn DetectionEngine>,
    sensor: Box<dyn AudioActivitySensor>,
    mode: DetectionMode,
    allowlist: Vec<String>,
    meeting_active: bool,
    last_event_sent: Option<DetectionEvent>,
}

impl DetectionManager {
    /// Create a new detection manager.
    pub fn new(
        engine: Box<dyn DetectionEngine>,
        sensor: Box<dyn AudioActivitySensor>,
        allowlist: Vec<String>,
    ) -> Self {
        let mode = engine.mode();
        Self {
            engine,
            sensor,
            mode,
            allowlist,
            meeting_active: false,
            last_event_sent: None,
        }
    }

    /// Run a single detection check. Returns an event if the state changed.
    pub async fn check(&mut self) -> Result<Option<DetectionEvent>, DetectionError> {
        let meeting = self.engine.is_meeting_active(&self.allowlist).await?;

        match meeting {
            Some(true) => {
                if !self.meeting_active {
                    self.meeting_active = true;
                    let app = self
                        .engine
                        .foreground_app()
                        .await?
                        .unwrap_or_else(|| "unknown".to_string());
                    let event = DetectionEvent::MeetingStarted {
                        app_name: app.clone(),
                        detected_at: SystemTime::now(),
                        detection_mode: self.mode,
                    };
                    self.last_event_sent = Some(event.clone());
                    return Ok(Some(event));
                }
            }
            Some(false) => {
                if self.meeting_active {
                    self.meeting_active = false;
                    let app = match &self.last_event_sent {
                        Some(DetectionEvent::MeetingStarted { app_name, .. }) => app_name.clone(),
                        _ => "unknown".to_string(),
                    };
                    let event = DetectionEvent::MeetingEnded {
                        app_name: app,
                        ended_at: SystemTime::now(),
                    };
                    self.last_event_sent = Some(event.clone());
                    return Ok(Some(event));
                }
            }
            None => {
                // Uncertain — keep current state
            }
        }

        Ok(None)
    }

    /// Update the allowlist.
    pub fn set_allowlist(&mut self, allowlist: Vec<String>) {
        self.allowlist = allowlist;
    }

    /// Set the detection mode on both manager and engine.
    pub fn set_mode(&mut self, mode: DetectionMode) {
        self.mode = mode;
        self.engine.set_mode(mode);
    }

    /// Current detection mode.
    pub fn mode(&self) -> DetectionMode {
        self.mode
    }

    /// Whether a meeting is currently detected as active.
    pub fn is_meeting_active(&self) -> bool {
        self.meeting_active
    }

    /// Get a reference to the underlying engine.
    pub fn engine(&self) -> &dyn DetectionEngine {
        &*self.engine
    }
}

/// Default allowlist for meeting applications.
pub fn default_allowlist() -> Vec<String> {
    vec![
        "zoom".to_string(),
        "zoom.us".to_string(),
        "Teams".to_string(),
        "ms-teams".to_string(),
        "Webex".to_string(),
        "slack".to_string(),
        "Slack".to_string(),
        "Google Meet".to_string(),
        "discord".to_string(),
        "Discord".to_string(),
        "Skype".to_string(),
        "telegram".to_string(),
        "WhatsApp".to_string(),
        "Chrome".to_string(),
        "Firefox".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct MockEngine {
        mode: std::sync::Mutex<DetectionMode>,
        foreground: &'static str,
    }

    impl MockEngine {
        fn new(foreground: &'static str) -> Self {
            Self {
                mode: std::sync::Mutex::new(DetectionMode::Auto),
                foreground,
            }
        }
    }

    #[async_trait]
    impl DetectionEngine for MockEngine {
        async fn is_meeting_active(
            &self,
            allowlist: &[String],
        ) -> Result<Option<bool>, DetectionError> {
            Ok(Some(allowlist.iter().any(|a| {
                self.foreground.to_lowercase().contains(&a.to_lowercase())
            })))
        }

        async fn foreground_app(&self) -> Result<Option<String>, DetectionError> {
            Ok(Some(self.foreground.to_string()))
        }

        fn set_mode(&mut self, mode: DetectionMode) {
            *self.mode.lock().unwrap() = mode;
        }

        fn mode(&self) -> DetectionMode {
            *self.mode.lock().unwrap()
        }
    }

    struct MockSensor;

    #[async_trait]
    impl AudioActivitySensor for MockSensor {
        async fn is_audio_input_active(&self) -> Result<bool, DetectionError> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn test_detection_manager_detects_meeting() {
        let engine = Box::new(MockEngine::new("zoom.us"));
        let sensor = Box::new(MockSensor);
        let allowlist = default_allowlist();

        let mut manager = DetectionManager::new(engine, sensor, allowlist);

        // Should detect zoom
        let event = manager.check().await.unwrap();
        assert!(event.is_some());
        match event.unwrap() {
            DetectionEvent::MeetingStarted { app_name, .. } => {
                assert_eq!(app_name, "zoom.us");
            }
            _ => panic!("expected MeetingStarted"),
        }

        assert!(manager.is_meeting_active());
    }

    #[tokio::test]
    async fn test_detection_manager_no_false_positive() {
        let engine = Box::new(MockEngine::new("terminal"));
        let sensor = Box::new(MockSensor);
        let allowlist = default_allowlist();

        let mut manager = DetectionManager::new(engine, sensor, allowlist);

        let event = manager.check().await.unwrap();
        assert!(event.is_none());
        assert!(!manager.is_meeting_active());
    }

    #[tokio::test]
    async fn test_detection_manager_meeting_end() {
        struct ToggleEngine {
            counter: std::sync::Mutex<u32>,
        }

        #[async_trait]
        impl DetectionEngine for ToggleEngine {
            async fn is_meeting_active(
                &self,
                allowlist: &[String],
            ) -> Result<Option<bool>, DetectionError> {
                let mut c = self.counter.lock().unwrap();
                *c += 1;
                Ok(Some(*c <= 2)) // active for first 2 calls
            }

            async fn foreground_app(&self) -> Result<Option<String>, DetectionError> {
                Ok(Some("zoom.us".to_string()))
            }

            fn set_mode(&mut self, _mode: DetectionMode) {}
            fn mode(&self) -> DetectionMode {
                DetectionMode::Auto
            }
        }

        let engine = Box::new(ToggleEngine {
            counter: std::sync::Mutex::new(0),
        });
        let sensor = Box::new(MockSensor);
        let allowlist = default_allowlist();

        let mut manager = DetectionManager::new(engine, sensor, allowlist);

        // First check: meeting starts
        let event1 = manager.check().await.unwrap();
        assert!(matches!(
            event1,
            Some(DetectionEvent::MeetingStarted { .. })
        ));

        // Second check: still meeting
        let event2 = manager.check().await.unwrap();
        assert!(event2.is_none());

        // Third check: meeting ended
        let event3 = manager.check().await.unwrap();
        assert!(matches!(event3, Some(DetectionEvent::MeetingEnded { .. })));

        assert!(!manager.is_meeting_active());
    }

    #[test]
    fn test_default_allowlist() {
        let list = default_allowlist();
        assert!(!list.is_empty());
        assert!(list.contains(&"zoom".to_string()));
        assert!(list.contains(&"Teams".to_string()));
    }

    #[test]
    fn test_detection_mode_display() {
        assert_eq!(DetectionMode::Manual.to_string(), "manual");
        assert_eq!(DetectionMode::Auto.to_string(), "auto");
        assert_eq!(DetectionMode::Armed.to_string(), "armed");
    }

    #[test]
    fn test_detection_mode_equality() {
        assert_eq!(DetectionMode::Manual, DetectionMode::Manual);
        assert_ne!(DetectionMode::Manual, DetectionMode::Auto);
    }
}
