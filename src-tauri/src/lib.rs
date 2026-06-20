mod ui;

use steno_core::Config;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "steno=info,warn".into()),
        )
        .init();

    let config = Config::default();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(config)
        .invoke_handler(tauri::generate_handler![
            crate::ui::commands::start_capture,
            crate::ui::commands::stop_capture,
            crate::ui::commands::is_capturing,
            crate::ui::commands::get_capabilities,
            crate::ui::commands::set_detection_mode,
            crate::ui::commands::get_detection_mode,
        ])
        .setup(|_app| {
            let _tray_cfg = crate::ui::tray::setup_tray();
            tracing::info!("Steno initialized (tray: {})", _tray_cfg.tooltip);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running steno");
}