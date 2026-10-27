// Einstiegspunkt der Tauri-App. Die Befehle für das Frontend stehen unten in
// `invoke_handler`, die eigentliche Arbeit machen die Module.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod deadlines;
mod diagnostics;
mod download;
mod log;
mod parse;
mod scan;
mod scan_cache;
mod secure;
mod session;
#[cfg(test)]
mod testutil;
mod tray;
mod types;
mod util;

use std::sync::atomic::Ordering;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(session::AppState::new())
        .setup(|app| {
            if let Ok(dir) = app.path().app_log_dir() {
                log::init(&dir);
            }
            log::log(
                "INFO",
                &format!("Moodle Scraper {} gestartet", env!("CARGO_PKG_VERSION")),
            );
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let state = window.app_handle().state::<session::AppState>();
                    if state.background.load(Ordering::Relaxed) {
                        // Im Hintergrundbetrieb nur verstecken, das Tray-Symbol bleibt
                        api.prevent_close();
                        let _ = window.hide();
                    } else {
                        window.app_handle().exit(0);
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            session::open_login,
            session::check_login,
            session::restore_session,
            session::set_remember,
            session::logout,
            session::list_courses,
            scan::scan_course,
            scan::clear_scan_cache,
            deadlines::list_deadlines,
            deadlines::export_ics,
            diagnostics::export_diagnostics,
            tray::set_background,
            download::download_courses,
            download::preview_job,
            download::cancel_download,
            download::default_download_dir
        ])
        .run(tauri::generate_context!())
        .expect("Fehler beim Starten von Moodle Scraper");
}
