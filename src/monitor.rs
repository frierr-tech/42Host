//! Мониторинг ресурсов процесса сервера через sysinfo (§9).

use sysinfo::{Pid, ProcessesToUpdate, System};

pub struct Monitor {
    sys: System,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Sample {
    pub ram_mb: f64,
    pub cpu_percent: f32,
}

impl Monitor {
    pub fn new() -> Self {
        Self { sys: System::new() }
    }

    /// Снять метрики процесса по PID. Возвращает None, если процесс не найден.
    /// Для корректного CPU метод нужно вызывать периодически (раз в ~1 сек).
    pub fn sample(&mut self, pid: u32) -> Option<Sample> {
        let pid = Pid::from_u32(pid);
        self.sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
        let proc = self.sys.process(pid)?;
        Some(Sample {
            ram_mb: proc.memory() as f64 / (1024.0 * 1024.0),
            cpu_percent: proc.cpu_usage(),
        })
    }
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Человеко-читаемый формат RAM (§9).
pub fn format_ram(mb: f64) -> String {
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{:.0} MB", mb)
    }
}

/// Формат uptime HH:MM:SS (§5.2).
pub fn format_uptime(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}
