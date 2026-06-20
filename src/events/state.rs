/// Application state machine for the steno-core lifecycle.
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// High-level application state describing what the app is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppState {
    /// Initially idle — no meeting detected, not capturing.
    Idle,

    /// A meeting candidate was detected (foreground app match + optional audio activity).
    /// In `armed` mode, the app pauses here and waits for user confirmation.
    MeetingCandidate {
        detected_at: Instant,
        app_name: &'static str,
    },

    /// Actively capturing audio (mic + system audio if available).
    Capturing {
        started_at: Instant,
    },

    /// Capturing and simultaneously transcribing.
    CapturingAndTranscribing {
        started_at: Instant,
    },
}

impl AppState {
    pub fn is_idle(&self) -> bool {
        matches!(self, AppState::Idle)
    }

    pub fn is_capturing(&self) -> bool {
        matches!(self, AppState::Capturing { .. } | AppState::CapturingAndTranscribing { .. })
    }

    pub fn elapsed(&self) -> Option<std::time::Duration> {
        match self {
            AppState::Capturing { started_at }
            | AppState::CapturingAndTranscribing { started_at }
            | AppState::MeetingCandidate { detected_at: started_at, .. } => {
                Some(started_at.elapsed())
            }
            _ => None,
        }
    }

    pub fn transition_to_capturing(&mut self) {
        *self = AppState::Capturing {
            started_at: Instant::now(),
        };
    }

    pub fn transition_to_idle(&mut self) {
        *self = AppState::Idle;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_starts_idle() {
        let state = AppState::default();
        assert!(state.is_idle());
        assert!(!state.is_capturing());
        assert!(state.elapsed().is_none());
    }

    #[test]
    fn test_transition_to_capturing() {
        let mut state = AppState::Idle;
        state.transition_to_capturing();
        assert!(state.is_capturing());
        assert!(!state.is_idle());
        assert!(state.elapsed().is_some());
    }

    #[test]
    fn test_transition_back_to_idle() {
        let mut state = AppState::Idle;
        state.transition_to_capturing();
        state.transition_to_idle();
        assert!(state.is_idle());
        assert!(!state.is_capturing());
    }

    #[test]
    fn test_elapsed_increases() {
        let mut state = AppState::Idle;
        state.transition_to_capturing();
        let elapsed_1 = state.elapsed().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed_2 = state.elapsed().unwrap();
        assert!(elapsed_2 > elapsed_1);
    }
}
