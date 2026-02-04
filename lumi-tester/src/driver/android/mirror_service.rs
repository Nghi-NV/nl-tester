//! nl-mirror service management
//!
//! This module manages the nl-android (nl-mirror) helper service on Android devices.
//! It handles deployment, startup, and port forwarding for mock location and other features.

use crate::driver::android::adb;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// On-device path for the nl-mirror APK
const DEVICE_APK_PATH: &str = "/data/local/tmp/nl-mirror.apk";

/// nl-mirror command server port
const MIRROR_PORT: u16 = 8889;

/// MirrorService manages the nl-mirror helper on Android devices
pub struct MirrorService;

impl MirrorService {
    /// Get the path to the nl-mirror APK
    /// Only looks in the resources/apk folder
    pub fn find_apk_path() -> Option<PathBuf> {
        // Get current working directory
        let cwd = std::env::current_dir().ok()?;

        // Only check resources/apk folder
        let apk_path = cwd.join("resources/apk/nl-mirror-debug.apk");

        if apk_path.exists() {
            Some(apk_path)
        } else {
            None
        }
    }

    /// Check if nl-mirror is running on the device
    pub async fn is_running(serial: Option<&str>) -> bool {
        let cmd = "pgrep -f 'dev.nl.mirror.core.App' 2>/dev/null || true";
        match adb::shell(serial, cmd).await {
            Ok(output) => !output.trim().is_empty(),
            Err(_) => false,
        }
    }

    /// Verify connectivity to nl-mirror port
    pub async fn verify_connection() -> bool {
        // Try to connect to the forwarded port
        match std::net::TcpStream::connect_timeout(
            &"127.0.0.1:8889".parse().unwrap(),
            std::time::Duration::from_millis(200),
        ) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Get the APK file size on device (0 if not exists)
    async fn get_device_apk_size(serial: Option<&str>) -> u64 {
        let cmd = format!("stat -c %s {} 2>/dev/null || echo 0", DEVICE_APK_PATH);
        match adb::shell(serial, &cmd).await {
            Ok(output) => output.trim().parse().unwrap_or(0),
            Err(_) => 0,
        }
    }

    /// Deploy nl-mirror APK to device if needed
    /// Returns true if APK was pushed, false if already up to date
    pub async fn deploy_if_needed(serial: Option<&str>, local_apk: &Path) -> Result<bool> {
        // Get local APK size
        let local_size = std::fs::metadata(local_apk)
            .map_err(|e| anyhow!("Failed to read APK metadata: {}", e))?
            .len();

        // Get device APK size
        let device_size = Self::get_device_apk_size(serial).await;

        // Compare sizes (basic but fast check)
        if local_size == device_size {
            eprintln!("  ✓ nl-mirror APK already up to date");
            return Ok(false);
        }

        eprintln!("  📦 Deploying nl-mirror APK ({} bytes)...", local_size);

        // Push APK to device
        let apk_path_str = local_apk.to_string_lossy();
        let serial_args: Vec<String> = match serial {
            Some(s) => vec!["-s".to_string(), s.to_string()],
            None => vec![],
        };

        let mut args = serial_args;
        args.extend([
            "push".to_string(),
            apk_path_str.to_string(),
            DEVICE_APK_PATH.to_string(),
        ]);

        let output = tokio::process::Command::new("adb")
            .args(&args)
            .output()
            .await
            .map_err(|e| anyhow!("Failed to push APK: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to push APK: {}", stderr));
        }

        eprintln!("  ✓ nl-mirror APK deployed");
        Ok(true)
    }

    /// Stop nl-mirror service if running
    pub async fn stop(serial: Option<&str>) -> Result<()> {
        let _ = adb::shell(
            serial,
            "pkill -f 'app_process.*nl-mirror' 2>/dev/null || true",
        )
        .await;
        let _ = adb::shell(serial, "pkill -f 'dev.nl.mirror' 2>/dev/null || true").await;
        Ok(())
    }

    /// Start nl-mirror service
    pub async fn start(serial: Option<&str>) -> Result<()> {
        // Stop any existing instance first
        Self::stop(serial).await?;

        eprintln!("  🚀 Starting nl-mirror service...");

        // Start server in background
        // Use sh -c with & to detach from adb
        let cmd = format!(
            "sh -c 'CLASSPATH={} app_process / dev.nl.mirror.core.App >/dev/null 2>&1 &'",
            DEVICE_APK_PATH
        );

        let _ = adb::shell(serial, &cmd).await;

        // Wait for startup (up to 2 seconds)
        for i in 0..10 {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            if Self::is_running(serial).await {
                eprintln!("  ✓ nl-mirror service started ({}ms)", (i + 1) * 200);
                return Ok(());
            }
        }

        Err(anyhow!("nl-mirror service failed to start"))
    }

    /// Setup ADB port forwarding for nl-mirror (command + audio)
    pub async fn setup_port_forward(serial: Option<&str>) -> Result<()> {
        let serial_args: Vec<String> = match serial {
            Some(s) => vec!["-s".to_string(), s.to_string()],
            None => vec![],
        };

        // Forward command port (8889)
        let mut args = serial_args.clone();
        args.extend([
            "forward".to_string(),
            format!("tcp:{}", MIRROR_PORT),
            format!("tcp:{}", MIRROR_PORT),
        ]);

        let output = tokio::process::Command::new("adb")
            .args(&args)
            .output()
            .await
            .map_err(|e| anyhow!("Failed to forward port {}: {}", MIRROR_PORT, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Failed to forward port {}: {}",
                MIRROR_PORT,
                stderr
            ));
        }

        // Forward audio port (8890)
        let mut audio_args = serial_args;
        audio_args.extend([
            "forward".to_string(),
            "tcp:8890".to_string(),
            "tcp:8890".to_string(),
        ]);

        let _ = tokio::process::Command::new("adb")
            .args(&audio_args)
            .output()
            .await;

        Ok(())
    }

    /// Initialize nl-mirror session: deploy if needed, start service, setup port forward
    pub async fn init_session(serial: Option<&str>) -> Result<()> {
        // 1. Find APK
        let apk_path = Self::find_apk_path()
            .ok_or_else(|| anyhow!("nl-mirror APK not found. Please build nl-android first."))?;

        // 2. Setup port forwarding first (fast)
        Self::setup_port_forward(serial).await?;

        // 3. Deploy if needed
        let _ = Self::deploy_if_needed(serial, &apk_path).await?;

        // 4. Start if not running or not reachable
        let is_process_running = Self::is_running(serial).await;
        let is_reachable = if is_process_running {
            Self::verify_connection().await
        } else {
            false
        };

        if !is_process_running || !is_reachable {
            if is_process_running {
                eprintln!("  ⚠️ nl-mirror process exists but unreachable. Restarting...");
            }
            Self::start(serial).await?;

            // Verify again after start
            // Wait a bit for server to bind
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if !Self::verify_connection().await {
                return Err(anyhow!("nl-mirror started but port 8889 is invalid"));
            }
        } else {
            eprintln!("  ✓ nl-mirror already running");
        }

        Ok(())
    }
}
