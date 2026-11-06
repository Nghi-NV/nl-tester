use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Manager};

#[derive(Default)]
pub struct AdbServerLifecycle {
    started_by_app: AtomicBool,
}

impl AdbServerLifecycle {
    pub fn mark_started_if_missing(&self) {
        if !server_is_running() {
            self.started_by_app.store(true, Ordering::Release);
        }
    }
}

pub fn cleanup_on_exit(app: &AppHandle) {
    if app
        .state::<AdbServerLifecycle>()
        .started_by_app
        .swap(false, Ordering::AcqRel)
    {
        stop_server();
    }
}

pub fn cleanup_orphaned_app_server(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        let Some(resource_dir) = app.path().resource_dir().ok() else {
            return;
        };
        let expected_adb = resource_dir.join("resources/binaries/adb");
        if server_executable_path().as_deref() == Some(expected_adb.as_path()) {
            stop_server();
        }
    }

    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

fn server_is_running() -> bool {
    TcpStream::connect_timeout(
        &SocketAddr::from((Ipv4Addr::LOCALHOST, 5037)),
        Duration::from_millis(150),
    )
    .is_ok()
}

#[cfg(target_os = "macos")]
fn server_executable_path() -> Option<std::path::PathBuf> {
    let pids = lumi_tester::utils::process::hidden_std_command("/usr/sbin/lsof")
        .args(["-t", "-iTCP:5037", "-sTCP:LISTEN"])
        .output()
        .ok()?;
    let pid = String::from_utf8_lossy(&pids.stdout)
        .lines()
        .next()?
        .trim()
        .to_string();
    let files = lumi_tester::utils::process::hidden_std_command("/usr/sbin/lsof")
        .args(["-a", "-p", &pid, "-d", "txt", "-Fn"])
        .output()
        .ok()?;
    String::from_utf8_lossy(&files.stdout)
        .lines()
        .find_map(|line| line.strip_prefix('n').map(std::path::PathBuf::from))
}

fn stop_server() {
    let Ok(adb) = lumi_tester::utils::binary_resolver::find_adb() else {
        return;
    };
    let mut command = lumi_tester::utils::process::hidden_std_command(adb);
    lumi_tester::utils::process::configure_hidden_std_command(&mut command);
    let _ = command
        .arg("kill-server")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}
