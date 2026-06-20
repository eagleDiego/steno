/// DetectionMode — the three meeting detection modes.
use serde::{Deserialize, Serialize};

/// Three detection modes selectable in settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetectionMode {
    /// User starts/stops capture explicitly.
    Manual,
    /// Capture begins automatically when criteria are met, stops when absent.
    Auto,
    /// Auto-detect but require one click to confirm before capture begins.
    Armed,
}

impl DetectionMode {
    /// Returns true if this mode automatically starts capture.
    pub fn auto_starts(&self) -> bool {
        matches!(self, DetectionMode::Auto)
    }

    /// Returns true if this mode requires user confirmation.
    pub fn requires_confirmation(&self) -> bool {
        matches!(self, DetectionMode::Armed)
    }

    /// Returns true if this mode is entirely manual.
    pub fn is_manual(&self) -> bool {
        matches!(self, DetectionMode::Manual)
    }
}

impl std::fmt::Display for DetectionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectionMode::Manual => write!(f, "manual"),
            DetectionMode::Auto => write!(f, "auto"),
            DetectionMode::Armed => write!(f, "armed"),
        }
    }
}

impl Default for DetectionMode {
    fn default() -> Self {
        Self::Armed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_starts() {
        assert!(DetectionMode::Auto.auto_starts());
        assert!(!DetectionMode::Manual.auto_starts());
        assert!(!DetectionMode::Armed.auto_starts());
    }

    #[test]
    fn test_requires_confirmation() {
        assert!(DetectionMode::Armed.requires_confirmation());
        assert!(!DetectionMode::Auto.requires_confirmation());
        assert!(!DetectionMode::Manual.requires_confirmation());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", DetectionMode::Manual), "manual");
        assert_eq!(format!("{}", DetectionMode::Auto), "auto");
        assert_eq!(format!("{}", DetectionMode::Armed), "armed");
    }

    #[test]
    fn test_default_is_armed() {
        assert_eq!(DetectionMode::default(), DetectionMode::Armed);
    }
}
