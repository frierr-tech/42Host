//! Разделяемое состояние приложения и runtime серверов (§3.3, §11.2).

use crate::config::AppConfig;
use crate::manager::ProcessHandle;
use crate::monitor::{Monitor, Sample};
use crate::server::{LogLine, Server, ServerStatus};
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

/// Рантайм-состояние одного сервера (не персистентное).
pub struct Runtime {
    pub status: ServerStatus,
    pub handle: Option<ProcessHandle>,
    pub started_at: Option<Instant>,
    pub last_sample: Sample,
    pub console: VecDeque<LogLine>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            status: ServerStatus::Stopped,
            handle: None,
            started_at: None,
            last_sample: Sample::default(),
            console: VecDeque::new(),
        }
    }
}

impl Runtime {
    pub fn pid(&self) -> Option<u32> {
        self.handle.as_ref().map(|h| h.pid)
    }

    pub fn uptime_secs(&self) -> u64 {
        match (self.status.is_active(), self.started_at) {
            (true, Some(t)) => t.elapsed().as_secs(),
            _ => 0,
        }
    }

    pub fn push_log(&mut self, line: LogLine, max: usize) {
        self.console.push_back(line);
        while self.console.len() > max {
            self.console.pop_front();
        }
    }
}

pub struct AppState {
    pub config: AppConfig,
    pub servers: Vec<Server>,
    pub runtimes: HashMap<String, Runtime>,
    pub selected: Option<String>,
    pub monitor: Monitor,
}

impl AppState {
    pub fn new(config: AppConfig, servers: Vec<Server>) -> Self {
        let mut runtimes = HashMap::new();
        for s in &servers {
            runtimes.insert(s.id.clone(), Runtime::default());
        }
        Self {
            config,
            servers,
            runtimes,
            selected: None,
            monitor: Monitor::new(),
        }
    }

    pub fn server(&self, id: &str) -> Option<&Server> {
        self.servers.iter().find(|s| s.id == id)
    }

    pub fn server_mut(&mut self, id: &str) -> Option<&mut Server> {
        self.servers.iter_mut().find(|s| s.id == id)
    }

    pub fn runtime(&self, id: &str) -> Option<&Runtime> {
        self.runtimes.get(id)
    }

    pub fn runtime_mut(&mut self, id: &str) -> &mut Runtime {
        self.runtimes.entry(id.to_string()).or_default()
    }
}
