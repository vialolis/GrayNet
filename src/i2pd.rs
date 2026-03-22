// i2pd.rs
// Всё что связано с управлением процессом i2pd.
//
// Этот модуль не знает ничего про UI — только про запуск/стоп процесса.
// Разделение ответственности: если завтра меняешь как запускается i2pd,
// правишь только этот файл.

use std::process::{Command, Stdio};
use crate::error::{AppError, AppResult};
use crate::state::{AppState, DaemonStatus};

/// Запустить i2pd.
/// Принимает:
///   - app_handle: нужен для resolve_resource (пути к бандлу)
///   - state: инжектированный стейт приложения
///
/// В Rust & перед типом — это borrow (ссылка), аналог const& в C++.
/// Мы берём state на время функции, не перемещаем владение.
pub fn start(
    app_handle: &tauri::AppHandle,
    state: &AppState,
) -> AppResult<String> {
    // Sync сначала — может процесс уже упал сам
    state.sync_status();

    // Проверяем текущий статус
    {
        // Блок нужен чтобы lock освободился до spawn — иначе deadlock
        let status = state.daemon_status.lock().unwrap();
        if *status == DaemonStatus::Running || *status == DaemonStatus::Starting {
            return Err(AppError::AlreadyRunning);
        }
    }

    // Получаем пути через config модуль
    let exe_path = crate::config::get_i2pd_path(app_handle)?;

    // Устанавливаем статус Starting
    *state.daemon_status.lock().unwrap() = DaemonStatus::Starting;

    // Строим команду запуска
    // Stdio::null() — перенаправляем stdout/stderr в /dev/null чтобы не засорять
    // В будущем можно заменить на Stdio::piped() для стриминга логов в UI
    let mut cmd = Command::new(&exe_path);
    cmd.stdout(Stdio::null())
       .stderr(Stdio::null());

    // Добавляем конфиг если есть — i2pd будет работать и без него (дефолты)
    if let Ok(config_path) = crate::config::get_i2pd_config_path(app_handle) {
        cmd.arg("--conf").arg(&config_path);
        log::info!("Using config: {}", config_path.display());
    } else {
        log::warn!("No config found, i2pd will use defaults");
    }

    let i2pd_data_dir = crate::config::graynet_data_dir()?.join("i2pd");
        cmd.arg(format!("--datadir={}", i2pd_data_dir.display()));
    // Пишем логи i2pd в файл вместо stdout
    let log_path = crate::config::graynet_data_dir()
        .map(|d| d.join("logs").join("i2pd.log"))
        .unwrap_or_default();
    cmd.arg("--log=file")
       .arg(format!("--logfile={}", log_path.display()));

    // Платформо-специфичные флаги
    // На Windows скрываем консольное окно i2pd
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    log::info!("Spawning i2pd: {}", exe_path.display());

    // spawn() запускает процесс и сразу возвращает Child хэндл.
    // Это не блокирует — i2pd работает параллельно.
    // map_err конвертирует io::Error в наш AppError.
    let child = cmd.spawn()
        .map_err(|e| AppError::SpawnFailed(e.to_string()))?;

    // Сохраняем хэндл и обновляем стейт
    *state.i2pd_process.lock().unwrap() = Some(child);
    *state.daemon_status.lock().unwrap() = DaemonStatus::Running;
    *state.started_at.lock().unwrap() = Some(std::time::Instant::now());

    log::info!("i2pd started successfully");
    Ok(format!("i2pd started from {}", exe_path.display()))
}

/// Остановить i2pd.
pub fn stop(state: &AppState) -> AppResult<String> {
    state.sync_status();

    let mut process_lock = state.i2pd_process.lock().unwrap();

    if let Some(mut child) = process_lock.take() {
        // .take() забирает значение из Option, оставляя None — аналог std::move в C++
        *state.daemon_status.lock().unwrap() = DaemonStatus::Stopping;

        // Сначала пробуем graceful shutdown через SIGTERM (Linux/macOS)
        // На Windows kill() сразу TerminateProcess
        #[cfg(not(target_os = "windows"))]
        {
            use std::os::unix::process::CommandExt;
            // SIGTERM даёт процессу шанс сохранить состояние
            unsafe {
                libc::kill(child.id() as libc::pid_t, libc::SIGTERM);
            }
            // Ждём до 5 секунд на graceful выход
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                if let Ok(Some(_)) = child.try_wait() {
                    break; // Процесс завершился
                }
                if std::time::Instant::now() > deadline {
                    // Не успел — убиваем принудительно
                    log::warn!("i2pd did not exit gracefully, killing forcefully");
                    let _ = child.kill();
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        // На Windows — сразу kill
        #[cfg(target_os = "windows")]
        {
            child.kill().map_err(|e| AppError::Other(e.to_string()))?;
        }

        // Ждём завершения чтобы не оставить zombie process
        let _ = child.wait();

        *state.daemon_status.lock().unwrap() = DaemonStatus::Stopped;
        *state.started_at.lock().unwrap() = None;

        log::info!("i2pd stopped");
        Ok("i2pd stopped".to_string())
    } else {
        Err(AppError::NotRunning)
    }
}

/// Получить текущий статус для UI.
/// Этот метод вызывается часто (polling из фронта), поэтому лёгкий.
pub fn get_status(state: &AppState) -> crate::state::StatusResponse {
    state.sync_status();

    let status = state.daemon_status.lock().unwrap().clone();
    let uptime = state.uptime_secs();

    let message = match &status {
        DaemonStatus::Stopped  => "I2P daemon is not running".to_string(),
        DaemonStatus::Starting => "Starting I2P daemon...".to_string(),
        DaemonStatus::Running  => {
            if let Some(secs) = uptime {
                format!("Running for {}m {}s", secs / 60, secs % 60)
            } else {
                "Running".to_string()
            }
        }
        DaemonStatus::Stopping => "Stopping I2P daemon...".to_string(),
        DaemonStatus::Error(e) => format!("Error: {}", e),
    };

    crate::state::StatusResponse { status, uptime_secs: uptime, message }
}