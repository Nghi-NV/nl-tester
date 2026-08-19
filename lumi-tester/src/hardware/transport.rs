use anyhow::{anyhow, Context, Result};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::hardware::protocol::ResponseLine;

pub struct SerialTransport {
    port: Option<Box<dyn serialport::SerialPort>>,
    port_name: Option<String>,
    pub node_id: Option<u8>,
    pub wire_format: Option<String>,
    log_file: Option<File>,
    log_path: PathBuf,
}

impl SerialTransport {
    pub fn new() -> Self {
        let log_path = std::env::current_dir()
            .unwrap_or_default()
            .join("hardware_control_serial.log");
        Self {
            port: None,
            port_name: None,
            node_id: Some(1),
            wire_format: Some("@{node} {command}\n".to_string()),
            log_file: None,
            log_path,
        }
    }

    pub fn connect(&mut self, port_name: &str, baudrate: u32) -> Result<()> {
        let mut port = serialport::new(port_name, baudrate)
            .timeout(Duration::from_millis(100))
            .open()
            .with_context(|| format!("Failed to open serial port '{}'", port_name))?;

        // Allow USB-Serial / MCU DTR line to stabilize and clear startup noise
        std::thread::sleep(Duration::from_millis(100));
        let _ = port.clear(serialport::ClearBuffer::All);

        self.port = Some(port);
        self.port_name = Some(port_name.to_string());
        self.init_log()?;
        self.log_event("OPEN", &format!("Port {} @ {}", port_name, baudrate));
        Ok(())
    }

    pub fn disconnect(&mut self) {
        if self.port.is_some() {
            self.log_event("CLOSE", "Serial port closed");
            self.port = None;
            self.port_name = None;
        }
    }

    pub fn is_connected(&self) -> bool {
        self.port.is_some()
    }

    fn init_log(&mut self) -> Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;
        self.log_file = Some(file);
        Ok(())
    }

    fn check_rotate_log(&mut self) {
        const MAX_BYTES: u64 = 2 * 1024 * 1024; // 2 MB
        if let Ok(metadata) = std::fs::metadata(&self.log_path) {
            if metadata.len() >= MAX_BYTES {
                self.log_file = None;
                let path2 = self.log_path.with_extension("log.2");
                let path1 = self.log_path.with_extension("log.1");

                let _ = std::fs::remove_file(&path2);
                let _ = std::fs::rename(&path1, &path2);
                let _ = std::fs::rename(&self.log_path, &path1);

                let _ = self.init_log();
            }
        }
    }

    fn log_event(&mut self, event_type: &str, msg: &str) {
        self.check_rotate_log();
        if let Some(f) = &mut self.log_file {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S.%3f");
            let line = format!("[{}] [{}] {}\n", timestamp, event_type, msg);
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }

    pub fn request<F>(&mut self, cmd: &str, matcher: F, timeout_s: f64) -> Result<ResponseLine>
    where
        F: Fn(&ResponseLine) -> bool,
    {
        let trimmed = cmd.trim();
        let wire_cmd = if trimmed.starts_with('@') || (trimmed.starts_with('[') && !trimmed.contains("{command}")) {
            format!("{}\n", trimmed)
        } else if let Some(ref fmt) = self.wire_format {
            let node_str = self.node_id.map(|n| n.to_string()).unwrap_or_else(|| "1".to_string());
            let rendered = fmt
                .replace("{node}", &node_str)
                .replace("{node_id}", &node_str)
                .replace("{nodeId}", &node_str)
                .replace("{command}", trimmed)
                .replace("{cmd}", trimmed);
            if !rendered.ends_with('\n') {
                format!("{}\n", rendered)
            } else {
                rendered
            }
        } else if let Some(nid) = self.node_id {
            format!("@{} {}\n", nid, trimmed)
        } else {
            format!("{}\n", trimmed)
        };

        self.log_event("TX", wire_cmd.trim());

        let port = self
            .port
            .as_mut()
            .ok_or_else(|| anyhow!("Serial port is not connected"))?;

        // Clear stale unread bytes from RX queue before sending new command
        let _ = port.clear(serialport::ClearBuffer::Input);

        // Write command
        port.write_all(wire_cmd.as_bytes())
            .with_context(|| "Failed to write command to serial port")?;
        port.flush()?;

        let start = Instant::now();
        let timeout_duration = Duration::from_secs_f64(timeout_s);
        let mut buffer = Vec::new();

        loop {
            if start.elapsed() > timeout_duration {
                let port = self.port.as_mut().unwrap();
                let _ = port.flush();
                self.log_event("TIMEOUT", &format!("Timeout waiting for command: {}", cmd.trim()));
                anyhow::bail!(
                    "Firmware command timeout ({:.1}s) on port '{}' for command: '{}'",
                    timeout_s,
                    self.port_name.as_deref().unwrap_or("unknown"),
                    cmd.trim()
                );
            }

            let mut byte_buf = [0u8; 1];
            let port_ref = self.port.as_mut().unwrap();
            match port_ref.read(&mut byte_buf) {
                Ok(1) => {
                    let b = byte_buf[0];
                    if b == b'\n' {
                        let line_str = String::from_utf8_lossy(&buffer).trim().to_string();
                        buffer.clear();

                        if !line_str.is_empty() {
                            self.log_event("RX", &line_str);
                            if let Ok(resp) = ResponseLine::parse(&line_str) {
                                if resp.kind == "error" {
                                    let code = resp.get_str("code").unwrap_or("UNKNOWN");
                                    let msg = resp.get_str("message").unwrap_or(&line_str);
                                    anyhow::bail!("Firmware error [{}]: {}", code, msg);
                                }
                                if matcher(&resp) {
                                    return Ok(resp);
                                }
                            }
                        }
                    } else if b != b'\r' {
                        buffer.push(b);
                    }
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    self.log_event("RX_ERROR", &e.to_string());
                    anyhow::bail!("Serial read error: {}", e);
                }
            }
        }
    }
}
