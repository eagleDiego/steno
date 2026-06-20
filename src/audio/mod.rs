pub mod mic;
pub mod wav_writer;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "linux")]
pub mod linux;

use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use tokio::sync::mpsc;

use crate::error::CaptureError;
pub use crate::error::CaptureError as AudioCaptureError;

// ── Capability types ────────────────────────────────────────────────

/// Capability flags returned by each platform backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureCapabilities {
    pub mic_available: bool,
    pub system_audio_available: bool,
    pub device_change_events: bool,
    pub max_sample_rate: u32,
    pub supported_sample_rates: Vec<u32>,
    pub supported_channel_modes: Vec<ChannelMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChannelMode {
    /// Mic + system as independent WAVs
    Separate,
    /// Single mixed WAV
    Mixed,
    /// Both separate and mixed
    Both,
}

// ── Configuration ──────────────────────────────────────────────────

/// Configuration for starting a capture session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureConfig {
    pub mic_enabled: bool,
    pub system_audio_enabled: bool,
    pub sample_rate: u32,
    pub channels: u16,
    pub channel_mode: ChannelMode,
    pub output_dir: std::path::PathBuf,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            mic_enabled: true,
            system_audio_enabled: false,
            sample_rate: 16000,
            channels: 1,
            channel_mode: ChannelMode::Separate,
            output_dir: std::path::PathBuf::from("."),
        }
    }
}

// ── Audio packet ─────────────────────────────────────────────────────

/// A real-time audio packet with timestamp.
#[derive(Debug, Clone)]
pub struct AudioPacket {
    pub timestamp: SystemTime,
    pub stream_id: StreamId,
    pub data: Bytes,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Identifier for a capture stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StreamId {
    Mic,
    System,
    Mixed,
}

// ── AudioCaptureBackend trait ────────────────────────────────────────

/// The central abstraction for audio capture.
/// Each platform (macOS, Windows, Linux) implements this trait
/// for system audio; mic capture is cross-platform via `cpal`.
#[async_trait]
pub trait AudioCaptureBackend: Send + Sync + 'static {
    /// Query the capabilities of this backend on the current system.
    fn capabilities(&self) -> CaptureCapabilities;

    /// Start capturing audio. Packets are pushed into the sender.
    async fn start(
        &mut self,
        config: CaptureConfig,
        packet_tx: mpsc::Sender<AudioPacket>,
    ) -> Result<(), CaptureError>;

    /// Stop capture and flush any remaining buffered frames.
    async fn stop(&mut self) -> Result<(), CaptureError>;

    /// Handle a device change mid-session (optional).
    async fn on_device_change(&mut self) -> Result<(), CaptureError> {
        Ok(())
    }
}

// ── AudioCaptureManager ──────────────────────────────────────────────

/// Manages the lifecycle of mic + system audio capture.
pub struct AudioCaptureManager {
    mic_backend: Option<Box<dyn AudioCaptureBackend>>,
    system_backend: Option<Box<dyn AudioCaptureBackend>>,
    config: CaptureConfig,
    packet_tx: Option<mpsc::Sender<AudioPacket>>,
    packet_rx: Option<mpsc::Receiver<AudioPacket>>,
}

impl AudioCaptureManager {
    /// Create a new capture manager with the given configuration.
    pub fn new(config: CaptureConfig) -> Self {
        let (tx, rx) = mpsc::channel(1000);
        Self {
            mic_backend: None,
            system_backend: None,
            config,
            packet_tx: Some(tx),
            packet_rx: Some(rx),
        }
    }

    /// Register mic backend.
    pub fn with_mic_backend(mut self, backend: Box<dyn AudioCaptureBackend>) -> Self {
        self.mic_backend = Some(backend);
        self
    }

    /// Register system audio backend.
    pub fn with_system_backend(mut self, backend: Box<dyn AudioCaptureBackend>) -> Self {
        self.system_backend = Some(backend);
        self
    }

    /// Get a shared receiver for capture packets.
    pub fn packet_receiver(&mut self) -> Option<mpsc::Receiver<AudioPacket>> {
        self.packet_rx.take()
    }

    /// Start both capture backends.
    pub async fn start(&mut self) -> Result<(), CaptureError> {
        let tx = self.packet_tx.as_ref().ok_or_else(|| {
            CaptureError::ChannelClosed("packet sender already taken".into())
        })?;

        if self.config.mic_enabled {
            if let Some(backend) = &mut self.mic_backend {
                backend
                    .start(self.config.clone(), tx.clone())
                    .await?;
            }
        }

        if self.config.system_audio_enabled {
            if let Some(backend) = &mut self.system_backend {
                backend
                    .start(self.config.clone(), tx.clone())
                    .await?;
            }
        }

        Ok(())
    }

