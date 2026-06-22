//! Управление процессом сервера (§8): запуск, остановка, чтение вывода.
//!
//! Архитектура: процесс порождается через `std::process`, его stdout/stderr
//! читаются в фоновых потоках и отправляются в UI через `async-channel`,
//! который потребляется на главном контексте GLib (`glib::spawn_future_local`).

use crate::server::{LogLine, Server};
use anyhow::{bail, Context};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

/// События от процесса сервера, доставляемые в UI.
#[derive(Debug, Clone)]
pub enum ServerEvent {
    Log(LogLine),
    /// Процесс завершился с указанным кодом (None — убит сигналом).
    Exited(Option<i32>),
}

/// Дескриптор запущенного процесса сервера.
pub struct ProcessHandle {
    child: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    pub pid: u32,
}

impl ProcessHandle {
    /// Отправить команду в stdin сервера (добавляет `\n`).
    pub fn send_command(&self, cmd: &str) -> std::io::Result<()> {
        let mut guard = self.stdin.lock().unwrap();
        if let Some(stdin) = guard.as_mut() {
            stdin.write_all(cmd.as_bytes())?;
            stdin.write_all(b"\n")?;
            stdin.flush()?;
            Ok(())
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "stdin закрыт"))
        }
    }

    /// Мягкая остановка: отправить `stop` (§8.2). Фактическое завершение
    /// отслеживается потоком ожидания, который пришлёт `ServerEvent::Exited`.
    pub fn stop_soft(&self) -> std::io::Result<()> {
        self.send_command("stop")
    }

    /// Жёсткое завершение процесса (SIGKILL).
    pub fn kill(&self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
    }
}

/// Убедиться, что eula.txt существует и принят (§8.1).
/// Возвращает true, если eula уже принята или была принята сейчас.
pub fn ensure_eula(server: &Server, accept: bool) -> std::io::Result<bool> {
    let path = server.eula_file();
    let accepted = std::fs::read_to_string(&path)
        .map(|s| s.lines().any(|l| l.trim().eq_ignore_ascii_case("eula=true")))
        .unwrap_or(false);
    if accepted {
        return Ok(true);
    }
    if accept {
        std::fs::write(&path, "eula=true\n")?;
        return Ok(true);
    }
    Ok(false)
}

fn find_on_path(program: &Path) -> Option<PathBuf> {
    if program.components().count() != 1 {
        return None;
    }

    #[cfg(windows)]
    let names: Vec<PathBuf> = if program.extension().is_some() {
        vec![program.to_path_buf()]
    } else {
        std::env::var_os("PATHEXT")
            .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".into())
            .to_string_lossy()
            .split(';')
            .filter(|ext| !ext.is_empty())
            .map(|ext| PathBuf::from(format!("{}{}", program.to_string_lossy(), ext)))
            .collect()
    };
    #[cfg(not(windows))]
    let names = vec![program.to_path_buf()];

    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for name in &names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn bundled_java_path() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    #[cfg(windows)]
    let java = Path::new("bin").join("java.exe");
    #[cfg(not(windows))]
    let java = Path::new("bin").join("java");

    [
        exe_dir.join("runtime").join(&java),
        exe_dir.parent()?.join("runtime").join(&java),
    ]
    .into_iter()
    .find(|candidate| candidate.is_file())
}

/// Найти рабочую Java. Если сохранённый абсолютный путь устарел, пробуем
/// приватный runtime из установщика, затем Java из PATH.
pub fn resolve_java_path(server: &Server) -> anyhow::Result<PathBuf> {
    let configured_text = server.java_path.to_string_lossy();
    let configured = PathBuf::from(configured_text.trim().trim_matches('"'));

    let configured_path = if configured.is_absolute() {
        configured.clone()
    } else if configured.components().count() > 1 {
        server.path.join(&configured)
    } else {
        PathBuf::new()
    };

    if !configured_path.as_os_str().is_empty() && configured_path.is_file() {
        return Ok(configured_path);
    }
    if let Some(path) = find_on_path(&configured) {
        return Ok(path);
    }
    if let Some(path) = bundled_java_path() {
        return Ok(path);
    }

    #[cfg(windows)]
    let default_java = Path::new("java.exe");
    #[cfg(not(windows))]
    let default_java = Path::new("java");
    if let Some(path) = find_on_path(default_java) {
        return Ok(path);
    }

    bail!(
        "Java не найдена: сохранённый путь «{}» не существует, встроенной Java нет, а {} отсутствует в PATH. Укажите Java в настройках",
        server.java_path.display(),
        default_java.display()
    )
}

