// setup.rs
// Первичная настройка окружения при запуске.
// Создаёт нужные директории, проверяет зависимости.
//
// Отдельный модуль потому что эта логика нужна только один раз при старте,
// и не должна смешиваться с основной логикой управления процессами.

use std::fs;
use crate::error::AppResult;
use crate::config::graynet_data_dir;

/// Создать всю структуру директорий GrayNet.
/// Если директории уже существуют — create_dir_all молча пропустит.
pub fn create_directories() -> AppResult<()> {
    let base = graynet_data_dir()?;

    // Список всех нужных поддиректорий
    // &[...] — slice литерал, аналог std::initializer_list в C++
    let dirs = [
        base.join("bin"),
        base.join("config"),
        base.join("logs"),
        base.join("browser"),
        base.join("browser").join("profile"),
        base.join("i2pd"),
        base.join("i2pd").join("graynet"),
    ];

    // iter() даёт итератор, for_each — как forEach в JS или for в Python
    for dir in &dirs {
        fs::create_dir_all(dir)?;
        log::debug!("Ensured directory: {}", dir.display());
    }

    log::info!("GrayNet directories ready at: {}", base.display());
    Ok(())
}

/// Полный setup при запуске приложения.
/// Вызывается из main() один раз.
pub fn run(app_handle: &tauri::AppHandle) -> AppResult<()> {
    log::info!("Running GrayNet setup...");

    // 1. Создаём директории
    create_directories()?;
    // 2. Создаём proxy.pac если не существует
    crate::browser::write_pac_file()?;

    // 3. Проверяем зависимости и логируем
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