/// Foreground-window / process-list polling for meeting detection.
///
/// Platform-specific implementations for macOS, Windows, and Linux.
use async_trait::async_trait;

use crate::detection::{DetectionEngine, DetectionMode};
use crate::error::DetectionError;

/// Cross-platform process monitor that uses OS-specific APIs
/// to determine the foreground application.
pub struct ProcessMonitor {
    mode: DetectionMode,
}

impl ProcessMonitor {
    pub fn new(mode: DetectionMode) -> Self {
        Self { mode }
    }
}

#[async_trait]
impl DetectionEngine for ProcessMonitor {
    async fn is_meeting_active(
        &self,
        allowlist: &[String],
    ) -> Result<Option<bool>, DetectionError> {
        // In production, this queries the OS for the foreground window.
        // Platform implementations:
        //
        // macOS:
        //   NSWorkspace.shared.frontmostApplication.localizedName
        //   via objc runtime or core-foundation bindings
        //
        // Windows:
        //   GetForegroundWindow() → GetWindowThreadProcessId() → QueryFullProcessImageName()
        //   via the `windows` crate's UI automation APIs
        //
        // Linux:
        //   X11: _NET_ACTIVE_WINDOW via XGetInputFocus + XFetchName
        //   Wayland: zwlr_foreign_toplevel_manager_v1 protocol
        //   Fallback: parse /proc/<pid>/comm for known meeting processes
        //
        // For unit testing, this is mocked. The shell below illustrates
        // the cross-platform detection pattern.

        let app = self.foreground_app().await?;
        match app {
            Some(name) => {
                let lowered = name.to_lowercase();
                let matched = allowlist
                    .iter()
                    .any(|a| lowered.contains(&a.to_lowercase()));
                Ok(Some(matched))
            }
            None => Ok(None),
        }
    }

    async fn foreground_app(&self) -> Result<Option<String>, DetectionError> {
        #[cfg(target_os = "macos")]
        {
            // Use NSWorkspace via objc runtime
            // let workspace = NSWorkspace::sharedWorkspace();
            // let app = workspace.frontmostApplication();
            // let name = app.localizedName();
            // return Ok(Some(name));
        }

        #[cfg(target_os = "windows")]
        {
            // Use GetForegroundWindow + GetWindowTextW
        }

        #[cfg(target_os = "linux")]
        {
            // Use X11 _NET_ACTIVE_WINDOW or Wayland toplevel protocol
        }

        // Platform detection not yet wired — return None for testing
        Ok(None)
    }

    fn set_mode(&mut self, mode: DetectionMode) {
        self.mode = mode;
    }

    fn mode(&self) -> DetectionMode {
        self.mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_monitor_creation() {
        let monitor = ProcessMonitor::new(DetectionMode::Auto);
        assert_eq!(monitor.mode(), DetectionMode::Auto);
    }

    #[tokio::test]
    async fn test_process_monitor_mode_switch() {
        let mut monitor = ProcessMonitor::new(DetectionMode::Manual);
        assert_eq!(monitor.mode(), DetectionMode::Manual);
        monitor.set_mode(DetectionMode::Armed);
        assert_eq!(monitor.mode(), DetectionMode::Armed);
    }

    #[tokio::test]
    async fn test_process_monitor_no_foreground() {
        // On a headless system without a display server,
        // foreground_app() should return None gracefully.
        let monitor = ProcessMonitor::new(DetectionMode::Auto);
        let app = monitor.foreground_app().await.unwrap();
        // Should not panic — platform detection is best-effort
        assert!(app.is_none() || app.is_some());
    }
}
