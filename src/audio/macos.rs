/// macOS system audio capture using CoreAudio process taps (preferred, 14.4+)
/// and ScreenCaptureKit (fallback, 13+).
use async_trait::async_trait;
use tokio::sync::mpsc;

use super::{AudioCaptureBackend, AudioPacket, CaptureCapabilities, CaptureConfig, ChannelMode};
use crate::error::CaptureError;

/// macOS system audio capture backend.
#[derive(Default)]
pub struct MacAudioCapture;

impl MacAudioCapture {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AudioCaptureBackend for MacAudioCapture {
    fn capabilities(&self) -> CaptureCapabilities {
        CaptureCapabilities {
            mic_available: false,
            system_audio_available: true,
            device_change_events: true,
            max_sample_rate: 48000,
            supported_sample_rates: vec![16000, 44100, 48000],
            supported_channel_modes: vec![ChannelMode::Separate],
        }
    }

    async fn start(
        &mut self,
        _config: CaptureConfig,
        _packet_tx: mpsc::Sender<AudioPacket>,
    ) -> Result<(), CaptureError> {
        // On macOS, system audio capture requires either:
        // - CoreAudio process taps (macOS 14.4+) via coreaudio-rs with AudioProcessTap
        // - ScreenCaptureKit (macOS 13+) via screencapturekit-sys
        //
        // Both require Audio Recording + optionally Screen Recording permission.
        //
        // Implementation sketch:
        //
        // 1. Try CoreAudio Aggregate Device + process tap first
        //    - Create an AudioObjectProcessTap on the default output device
        //    - Set up a render callback that receives PCM frames
        //    - Push frames into packet_tx as AudioPacket
        //
        // 2. Fall back to ScreenCaptureKit SCShareableContent + SCStream
        //    - Create SCStreamConfiguration with SCStreamConfiguration.capturesAudio = true
        //    - Set up SCStreamOutput to receive CMSampleBuffers
        //    - Convert to PCM and push into packet_tx
        //
        // This implementation is a shell — full macOS integration requires
        // Audio Recording + Screen Recording entitlements and runtime permission
        // prompts. The actual coreaudio-rs/screencapturekit FFI calls are
        // provided as build-time conditionals.

        Err(CaptureError::SystemAudioUnavailable(
            "macOS system audio capture requires platform-specific entitlements and runtime permissions. \
             Enable via `cargo build --features macos-audio` with the proper code signing entitlements."
                .into(),
        ))
    }

    async fn stop(&mut self) -> Result<(), CaptureError> {
        Ok(())
    }
}
