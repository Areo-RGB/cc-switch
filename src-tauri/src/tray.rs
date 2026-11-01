//! Minimal Desk tray integration.
//!
//! The routing menu was intentionally removed in Phase 1. The tray only
//! opens the main window and exits the application.

use tauri::menu::{Menu, MenuBuilder, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::store::AppState;

pub const TRAY_ID: &str = "cc-switch";

pub fn create_tray_menu(app: &AppHandle, _state: &AppState) -> tauri::Result<Menu<tauri::Wry>> {
    MenuBuilder::new(app)
        .item(&MenuItem::with_id(
            app,
            "open",
            "Open Desk",
            true,
            None::<&str>,
        )?)
        .item(&MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?)
        .build()
}

pub fn handle_tray_icon_event(tray: &TrayIcon, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        show_main_window(tray.app_handle());
    }
}

pub fn handle_tray_menu_event(app: &AppHandle, event_id: &str) {
    match event_id {
        "open" => show_main_window(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