    /// Stop both capture backends.
    pub async fn stop(&mut self) -> Result<(), CaptureError> {
        if let Some(backend) = &mut self.mic_backend {
            backend.stop().await?;
        }
        if let Some(backend) = &mut self.system_backend {
            backend.stop().await?;
        }
        Ok(())
    }

    /// Query combined capabilities from all registered backends.
    pub fn capabilities(&self) -> CaptureCapabilities {
        let mut caps = CaptureCapabilities {
            mic_available: false,
            system_audio_available: false,
            device_change_events: false,
            max_sample_rate: 0,
            supported_sample_rates: vec![],
            supported_channel_modes: vec![ChannelMode::Separate],
        };

        if let Some(ref backend) = self.mic_backend {
            let bcaps = backend.capabilities();
            caps.mic_available = bcaps.mic_available;
            caps.max_sample_rate = caps.max_sample_rate.max(bcaps.max_sample_rate);
            if !bcaps.supported_sample_rates.is_empty() {
                caps.supported_sample_rates = bcaps.supported_sample_rates.clone();
            }
        }

        if let Some(ref backend) = self.system_backend {
            let bcaps = backend.capabilities();
            caps.system_audio_available = bcaps.system_audio_available;
            caps.device_change_events = bcaps.device_change_events;
        }

        caps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mock backend for unit testing.
    struct MockBackend {
        caps: CaptureCapabilities,
        started: std::sync::atomic::AtomicBool,
    }

    impl MockBackend {
        fn new(caps: CaptureCapabilities) -> Self {
            Self {
                caps,
                started: std::sync::atomic::AtomicBool::new(false),
            }
        }
    }

    #[async_trait]
    impl AudioCaptureBackend for MockBackend {
        fn capabilities(&self) -> CaptureCapabilities {
            self.caps.clone()
        }

        async fn start(
            &mut self,
            _config: CaptureConfig,
            packet_tx: mpsc::Sender<AudioPacket>,
        ) -> Result<(), CaptureError> {
            self.started.store(true, std::sync::atomic::Ordering::SeqCst);
            // Send a test packet
            let packet = AudioPacket {
                timestamp: SystemTime::now(),
                stream_id: StreamId::Mic,
                data: Bytes::from_static(&[0u8; 320]),
                sample_rate: 16000,
                channels: 1,
            };
            packet_tx.send(packet).await.map_err(|_| CaptureError::ChannelClosed("send failed".into()))
        }

        async fn stop(&mut self) -> Result<(), CaptureError> {
            self.started.store(false, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    fn test_caps() -> CaptureCapabilities {
        CaptureCapabilities {
            mic_available: true,
            system_audio_available: false,
            device_change_events: false,
            max_sample_rate: 48000,
            supported_sample_rates: vec![16000, 48000],
            supported_channel_modes: vec![ChannelMode::Separate],
        }
    }

    #[tokio::test]
    async fn test_capture_manager_start_stop() {
        let mut manager = AudioCaptureManager::new(CaptureConfig::default())
            .with_mic_backend(Box::new(MockBackend::new(test_caps())));

        let mut rx = manager.packet_receiver().unwrap();

        manager.start().await.unwrap();

        // Should receive the test packet
        let packet = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        assert_eq!(packet.stream_id, StreamId::Mic);
        assert_eq!(packet.sample_rate, 16000);

        manager.stop().await.unwrap();
    }

    #[test]
    fn test_capabilities_query() {
        let manager = AudioCaptureManager::new(CaptureConfig::default())
            .with_mic_backend(Box::new(MockBackend::new(test_caps())));

        let caps = manager.capabilities();
        assert!(caps.mic_available);
        assert!(!caps.system_audio_available);
        assert_eq!(caps.max_sample_rate, 48000);
    }

    #[test]
    fn test_capture_config_defaults() {
        let config = CaptureConfig::default();
        assert!(config.mic_enabled);
        assert!(!config.system_audio_enabled);
        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.channels, 1);
    }

    #[tokio::test]
    async fn test_manager_without_backend() {
        let mut manager = AudioCaptureManager::new(CaptureConfig {
            mic_enabled: false,
            system_audio_enabled: false,
            ..Default::default()
        });

        // Should succeed with no backends
        manager.start().await.unwrap();
        manager.stop().await.unwrap();
    }
}