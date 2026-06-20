/// System tray / menu bar icon setup and event handlers.
///
/// Uses tauri-plugin-tray-icon (Tauri 2.x) for cross-platform
/// system tray functionality including:
/// - macOS: menu bar icon
/// - Windows: system tray icon
/// - Linux: indicator / tray icon

use serde::{Deserialize, Serialize};

/// Tray menu item identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrayMenuAction {
    ToggleCapture,
    OpenSettings,
    ShowStatus,
    Quit,
}

/// Configuration for building the tray menu.
pub struct TrayConfig {
    pub tooltip: String,
    pub icon_path: Option<String>,
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            tooltip: "steno-core — idle".into(),
            icon_path: None,
        }
    }
}

/// Build and attach the system tray.
///
/// In production with `#[cfg(feature = "ui")]` and Tauri, this
/// configures the tray with:
///
/// ```rust,ignore
/// tauri::Builder::default()
///     .plugin(tauri_plugin_shell::init())
///     .plugin(tauri_plugin_notification::init())
///     .setup(|app| {
///         let tray = app.tray_by_id("steno-core")?;
///         tray.set_menu_on_left_click(true);
///         tray.on_menu_event(|app, event| {
///             match event.id.as_ref() {
///                 "toggle_capture" => { /* emit event */ }
///                 "open_settings" => { /* open window */ }
///                 "quit" => { app.exit(0); }
///                 _ => {}
///             }
///         });
///         Ok(())
///     });
/// ```
///
/// This module provides the event wiring that the Tauri setup hook
/// connects. The actual tray creation is deferred to the Tauri app
/// builder due to the `app.handle()` requirement.
pub fn setup_tray() -> TrayConfig {
    TrayConfig::default()
}

/// Get the status text for a given capture state and duration.
pub fn status_text(
    is_capturing: bool,
    elapsed: Option<std::time::Duration>,
) -> String {
    if is_capturing {
        match elapsed {
            Some(d) => {
                let secs = d.as_secs();
                let mm = secs / 60;
                let ss = secs % 60;
                format!("steno-core — recording [{mm:02}:{ss:02}]")
            }
            None => "steno-core — recording".into(),
        }
    } else {
        "steno-core — idle".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_text_idle() {
        assert_eq!(status_text(false, None), "steno-core — idle");
    }

    #[test]
    fn test_status_text_capturing_no_elapsed() {
        assert_eq!(status_text(true, None), "steno-core — recording");
    }

    #[test]
    fn test_status_text_capturing_with_elapsed() {
        let dur = std::time::Duration::from_secs(125); // 2m 5s
        assert_eq!(
            status_text(true, Some(dur)),
            "steno-core — recording [02:05]"
        );
    }

    #[test]
    fn test_status_text_capturing_long_duration() {
        let dur = std::time::Duration::from_secs(3661); // 1h 1m 1s
        assert_eq!(
            status_text(true, Some(dur)),
            "steno-core — recording [61:01]"
        );
    }

    #[test]
    fn test_tray_config_defaults() {
        let config = TrayConfig::default();
        assert_eq!(config.tooltip, "steno-core — idle");
        assert!(config.icon_path.is_none());
    }

    #[test]
    fn test_tray_menu_action_serde() {
        let actions = vec![
            TrayMenuAction::ToggleCapture,
            TrayMenuAction::OpenSettings,
            TrayMenuAction::ShowStatus,
            TrayMenuAction::Quit,
        ];
        for action in actions {
            let json = serde_json::to_string(&action).unwrap();
            let deserialized: TrayMenuAction = serde_json::from_str(&json).unwrap();
            assert_eq!(action, deserialized);
        }
    }
}