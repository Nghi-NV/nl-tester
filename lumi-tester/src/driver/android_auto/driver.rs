// Android Auto DHU Driver
// Uses Desktop Head Unit (DHU) console commands for input

use anyhow::Result;
use async_trait::async_trait;
use colored::Colorize;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use crate::driver::android::adb;
use crate::driver::traits::{PlatformDriver, Selector, SwipeDirection};

/// Android Auto driver using DHU console commands
pub struct AndroidAutoDriver {
    /// DHU process handle
    dhu_process: Arc<Mutex<Option<tokio::process::Child>>>,
    /// DHU stdin for sending commands
    dhu_stdin: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
    /// Device serial (for ADB fallback commands)
    serial: Option<String>,
    /// Display size (typically 800x480 for Android Auto)
    screen_size: (u32, u32),
}

impl AndroidAutoDriver {
    /// Create a new Android Auto driver
    pub async fn new(serial: Option<&str>, start_dhu: bool) -> Result<Self> {
        let selected_serial = if let Some(s) = serial {
            Some(s.to_string())
        } else {
            let devices = adb::get_devices().await?;
            if devices.len() == 1 {
                Some(devices[0].serial.clone())
            } else if devices.is_empty() {
                anyhow::bail!("No Android devices connected");
            } else {
                anyhow::bail!("Multiple devices connected. Please specify one with --device");
            }
        };

        let mut driver = Self {
            dhu_process: Arc::new(Mutex::new(None)),
            dhu_stdin: Arc::new(Mutex::new(None)),
            serial: selected_serial,
            screen_size: (800, 480),
        };

        if start_dhu {
            driver.start_dhu().await?;
        }

        Ok(driver)
    }

    /// Start the Desktop Head Unit (DHU) process
    pub async fn start_dhu(&mut self) -> Result<()> {
        let sdk_path = std::env::var("ANDROID_SDK_ROOT")
            .or_else(|_| std::env::var("ANDROID_HOME"))
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_default();
                format!("{}/Library/Android/sdk", home)
            });

        let dhu_path = format!("{}/extras/google/auto/desktop-head-unit", sdk_path);

        if !std::path::Path::new(&dhu_path).exists() {
            anyhow::bail!(
                "DHU not found at {}. Install Android Auto Desktop Head Unit via SDK Manager.",
                dhu_path
            );
        }

        println!("  {} Starting Android Auto DHU...", "🚗".cyan());

        let mut child = tokio::process::Command::new(&dhu_path)
            .arg("--usb")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take();
        *self.dhu_process.lock().await = Some(child);
        *self.dhu_stdin.lock().await = stdin;

        // Wait for DHU to connect to USB device (typically takes 10-20 seconds)
        println!("  {} Waiting for USB connection (8s)...", "📲".cyan());
        tokio::time::sleep(tokio::time::Duration::from_secs(8)).await;

        // Verify DHU is still running
        {
            let mut guard = self.dhu_process.lock().await;
            if let Some(ref mut proc) = *guard {
                match proc.try_wait() {
                    Ok(Some(status)) => {
                        anyhow::bail!("DHU exited unexpectedly with status: {:?}", status);
                    }
                    Ok(None) => {
                        // Process is still running, good
                    }
                    Err(e) => {
                        anyhow::bail!("Failed to check DHU process status: {}", e);
                    }
                }
            }
        }

        println!("  {} DHU started", "✓".green());

        Ok(())
    }

    /// Send command to DHU console
    pub async fn send_dhu_command(&self, command: &str) -> Result<()> {
        let mut stdin_guard = self.dhu_stdin.lock().await;

        if let Some(ref mut stdin) = *stdin_guard {
            stdin.write_all(format!("{}\n", command).as_bytes()).await?;
            stdin.flush().await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            Ok(())
        } else {
            anyhow::bail!("DHU not connected. Call start_dhu() first or ensure DHU is running.");
        }
    }
}

#[async_trait]
impl PlatformDriver for AndroidAutoDriver {
    fn platform_name(&self) -> &str {
        "android_auto"
    }

    fn device_serial(&self) -> Option<String> {
        self.serial.clone()
    }

    async fn launch_app(&self, app_id: &str, _clear_state: bool) -> Result<()> {
        adb::shell(
            self.serial.as_deref(),
            &format!(
                "am start -n {}/$(cmd package resolve-activity --brief {} | tail -n 1)",
                app_id, app_id
            ),
        )
        .await?;
        Ok(())
    }

    async fn stop_app(&self, app_id: &str) -> Result<()> {
        adb::shell(self.serial.as_deref(), &format!("am force-stop {}", app_id)).await?;
        Ok(())
    }

    async fn tap(&self, selector: &Selector) -> Result<()> {
        match selector {
            Selector::Point { x, y } => self.send_dhu_command(&format!("tap {} {}", x, y)).await,
            _ => anyhow::bail!(
                "Android Auto only supports coordinate-based tap. Use tap with point: \"x,y\""
            ),
        }
    }

    async fn long_press(&self, _selector: &Selector, _duration_ms: u64) -> Result<()> {
        anyhow::bail!("Long press not supported on Android Auto DHU")
    }

    async fn double_tap(&self, selector: &Selector) -> Result<()> {
        self.tap(selector).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        self.tap(selector).await
    }

