//! The icon in the menu bar (macOS) or system tray (Windows, Linux). It keeps
//! the app reachable while its window is closed.

use std::sync::Arc;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

use crate::engine::Engine;

const TRAY_ID: &str = "main";

/// Where the icon lives, as the user would call it.
#[cfg(target_os = "macos")]
pub const PLACE: &str = "la barra de menús";
#[cfg(not(target_os = "macos"))]
pub const PLACE: &str = "la bandeja del sistema";

/// The first line of the menu. It is not clickable; it says how the links are doing.
struct StatusItem(MenuItem<Wry>);

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let status = MenuItem::with_id(app, "status", "Sin enlaces guardados", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Abrir EnlaceTech", true, None::<&str>)?;
    let check = MenuItem::with_id(app, "check", "Comprobar ahora", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &status,
            &PredefinedMenuItem::separator(app)?,
            &open,
            &check,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("EnlaceTech")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main(app),
            "check" => {
                let started = app
                    .try_state::<Arc<Engine>>()
                    .is_some_and(|engine| engine.inner().check_now(None).is_ok());
                // It cannot check without a session: open the window so the user can sign in.
                if !started {
                    show_main(app);
                }
            }
            "quit" => app.exit(0),
            _ => {}
        });

    // macOS tints a one-colour "template" image to match the menu bar. The
    // other systems have no such thing, so they get the app's own icon.
    #[cfg(target_os = "macos")]
    {
        let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
        tray = tray.icon(icon).icon_as_template(true);
    }
    #[cfg(not(target_os = "macos"))]
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    app.manage(StatusItem(status));
    Ok(())
}

/// Brings the main window back, wherever it was: hidden, minimized or behind others.
pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Updates the status line of the menu and the icon's tooltip.
pub fn set_status(app: &AppHandle, text: &str) {
    if let Some(status) = app.try_state::<StatusItem>() {
        let _ = status.0.set_text(text);
    }
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(format!("EnlaceTech · {text}")));
    }
}
