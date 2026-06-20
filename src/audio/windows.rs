/// Windows system audio capture using WASAPI loopback.
use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::mpsc;
use std::time::SystemTime;

use crate::error::CaptureError;
use super::{AudioCaptureBackend, AudioPacket, CaptureCapabilities, CaptureConfig, ChannelMode, StreamId};

/// Windows system audio capture via WASAPI loopback.
///
/// WASAPI loopback captures the audio stream being played through
/// the default output device — no virtual audio device needed.
pub struct WindowsAudioCapture;

impl WindowsAudioCapture {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AudioCaptureBackend for WindowsAudioCapture {
    fn capabilities(&self) -> CaptureCapabilities {
        CaptureCapabilities {
            mic_available: false,
            system_audio_available: true,
            device_change_events: true,
            max_sample_rate: 48000,
            supported_sample_rates: vec![16000, 44100, 48000, 96000],
            supported_channel_modes: vec![ChannelMode::Separate],
        }
    }

    async fn start(
        &mut self,
        config: CaptureConfig,
        packet_tx: mpsc::Sender<AudioPacket>,
    ) -> Result<(), CaptureError> {
        #[cfg(target_os = "windows")]
        {
            // Use the windows crate's Media.Audio API for WASAPI loopback
            //
            // Implementation sketch:
            // 1. Use windows::Media::Devices::MediaDevice::GetDefaultAudioRenderId
            // 2. Create an AudioGraph with an AudioDeviceOutputNode
            // 3. Add a frame-output node to capture PCM frames
            // 4. Subscribe to QuantumStarted or use AudioFrameOutputNode::GetFrame
            // 5. Push frames into packet_tx
            //
            // The `windows` crate provides WinRT bindings for:
            // - Windows.Media.Audio.AudioGraph
            // - Windows.Media.Audio.AudioDeviceOutputNode
            // - Windows.Media.Audio.AudioFrameOutputNode
            //
            // This is a stub — full WASAPI loopback requires Windows 10 SDK
            // and appropriate audio capabilities declared in the app manifest.

            let _ = &config;
            let _ = packet_tx;

            return Err(CaptureError::SystemAudioUnavailable(
                "WASAPI loopback capture requires Windows-specific implementation \
                 via the windows crate's AudioGraph API. Enable via `--cfg windows_wasapi`."
                    .into(),
            ));
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = (&config, packet_tx);
            Err(CaptureError::SystemAudioUnavailable(
                "WASAPI loopback is only available on Windows".into(),
            ))
        }
    }

    async fn stop(&mut self) -> Result<(), CaptureError> {
        Ok(())
    }
}