    async fn right_click(&self, _selector: &Selector) -> Result<()> {
        anyhow::bail!("Right click not supported on Android Auto")
    }

    async fn input_text(&self, _text: &str, _unicode: bool) -> Result<()> {
        anyhow::bail!("Text input on Android Auto requires voice input or phone keyboard")
    }

    async fn erase_text(&self, _char_count: Option<u32>) -> Result<()> {
        anyhow::bail!("Erase text not supported on Android Auto DHU")
    }

    async fn hide_keyboard(&self) -> Result<()> {
        self.send_dhu_command("keycode back").await
    }

    async fn swipe(
        &self,
        direction: SwipeDirection,
        _duration_ms: Option<u64>,
        _from: Option<Selector>,
    ) -> Result<()> {
        let dpad_cmd = match direction {
            SwipeDirection::Up => "dpad down",
            SwipeDirection::Down => "dpad up",
            SwipeDirection::Left => "dpad right",
            SwipeDirection::Right => "dpad left",
        };
        self.send_dhu_command(dpad_cmd).await
    }

    async fn scroll_until_visible(
        &self,
        _selector: &Selector,
        _max_scrolls: u32,
        _direction: Option<SwipeDirection>,
        _from: Option<Selector>,
    ) -> Result<bool> {
        anyhow::bail!("scroll_until_visible not supported on Android Auto. Use dpad commands.")
    }

    async fn is_visible(&self, _selector: &Selector) -> Result<bool> {
        Ok(false) // Can't check visibility without UI dump
    }

    async fn wait_for_element(&self, _selector: &Selector, _timeout_ms: u64) -> Result<bool> {
        anyhow::bail!("wait_for_element not supported on Android Auto. Use wait command instead.")
    }

    async fn wait_for_absence(&self, _selector: &Selector, _timeout_ms: u64) -> Result<bool> {
        anyhow::bail!("wait_for_absence not supported on Android Auto. Use wait command instead.")
    }

    async fn get_element_text(&self, _selector: &Selector) -> Result<String> {
        anyhow::bail!("get_element_text not supported on Android Auto")
    }

    async fn open_link(&self, url: &str, _app_id: Option<&str>) -> Result<()> {
        adb::shell(
            self.serial.as_deref(),
            &format!("am start -a android.intent.action.VIEW -d '{}'", url),
        )
        .await?;
        Ok(())
    }

    async fn compare_screenshot(
        &self,
        _reference_path: &Path,
        _tolerance_percent: f64,
    ) -> Result<f64> {
        anyhow::bail!("compare_screenshot not fully supported on Android Auto")
    }

    async fn take_screenshot(&self, path: &str) -> Result<()> {
        self.send_dhu_command(&format!("screenshot {}", path)).await
    }

    async fn start_recording(&self, _path: &str) -> Result<()> {
        anyhow::bail!("Recording not supported on Android Auto DHU")
    }

    async fn stop_recording(&self) -> Result<()> {
        Ok(())
    }

    async fn back(&self) -> Result<()> {
        self.send_dhu_command("keycode back").await
    }

    async fn home(&self) -> Result<()> {
        self.send_dhu_command("keycode home").await
    }

    async fn get_screen_size(&self) -> Result<(u32, u32)> {
        Ok(self.screen_size)
    }

    async fn dump_ui_hierarchy(&self) -> Result<String> {
        Ok("<hierarchy><!-- UI dump not supported for Android Auto --></hierarchy>".to_string())
    }

    async fn dump_logs(&self, lines: u32) -> Result<String> {
        adb::shell(self.serial.as_deref(), &format!("logcat -d -t {}", lines)).await
    }

    async fn set_permissions(
        &self,
        app_id: &str,
        permissions: &std::collections::HashMap<String, String>,
    ) -> Result<()> {
        for (perm, state) in permissions {
            let cmd = if state.eq_ignore_ascii_case("deny") {
                "revoke"
            } else {
                "grant"
            };
            let _ = adb::shell(
                self.serial.as_deref(),
                &format!("pm {} {} {}", cmd, app_id, perm),
            )
            .await;
        }
        Ok(())
    }

    async fn select_display(&self, display_id: u32) -> Result<()> {
        println!(
            "  Note: Android Auto display is managed by DHU (display {})",
            display_id
        );
        Ok(())
    }

    async fn detect_android_auto_display(&self) -> Result<Option<u32>> {
        Ok(Some(1)) // Virtual display ID
    }

    async fn press_key(&self, key: &str) -> Result<()> {
        let keycode = match key.to_lowercase().as_str() {
            "home" => "home",
            "back" => "back",
            "call" | "phone" => "call",
            "end_call" | "endcall" => "endcall",
            "search" => "search",
            "play" | "pause" | "play_pause" => "media_play_pause",
            "next" | "media_next" => "media_next",
            "previous" | "prev" | "media_previous" => "media_previous",
            "navigation" | "nav" => "navigation",
            _ => key,
        };
        self.send_dhu_command(&format!("keycode {}", keycode)).await
    }
}

impl Drop for AndroidAutoDriver {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.dhu_process.try_lock() {
            if let Some(ref mut child) = *guard {
                let _ = child.start_kill();
            }
        }
    }
}
