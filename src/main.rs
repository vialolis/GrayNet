#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod error;
mod state;
mod config;
mod i2pd;
mod browser;
mod setup;
mod download;

use sysinfo::SystemExt;
use sysinfo::PidExt;
use tauri::{
    Manager,
    SystemTray, SystemTrayMenu, SystemTrayMenuItem, CustomMenuItem,
    SystemTrayEvent,
};
use state::AppState;

// ─── Tauri команды ────────────────────────────────────────────────────────────

#[tauri::command]
fn start_i2pd(
    app_handle: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<String, error::AppError> {
    i2pd::start(&app_handle, &state)
}

#[tauri::command]
fn stop_i2pd(
    state: tauri::State<AppState>,
) -> Result<String, error::AppError> {
    i2pd::stop(&state)
}

#[tauri::command]
fn get_status(
    state: tauri::State<AppState>,
) -> state::StatusResponse {
    i2pd::get_status(&state)
}

#[tauri::command]
fn open_browser(
    app_handle: tauri::AppHandle,
) -> Result<String, error::AppError> {
    browser::launch(&app_handle)
}

#[tauri::command]
fn check_deps(
    app_handle: tauri::AppHandle,
) -> config::DependencyCheck {
    config::check_dependencies(&app_handle)
}

// ─── System Tray ──────────────────────────────────────────────────────────────

fn build_tray_menu() -> SystemTrayMenu {
    let show    = CustomMenuItem::new("show",    "Open GrayNet");
    let browser = CustomMenuItem::new("browser", "Open I2P Browser");
    let quit    = CustomMenuItem::new("quit",    "Quit (stop I2P)");

    SystemTrayMenu::new()
        .add_item(show)
        .add_item(browser)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit)
}

fn handle_tray_event(app: &tauri::AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::DoubleClick { .. } => {
            show_window(app);
        }

        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "show" => {
                show_window(app);
            }
            "browser" => {
                if let Err(e) = browser::launch(app) {
                    log::error!("Failed to launch browser from tray: {}", e);
                }
            }
            "quit" => {
                log::info!("Quit from tray — stopping i2pd...");
                let state = app.state::<AppState>();
                let mut lock = state.i2pd_process.lock().unwrap();
                if let Some(ref mut child) = *lock {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                *lock = None;
                drop(lock);
                // Удаляем lock файл при выходе
                let lock_path = std::env::temp_dir().join("graynet_running.lock");
                let _ = std::fs::remove_file(&lock_path);
                std::process::exit(0);
            }
            _ => {}
        },

        _ => {}
    }
}

fn show_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.unminimize();
    }
}

#[tauri::command]
fn check_browser_installed(
    app_handle: tauri::AppHandle,
) -> bool {
    !download::browser_needs_install(&app_handle)
}

#[tauri::command]
async fn install_browser(
    app_handle: tauri::AppHandle,
    window: tauri::Window,
) -> Result<String, error::AppError> {
    download::download_browser_if_missing(&app_handle, &window).await?;
    Ok("Browser installed successfully".to_string())
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    log::info!("GrayNet starting...");

    // ── Single instance protection ────────────────────────────────────────
    // Читаем lock файл — если есть и процесс с тем PID жив, выходим
    let lock_path = std::env::temp_dir().join("graynet_running.lock");
    if lock_path.exists() {
        if let Ok(pid_str) = std::fs::read_to_string(&lock_path) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                let mut sys = sysinfo::System::new();
                sys.refresh_processes();
                if sys.process(sysinfo::Pid::from_u32(pid)).is_some() {
                    log::info!("GrayNet already running (PID {}), exiting", pid);
                    std::process::exit(0);
                }
                // Процесс мёртв — удаляем старый lock
                log::info!("Stale lock found (PID {}), removing", pid);
            }
        }
    }
    // Записываем наш PID
    let _ = std::fs::write(&lock_path, std::process::id().to_string());
    // ─────────────────────────────────────────────────────────────────────

    let tray = SystemTray::new().with_menu(build_tray_menu());

    tauri::Builder::default()
        .manage(AppState::new())
        .system_tray(tray)

        .setup(|app| {
            let handle = app.handle();
            if let Err(e) = setup::run(&handle) {
                log::error!("Setup failed: {}", e);
            }
            Ok(())
        })

        .on_system_tray_event(handle_tray_event)

        .on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event.event() {
                api.prevent_close();
                let _ = event.window().hide();
                log::info!("Window hidden to tray");

                // Показываем уведомление только первый раз
                let app_handle = event.window().app_handle();
                let state = app_handle.state::<AppState>();
                let mut first = state.first_hide.lock().unwrap();
                if *first {
                    *first = false;
                    let _ = app_handle.tray_handle()
                        .set_tooltip("GrayNet is running. Right-click tray icon → Quit to stop.");
                }
            }
        })

        .invoke_handler(tauri::generate_handler![
            start_i2pd,
            stop_i2pd,
            get_status,
            open_browser,
            check_deps,
            check_browser_installed,
            install_browser,
        ])

        .run(tauri::generate_context!())
        .expect("Fatal: Tauri application crashed");
}