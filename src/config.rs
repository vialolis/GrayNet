use std::path::PathBuf;
use crate::error::{AppError, AppResult};


fn i2pd_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "i2pd.exe"
    } else {
        "i2pd"
    }
}

pub fn graynet_data_dir() -> AppResult<PathBuf> {
    dirs::data_local_dir()
        .ok_or_else(|| AppError::Other("Could not find AppData\\Local".to_string()))
        .map(|d| d.join("GrayNet"))
}

pub fn get_i2pd_path(app_handle: &tauri::AppHandle) -> AppResult<PathBuf> {
    let binary_name = i2pd_binary_name();

    if let Some(bundled) = app_handle
        .path_resolver()
        .resolve_resource(format!("binaries/{}", binary_name))
    {
        if bundled.exists() {
            return Ok(bundled);
        }
    }

    let path = graynet_data_dir()?.join("bin").join(binary_name);
    if path.exists() {
        Ok(path)
    } else {
        Err(AppError::BinaryNotFound(path.display().to_string()))
    }
}

pub fn get_browser_path(app_handle: &tauri::AppHandle) -> AppResult<PathBuf> {
    let browser_dir = graynet_data_dir()?.join("browser");

    if let Ok(entries) = std::fs::read_dir(&browser_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                for name in &["LibreWolf-Portable.exe", "librewolf.exe"] {
                    let exe = entry.path().join(name);
                    if exe.exists() {
                        return Ok(exe);
                    }
                }
            }
        }
    }

    let legacy = browser_dir.join("LibreWolf-Portable.exe");
    if legacy.exists() {
        return Ok(legacy);
    }

    if let Some(bundled) = app_handle
        .path_resolver()
        .resolve_resource("binaries/LibreWolf-Portable.exe")
    {
        if bundled.exists() {
            return Ok(bundled);
        }
    }

    Err(AppError::BinaryNotFound(
        browser_dir.display().to_string()
    ))
}

pub fn get_i2pd_config_path(app_handle: &tauri::AppHandle) -> AppResult<PathBuf> {
    if let Some(bundled) = app_handle
        .path_resolver()
        .resolve_resource("config/i2pd.conf")
    {
        if bundled.exists() {
            return Ok(bundled);
        }
    }

    let path = graynet_data_dir()?.join("config").join("i2pd.conf");
    if path.exists() {
        Ok(path)
    } else {
        Err(AppError::ConfigNotFound(path.display().to_string()))
    }
}

pub fn get_pac_path() -> AppResult<PathBuf> {
    Ok(graynet_data_dir()?.join("proxy.pac"))
}

pub fn get_browser_profile_path() -> AppResult<PathBuf> {
    Ok(graynet_data_dir()?.join("browser").join("profile"))
}

#[derive(serde::Serialize, Debug)]
pub struct DependencyCheck {
    pub i2pd_found: bool,
    pub browser_found: bool,
    pub config_found: bool,
    pub missing: Vec<String>,
}

pub fn check_dependencies(app_handle: &tauri::AppHandle) -> DependencyCheck {
    let mut missing = Vec::new();

    let i2pd_found = match get_i2pd_path(app_handle) {
        Ok(p) => { log::info!("i2pd found at: {}", p.display()); true }
        Err(e) => { missing.push(e.to_string()); false }
    };

    let browser_found = match get_browser_path(app_handle) {
        Ok(p) => { log::info!("Browser found at: {}", p.display()); true }
        Err(e) => { missing.push(e.to_string()); false }
    };

    let config_found = match get_i2pd_config_path(app_handle) {
        Ok(p) => { log::info!("Config found at: {}", p.display()); true }
        Err(e) => { log::warn!("Config not found (will use defaults): {}", e); false }
    };

    DependencyCheck { i2pd_found, browser_found, config_found, missing }
}