//! Запуск внешних приложений: терминал+редактор и файловый менеджер (§6.2.2, §5.4).

use crate::config::AppConfig;
use std::path::Path;
use std::process::{Command, Stdio};

/// Построить аргументы запуска редактора в терминале.
/// kitty/foot:   `kitty -- nvim <file>`
/// alacritty/др: `alacritty -e nvim <file>`
fn terminal_command(cfg: &AppConfig, target: &Path) -> Command {
    let term = cfg.terminal.trim();
    let editor = cfg.editor.trim();
    let file = target.to_string_lossy().to_string();

    let mut cmd = Command::new(term);
    match term {
        "kitty" | "foot" => {
            cmd.arg("--").arg(editor).arg(file);
        }
        _ => {
            // alacritty, gnome-terminal, и большинство xterm-совместимых
            cmd.arg("-e").arg(editor).arg(file);
        }
    }
    cmd
}

/// Открыть файл в выбранном терминале+редакторе, отвязав процесс (§6.2.2).
pub fn open_in_editor(cfg: &AppConfig, target: &Path) -> std::io::Result<()> {
    let mut cmd = terminal_command(cfg, target);
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    detach(&mut cmd);
    cmd.spawn().map(|_| ())
}

/// Открыть терминал с рабочим каталогом `dir` (для конфигов плагинов, §6.2.2).
pub fn open_terminal_at(cfg: &AppConfig, dir: &Path) -> std::io::Result<()> {
    let term = if cfg.terminal.trim().is_empty() { "kitty" } else { cfg.terminal.trim() };
    let mut cmd = Command::new(term);
    // Большинство терминалов наследуют cwd; для надёжности задаём явно.
    cmd.current_dir(dir);
    match term {
        "kitty" => {
            cmd.arg("--working-directory").arg(dir);
        }
        "gnome-terminal" => {
            cmd.arg("--working-directory").arg(dir);
        }
        "foot" => {
            cmd.arg("--working-directory").arg(dir);
        }
        "alacritty" => {
            cmd.arg("--working-directory").arg(dir);
        }
        _ => {}
    }
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    detach(&mut cmd);
    cmd.spawn().map(|_| ())
}

/// Открыть путь в файловом менеджере (§5.4).
pub fn open_in_file_manager(cfg: &AppConfig, target: &Path) -> std::io::Result<()> {
    let fm = if cfg.file_manager.trim().is_empty() { "xdg-open" } else { cfg.file_manager.trim() };
    let mut cmd = Command::new(fm);
    cmd.arg(target);
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    detach(&mut cmd);
    cmd.spawn().map(|_| ())
}

/// Отвязать дочерний процесс, чтобы он пережил закрытие 42Host.
/// Создаём новую сессию (setsid) через pre_exec, чтобы процесс терминала
/// не получил SIGHUP при выходе 42Host.
#[cfg(unix)]
fn detach(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    unsafe {
        cmd.pre_exec(|| {
            // setsid(2): отделяет процесс в новую сессию.
            if libc::setsid() == -1 {
                // Не критично — продолжаем запуск.
            }
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn detach(_cmd: &mut Command) {}
