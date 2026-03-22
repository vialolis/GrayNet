// browser.rs
// Всё что связано с запуском LibreWolf.
//
// Генерация прокси-конфига, PAC файла, профиля браузера.

use std::fs;
use std::process::Command;
use crate::error::{AppError, AppResult};
use crate::config::{get_browser_path, get_pac_path, get_browser_profile_path};

/// Сгенерировать proxy.pac файл.
/// PAC (Proxy Auto-Config) — JavaScript функция которую браузер вызывает
/// для каждого URL чтобы решить использовать ли прокси.
pub fn write_pac_file() -> AppResult<()> {
    let pac_path = get_pac_path()?;

    // Если уже есть — не перезаписываем (юзер мог изменить)
    if pac_path.exists() {
        log::debug!("proxy.pac already exists, skipping");
        return Ok(());
    }

    let pac_content = r#"function FindProxyForURL(url, host) {
    // .i2p и .gn домены — через i2p прокси
    if (dnsDomainIs(host, ".i2p"))
        return "PROXY 127.0.0.1:4444";

    if (dnsDomainIs(host, ".gn"))
        return "PROXY 127.0.0.1:4444";

    // Всё остальное — напрямую
    return "DIRECT";
}
"#;

    fs::write(&pac_path, pac_content)?;
    log::info!("Created proxy.pac at {}", pac_path.display());
    Ok(())
}

/// Записать prefs.js для профиля LibreWolf.
/// Firefox/LibreWolf читает этот файл при старте и применяет настройки.
fn write_browser_prefs() -> AppResult<()> {
    let profile_dir = get_browser_profile_path()?;
    fs::create_dir_all(&profile_dir)?;

    let pac_path = get_pac_path()?;

    // Формируем file:// URL для PAC файла
    // На Windows пути надо нормализовать: \ -> /
    let pac_url = if cfg!(target_os = "windows") {
        format!(
            "file:///{}",
            pac_path.display().to_string().replace('\\', "/")
        )
    } else {
        format!("file://{}", pac_path.display())
    };

    // format! — аналог f-string в Python или std::format в C++20
    let prefs_content = format!(
    r#"// GrayNet auto-generated preferences - do not edit manually
user_pref("network.proxy.type", 2);
user_pref("network.proxy.autoconfig_url", "{pac_url}");

// HTTPS-Only полностью отключён — i2p/gn сеть работает на HTTP
user_pref("dom.security.https_only_mode", false);
user_pref("dom.security.https_only_mode_ever_enabled", false);
user_pref("dom.security.https_only_mode_pbm", false);

// DNS over HTTPS отключён — резолвинг идёт через i2pd прокси
user_pref("network.trr.mode", 5);

// Отключаем телеметрию
user_pref("datareporting.healthreport.uploadEnabled", false);
user_pref("datareporting.policy.dataSubmissionEnabled", false);

// Отключаем предупреждение "Not Secure" для HTTP
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

    // NoScript — ID расширения это имя файла
    // Официальный ID NoScript: {73a6fe31-595d-460b-a920-fcc0f8843232}
    let noscript_dest = ext_dir.join("{73a6fe31-595d-460b-a920-fcc0f8843232}.xpi");

    if noscript_dest.exists() {
        log::info!("NoScript already installed");
        return Ok(());
    }

    // Берём из бандла
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

/// Запустить LibreWolf с нужным профилем.
pub fn launch(app_handle: &tauri::AppHandle) -> AppResult<String> {
    let browser_path = get_browser_path(app_handle)?;

    // Подготовить PAC и профиль
    write_pac_file()?;
    write_browser_prefs()?;
    install_extensions(app_handle)?;

    let profile_dir = get_browser_profile_path()?;

    log::info!("Launching browser: {}", browser_path.display());

    // LibreWolf/Firefox флаги:
    // -profile <dir>  — использовать конкретный профиль
    // -no-remote      — запустить новый процесс даже если уже открыт FF
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