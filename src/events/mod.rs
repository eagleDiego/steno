/// Event types used throughout the application for inter-module communication.
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::detection::{DetectionEvent};
use uuid::Uuid;

/// The central event type for the steno-core event bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    /// Capture lifecycle events
    CaptureStarted {
        session_id: Uuid,
        mic_active: bool,
        system_audio_active: bool,
    },
    CaptureStopped {
        session_id: Uuid,
        duration_secs: f64,
    },

    /// Detection lifecycle events (wraps DetectionEvent)
    MeetingDetected(DetectionEvent),

    /// UI state change requests
    UiStateChange(UiState),

    /// Device changes
    DeviceChanged { device_type: String, event: String },

    /// Error events that should be surfaced to the UI
    ErrorOccurred {
        module: String,
        message: String,
        severity: ErrorSeverity,
    },

    /// System audio availability changed
    SystemAudioAvailability(bool),

    /// Application lifecycle
    AppShutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Top-level UI state the application can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiState {
    Idle,
    MeetingDetected,
    ArmingAwaitingConfirm,
    Capturing,
    CapturingAndTranscribing,
}

/// A shared event bus using tokio broadcast channels.
#[derive(Debug, Clone)]
pub struct EventBus {
    tx: broadcast::Sender<AppEvent>,
}

impl EventBus {
    /// Create a new event bus with the given channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Publish an event to all subscribers.
    pub fn publish(&self, event: AppEvent) {
        let _ = self.tx.send(event);
    }

    /// Subscribe to receive events.
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.tx.subscribe()
    }

    /// Get the sender for direct use in async contexts.
    pub fn sender(&self) -> broadcast::Sender<AppEvent> {
        self.tx.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus_publish_subscribe() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();

        bus.publish(AppEvent::CaptureStarted {
            session_id: Uuid::new_v4(),
            mic_active: true,
            system_audio_active: false,
        });

        match rx.try_recv() {
            Ok(AppEvent::CaptureStarted { mic_active, .. }) => assert!(mic_active),
            other => panic!("unexpected event: {:?}", other),
        }
    }

    #[test]
    fn test_event_bus_multiple_subscribers() {
        let bus = EventBus::new(16);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        bus.publish(AppEvent::AppShutdown);

        assert!(matches!(rx1.try_recv(), Ok(AppEvent::AppShutdown)));
        assert!(matches!(rx2.try_recv(), Ok(AppEvent::AppShutdown)));
    }

    #[test]
    fn test_ui_state_transitions() {
        let states = vec![
            UiState::Idle,
            UiState::MeetingDetected,
            UiState::ArmingAwaitingConfirm,
            UiState::Capturing,
            UiState::CapturingAndTranscribing,
        ];
        for state in &states {
            let event = AppEvent::UiStateChange(*state);
            match event {
                AppEvent::UiStateChange(s) => assert_eq!(&s, state),
                _ => panic!("wrong variant"),
            }
        }
    }

    #[test]
    fn test_error_severity_order() {
        assert!((ErrorSeverity::Info as u8) < (ErrorSeverity::Warning as u8));
        assert!((ErrorSeverity::Warning as u8) < (ErrorSeverity::Error as u8));
        assert!((ErrorSeverity::Error as u8) < (ErrorSeverity::Critical as u8));
    }
}
