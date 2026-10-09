#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Kreative Kompanion desktop app (Apps 1): a Kompanion server's web UI in its own window, a
//! tray icon, and notifications through Tauri's plugin (the web app calls it when present).
//!
//! Client mode (REL-01): the app connects to any Kompanion server. The first start shows a
//! local page (`fallback/index.html`) that asks for the address; the choice is saved in the
//! app's config folder and the tray's "Change server" asks again. `--server <url>` or
//! `KOMPANION_SERVER` picks one without asking.

mod server_url;

use std::path::PathBuf;
use std::time::Duration;

use tauri::ipc::CapabilityBuilder;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
use tauri::{AppHandle, Manager, Url};
use tauri_plugin_notification::init;

/// Suggested on the first start; release builds set it to the project's own server.
const SUGGESTED: Option<&str> = option_env!("KOMPANION_DEFAULT_SERVER");

/// The local page that asks for the server, kept to come back to it.
struct Picker(Url);

/// What the picker page shows in its field.
#[derive(serde::Serialize)]
struct Choice {
    current: Option<String>,
    suggested: Option<String>,
}

fn main() {
    tauri::Builder::default()
        .plugin(init())
        .invoke_handler(tauri::generate_handler![server_choice, connect])
        .setup(|app| {
            let open = MenuItem::with_id(app, "open", "Open Kompanion", true, None::<&str>)?;
            let change = MenuItem::with_id(app, "change", "Change server…", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &change, &quit])?;

            let _tray = tauri::tray::TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Kreative Kompanion")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show(app),
                    "change" => change_server(app),
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
                        show(tray.app_handle());
                    }
                })
                .build(app)?;

            let handle = app.handle();
            if let Some(window) = app.get_webview_window("main") {
                app.manage(Picker(window.url()?));
            }
            let asked = server_from_args().and_then(|a| server_url::normalize(&a).ok());
            if let Some(server) = asked.or_else(|| saved_server(handle)) {
                open_server(handle, &server)?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Kreative Kompanion");
}

/// The saved server and the suggested one, for the picker page.
#[tauri::command]
fn server_choice(app: AppHandle) -> Choice {
    Choice {
        current: saved_server(&app),
        suggested: SUGGESTED.map(str::to_owned),
    }
}

/// Checks that a Kompanion server answers at `address`, saves it and opens it.
#[tauri::command(async)]
fn connect(app: AppHandle, address: String) -> Result<(), String> {
    let server = server_url::normalize(&address).map_err(str::to_owned)?;
    let answered = ureq::get(&format!("{server}/api/status"))
        .timeout(Duration::from_secs(10))
        .call()
        .is_ok();
    if !answered {
        return Err("No Kompanion server answered at this address.".into());
    }
    let file = config_file(&app).ok_or("The app's settings folder is missing.")?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&file, &server).map_err(|e| e.to_string())?;
    open_server(&app, &server)
}

/// `--server <url>` on the command line, else `KOMPANION_SERVER`.
fn server_from_args() -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--server" {
            return args.next();
        }
        if let Some(value) = arg.strip_prefix("--server=") {
            return Some(value.to_owned());
        }
    }
    std::env::var("KOMPANION_SERVER").ok()
}

/// Where the chosen server is kept.
fn config_file(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|dir| dir.join("server"))
}

/// The server saved by an earlier start.
fn saved_server(app: &AppHandle) -> Option<String> {
    let text = std::fs::read_to_string(config_file(app)?).ok()?;
    server_url::normalize(&text).ok()
}

/// Lets the server's pages show notifications, then loads the server in the window.
fn open_server(app: &AppHandle, server: &str) -> Result<(), String> {
    let url = Url::parse(&format!("{server}/")).map_err(|e| e.to_string())?;
    let origin = url.origin().ascii_serialization();
    // One capability per server; the identifier only needs to be unique.
    let id: String = origin.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    app.add_capability(
        CapabilityBuilder::new(format!("server-{id}"))
            .remote(format!("{origin}/*"))
            .window("main")
            .permission("core:default")
            .permission("notification:default"),
    )
    .map_err(|e| e.to_string())?;
    if let Some(window) = app.get_webview_window("main") {
        window.navigate(url).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Tray "Change server": back to the picker page.
fn change_server(app: &AppHandle) {
    if let (Some(window), Some(picker)) = (app.get_webview_window("main"), app.try_state::<Picker>()) {
        let _ = window.navigate(picker.0.clone());
    }
    show(app);
}

fn show(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}
