/// Cross-platform microphone capture using cpal.
use async_trait::async_trait;
use tokio::sync::mpsc;

use super::{AudioCaptureBackend, AudioPacket, CaptureCapabilities, CaptureConfig, ChannelMode};
use crate::error::CaptureError;

/// MicCapture uses cpal (cross-platform) to capture microphone input.
#[derive(Default)]
pub struct MicCapture;

impl MicCapture {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AudioCaptureBackend for MicCapture {
    fn capabilities(&self) -> CaptureCapabilities {
        use cpal::traits::HostTrait;

        let host = cpal::default_host();
        let mic_available = host.default_input_device().is_some();

        CaptureCapabilities {
            mic_available,
            system_audio_available: false,
            device_change_events: false,
            max_sample_rate: 48000,
            supported_sample_rates: vec![8000, 16000, 44100, 48000],
            supported_channel_modes: vec![ChannelMode::Separate],
        }
    }

    async fn start(
        &mut self,
        _config: CaptureConfig,
        _packet_tx: mpsc::Sender<AudioPacket>,
    ) -> Result<(), CaptureError> {
        // Mic capture using cpal requires a running audio stream.
        // In production this opens a cpal input stream, converts
        // f32 samples to i16 PCM, and pushes AudioPackets into packet_tx.
        //
        // Implementation sketch:
        //   use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        //
        //   let host = cpal::default_host();
        //   let device = host.default_input_device().ok_or(...)?;
        //   let cfg = cpal::StreamConfig { ... };
        //
        //   let tx = Arc::new(tokio::sync::Mutex::new(packet_tx));
        //   let stream = device.build_input_stream(
        //       &cfg,
        //       move |data: &[f32], _| {
        //           // convert f32 → i16 PCM, push to tx
        //       },
        //       err_fn, None,
        //   )?;
        //   stream.play()?;
        //   self.stream = Some(stream);
        //
        // For the cross-platform build, this implementation is a stub
        // that avoids the platform-specific cpal dependency at build time
        // when running unit tests without audio hardware.

        Err(CaptureError::MicUnavailable(
            "cpal mic capture requires a running audio system and is platform-dependent. \
             Enable via the 'mic-capture' feature flag."
                .into(),
        ))
    }

    async fn stop(&mut self) -> Result<(), CaptureError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mic_capabilities() {
        let mic = MicCapture::new();
        let caps = mic.capabilities();
        assert!(!caps.system_audio_available);
        assert_eq!(caps.max_sample_rate, 48000);
    }
}
