use std::process::{Command, Stdio};
use crate::error::{AppError, AppResult};
use crate::state::{AppState, DaemonStatus};

pub fn start(
    app_handle: &tauri::AppHandle,
    state: &AppState,
) -> AppResult<String> {
    state.sync_status();

    {
        let status = state.daemon_status.lock().unwrap();
        if *status == DaemonStatus::Running || *status == DaemonStatus::Starting {
            return Err(AppError::AlreadyRunning);
        }
    }

    let exe_path = crate::config::get_i2pd_path(app_handle)?;

    *state.daemon_status.lock().unwrap() = DaemonStatus::Starting;

    let mut cmd = Command::new(&exe_path);
    cmd.stdout(Stdio::null())
       .stderr(Stdio::null());

    if let Ok(config_path) = crate::config::get_i2pd_config_path(app_handle) {
        cmd.arg("--conf").arg(&config_path);
        log::info!("Using config: {}", config_path.display());
    } else {
        log::warn!("No config found, i2pd will use defaults");
    }

    let i2pd_data_dir = crate::config::graynet_data_dir()?.join("i2pd");
        cmd.arg(format!("--datadir={}", i2pd_data_dir.display()));
    let log_path = crate::config::graynet_data_dir()
        .map(|d| d.join("logs").join("i2pd.log"))
        .unwrap_or_default();
    cmd.arg("--log=file")
       .arg(format!("--logfile={}", log_path.display()));

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    log::info!("Spawning i2pd: {}", exe_path.display());

    let child = cmd.spawn()
        .map_err(|e| AppError::SpawnFailed(e.to_string()))?;

    *state.i2pd_process.lock().unwrap() = Some(child);
    *state.daemon_status.lock().unwrap() = DaemonStatus::Running;
    *state.started_at.lock().unwrap() = Some(std::time::Instant::now());

    log::info!("i2pd started successfully");
    Ok(format!("i2pd started from {}", exe_path.display()))
}

pub fn stop(state: &AppState) -> AppResult<String> {
    state.sync_status();

    let mut process_lock = state.i2pd_process.lock().unwrap();

    if let Some(mut child) = process_lock.take() {
        *state.daemon_status.lock().unwrap() = DaemonStatus::Stopping;

        #[cfg(not(target_os = "windows"))]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                libc::kill(child.id() as libc::pid_t, libc::SIGTERM);
            }
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                if let Ok(Some(_)) = child.try_wait() {
                    break;
                }
                if std::time::Instant::now() > deadline {
                    log::warn!("i2pd did not exit gracefully, killing forcefully");
                    let _ = child.kill();
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        #[cfg(target_os = "windows")]
        {
            child.kill().map_err(|e| AppError::Other(e.to_string()))?;
        }

        let _ = child.wait();

        *state.daemon_status.lock().unwrap() = DaemonStatus::Stopped;
        *state.started_at.lock().unwrap() = None;

        log::info!("i2pd stopped");
        Ok("i2pd stopped".to_string())
    } else {
        Err(AppError::NotRunning)
    }
}

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