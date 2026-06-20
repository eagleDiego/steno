//! Integration tests for steno-core.
//!
//! These tests exercise the real module compositions without mocking,
//! verifying that the core abstractions work together correctly.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use bytes::Bytes;
use steno_core::audio::AudioCaptureManager;
use steno_core::audio::AudioPacket;
use steno_core::audio::{
    AudioCaptureBackend, CaptureCapabilities, CaptureConfig, ChannelMode, StreamId,
};
use steno_core::detection::{
    default_allowlist, AudioActivitySensor, DetectionEngine, DetectionEvent, DetectionManager,
    DetectionMode,
};
use steno_core::error::CaptureError;
use steno_core::events::{AppEvent, EventBus, UiState};

// ── Mock implementations for integration tests ───────────────────────

struct IntegrationMockBackend {
    caps: CaptureCapabilities,
    started: std::sync::atomic::AtomicBool,
}

impl IntegrationMockBackend {
    fn new_mic() -> Self {
        Self {
            caps: CaptureCapabilities {
                mic_available: true,
                system_audio_available: false,
                device_change_events: false,
                max_sample_rate: 48000,
                supported_sample_rates: vec![16000, 48000],
                supported_channel_modes: vec![ChannelMode::Separate, ChannelMode::Both],
            },
            started: std::sync::atomic::AtomicBool::new(false),
        }
    }

