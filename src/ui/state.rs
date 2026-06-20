/// Shared application state types for Tauri integration.
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::audio::CaptureConfig;
use crate::detection::DetectionMode;
use crate::events::{AppEvent, EventBus, UiState};

/// Thread-safe application state shared across all Tauri commands.
pub struct ManagedState {
    pub event_bus: EventBus,
    pub config: RwLock<CaptureConfig>,
    pub detection_mode: RwLock<DetectionMode>,
    pub ui_state: RwLock<UiState>,
    pub session_active: RwLock<bool>,
    pub capture_started_at: RwLock<Option<std::time::Instant>>,
}

impl ManagedState {
    pub fn new() -> Self {
        Self {
            event_bus: EventBus::new(256),
            config: RwLock::new(CaptureConfig::default()),
            detection_mode: RwLock::new(DetectionMode::Armed),
            ui_state: RwLock::new(UiState::Idle),
            session_active: RwLock::new(false),
            capture_started_at: RwLock::new(None),
        }
    }
}

/// Icon state derived from the current application state.
/// Used by the system tray to update the icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayIconState {
    Idle,
    MeetingDetected,
    Capturing,
    CapturingAndTranscribing,
}

impl From<UiState> for TrayIconState {
    fn from(state: UiState) -> Self {
        match state {
            UiState::Idle => TrayIconState::Idle,
            UiState::MeetingDetected => TrayIconState::MeetingDetected,
            UiState::ArmingAwaitingConfirm => TrayIconState::MeetingDetected,
            UiState::Capturing => TrayIconState::Capturing,
            UiState::CapturingAndTranscribing => TrayIconState::CapturingAndTranscribing,
        }
    }
}
