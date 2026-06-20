/// Unified error type for the steno-core library.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Audio capture failed: {0}")]
    Capture(#[from] CaptureError),

    #[error("Detection failed: {0}")]
    Detection(#[from] DetectionError),

    #[error("Transcription failed: {0}")]
    Transcription(#[from] TranscriptionError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Consent error: {0}")]
    Consent(#[from] ConsentError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

// Manual Serialize impl for Tauri IPC — serializes as just the error message string.
impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

// ── Audio errors ─────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("Microphone unavailable: {0}")]
    MicUnavailable(String),

    #[error("System audio unavailable: {0}")]
    SystemAudioUnavailable(String),

    #[error("Device disconnected during capture: {0}")]
    DeviceDisconnected(String),

    #[error("Buffer underrun")]
    BufferUnderrun,

    #[error("Backend-specific error: {0}")]
    Backend(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Channel closed: {0}")]
    ChannelClosed(String),
}

// ── Detection errors ─────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum DetectionError {
    #[error("Platform detection unavailable: {0}")]
    PlatformUnavailable(String),

    #[error("Process query failed: {0}")]
    ProcessQuery(String),

    #[error("Audio sensor error: {0}")]
    AudioSensor(String),

    #[error("{0}")]
    Other(String),
}

// ── Transcription errors ─────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum TranscriptionError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Rate limited")]
    RateLimited(std::time::Duration),

    #[error("Model unavailable: {0}")]
    ModelUnavailable(String),

    #[error("Segmentation error: {0}")]
    Segmentation(String),

    #[error("Sink error: {0}")]
    Sink(#[from] TranscriptSinkError),

    #[error("{0}")]
    Other(String),
}

// ── Storage errors ───────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Session not found: {0}")]
    SessionNotFound(uuid::Uuid),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// ── Consent errors ───────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ConsentError {
    #[error("Consent not yet given")]
    NotYetGiven,

    #[error("Consent already given")]
    AlreadyGiven,

    #[error("Log write failed: {0}")]
    LogWrite(String),
}

// ── Sink errors ──────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum TranscriptSinkError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
