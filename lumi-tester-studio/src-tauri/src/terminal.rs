use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter, State};

static NEXT_TERMINAL_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Default)]
pub struct TerminalSessions {
    sessions: Arc<Mutex<HashMap<String, TerminalSession>>>,
}

impl Drop for TerminalSessions {
    fn drop(&mut self) {
        if let Ok(mut sessions) = self.sessions.lock() {
            for (_, mut session) in sessions.drain() {
                let _ = session.killer.kill();
            }
        }
    }
}

struct TerminalSession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalInfo {
    id: String,
    shell: String,
    working_directory: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalOutput {
    terminal_id: String,
    data: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalExit {
    terminal_id: String,
    code: Option<u32>,
}

#[tauri::command]
pub fn start_terminal(
    app: AppHandle,
    workspace: State<'_, crate::workspace::WorkspaceState>,
    terminals: State<'_, TerminalSessions>,
    workspace_path: String,
) -> Result<TerminalInfo, String> {
    let working_directory = workspace.resolve_existing(std::path::Path::new(&workspace_path))?;
    if !working_directory.is_dir() {
        return Err("Terminal working directory must be a folder inside the open workspace".to_string());
    }
    let (shell, mut command) = shell_command(&working_directory)?;
    let pty = native_pty_system();
    let pair = pty
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| error.to_string())?;
    command.cwd(&working_directory);

    let mut child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| error.to_string())?;
    let killer = child.clone_killer();
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| error.to_string())?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| error.to_string())?;
    let master = pair.master;
    let id = format!("terminal-{}", NEXT_TERMINAL_ID.fetch_add(1, Ordering::Relaxed));
    let sessions = Arc::clone(&terminals.sessions);

    sessions.lock().map_err(|error| error.to_string())?.insert(
        id.clone(),
        TerminalSession {
            master,
            writer,
            killer,
        },
    );

    let output_app = app.clone();
    let output_id = id.clone();
    thread::spawn(move || {
        let mut reader = reader;
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(size) => {
                    let data = String::from_utf8_lossy(&buffer[..size]).into_owned();
                    if output_app
                        .emit(
                            "terminal-output",
                            TerminalOutput {
                                terminal_id: output_id.clone(),
                                data,
                            },
                        )
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
    });

    let exit_app = app;
    let exit_id = id.clone();
    thread::spawn(move || {
        let code = child.wait().ok().map(|status| status.exit_code());
        if let Ok(mut sessions) = sessions.lock() {
            sessions.remove(&exit_id);
        }
        let _ = exit_app.emit(
            "terminal-exit",
            TerminalExit {
                terminal_id: exit_id,
                code,
            },
        );
    });

    Ok(TerminalInfo {
        id,
        shell: shell.display().to_string(),
        working_directory: working_directory.display().to_string(),
    })
}

#[tauri::command]
pub fn write_terminal(
    terminals: State<'_, TerminalSessions>,
    terminal_id: String,
    data: String,
) -> Result<(), String> {
    if data.len() > 64 * 1024 {
        return Err("Terminal input exceeds 64 KiB".to_string());
    }
    let mut sessions = terminals.sessions.lock().map_err(|error| error.to_string())?;
    let session = sessions
        .get_mut(&terminal_id)
        .ok_or_else(|| "Terminal session is no longer running".to_string())?;
    session
        .writer
        .write_all(data.as_bytes())
        .and_then(|_| session.writer.flush())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn resize_terminal(
    terminals: State<'_, TerminalSessions>,
    terminal_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    if cols == 0 || rows == 0 {
        return Ok(());
    }
    let sessions = terminals.sessions.lock().map_err(|error| error.to_string())?;
    let session = sessions
        .get(&terminal_id)
        .ok_or_else(|| "Terminal session is no longer running".to_string())?;
    session
        .master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn stop_terminal(
    terminals: State<'_, TerminalSessions>,
    terminal_id: String,
) -> Result<bool, String> {
    let session = terminals
        .sessions
        .lock()
        .map_err(|error| error.to_string())?
        .remove(&terminal_id);
    if let Some(mut session) = session {
        session.killer.kill().map_err(|error| error.to_string())?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn shell_command(_working_directory: &std::path::Path) -> Result<(PathBuf, CommandBuilder), String> {
    #[cfg(unix)]
    let shell = std::env::var_os("SHELL")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/bin/sh"));

    #[cfg(windows)]
    let shell = std::env::var_os("COMSPEC")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("powershell.exe"));

    if shell.is_absolute() && !shell.is_file() {
        return Err(format!("Configured shell does not exist: {}", shell.display()));
    }

    let mut command = CommandBuilder::new(&shell);
    #[cfg(unix)]
    command.arg("-l");
    Ok((shell, command))
}
