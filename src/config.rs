//! Конфигурация приложения и персистентность списка серверов (§3.2, §7).

use crate::server::Server;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::System
    }
}

/// Глобальные настройки приложения (~/.config/42host/settings.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default = "default_console_font_size")]
    pub console_font_size: u32,
    #[serde(default = "default_terminal")]
    pub terminal: String,
    #[serde(default = "default_editor")]
    pub editor: String,
    #[serde(default = "default_file_manager")]
    pub file_manager: String,
    #[serde(default = "default_java")]
    pub java_path: String,
    #[serde(default)]
    pub auto_restart: bool,
    #[serde(default = "default_restart_delay")]
    pub restart_delay_secs: u32,
    #[serde(default = "default_console_buffer")]
    pub console_buffer_max: usize,
    #[serde(default)]
    pub default_server_dir: Option<PathBuf>,
}

fn default_console_font_size() -> u32 {
    12
}
fn default_terminal() -> String {
    "kitty".into()
}
fn default_editor() -> String {
    "nvim".into()
}
fn default_file_manager() -> String {
    "xdg-open".into()
}
fn default_java() -> String {
    "java".into()
}
fn default_restart_delay() -> u32 {
    5
}
fn default_console_buffer() -> usize {
    10000
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            font_family: None,
            console_font_size: default_console_font_size(),
            terminal: default_terminal(),
            editor: default_editor(),
            file_manager: default_file_manager(),
            java_path: default_java(),
            auto_restart: false,
            restart_delay_secs: default_restart_delay(),
            console_buffer_max: default_console_buffer(),
            default_server_dir: None,
        }
    }
}

/// Каталог конфигурации ~/.config/42host/.
pub fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("42host")
}

pub fn cache_logs_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("42host").join("logs")
}

fn servers_file() -> PathBuf {
    config_dir().join("servers.json")
}

fn settings_file() -> PathBuf {
    config_dir().join("settings.json")
}

fn ensure_dir() {
    let _ = std::fs::create_dir_all(config_dir());
}

pub fn load_config() -> AppConfig {
    match std::fs::read_to_string(settings_file()) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(cfg: &AppConfig) -> anyhow::Result<()> {
    ensure_dir();
    std::fs::write(settings_file(), serde_json::to_string_pretty(cfg)?)?;
    Ok(())
}

pub fn load_servers() -> Vec<Server> {
    match std::fs::read_to_string(servers_file()) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn save_servers(servers: &[Server]) -> anyhow::Result<()> {
    ensure_dir();
    std::fs::write(servers_file(), serde_json::to_string_pretty(servers)?)?;
    Ok(())
}
