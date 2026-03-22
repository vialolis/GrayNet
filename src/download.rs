// download.rs
// Скачивание и распаковка LibreWolf Portable при первом запуске.
//
// Схема работы:
// 1. Проверяем есть ли уже браузер
// 2. Если нет — скачиваем ZIP с официального сайта LibreWolf
// 3. Показываем прогресс во фронт через tauri events
// 4. Распаковываем ZIP в GrayNet\browser\
// 5. Удаляем ZIP

use std::io::{Read, Write};
use std::path::PathBuf;
use futures_util::StreamExt; // для .next() на stream
use crate::error::{AppError, AppResult};

// Официальная ссылка на LibreWolf Portable для Windows.
// При выходе новой версии LibreWolf — обновить эту константу.
// Формат: librewolf-{version}.en-US.win64-portable.zip
const LIBREWOLF_VERSION: &str = "146.0.1-1";
// Файлы хостятся на GitLab через API
const LIBREWOLF_PORTABLE_URL: &str =
    "https://gitlab.com/api/v4/projects/44042130/packages/generic/librewolf/146.0.1-1/librewolf-146.0.1-1-windows-x86_64-portable.zip";

/// Структура прогресса — отправляется во фронт как JSON событие.
/// serde::Serialize позволяет tauri::Window::emit() сериализовать это в JSON.
#[derive(serde::Serialize, Clone)]
pub struct DownloadProgress {
    /// Текущий этап для отображения в UI
    pub stage: String,
    /// Прогресс от 0 до 100
    pub percent: u8,
    /// Человекочитаемый статус
    pub message: String,
}

impl DownloadProgress {
    fn new(stage: &str, percent: u8, message: &str) -> Self {
        Self {
            stage: stage.to_string(),
            percent,
            message: message.to_string(),
        }
    }
}

/// Главная функция — проверяет наличие браузера и скачивает если нужно.
///
/// app_handle — нужен для resolve_resource и путей
/// window — нужен для emit событий прогресса во фронт
///
/// async fn — асинхронная функция, не блокирует UI поток.
/// В C++ аналог std::future, в Python — async def.
pub async fn download_browser_if_missing(
    app_handle: &tauri::AppHandle,
    window: &tauri::Window,
) -> AppResult<()> {
    // Проверяем есть ли уже браузер
    if crate::config::get_browser_path(app_handle).is_ok() {
        log::info!("Browser already installed, skipping download");
        return Ok(());
    }

    log::info!("Browser not found, starting download...");

    // Папка куда распакуем браузер
    let browser_dir = crate::config::graynet_data_dir()?.join("browser");
    std::fs::create_dir_all(&browser_dir)?;

    // Временный файл для скачанного ZIP
    let zip_path = browser_dir.join("librewolf-portable.zip");

    // --- Этап 1: Скачать ZIP ---
    emit_progress(&window, "download", 0, "Starting download...");

    download_file(LIBREWOLF_PORTABLE_URL, &zip_path, &window).await?;

    emit_progress(&window, "download", 100, "Download complete");

    // --- Этап 2: Распаковать ZIP ---
    emit_progress(&window, "extract", 0, "Extracting...");

    extract_zip(&zip_path, &browser_dir, &window)?;

    // --- Этап 3: Убрать ZIP ---
    if zip_path.exists() {
        std::fs::remove_file(&zip_path)
            .map_err(|e| AppError::IoError(format!("Failed to remove zip: {}", e)))?;
    }

    emit_progress(&window, "done", 100, "Browser ready!");
    log::info!("LibreWolf installed successfully at {}", browser_dir.display());

    Ok(())
}

