// error.rs
// Централизованный тип ошибок для всего приложения.
//
// В Rust нет исключений как в C++/Python — вместо них Result<T, E>.
// Здесь мы определяем наш тип E, чтобы все модули возвращали одинаковые ошибки.
// #[derive(...)] — компилятор автоматически генерирует реализации трейтов,
// аналог __repr__, __str__ и операторов в Python/C++.

use std::fmt;

/// Все возможные ошибки приложения в одном enum.
/// В C++ это был бы набор исключений, в Python — иерархия Exception.
#[derive(Debug)]  // позволяет печатать через {:?}
pub enum AppError {
    /// i2pd / browser бинарь не найден
    BinaryNotFound(String),
    /// Конфиг файл не найден
    ConfigNotFound(String),
    /// Не удалось создать директорию или файл
    IoError(String),
    /// Процесс уже запущен
    AlreadyRunning,
    /// Процесс не запущен
    NotRunning,
    /// Не удалось запустить процесс
    SpawnFailed(String),
    /// Любая другая ошибка
    Other(String),
}

// Реализуем Display — это то, что покажется пользователю.
// В Python это __str__, в C++ — operator<< для ostream.
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

// Tauri требует что ошибки в командах реализовывали serde::Serialize,
// чтобы их можно было отправить во фронт как JSON.
// Мы просто сериализуем как строку.
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// Удобный конвертер из std::io::Error — аналог implicit cast в C++.
// Позволяет писать io_operation()? вместо io_operation().map_err(AppError::IoError)
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::IoError(e.to_string())
    }
}

// Короткий псевдоним — вместо Result<T, AppError> пишем AppResult<T>
pub type AppResult<T> = Result<T, AppError>;