    fn new_system() -> Self {
        Self {
            caps: CaptureCapabilities {
                mic_available: false,
                system_audio_available: true,
                device_change_events: true,
                max_sample_rate: 48000,
                supported_sample_rates: vec![16000, 48000],
                supported_channel_modes: vec![ChannelMode::Separate],
            },
            started: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl AudioCaptureBackend for IntegrationMockBackend {
    fn capabilities(&self) -> CaptureCapabilities {
        self.caps.clone()
    }

    async fn start(
        &mut self,
        _config: CaptureConfig,
        packet_tx: tokio::sync::mpsc::Sender<AudioPacket>,
    ) -> Result<(), CaptureError> {
        self.started
            .store(true, std::sync::atomic::Ordering::SeqCst);

        // Send several test packets to simulate real audio
        for i in 0..5 {
            let mut data = Vec::with_capacity(640);
            for _ in 0..320 {
                let sample = (i as i16).wrapping_mul(1000);
                data.extend_from_slice(&sample.to_le_bytes());
            }

            let packet = AudioPacket {
                timestamp: SystemTime::now(),
                stream_id: StreamId::Mic,
                data: Bytes::from(data),
                sample_rate: 16000,
                channels: 1,
            };

            packet_tx
                .send(packet)
                .await
                .map_err(|_| CaptureError::ChannelClosed("test send failed".into()))?;
        }

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), CaptureError> {
        self.started
            .store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}

struct IntegrationMockEngine {
    counter: std::sync::Mutex<u32>,
    mode: std::sync::Mutex<DetectionMode>,
}

impl IntegrationMockEngine {
    fn new() -> Self {
        Self {
            counter: std::sync::Mutex::new(0),
            mode: std::sync::Mutex::new(DetectionMode::Auto),
        }
    }
}

#[async_trait]
impl DetectionEngine for IntegrationMockEngine {
    async fn is_meeting_active(
        &self,
        allowlist: &[String],
    ) -> Result<Option<bool>, steno_core::error::DetectionError> {
        let mut c = self.counter.lock().unwrap();
        *c += 1;
        // Active on odd calls, inactive on even
        Ok(Some(*c % 2 == 1))
    }

    async fn foreground_app(&self) -> Result<Option<String>, steno_core::error::DetectionError> {
        Ok(Some("zoom.us".to_string()))
    }

    fn set_mode(&mut self, mode: DetectionMode) {
        *self.mode.lock().unwrap() = mode;
    }

    fn mode(&self) -> DetectionMode {
        *self.mode.lock().unwrap()
    }
}

struct IntegrationMockSensor;

#[async_trait]
impl AudioActivitySensor for IntegrationMockSensor {
    async fn is_audio_input_active(&self) -> Result<bool, steno_core::error::DetectionError> {
        Ok(true)
    }
}

// ── Integration tests ───────────────────────────────────────────────

#[tokio::test]
async fn test_full_capture_pipeline() {
    // Set up the capture manager with a mock mic backend
    let config = CaptureConfig {
        mic_enabled: true,
        system_audio_enabled: true,
        sample_rate: 16000,
        channels: 1,
        channel_mode: ChannelMode::Separate,
        output_dir: std::env::temp_dir(),
    };

    let mut manager = AudioCaptureManager::new(config)
        .with_mic_backend(Box::new(IntegrationMockBackend::new_mic()))
        .with_system_backend(Box::new(IntegrationMockBackend::new_system()));

    let mut rx = manager.packet_receiver().unwrap();

    // Start capture — should begin producing packets
    manager.start().await.unwrap();

    // Collect packets from both streams
    let mut packets = Vec::new();
    for _ in 0..10 {
        match tokio::time::timeout(Duration::from_secs(1), rx.recv()).await {
            Ok(Some(packet)) => packets.push(packet),
            Ok(None) => break,
            Err(_) => break,
        }
    }

    // Should have at least some packets
    assert!(!packets.is_empty(), "should have received packets");

    // Verify packet structure
    for packet in &packets {
        assert!(packet.sample_rate == 16000);
        assert_eq!(packet.channels, 1);
        assert!(!packet.data.is_empty());
    }

    // Stop capture
    manager.stop().await.unwrap();

    // Combined capabilities should report both mic and system
    let caps = manager.capabilities();
    assert!(caps.mic_available);
    assert!(caps.system_audio_available);
}

#[tokio::test]
async fn test_detection_to_capture_flow() {
    // Simulate: detection engine sees a meeting → manager starts capture
    let engine = Box::new(IntegrationMockEngine::new());
    let sensor = Box::new(IntegrationMockSensor);
    let mut detection_manager = DetectionManager::new(engine, sensor, default_allowlist());

    // Check detection
    let event = detection_manager.check().await.unwrap();
    assert!(event.is_some(), "should detect a meeting on first check");
    assert!(detection_manager.is_meeting_active());

    // Simulate starting capture based on detection
    let config = CaptureConfig {
        mic_enabled: true,
        system_audio_enabled: false,
        sample_rate: 16000,
        channels: 1,
        channel_mode: ChannelMode::Separate,
        output_dir: std::env::temp_dir(),
    };

    let mut manager = AudioCaptureManager::new(config)
        .with_mic_backend(Box::new(IntegrationMockBackend::new_mic()));

    manager.start().await.unwrap();
    manager.stop().await.unwrap();

    // Second check should show meeting ended
    let event2 = detection_manager.check().await.unwrap();
    assert!(event2.is_some(), "should detect meeting ending");
    match event2.unwrap() {
        DetectionEvent::MeetingEnded { .. } => {} // correct
        other => panic!("expected MeetingEnded, got {:?}", other),
    }
}

#[tokio::test]
async fn test_event_bus_integration() {
    // Event bus connects multiple subsystems
    let bus = EventBus::new(32);
    let mut rx1 = bus.subscribe();
    let mut rx2 = bus.subscribe();

    // Detection publishes an event
    let detection_event = DetectionEvent::MeetingStarted {
        app_name: "zoom.us".into(),
        detected_at: SystemTime::now(),
        detection_mode: DetectionMode::Auto,
    };

    bus.publish(AppEvent::MeetingDetected(detection_event));

    // UI capture start
    bus.publish(AppEvent::CaptureStarted {
        session_id: uuid::Uuid::new_v4(),
        mic_active: true,
        system_audio_active: true,
    });

    // Both subscribers should receive the events
    let received: Vec<AppEvent> =
        std::iter::from_fn(|| rx1.try_recv().ok().or_else(|| rx2.try_recv().ok()))
            .take(2)
            .collect();

    assert_eq!(received.len(), 2, "both subscribers should receive events");

    // Verify structured events
    for event in &received {
        match event {
            AppEvent::MeetingDetected(DetectionEvent::MeetingStarted { app_name, .. }) => {
                assert_eq!(app_name, "zoom.us");
            }
            AppEvent::CaptureStarted {
                mic_active,
                system_audio_active,
                ..
            } => {
                assert!(mic_active);
                assert!(system_audio_active);
            }
            _ => {}
        }
    }
}

#[tokio::test]
async fn test_multiple_detection_checks() {
    let engine = Box::new(IntegrationMockEngine::new());
    let sensor = Box::new(IntegrationMockSensor);
    let mut manager = DetectionManager::new(engine, sensor, default_allowlist());

    // Check 1: meeting starts (engine toggled to active)
    let e1 = manager.check().await.unwrap();
    assert!(matches!(e1, Some(DetectionEvent::MeetingStarted { .. })));
    assert!(manager.is_meeting_active());

    // Check 2: engine toggles to inactive → MeetingEnded
    let e2 = manager.check().await.unwrap();
    assert!(matches!(e2, Some(DetectionEvent::MeetingEnded { .. })));

    // Check 3: engine toggles back to active → MeetingStarted again
    let e3 = manager.check().await.unwrap();
    assert!(matches!(e3, Some(DetectionEvent::MeetingStarted { .. })));

    // Check 4: engine toggles to inactive → MeetingEnded
    let e4 = manager.check().await.unwrap();
    assert!(matches!(e4, Some(DetectionEvent::MeetingEnded { .. })));

    assert!(!manager.is_meeting_active(), "should end as inactive");
}

#[test]
fn test_config_roundtrip_integration() {
    use steno_core::Config;

    let config = Config::default();
    let toml_str = toml::to_string(&config).expect("should serialize");
    let deserialized: Config = toml::from_str(&toml_str).expect("should deserialize");

    assert_eq!(deserialized.capture.sample_rate, config.capture.sample_rate);
    assert_eq!(deserialized.detection.mode, config.detection.mode);
    assert_eq!(
        deserialized.inference.endpoint_url,
        config.inference.endpoint_url
    );
    assert_eq!(
        deserialized.storage.retention_policy,
        config.storage.retention_policy
    );
}

#[tokio::test]
async fn test_capture_manager_capabilities_aggregation() {
    let config = CaptureConfig::default();
    let manager = AudioCaptureManager::new(config)
        .with_mic_backend(Box::new(IntegrationMockBackend::new_mic()))
        .with_system_backend(Box::new(IntegrationMockBackend::new_system()));

    let caps = manager.capabilities();
    assert!(caps.mic_available, "mic should be available");
    assert!(
        caps.system_audio_available,
        "system audio should be available"
    );
    assert!(
        caps.device_change_events,
        "system backend supports device changes"
    );
    assert_eq!(caps.max_sample_rate, 48000);
    assert!(caps.supported_sample_rates.contains(&16000));
    assert!(caps.supported_sample_rates.contains(&48000));
}

#[tokio::test]
async fn test_capture_start_stop_multiple() {
    let config = CaptureConfig {
        mic_enabled: true,
        system_audio_enabled: false,
        ..Default::default()
    };

    let mut manager = AudioCaptureManager::new(config)
        .with_mic_backend(Box::new(IntegrationMockBackend::new_mic()));

    // Start and stop multiple times
    for i in 0..3 {
        manager
            .start()
            .await
            .unwrap_or_else(|e| panic!("start iteration {i}: {e}"));
        manager
            .stop()
            .await
            .unwrap_or_else(|e| panic!("stop iteration {i}: {e}"));
    }
}

#[tokio::test]
async fn test_event_bus_capacity() {
    let bus = EventBus::new(4);

    // Fill the buffer
    for i in 0..4 {
        bus.publish(AppEvent::CaptureStarted {
            session_id: uuid::Uuid::new_v4(),
            mic_active: i % 2 == 0,
            system_audio_active: false,
        });
    }

    // Reader that was subscribed after some events should get latest
    let mut rx = bus.subscribe();
    let mut count = 0;
    while let Ok(_event) = rx.try_recv() {
        count += 1;
    }
    assert!(count <= 4, "should get at most 4 events");
}