/// Скачать файл по URL с отображением прогресса.
///
/// reqwest::get возвращает Future — мы await его.
/// .bytes_stream() даёт нам поток кусков данных (chunks).
async fn download_file(
    url: &str,
    dest: &PathBuf,
    window: &tauri::Window,
) -> AppResult<()> {
    log::info!("Downloading: {}", url);

    let response = reqwest::get(url)
        .await
        .map_err(|e| AppError::Other(format!("Download failed: {}", e)))?;

    // Проверяем HTTP статус
    if !response.status().is_success() {
        return Err(AppError::Other(format!(
            "HTTP error: {}",
            response.status()
        )));
    }

    // Content-Length нужен для расчёта процента прогресса.
    // Option<u64> — может не быть в заголовках, тогда прогресс будет без %
    let total_size = response.content_length();

    log::info!(
        "File size: {}",
        total_size
            .map(|s| format!("{:.1} MB", s as f64 / 1_048_576.0))
            .unwrap_or("unknown".to_string())
    );

    // Открываем файл для записи
    let mut file = std::fs::File::create(dest)
        .map_err(|e| AppError::IoError(format!("Cannot create file: {}", e)))?;

    // bytes_stream() возвращает Stream<Item = Result<Bytes>>
    // Это аналог генератора в Python или IAsyncEnumerable в C#
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_percent: u8 = 0;

    // while let — итерируем по кускам потока
    // StreamExt::next() — получаем следующий кусок
    while let Some(chunk) = stream.next().await {
        let chunk = chunk
            .map_err(|e| AppError::Other(format!("Stream error: {}", e)))?;

        file.write_all(&chunk)
            .map_err(|e| AppError::IoError(format!("Write error: {}", e)))?;

        downloaded += chunk.len() as u64;

        // Обновляем прогресс только если изменился на 1%+ — не спамим события
        if let Some(total) = total_size {
            let percent = ((downloaded as f64 / total as f64) * 100.0) as u8;
            if percent > last_percent {
                last_percent = percent;
                let mb_done = downloaded as f64 / 1_048_576.0;
                let mb_total = total as f64 / 1_048_576.0;
                emit_progress(
                    window,
                    "download",
                    percent,
                    &format!("{:.1} / {:.1} MB", mb_done, mb_total),
                );
            }
        } else {
            // Размер неизвестен — показываем только скачанные MB
            let mb_done = downloaded as f64 / 1_048_576.0;
            emit_progress(
                window,
                "download",
                50, // фиксированный "неизвестный" прогресс
                &format!("{:.1} MB downloaded...", mb_done),
            );
        }
    }

    log::info!("Download complete: {} bytes", downloaded);
    Ok(())
}

/// Распаковать ZIP архив в папку назначения.
///
/// zip крейт работает синхронно — это нормально для распаковки,
/// она быстрая и не блокирует надолго.
fn extract_zip(
    zip_path: &PathBuf,
    dest_dir: &PathBuf,
    window: &tauri::Window,
) -> AppResult<()> {
    log::info!("Extracting {} to {}", zip_path.display(), dest_dir.display());

    let file = std::fs::File::open(zip_path)
        .map_err(|e| AppError::IoError(format!("Cannot open zip: {}", e)))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::Other(format!("Invalid zip: {}", e)))?;

    let total = archive.len();

    for i in 0..total {
        let mut zip_file = archive
            .by_index(i)
            .map_err(|e| AppError::Other(format!("Zip error at {}: {}", i, e)))?;

        // Путь внутри ZIP — нормализуем для безопасности
        // (защита от path traversal атак типа "../../evil.exe")
        let outpath = match zip_file.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => {
                log::warn!("Skipping suspicious path in zip at index {}", i);
                continue;
            }
        };

        // Директория внутри ZIP
        if zip_file.is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            // Файл — создаём родительскую директорию и пишем
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut outfile = std::fs::File::create(&outpath)
                .map_err(|e| AppError::IoError(format!("Cannot create {}: {}", outpath.display(), e)))?;

            // std::io::copy — копирует из Reader в Writer по кускам
            // В C++ аналог std::copy с stream iterators
            std::io::copy(&mut zip_file, &mut outfile)
                .map_err(|e| AppError::IoError(format!("Extract error: {}", e)))?;
        }

        // Прогресс распаковки
        let percent = ((i as f64 / total as f64) * 100.0) as u8;
        // Обновляем каждые 5% чтобы не спамить
        if percent % 5 == 0 {
            emit_progress(
                window,
                "extract",
                percent,
                &format!("Extracting files... {}/{}", i + 1, total),
            );
        }
    }

    log::info!("Extraction complete: {} files", total);
    Ok(())
}

/// Отправить событие прогресса во фронт.
/// Фронт слушает "browser-setup-progress" и обновляет UI.
fn emit_progress(window: &tauri::Window, stage: &str, percent: u8, message: &str) {
    let progress = DownloadProgress::new(stage, percent, message);
    // emit возвращает Result — игнорируем ошибку если окно закрыто
    let _ = window.emit("browser-setup-progress", progress);
    log::debug!("Progress [{}] {}%: {}", stage, percent, message);
}

/// Проверить нужна ли установка браузера — для фронта.
pub fn browser_needs_install(app_handle: &tauri::AppHandle) -> bool {
    crate::config::get_browser_path(app_handle).is_err()
}