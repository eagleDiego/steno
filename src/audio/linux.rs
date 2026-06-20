/// Linux system audio capture using PipeWire (preferred) or PulseAudio (fallback).
use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::mpsc;

use crate::error::CaptureError;
use super::{AudioCaptureBackend, AudioPacket, CaptureCapabilities, CaptureConfig, ChannelMode, StreamId};

/// Linux system audio capture backend.
///
/// Uses PipeWire's monitor port on the default sink (preferred).
/// Falls back to PulseAudio .monitor source when PipeWire is absent.
pub struct LinuxAudioCapture;

impl LinuxAudioCapture {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AudioCaptureBackend for LinuxAudioCapture {
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
        config: CaptureConfig,
        packet_tx: mpsc::Sender<AudioPacket>,
    ) -> Result<(), CaptureError> {
        #[cfg(target_os = "linux")]
        {
            // Implementation sketch:
            //
            // 1. Try pw_init() to check PipeWire presence
            // 2. If PipeWire available:
            //    a. Create a PipeWire main loop
            //    b. Create a stream on the monitor port of the default sink
            //    c. Register an audio processing callback
            //    d. Convert raw float samples to i16 PCM
            //    e. Push into packet_tx
            // 3. If PipeWire unavailable:
            //    a. Use libpulse-binding to connect to PulseAudio
            //    b. Create a stream on the .monitor source of the default sink
            //    c. Same conversion and push pattern
            //
            // The `pipewire` crate provides Rust bindings for:
            // - pw_init / pw_main_loop
            // - pw_stream with pw_stream_connect(PW_ID_ANY, PW_STREAM_FLAG_MAP)
            // - Audio processing callback
            //
            // The `libpulse-binding` crate provides PulseAudio stream APIs.
            //
            // This is a stub — full PipeWire/PulseAudio integration requires
            // the pipewire daemon headers at build time.

            let _ = (&config, packet_tx);

            return Err(CaptureError::SystemAudioUnavailable(
                "PipeWire/PulseAudio capture requires platform-specific implementation. \
                 Enable via `cargo build --features linux-audio` with the appropriate system packages."
                    .into(),
            ));
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (&config, packet_tx);
            Err(CaptureError::SystemAudioUnavailable(
                "PipeWire/PulseAudio is only available on Linux".into(),
            ))
        }
    }

    async fn stop(&mut self) -> Result<(), CaptureError> {
        Ok(())
    }
}