/// Tauri commands — the IPC bridge between the Svelte frontend and Rust backend.
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::audio::{AudioCaptureManager, CaptureCapabilities, CaptureConfig};
use crate::detection::{DetectionManager, DetectionMode};
use crate::error::AppError;
use crate::events::{AppEvent, EventBus, UiState};

/// Shared application state managed by Tauri.
pub struct TauriAppState {
    pub event_bus: EventBus,
    pub capture_manager: RwLock<Option<AudioCaptureManager>>,
    pub detection_manager: RwLock<Option<DetectionManager>>,
    pub config: RwLock<CaptureConfig>,
    pub is_capturing: RwLock<bool>,
}

impl TauriAppState {
    pub fn new() -> Self {
        Self {
            event_bus: EventBus::new(256),
            capture_manager: RwLock::new(None),
            detection_manager: RwLock::new(None),
            config: RwLock::new(CaptureConfig::default()),
            is_capturing: RwLock::new(false),
        }
    }
}

// ── Tauri command handlers ───────────────────────────────────────────

/// Start audio capture with the current configuration.
#[cfg(feature = "ui")]
#[tauri::command]
pub async fn start_capture(state: tauri::State<'_, TauriAppState>) -> Result<(), AppError> {
    let config = state.config.read().await.clone();
    let mut manager = AudioCaptureManager::new(config);

    // Load platform-specific backends here
    // On the current platform, the appropriate backend is selected
    // via cfg attributes in the audio module.

    manager.start().await?;

    let mut is_capturing = state.is_capturing.write().await;
    *is_capturing = true;

    state.event_bus.publish(AppEvent::CaptureStarted {
        session_id: uuid::Uuid::new_v4(),
        mic_active: true,
        system_audio_active: false,
    });

    let mut cm = state.capture_manager.write().await;
    *cm = Some(manager);

    Ok(())
}

/// Stop audio capture.
#[cfg(feature = "ui")]
#[tauri::command]
pub async fn stop_capture(state: tauri::State<'_, TauriAppState>) -> Result<(), AppError> {
    let mut cm = state.capture_manager.write().await;
    if let Some(ref mut manager) = *cm {
        manager.stop().await?;
    }
    *cm = None;

    let mut is_capturing = state.is_capturing.write().await;
    *is_capturing = false;

    state.event_bus.publish(AppEvent::CaptureStopped {
        session_id: uuid::Uuid::new_v4(),
        duration_secs: 0.0,
    });

    Ok(())
}

/// Get current capture capabilities.
#[cfg(feature = "ui")]
#[tauri::command]
pub async fn get_capabilities(
    state: tauri::State<'_, TauriAppState>,
) -> Result<CaptureCapabilities, AppError> {
    let cm = state.capture_manager.read().await;
    match &*cm {
        Some(manager) => Ok(manager.capabilities()),
        None => Ok(CaptureCapabilities {
            mic_available: false,
            system_audio_available: false,
            device_change_events: false,
            max_sample_rate: 0,
            supported_sample_rates: vec![],
            supported_channel_modes: vec![],
        }),
    }
}

/// Update detection mode.
#[cfg(feature = "ui")]
#[tauri::command]
pub async fn set_detection_mode(
    state: tauri::State<'_, TauriAppState>,
    mode: DetectionMode,
) -> Result<(), AppError> {
    let mut dm = state.detection_manager.write().await;
    if let Some(ref mut manager) = *dm {
        manager.set_mode(mode);
    }
    Ok(())
}

/// Get current detection mode.
#[cfg(feature = "ui")]
#[tauri::command]
pub async fn get_detection_mode(
    state: tauri::State<'_, TauriAppState>,
) -> Result<DetectionMode, AppError> {
    let dm = state.detection_manager.read().await;
    match &*dm {
        Some(manager) => Ok(manager.mode()),
        None => Ok(DetectionMode::Armed),
    }
}

/// Check if currently capturing.
#[cfg(feature = "ui")]
#[tauri::command]
pub async fn is_capturing(state: tauri::State<'_, TauriAppState>) -> Result<bool, AppError> {
    Ok(*state.is_capturing.read().await)
}
