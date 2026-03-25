use std::fs;
use crate::error::AppResult;
use crate::config::graynet_data_dir;

pub fn create_directories() -> AppResult<()> {
    let base = graynet_data_dir()?;

    let dirs = [
        base.join("bin"),
        base.join("config"),
        base.join("logs"),
        base.join("browser"),
        base.join("browser").join("profile"),
        base.join("i2pd"),
        base.join("i2pd").join("graynet"),
        base.join("i2pd").join("addressbook"),
    ];

    for dir in &dirs {
        fs::create_dir_all(dir)?;
        log::debug!("Ensured directory: {}", dir.display());
    }

    log::info!("GrayNet directories ready at: {}", base.display());
    Ok(())
}

pub fn copy_graynet_zones(app_handle: &tauri::AppHandle) -> AppResult<()> {
    let zones_dest = crate::config::graynet_data_dir()?
        .join("i2pd").join("graynet").join("zones.txt");
    let version_dest = crate::config::graynet_data_dir()?
        .join("i2pd").join("graynet").join("zones.version");

    const BUNDLED_VERSION: u32 = 1;

    let installed_version: u32 = std::fs::read_to_string(&version_dest)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    if installed_version >= BUNDLED_VERSION && zones_dest.exists() {
        log::info!("zones.txt is up to date (v{})", installed_version);
        return Ok(());
    }

    if let Some(parent) = zones_dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if let Some(src) = app_handle
        .path_resolver()
        .resolve_resource("config/zones.txt")
    {
        std::fs::copy(&src, &zones_dest)?;
        std::fs::write(&version_dest, BUNDLED_VERSION.to_string())?;
        log::info!("Updated zones.txt to v{}", BUNDLED_VERSION);
    }

    Ok(())
}

pub fn copy_default_addressbook(app_handle: &tauri::AppHandle) -> AppResult<()> {
    let dest = crate::config::graynet_data_dir()?
        .join("i2pd")           
        .join("addressbook")
        .join("addresses.csv");

    if dest.exists() {
        log::info!("Addressbook already exists, skipping");
        return Ok(());
    }

    if let Some(src) = app_handle
        .path_resolver()
        .resolve_resource("config/addresses.csv")
    {
        std::fs::copy(&src, &dest)?;
        log::info!("Copied default addressbook");
    } else {
        log::warn!("Default addressbook not found in bundle");
    }

    Ok(())
}

pub fn run(app_handle: &tauri::AppHandle) -> AppResult<()> {
    log::info!("Running GrayNet setup...");

    create_directories()?;
    crate::browser::write_pac_file()?;

    copy_default_addressbook(app_handle)?;
    copy_graynet_zones(app_handle)?;
    let deps = crate::config::check_dependencies(app_handle);
    if !deps.missing.is_empty() {
        for msg in &deps.missing {
            log::warn!("Missing dependency: {}", msg);
        }
    } else {
        log::info!("All dependencies found");
    }

    log::info!("Setup complete");
    Ok(())
}