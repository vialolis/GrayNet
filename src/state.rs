use std::process::Child;
use std::sync::Mutex;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")] 
pub enum DaemonStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error(String),
}

pub struct AppState {
    pub i2pd_process: Mutex<Option<Child>>,

    pub daemon_status: Mutex<DaemonStatus>,

    //pub first_hide: Mutex<bool>,
    pub started_at: Mutex<Option<Instant>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            i2pd_process: Mutex::new(None),
            daemon_status: Mutex::new(DaemonStatus::Stopped),
            //first_hide: Mutex::new(true),
            started_at: Mutex::new(None),
        }
    }

    pub fn sync_status(&self) {
        let mut process_lock = self.i2pd_process.lock().unwrap();
        let mut status_lock = self.daemon_status.lock().unwrap();

        if let Some(child) = process_lock.as_mut() {
            match child.try_wait() {
                Ok(Some(exit_status)) => {
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
                    *status_lock = DaemonStatus::Running;
                }
                Err(e) => {
                    *status_lock = DaemonStatus::Error(e.to_string());
                }
            }
        } else {
            if *status_lock == DaemonStatus::Running {
                *status_lock = DaemonStatus::Stopped;
            }
        }
    }

    pub fn uptime_secs(&self) -> Option<u64> {
        self.started_at
            .lock()
            .unwrap()
            .map(|t| t.elapsed().as_secs())
    }
}

#[derive(serde::Serialize)]
pub struct StatusResponse {
    pub status: DaemonStatus,
    pub uptime_secs: Option<u64>,
    pub message: String,
}