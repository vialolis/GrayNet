// state.rs
// Централизованный стейт приложения.
//
// В Tauri стейт — это объект который живёт всё время жизни приложения
// и инжектируется в команды автоматически.
// Аналог синглтона в C++, но безопасный — Rust контролирует доступ через Mutex.

use std::process::Child;
use std::sync::Mutex;
use std::time::Instant;

/// Состояние демона i2pd — стейт-машина.
/// В C++ это был бы enum class, в Python — Enum.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]  // "Stopped" -> "stopped" в JSON
pub enum DaemonStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error(String),
}

/// Главный стейт приложения.
/// pub(crate) означает "видимо внутри крейта (проекта), но не снаружи" —
/// аналог internal в C# или protected в C++ для модуля.
pub struct AppState {
    /// Хэндл запущенного процесса i2pd.
    /// Option<Child> — либо None (не запущен), либо Some(Child) (запущен).
    /// Mutex нужен потому что Tauri команды могут вызываться из разных потоков.
    pub i2pd_process: Mutex<Option<Child>>,

    /// Текущий статус для UI.
    pub daemon_status: Mutex<DaemonStatus>,

    pub first_hide: Mutex<bool>,
    /// Время запуска — чтобы показывать uptime.
    /// Option<Instant> — None если не запущен.
    pub started_at: Mutex<Option<Instant>>,
}

impl AppState {
    /// Конструктор — в Rust конвенция это associated function new().
    /// В C++ это был бы конструктор, в Python — __init__.
    pub fn new() -> Self {
        Self {
            i2pd_process: Mutex::new(None),
            daemon_status: Mutex::new(DaemonStatus::Stopped),
            first_hide: Mutex::new(true),
            started_at: Mutex::new(None),
        }
    }

    /// Проверить и обновить статус — если процесс завершился сам, обновить стейт.
    /// &self — immutable borrow, аналог const this* в C++ или self без = в Python.
    pub fn sync_status(&self) {
        let mut process_lock = self.i2pd_process.lock().unwrap();
        let mut status_lock = self.daemon_status.lock().unwrap();

        if let Some(child) = process_lock.as_mut() {
            // try_wait() проверяет завершился ли процесс без блокировки
            match child.try_wait() {
                Ok(Some(exit_status)) => {
                    // Процесс завершился сам (краш или чистый выход)
                    *process_lock = None;
                    *self.started_at.lock().unwrap() = None;
                    if exit_status.success() {
                        *status_lock = DaemonStatus::Stopped;
                    } else {
                        *status_lock = DaemonStatus::Error(
                            format!("i2pd exited with code: {}", exit_status)
                        );
                    }
                }
                Ok(None) => {
                    // Всё ещё работает
                    *status_lock = DaemonStatus::Running;
                }
                Err(e) => {
                    *status_lock = DaemonStatus::Error(e.to_string());
                }
            }
        } else {
            // Нет хэндла — процесс точно не запущен нами
            if *status_lock == DaemonStatus::Running {
                *status_lock = DaemonStatus::Stopped;
            }
        }
    }

    /// Получить uptime в секундах если запущен.
    pub fn uptime_secs(&self) -> Option<u64> {
        self.started_at
            .lock()
            .unwrap()
            .map(|t| t.elapsed().as_secs())
    }
}

/// Структура которую мы отправляем во фронт как JSON ответ на get_status.
/// #[derive(serde::Serialize)] — автоматическая сериализация в JSON.
#[derive(serde::Serialize)]
pub struct StatusResponse {
    pub status: DaemonStatus,
    pub uptime_secs: Option<u64>,
    pub message: String,
}