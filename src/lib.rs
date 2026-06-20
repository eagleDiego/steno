//! # steno-core
//!
//! Cross-platform meeting audio capture and relay core library.
//!
//! This library provides the core abstractions and implementations for:
//!
//! - **Audio capture**: Cross-platform microphone input via `cpal`, platform-specific
//!   system audio capture on macOS (CoreAudio/SCK), Windows (WASAPI loopback), and
//!   Linux (PipeWire/PulseAudio).
//! - **Meeting detection**: Foreground window monitoring + audio activity sensor
//!   in three modes (manual, auto, armed).
//! - **Event system**: Typed event bus via tokio broadcast channels.
//! - **UI integration**: Tauri commands and system tray setup for the app shell.
//! - **Configuration**: TOML-based config with serde, keychain-backed secrets.

pub mod audio;
pub mod config;
pub mod detection;
pub mod error;
pub mod events;
pub mod shared;
pub mod ui;

// Re-export key types for convenience.
pub use audio::{
    AudioCaptureBackend, AudioCaptureManager, AudioPacket, CaptureCapabilities,
    CaptureConfig, StreamId,
};
pub use error::CaptureError;
pub use config::Config;
pub use detection::{
    AudioActivitySensor, DetectionEngine, DetectionEvent, DetectionManager, DetectionMode,
};
pub use error::AppError;
pub use events::{AppEvent, EventBus, UiState};
pub use shared::{consent, storage};