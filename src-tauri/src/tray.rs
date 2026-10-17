//! Tray-Symbol und das Verhalten "Fenster schließen = in den Tray".

use crate::session::AppState;
use std::sync::atomic::Ordering;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, State};

const TRAY_ID: &str = "main-tray";

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Öffnen", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Beenden", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("Moodle Scraper")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Schaltet den Hintergrundbetrieb um: Tray-Symbol zeigen, Schließen versteckt nur das Fenster.
#[tauri::command]
pub fn set_background(app: AppHandle, state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state.background.store(enabled, Ordering::Relaxed);
    if enabled {
        if app.tray_by_id(TRAY_ID).is_none() {
            build_tray(&app).map_err(|e| e.to_string())?;
        }
    } else {
        let _ = app.remove_tray_by_id(TRAY_ID);
    }
    Ok(())
}
