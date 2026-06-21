//! Модель сервера (§3.3) и связанные типы.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Статус сервера (§3.3 / §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running,
    Crashed,
}

impl ServerStatus {
    /// CSS-класс статус-индикатора (цвета берутся из темы, §5.2).
    pub fn css_class(self) -> &'static str {
        match self {
            ServerStatus::Stopped => "stopped",
            ServerStatus::Starting => "starting",
            ServerStatus::Running => "running",
            ServerStatus::Crashed => "crashed",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ServerStatus::Stopped => "Остановлен",
            ServerStatus::Starting => "Запускается",
            ServerStatus::Running => "Работает",
            ServerStatus::Crashed => "Аварийно завершён",
        }
    }

    pub fn is_active(self) -> bool {
        matches!(self, ServerStatus::Running | ServerStatus::Starting)
    }
}

/// Персистентная модель сервера (хранится в servers.json, §3.2/§3.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub jar_file: PathBuf,
    #[serde(default)]
    pub avatar_path: Option<PathBuf>,
    #[serde(default = "default_jvm_args")]
    pub jvm_args: Vec<String>,
    #[serde(default = "default_java_path")]
    pub java_path: PathBuf,
    #[serde(default)]
    pub auto_restart: bool,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
}

fn default_jvm_args() -> Vec<String> {
    vec!["-Xmx2G".into(), "-Xms1G".into()]
}

fn default_java_path() -> PathBuf {
    PathBuf::from("java")
}

impl Server {
    pub fn new(name: String, path: PathBuf, jar_file: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            path,
            jar_file,
            avatar_path: None,
            jvm_args: default_jvm_args(),
            java_path: default_java_path(),
            auto_restart: false,
            created_at: Utc::now(),
        }
    }

    pub fn plugins_dir(&self) -> PathBuf {
        self.path.join("plugins")
    }

    pub fn properties_file(&self) -> PathBuf {
        self.path.join("server.properties")
    }

    pub fn eula_file(&self) -> PathBuf {
        self.path.join("eula.txt")
    }

    pub fn latest_log(&self) -> PathBuf {
        self.path.join("logs").join("latest.log")
    }

    /// Инициал для AdwAvatar при отсутствии картинки.
    pub fn initials(&self) -> String {
        self.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()
    }
}

/// Тип строки лога для цветовой разметки (§6.2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Info,
    Warn,
    Error,
    Chat,
}

impl LogKind {
    /// Грубая классификация строки лога Minecraft по содержимому.
    pub fn classify(line: &str) -> LogKind {
        let upper = line.to_uppercase();
        if upper.contains("ERROR") || upper.contains("SEVERE") || upper.contains("EXCEPTION") {
            LogKind::Error
        } else if upper.contains("WARN") {
            LogKind::Warn
        } else if line.contains("<") && line.contains(">") {
            LogKind::Chat
        } else {
            LogKind::Info
        }
    }

    pub fn css_class(self) -> Option<&'static str> {
        match self {
            LogKind::Error => Some("log-error"),
            LogKind::Warn => Some("log-warn"),
            LogKind::Chat => Some("log-chat"),
            LogKind::Info => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub text: String,
    pub kind: LogKind,
}

impl LogLine {
    pub fn new(text: String) -> Self {
        let kind = LogKind::classify(&text);
        Self { text, kind }
    }
}
