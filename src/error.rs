use std::fmt;

pub enum AppError {
    BinaryNotFound(String),
    ConfigNotFound(String),
    IoError(String),
    AlreadyRunning,
    NotRunning,
    SpawnFailed(String),
    Other(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::BinaryNotFound(path) =>
                write!(f, "Binary not found: {}", path),
            AppError::ConfigNotFound(path) =>
                write!(f, "Config not found: {}", path),
            AppError::IoError(msg) =>
                write!(f, "IO error: {}", msg),
            AppError::AlreadyRunning =>
                write!(f, "i2pd is already running"),
            AppError::NotRunning =>
                write!(f, "i2pd is not running"),
            AppError::SpawnFailed(msg) =>
                write!(f, "Failed to start process: {}", msg),
            AppError::Other(msg) =>
                write!(f, "{}", msg),
        }
    }
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::IoError(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;