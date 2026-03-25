use std::fs;
use std::process::Command;
use crate::error::{AppError, AppResult};
use crate::config::{get_browser_path, get_pac_path, get_browser_profile_path};

pub fn write_pac_file() -> AppResult<()> {
    let pac_path = get_pac_path()?;

    if pac_path.exists() {
        log::debug!("proxy.pac already exists, skipping");
        return Ok(());
    }

    let pac_content = r#"
    function FindProxyForURL(url, host) {
        if (host.endsWith('.i2p')) {
            return "PROXY 127.0.0.1:4444";
        }
        if (dnsDomainIs(host, ".gn"))
            return "PROXY 127.0.0.1:4444";
        }
        return "DIRECT";
    }
"#;

    fs::write(&pac_path, pac_content)?;
    log::info!("Created proxy.pac at {}", pac_path.display());
    Ok(())
}

fn write_browser_prefs() -> AppResult<()> {
    let profile_dir = get_browser_profile_path()?;
    fs::create_dir_all(&profile_dir)?;

    let pac_path = get_pac_path()?;

    let pac_url = if cfg!(target_os = "windows") {
        format!(
            "file:///{}",
            pac_path.display().to_string().replace('\\', "/")
        )
    } else {
        format!("file://{}", pac_path.display())
    };

    let prefs_content = format!(
    r#"// GrayNet auto-generated preferences - do not edit manually
user_pref("network.proxy.type", 2);
user_pref("network.proxy.autoconfig_url", "{pac_url}");

// HTTPS-Only
user_pref("dom.security.https_only_mode", false);
user_pref("dom.security.https_only_mode_ever_enabled", false);
user_pref("dom.security.https_only_mode_pbm", false);

// DNS over HTTPS
user_pref("network.trr.mode", 5);

user_pref("datareporting.healthreport.uploadEnabled", false);
user_pref("datareporting.policy.dataSubmissionEnabled", false);

user_pref("security.insecure_connection_text.enabled", false);
user_pref("security.insecure_connection_icon.enabled", false);
"#
);

    let prefs_path = profile_dir.join("prefs.js");
    fs::write(&prefs_path, prefs_content)?;
    log::info!("Written browser prefs to {}", prefs_path.display());
    Ok(())
}

pub fn install_extensions(app_handle: &tauri::AppHandle) -> AppResult<()> {
    let profile_dir = get_browser_profile_path()?;
    let ext_dir = profile_dir.join("extensions");
    std::fs::create_dir_all(&ext_dir)?;

    let noscript_dest = ext_dir.join("{73a6fe31-595d-460b-a920-fcc0f8843232}.xpi");

    if noscript_dest.exists() {
        log::info!("NoScript already installed");
        return Ok(());
    }

    if let Some(src) = app_handle
        .path_resolver()
        .resolve_resource("extensions/noscript.xpi")
    {
        std::fs::copy(&src, &noscript_dest)?;
        log::info!("NoScript installed");
    } else {
        log::warn!("NoScript xpi not found in bundle");
    }

    Ok(())
}

pub fn launch(app_handle: &tauri::AppHandle) -> AppResult<String> {
    let browser_path = get_browser_path(app_handle)?;

    write_pac_file()?;
    write_browser_prefs()?;
    install_extensions(app_handle)?;

    let profile_dir = get_browser_profile_path()?;

    log::info!("Launching browser: {}", browser_path.display());


    Command::new(&browser_path)
        .arg("-profile")
        .arg(&profile_dir)
        .arg("-no-remote")
        .spawn()
        .map_err(|e| AppError::SpawnFailed(
            format!("Failed to launch browser: {}", e)
        ))?;

    Ok("Browser launched".to_string())
}