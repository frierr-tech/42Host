//! Управление процессом сервера (§8): запуск, остановка, чтение вывода.
//!
//! Архитектура: процесс порождается через `std::process`, его stdout/stderr
//! читаются в фоновых потоках и отправляются в UI через `async-channel`,
//! который потребляется на главном контексте GLib (`glib::spawn_future_local`).

use crate::server::{LogLine, Server};
use std::io::{BufRead, BufReader, Write};
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

/// Запустить сервер. Возвращает дескриптор процесса и приёмник событий.
pub fn spawn_server(
    server: &Server,
) -> anyhow::Result<(ProcessHandle, async_channel::Receiver<ServerEvent>)> {
    let mut command = Command::new(&server.java_path);
    command
        .args(&server.jvm_args)
        .arg("-jar")
        .arg(&server.jar_file)
        .arg("nogui")
        .current_dir(&server.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn()?;
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

    Ok((handle, rx))
}
