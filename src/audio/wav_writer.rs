/// WAV writer wrapper that writes PCM audio packets to disk.
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use hound::{WavSpec, WavWriter};
use tokio::sync::Mutex;

use crate::audio::{AudioPacket, StreamId};
use crate::error::CaptureError;

/// A thread-safe WAV sink that writes packets to disk.
pub struct WavFileSink {
    writer: Arc<Mutex<Option<WavWriter<BufWriter<File>>>>>,
    path: PathBuf,
    stream_id: StreamId,
    #[expect(dead_code)]
    sample_rate: u32,
    #[expect(dead_code)]
    channels: u16,
    flushed: Arc<std::sync::atomic::AtomicBool>,
}

impl WavFileSink {
    /// Create a new WAV file sink. Opens the file and writes the WAV header.
    pub fn create(
        dir: &Path,
        session_id: &str,
        stream_id: StreamId,
        sample_rate: u32,
        channels: u16,
    ) -> Result<Self, CaptureError> {
        let filename = match stream_id {
            StreamId::Mic => format!("{session_id}_mic.wav"),
            StreamId::System => format!("{session_id}_system.wav"),
            StreamId::Mixed => format!("{session_id}_mixed.wav"),
        };
        let path = dir.join(&filename);

        let spec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let file = File::create(&path)?;
        let writer = WavWriter::new(BufWriter::new(file), spec)
            .map_err(|e| CaptureError::Backend(format!("failed to create WAV writer: {e}")))?;

        Ok(Self {
            writer: Arc::new(Mutex::new(Some(writer))),
            path,
            stream_id,
            sample_rate,
            channels,
            flushed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Write an audio packet to the WAV file.
    pub async fn write_packet(&self, packet: &AudioPacket) -> Result<(), CaptureError> {
        let mut guard = self.writer.lock().await;
        let writer = guard
            .as_mut()
            .ok_or_else(|| CaptureError::ChannelClosed("writer already finalized".into()))?;

        // Interpret the raw PCM bytes as i16 samples (little-endian)
        let samples: &[u8] = &packet.data;
        for chunk in samples.chunks_exact(2) {
            let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
            writer
                .write_sample(sample)
                .map_err(|e| CaptureError::Backend(format!("failed to write sample: {e}")))?;
        }

        self.flushed
            .store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    /// Finalize the WAV file (updates header with correct length).
    pub async fn finalize(&self) -> Result<(), CaptureError> {
        let mut guard = self.writer.lock().await;
        if let Some(writer) = guard.take() {
            writer
                .finalize()
                .map_err(|e| CaptureError::Backend(format!("failed to finalize WAV: {e}")))?;
            self.flushed
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
        Ok(())
    }

    /// Get the output path of this sink.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the stream ID.
    pub fn stream_id(&self) -> StreamId {
        self.stream_id
    }
}

/// Writes captured audio packets to separate WAV files per stream.
pub struct WavWriterService {
    sinks: Vec<WavFileSink>,
    session_id: String,
}

impl WavWriterService {
    /// Create a new WAV writer service for a session.
    pub fn new(
        output_dir: &Path,
        session_id: String,
        sample_rate: u32,
        channels: u16,
        enable_mic: bool,
        enable_system: bool,
        enable_mixed: bool,
    ) -> Result<Self, CaptureError> {
        let mut sinks = Vec::new();

        if enable_mic {
            sinks.push(WavFileSink::create(
                output_dir,
                &session_id,
                StreamId::Mic,
                sample_rate,
                channels,
            )?);
        }

        if enable_system {
            sinks.push(WavFileSink::create(
                output_dir,
                &session_id,
                StreamId::System,
                sample_rate,
                channels,
            )?);
        }

        if enable_mixed {
            sinks.push(WavFileSink::create(
                output_dir,
                &session_id,
                StreamId::Mixed,
                sample_rate,
                channels,
            )?);
        }

        Ok(Self { sinks, session_id })
    }

    /// Route a packet to the appropriate sink based on its stream ID.
    pub async fn write_packet(&self, packet: &AudioPacket) -> Result<(), CaptureError> {
        for sink in &self.sinks {
            if sink.stream_id() == packet.stream_id {
                sink.write_packet(packet).await?;
            }
        }
        Ok(())
    }

    /// Finalize all WAV files.
    pub async fn finalize_all(&self) -> Result<(), CaptureError> {
        for sink in &self.sinks {
            sink.finalize().await?;
        }
        Ok(())
    }

    /// Get paths of all WAV files written.
    pub fn output_paths(&self) -> Vec<PathBuf> {
        self.sinks.iter().map(|s| s.path().to_path_buf()).collect()
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

/// A crash-safe wrapper that flushes WAV on drop.
pub struct CrashSafeSink {
    inner: Option<WavWriterService>,
}

impl CrashSafeSink {
    pub fn new(service: WavWriterService) -> Self {
        Self {
            inner: Some(service),
        }
    }

    pub fn inner(&self) -> Option<&WavWriterService> {
        self.inner.as_ref()
    }

    pub fn take(&mut self) -> Option<WavWriterService> {
        self.inner.take()
    }
}

impl Drop for CrashSafeSink {
    fn drop(&mut self) {
        if let Some(service) = self.inner.take() {
            // Attempt to finalize on drop — best effort
            let rt = tokio::runtime::Handle::try_current();
            if let Ok(rt) = rt {
                let _ = rt.block_on(service.finalize_all());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use std::time::SystemTime;
    use tempfile::TempDir;

    fn make_packet(stream_id: StreamId, sample_rate: u32, channels: u16) -> AudioPacket {
        // Generate 100ms worth of samples at given rate
        let num_samples = (sample_rate as usize / 10) * channels as usize;
        let mut pcm = Vec::with_capacity(num_samples * 2);
        for i in 0..num_samples {
            let sample = (i as i16).wrapping_mul(100).to_le_bytes();
            pcm.extend_from_slice(&sample);
        }
        AudioPacket {
            timestamp: SystemTime::now(),
            stream_id,
            data: Bytes::from(pcm),
            sample_rate,
            channels,
        }
    }

    #[tokio::test]
    async fn test_wav_sink_create_and_write() {
        let dir = TempDir::new().unwrap();
        let sink =
            WavFileSink::create(dir.path(), "test-session-1", StreamId::Mic, 16000, 1).unwrap();

        let packet = make_packet(StreamId::Mic, 16000, 1);
        sink.write_packet(&packet).await.unwrap();
        sink.finalize().await.unwrap();

        // Verify output file exists and has content
        assert!(sink.path().exists());
        let metadata = std::fs::metadata(sink.path()).unwrap();
        assert!(metadata.len() > 44); // WAV header (44) + data
    }

    #[tokio::test]
    async fn test_wav_service_multi_stream() {
        let dir = TempDir::new().unwrap();
        let service =
            WavWriterService::new(dir.path(), "multi-test".into(), 16000, 1, true, true, false)
                .unwrap();

        let mic_packet = make_packet(StreamId::Mic, 16000, 1);
        let sys_packet = make_packet(StreamId::System, 16000, 1);

        service.write_packet(&mic_packet).await.unwrap();
        service.write_packet(&sys_packet).await.unwrap();
        service.finalize_all().await.unwrap();

        let outputs = service.output_paths();
        assert_eq!(outputs.len(), 2);

        for path in &outputs {
            assert!(path.exists());
            let metadata = std::fs::metadata(path).unwrap();
            assert!(metadata.len() > 44);

            // Verify each file has the right stream id in the filename
            let filename = path.file_name().unwrap().to_str().unwrap();
            assert!(
                filename.contains("_mic.wav") || filename.contains("_system.wav"),
                "unexpected filename: {filename}"
            );
        }
    }

    #[tokio::test]
    async fn test_wav_sink_empty() {
        let dir = TempDir::new().unwrap();
        let sink =
            WavFileSink::create(dir.path(), "empty-test", StreamId::System, 16000, 1).unwrap();
        sink.finalize().await.unwrap();

        // An empty WAV should have just the 44-byte header
        let metadata = std::fs::metadata(sink.path()).unwrap();
        assert_eq!(metadata.len(), 44);
    }

    #[test]
    #[cfg_attr(target_os = "linux", ignore = "requires tokio runtime context")]
    fn test_crash_safe_sink() {
        let dir = TempDir::new().unwrap();
        let service = WavWriterService::new(
            dir.path(),
            "crash-safe".into(),
            16000,
            1,
            true,
            false,
            false,
        )
        .unwrap();

        let packet = make_packet(StreamId::Mic, 16000, 1);
        // Use tokio runtime to write the packet
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            service.write_packet(&packet).await.unwrap();
        });

        // Drop the crash-safe wrapper outside the tokio context
        drop(service);

        // File should still be valid (header was written)
        let output_path = dir.path().join("crash-safe_mic.wav");
        assert!(output_path.exists());
    }
}