/// Запустить сервер. Возвращает дескриптор процесса и приёмник событий.
pub fn spawn_server(
    server: &Server,
) -> anyhow::Result<(ProcessHandle, async_channel::Receiver<ServerEvent>, PathBuf)> {
    if !server.path.is_dir() {
        bail!("папка сервера не найдена: {}", server.path.display());
    }

    let jar_path = if server.jar_file.is_absolute() {
        server.jar_file.clone()
    } else {
        server.path.join(&server.jar_file)
    };
    if !jar_path.is_file() {
        bail!("JAR сервера не найден: {}", jar_path.display());
    }

    let java_path = resolve_java_path(server)?;
    let mut command = Command::new(&java_path);
    command
        .args(&server.jvm_args)
        .arg("-jar")
        .arg(&jar_path)
        .arg("nogui")
        .current_dir(&server.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        // Java — консольное приложение. Не показываем отдельное окно: вывод уже
        // перенаправлен в консоль 42Host через stdout/stderr.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command
        .spawn()
        .with_context(|| format!("не удалось запустить Java: {}", java_path.display()))?;
    let pid = child.id();

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdin = child.stdin.take();

    let (tx, rx) = async_channel::unbounded::<ServerEvent>();

    // Чтение stdout построчно.
    if let Some(out) = stdout {
        let tx = tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(out);
            for line in reader.lines().map_while(Result::ok) {
                if tx.send_blocking(ServerEvent::Log(LogLine::new(line))).is_err() {
                    break;
                }
            }
        });
    }

    // Чтение stderr построчно.
    if let Some(err) = stderr {
        let tx = tx.clone();
        thread::spawn(move || {
            let reader = BufReader::new(err);
            for line in reader.lines().map_while(Result::ok) {
                if tx.send_blocking(ServerEvent::Log(LogLine::new(line))).is_err() {
                    break;
                }
            }
        });
    }

    let child = Arc::new(Mutex::new(child));

    // Поток ожидания завершения процесса.
    {
        let child = child.clone();
        let tx = tx.clone();
        thread::spawn(move || {
            let status = {
                let mut guard = child.lock().unwrap();
                guard.wait()
            };
            let code = status.ok().and_then(|s| s.code());
            let _ = tx.send_blocking(ServerEvent::Exited(code));
        });
    }

    let handle = ProcessHandle {
        child,
        stdin: Arc::new(Mutex::new(stdin)),
        pid,
    };

    Ok((handle, rx, java_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_java_path_falls_back_to_path() {
        let root = std::env::temp_dir().join(format!("42host-java-test-{}", uuid::Uuid::new_v4()));
        let bin = root.join("bin");
        std::fs::create_dir_all(&bin).unwrap();

        #[cfg(windows)]
        let java = bin.join("java.exe");
        #[cfg(not(windows))]
        let java = bin.join("java");
        std::fs::write(&java, b"test").unwrap();

        let mut server = Server::new("test".into(), root.clone(), root.join("server.jar"));
        server.java_path = root.join("deleted-runtime").join("bin").join("java.exe");

        let previous_path = std::env::var_os("PATH");
        std::env::set_var("PATH", &bin);
        let resolved = resolve_java_path(&server);
        if let Some(path) = previous_path {
            std::env::set_var("PATH", path);
        } else {
            std::env::remove_var("PATH");
        }

        assert_eq!(resolved.unwrap(), java);
        std::fs::remove_dir_all(root).unwrap();
    }
}
