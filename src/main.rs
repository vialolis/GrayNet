#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::fs;
use dirs;
use sysinfo::System;

static I2PD_PROCESS: Mutex<Option<Child>> = Mutex::new(None);

fn setup_graynet_environment() -> Result<(), String> {
    let data_dir = dirs::data_dir().ok_or("Could not find data directory")?;
    let graynet_dir = data_dir.join("GrayNet");

    // Create GrayNet directory
    fs::create_dir_all(&graynet_dir).map_err(|e| format!("Failed to create GrayNet dir: {}", e))?;

    // Create subdirs
    fs::create_dir_all(graynet_dir.join("config")).map_err(|e| format!("Failed to create config dir: {}", e))?;
    fs::create_dir_all(graynet_dir.join("logs")).map_err(|e| format!("Failed to create logs dir: {}", e))?;
    fs::create_dir_all(graynet_dir.join("bin")).map_err(|e| format!("Failed to create bin dir: {}", e))?;
    fs::create_dir_all(graynet_dir.join("browser")).map_err(|e| format!("Failed to create browser dir: {}", e))?;

    // Generate proxy.pac
    let pac_path = graynet_dir.join("proxy.pac");
    if !pac_path.exists() {
        let pac_content = r#"function FindProxyForURL(url, host) {

    if (dnsDomainIs(host, ".i2p"))
        return "PROXY 127.0.0.1:4444";

    if (dnsDomainIs(host, ".gn"))
        return "PROXY 127.0.0.1:4444";

    return "DIRECT";
}"#;
        fs::write(&pac_path, pac_content).map_err(|e| format!("Failed to write proxy.pac: {}", e))?;
    }

    Ok(())
}

fn launch_librewolf() -> Result<String, String> {
    let data_dir = dirs::data_dir().ok_or("Could not find data directory")?;
    let graynet_dir = data_dir.join("GrayNet");
    let browser_exe = graynet_dir.join("browser").join("LibreWolf-Portable.exe");
    let profile_dir = graynet_dir.join("browser").join("profile");

    // Ensure profile dir exists
    fs::create_dir_all(&profile_dir).map_err(|e| format!("Failed to create profile dir: {}", e))?;

    // Get PAC path
    let pac_path = graynet_dir.join("proxy.pac");
    let pac_url = format!("file:///{}", pac_path.display().to_string().replace("\\", "/"));

    // Create prefs.js
    let prefs_path = profile_dir.join("prefs.js");
    let prefs_content = format!(r#"user_pref("network.proxy.type", 2);
user_pref("network.proxy.autoconfig_url", "{}");
user_pref("dom.security.https_only_mode", true);
user_pref("dom.security.https_only_mode_ever_enabled", true);
user_pref("dom.security.https_only_mode_pbm", true);
user_pref("dom.security.https_only_mode_excluded_hosts", ".i2p,.gn");
"#, pac_url);
    fs::write(&prefs_path, prefs_content).map_err(|e| format!("Failed to write prefs.js: {}", e))?;

    // Launch browser
    Command::new(&browser_exe)
        .arg("-profile")
        .arg(&profile_dir)
        .spawn()
        .map_err(|e| format!("Failed to launch LibreWolf: {}", e))?;

    Ok("LibreWolf launched".to_string())
}

fn find_i2pd_exe() -> Result<PathBuf, String> {
    let data_dir = dirs::data_dir().ok_or("Could not find data directory")?;
    let i2pd_path = data_dir.join("GrayNet").join("bin").join("i2pd.exe");
    if i2pd_path.exists() {
        Ok(i2pd_path)
    } else {
        Err(format!("i2pd.exe not found at: {}", i2pd_path.display()))
    }
}

fn resolve_config_path(_exe_path: &PathBuf) -> Result<PathBuf, String> {
    let data_dir = dirs::data_dir().ok_or("Could not find data directory")?;
    let config_path = data_dir.join("GrayNet").join("config").join("i2pd.conf");
    if config_path.exists() {
        Ok(config_path)
    } else {
        Err(format!(
            "config file not found at: {}",
            config_path.display()
        ))
    }
}

#[tauri::command]
fn start_i2pd() -> Result<String, String> {
    let mut process_lock = I2PD_PROCESS.lock().unwrap();
    if process_lock.is_some() || find_i2pd_pid().is_some() {
        return Err("i2pd is already running".to_string());
    }

    let exe_path = find_i2pd_exe()?;
    let config_path = resolve_config_path(&exe_path)?;

    let child = Command::new(&exe_path)
        .arg("--conf")
        .arg(&config_path)
        .spawn()
        .map_err(|e| format!("Failed to start i2pd: {}", e))?;

    *process_lock = Some(child);
    Ok(format!("i2pd started (exe={})", exe_path.display()))
}

#[tauri::command]
fn stop_i2pd() -> Result<String, String> {
    let mut process_lock = I2PD_PROCESS.lock().unwrap();
    if let Some(mut child) = process_lock.take() {
        child.kill().map_err(|e| format!("Failed to stop i2pd: {}", e))?;
        return Ok("i2pd stopped".to_string());
    }

    // If we don't have a tracked handle, attempt to stop any running i2pd process.
    if let Some(pid) = find_i2pd_pid() {
        let mut system = System::new_all();
        system.refresh_process(pid);
        if let Some(process) = system.process(pid) {
            if process.kill() {
                return Ok("i2pd stopped".to_string());
            }
        }
    }

    Err("i2pd is not running".to_string())
}

fn find_i2pd_pid() -> Option<sysinfo::Pid> {
    let mut system = System::new_all();
    system.refresh_processes();
    system.processes().values().find_map(|p| {
        let name = p.name().to_ascii_lowercase();
        if name == "i2pd.exe" || name == "i2pd" {
            Some(p.pid())
        } else {
            None
        }
    })
}

#[tauri::command]
fn get_status() -> String {
    let mut process_lock = I2PD_PROCESS.lock().unwrap();

    // If we have a tracked child, check if it's still running.
    if let Some(child) = process_lock.as_mut() {
        match child.try_wait() {
            Ok(Some(_status)) => {
                // process exited
                *process_lock = None;
            }
            Ok(None) => {
                return "Running".to_string();
            }
            Err(_) => {
                // If we can't check, but handle exists, assume running.
                return "Running".to_string();
            }
        }
    }

    // Fallback: check system processes for i2pd.exe
    if find_i2pd_pid().is_some() {
        return "Running".to_string();
    }

    "Stopped".to_string()
}

#[tauri::command]
fn open_browser() -> Result<String, String> {
    launch_librewolf()
}

fn main() {
    // Setup GrayNet environment
    if let Err(e) = setup_graynet_environment() {
        eprintln!("Failed to setup GrayNet environment: {}", e);
        // Continue anyway
    }

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![start_i2pd, stop_i2pd, get_status, open_browser])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

        
