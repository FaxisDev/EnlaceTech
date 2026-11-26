mod engine;
mod model;
mod rapidgator;
mod secrets;
mod tray;

use std::sync::Arc;

use tauri::{AppHandle, Manager, State, WebviewWindow, WindowEvent};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::engine::{AddReport, Engine, Snapshot};

type Shared<'a> = State<'a, Arc<Engine>>;

#[tauri::command]
async fn get_snapshot(engine: Shared<'_>) -> Result<Snapshot, String> {
    Ok(engine.snapshot())
}

#[tauri::command]
async fn add_links(engine: Shared<'_>, text: String) -> Result<AddReport, String> {
    let (report, new_ids) = engine.add_links(&text);
    if !new_ids.is_empty() {
        engine.save().await;
        engine.emit();
        let engine = engine.inner().clone();
        tauri::async_runtime::spawn(async move { engine.run_check(Some(new_ids), true).await });
    }
    Ok(report)
}

#[tauri::command]
async fn remove_link(engine: Shared<'_>, id: String) -> Result<(), String> {
    engine.remove(&id).await;
    Ok(())
}

#[tauri::command]
async fn clear_completed(engine: Shared<'_>) -> Result<(), String> {
    engine.clear_completed().await;
    Ok(())
}

#[tauri::command]
async fn check_now(engine: Shared<'_>, id: Option<String>) -> Result<(), String> {
    engine.inner().check_now(id)
}

#[tauri::command]
async fn start_download(engine: Shared<'_>, id: String) -> Result<(), String> {
    engine.inner().start_download(&id)
}

#[tauri::command]
async fn cancel_download(engine: Shared<'_>, id: String) -> Result<(), String> {
    engine.cancel_download(&id);
    Ok(())
}

#[tauri::command]
async fn login(
    engine: Shared<'_>,
    email: String,
    password: String,
    code: Option<String>,
    remember: bool,
) -> Result<(), String> {
    engine.login(email, password, code, remember).await?;
    // Now that there is a session, find out what can be downloaded.
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn(async move { engine.run_check(None, false).await });
    Ok(())
}

#[tauri::command]
async fn logout(engine: Shared<'_>) -> Result<(), String> {
    engine.logout().await;
    Ok(())
}

/// Opens the system's folder picker. Leaves everything as it was if the user cancels.
#[tauri::command]
async fn pick_download_dir(
    app: AppHandle,
    window: WebviewWindow,
    engine: Shared<'_>,
) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Elige la carpeta de descargas")
        .set_directory(engine.download_dir())
        .set_parent(&window)
        .pick_folder(move |picked| {
            let _ = tx.send(picked);
        });

    let Some(picked) = rx.await.ok().flatten() else {
        return Ok(());
    };
    let path = picked
        .into_path()
        .map_err(|_| "No se pudo usar esa carpeta.".to_owned())?;
    engine.set_download_dir(Some(path)).await;
    Ok(())
}

/// Goes back to the system's Downloads folder.
#[tauri::command]
async fn reset_download_dir(engine: Shared<'_>) -> Result<(), String> {
    engine.set_download_dir(None).await;
    Ok(())
}

#[tauri::command]
async fn set_interval(engine: Shared<'_>, minutes: u32) -> Result<(), String> {
    engine.set_interval(minutes).await
}

#[tauri::command]
async fn set_notifications(engine: Shared<'_>, enabled: bool) -> Result<(), String> {
    engine.set_notifications(enabled).await;
    Ok(())
}

#[tauri::command]
async fn test_notification(engine: Shared<'_>) -> Result<(), String> {
    engine.test_notification()
}

#[tauri::command]
async fn set_keep_in_tray(engine: Shared<'_>, enabled: bool) -> Result<(), String> {
    engine.set_keep_in_tray(enabled).await;
    Ok(())
}

/// Shows a downloaded file in Finder / Explorer / the file manager.
#[tauri::command]
async fn reveal(app: AppHandle, engine: Shared<'_>, id: String) -> Result<(), String> {
    let path = engine
        .saved_path(&id)
        .ok_or_else(|| "Ese archivo aún no se ha descargado.".to_owned())?;
    if !path.exists() {
        return Err("El archivo ya no está en la carpeta donde se guardó.".into());
    }
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|_| "No se pudo mostrar el archivo.".to_owned())
}

#[tauri::command]
async fn open_download_dir(app: AppHandle, engine: Shared<'_>) -> Result<(), String> {
    let dir = engine.download_dir();
    if !dir.exists() {
        return Err("La carpeta de descargas no existe todavía.".into());
    }
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|_| "No se pudo abrir la carpeta.".to_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must come first. Opening the app while it is already running (for
        // instance hidden in the tray) brings that window back instead of
        // starting a second copy that would fight over the same files.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let engine = Engine::new(app.handle());
            app.manage(engine.clone());
            // If the system has no tray, the app still works: closing the
            // window then simply quits, as it would without this feature.
            if tray::build(app.handle()).is_ok() {
                engine.tray_is_ready();
            }
            engine.start();
            Ok(())
        })
        .on_window_event(|window, event| {
            let WindowEvent::CloseRequested { api, .. } = event else {
                return;
            };
            if window.label() != "main" {
                return;
            }
            let engine = window.state::<Arc<Engine>>();
            if engine.stays_in_tray() {
                // Keep checking in the background: hide the window instead of closing it.
                api.prevent_close();
                let _ = window.hide();
                engine.inner().window_hidden();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            add_links,
            remove_link,
            clear_completed,
            check_now,
            start_download,
            cancel_download,
            login,
            logout,
            pick_download_dir,
            reset_download_dir,
            set_interval,
            set_notifications,
            test_notification,
            set_keep_in_tray,
            reveal,
            open_download_dir,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // macOS: clicking the Dock icon while the window is hidden brings it back.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen {
                has_visible_windows: false,
                ..
            } = event
            {
                tray::show_main(app);
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}
