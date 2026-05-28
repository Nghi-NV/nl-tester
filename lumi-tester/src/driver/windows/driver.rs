use anyhow::{Context, Result};
use async_trait::async_trait;
use image::GenericImageView;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::driver::traits::{PlatformDriver, Selector, SwipeDirection};

/// Native Windows driver MVP backed by PowerShell and built-in Win32/.NET APIs.
pub struct WindowsDriver {
    device_name: Option<String>,
    launched_pids: Mutex<HashMap<String, u32>>,
    active_window_handle: Mutex<Option<isize>>,
}

#[derive(Debug, Clone)]
struct WindowsUiElement {
    control_type: String,
    name: String,
    automation_id: String,
    class_name: String,
    help_text: String,
    is_offscreen: bool,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl WindowsDriver {
    pub fn new() -> Self {
        Self {
            device_name: std::env::var("COMPUTERNAME").ok(),
            launched_pids: Mutex::new(HashMap::new()),
            active_window_handle: Mutex::new(None),
        }
    }

    fn ensure_windows_host() -> Result<()> {
        if cfg!(target_os = "windows") {
            Ok(())
        } else {
            anyhow::bail!(
                "Windows desktop automation is only supported on Windows hosts; remote Windows automation is not implemented"
            )
        }
    }

    fn powershell(script: &str) -> Result<String> {
        Self::ensure_windows_host()?;
        let executable = "powershell";
        let timeout = Duration::from_secs(30);

        let mut child = Command::new(executable)
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to run {}", executable))?;

        let deadline = Instant::now() + timeout;
        loop {
            if child.try_wait()?.is_some() {
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                anyhow::bail!("{} timed out after {}s", executable, timeout.as_secs());
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        let output = child
            .wait_with_output()
            .with_context(|| format!("failed to collect {} output", executable))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            anyhow::bail!(
                "{} failed with status {}{}",
                executable,
                output.status,
                if stderr.is_empty() {
                    String::new()
                } else {
                    format!(": {}", stderr)
                }
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn click_at(x: i32, y: i32, right_click: bool, double_click: bool) -> Result<()> {
        let button_down = if right_click { "0x0008" } else { "0x0002" };
        let button_up = if right_click { "0x0010" } else { "0x0004" };
        let repeat = if double_click { 2 } else { 1 };
        let script = format!(
            r#"
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class LumiMouse {{
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
  [DllImport("user32.dll")] public static extern void mouse_event(int dwFlags, int dx, int dy, int cButtons, int dwExtraInfo);
}}
"@
[LumiMouse]::SetCursorPos({x}, {y}) | Out-Null
for ($i = 0; $i -lt {repeat}; $i++) {{
  [LumiMouse]::mouse_event({button_down}, 0, 0, 0, 0)
  Start-Sleep -Milliseconds 40
  [LumiMouse]::mouse_event({button_up}, 0, 0, 0, 0)
  Start-Sleep -Milliseconds 80
}}
"#
        );
        Self::powershell(&script)?;
        Ok(())
    }

    fn send_keys(keys: &str) -> Result<()> {
        let script = format!(
            r#"
Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.SendKeys]::SendWait({})
"#,
            ps_string(keys)
        );
        Self::powershell(&script)?;
        Ok(())
    }

    fn ui_elements(&self) -> Result<Vec<WindowsUiElement>> {
        let handle = *self
            .active_window_handle
            .lock()
            .map_err(|_| anyhow::anyhow!("Windows active window handle lock poisoned"))?;
        Ok(Self::powershell(&windows_uia_dump_script(handle))?
            .lines()
            .filter_map(parse_windows_ui_element_line)
            .collect())
    }

    fn find_element(&self, selector: &Selector) -> Result<Option<WindowsUiElement>> {
        let mut matched = Vec::new();
        for element in self.ui_elements()? {
            if element_matches_selector(&element, selector)? {
                matched.push(element);
            }
        }

        let index = selector_index(selector).unwrap_or(0);
        Ok(matched.into_iter().nth(index))
    }
}

impl Default for WindowsDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PlatformDriver for WindowsDriver {
    fn platform_name(&self) -> &str {
        "windows"
    }

    fn device_serial(&self) -> Option<String> {
        self.device_name.clone()
    }

    async fn launch_app(&self, app_id: &str, clear_state: bool) -> Result<()> {
        if clear_state {
            anyhow::bail!("clear_state is not supported by the Windows MVP driver");
        }
        *self
            .active_window_handle
            .lock()
            .map_err(|_| anyhow::anyhow!("Windows active window handle lock poisoned"))? = None;

        let output = Self::powershell(&format!(
            r#"
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class LumiWin32 {{
  [DllImport("user32.dll")]
  public static extern bool SetForegroundWindow(IntPtr hWnd);
}}
"@
$process = Start-Process -FilePath {} -PassThru
try {{ [void]$process.WaitForInputIdle(10000) }} catch {{ }}
for ($i = 0; $i -lt 40; $i++) {{
  $process.Refresh()
  if ($process.MainWindowHandle -ne [IntPtr]::Zero) {{
    [void][LumiWin32]::SetForegroundWindow($process.MainWindowHandle)
    break
  }}
  Start-Sleep -Milliseconds 250
}}
$process.Refresh()
"$($process.Id)`t$([int64]$process.MainWindowHandle)"
"#,
            ps_string(app_id)
        ))?;
        let mut parts = output.trim().split('\t');
        let pid = parts.next().and_then(|value| value.parse::<u32>().ok());
        let handle = parts.next().and_then(|value| value.parse::<isize>().ok());
        if let Some(pid) = pid {
            self.launched_pids
                .lock()
                .map_err(|_| anyhow::anyhow!("Windows launched pid lock poisoned"))?
                .insert(app_id.to_string(), pid);
        }
        if let Some(handle) = handle.filter(|handle| *handle != 0) {
            *self
                .active_window_handle
                .lock()
                .map_err(|_| anyhow::anyhow!("Windows active window handle lock poisoned"))? =
                Some(handle);
        }
        Ok(())
    }

    async fn stop_app(&self, app_id: &str) -> Result<()> {
        Self::ensure_windows_host()?;

        if let Some(pid) = self
            .launched_pids
            .lock()
            .map_err(|_| anyhow::anyhow!("Windows launched pid lock poisoned"))?
            .remove(app_id)
        {
            Self::powershell(&format!(
                "Stop-Process -Id {} -Force -ErrorAction SilentlyContinue",
                pid
            ))?;
            *self
                .active_window_handle
                .lock()
                .map_err(|_| anyhow::anyhow!("Windows active window handle lock poisoned"))? = None;
            return Ok(());
        }

        if app_id.contains('\\') || app_id.contains('/') {
            Self::powershell(&format!(
                r#"
$target = {}
$processes = Get-CimInstance Win32_Process | Where-Object {{ $_.ExecutablePath -eq $target }}
foreach ($process in $processes) {{
  Stop-Process -Id $process.ProcessId -Force -ErrorAction Stop
}}
"#,
                ps_string(app_id)
            ))?;
            Ok(())
        } else {
            anyhow::bail!(
                "No launched PID is tracked for '{}'. Use the same test session that launched the app, or pass an executable path so Windows can stop by exact path.",
                app_id
            )
        }
    }

    async fn tap(&self, selector: &Selector) -> Result<()> {
        if let Selector::Point { x, y } = selector {
            return Self::click_at(*x, *y, false, false);
        }

        if let Some(element) = self.find_element(selector)? {
            return Self::click_at(
                (element.x + element.width / 2.0).round() as i32,
                (element.y + element.height / 2.0).round() as i32,
                false,
                false,
            );
        }

        anyhow::bail!("Windows element not found for selector {:?}", selector)
    }

    async fn long_press(&self, selector: &Selector, duration_ms: u64) -> Result<()> {
        let (x, y) = match selector {
            Selector::Point { x, y } => (*x, *y),
            _ => {
                let Some(element) = self.find_element(selector)? else {
                    anyhow::bail!("Windows element not found for selector {:?}", selector);
                };
                (
                    (element.x + element.width / 2.0).round() as i32,
                    (element.y + element.height / 2.0).round() as i32,
                )
            }
        };
        let script = format!(
            r#"
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class LumiMouse {{
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
  [DllImport("user32.dll")] public static extern void mouse_event(int dwFlags, int dx, int dy, int cButtons, int dwExtraInfo);
}}
"@
[LumiMouse]::SetCursorPos({x}, {y}) | Out-Null
[LumiMouse]::mouse_event(0x0002, 0, 0, 0, 0)
Start-Sleep -Milliseconds {duration_ms}
[LumiMouse]::mouse_event(0x0004, 0, 0, 0, 0)
"#
        );
        Self::powershell(&script)?;
        Ok(())
    }

    async fn double_tap(&self, selector: &Selector) -> Result<()> {
        if let Selector::Point { x, y } = selector {
            return Self::click_at(*x, *y, false, true);
        }

        if let Some(element) = self.find_element(selector)? {
            return Self::click_at(
                (element.x + element.width / 2.0).round() as i32,
                (element.y + element.height / 2.0).round() as i32,
                false,
                true,
            );
        }

        anyhow::bail!("Windows element not found for selector {:?}", selector)
    }

    async fn right_click(&self, selector: &Selector) -> Result<()> {
        if let Selector::Point { x, y } = selector {
            return Self::click_at(*x, *y, true, false);
        }

        if let Some(element) = self.find_element(selector)? {
            return Self::click_at(
                (element.x + element.width / 2.0).round() as i32,
                (element.y + element.height / 2.0).round() as i32,
                true,
                false,
            );
        }

        anyhow::bail!("Windows element not found for selector {:?}", selector)
    }

    async fn input_text(&self, text: &str, _unicode: bool) -> Result<()> {
        self.set_clipboard(text).await?;
        Self::send_keys("^v")
    }

    async fn erase_text(&self, char_count: Option<u32>) -> Result<()> {
        match char_count {
            Some(count) => {
                for _ in 0..count {
                    Self::send_keys("{BACKSPACE}")?;
                }
            }
            None => Self::send_keys("^a{DEL}")?,
        }
        Ok(())
    }

    async fn hide_keyboard(&self) -> Result<()> {
        Ok(())
    }

    async fn swipe(
        &self,
        direction: SwipeDirection,
        _duration_ms: Option<u64>,
        _from: Option<Selector>,
    ) -> Result<()> {
        let key = match direction {
            SwipeDirection::Up => "{PGDN}",
            SwipeDirection::Down => "{PGUP}",
            SwipeDirection::Left => "{LEFT}",
            SwipeDirection::Right => "{RIGHT}",
        };
        Self::send_keys(key)
    }

    async fn scroll_until_visible(
        &self,
        selector: &Selector,
        max_scrolls: u32,
        direction: Option<SwipeDirection>,
        from: Option<Selector>,
    ) -> Result<bool> {
        for _ in 0..max_scrolls {
            if self.is_visible(selector).await? {
                return Ok(true);
            }
            self.swipe(direction.unwrap_or(SwipeDirection::Up), None, from.clone())
                .await?;
            std::thread::sleep(Duration::from_millis(200));
        }
        self.is_visible(selector).await
    }

    async fn is_visible(&self, selector: &Selector) -> Result<bool> {
        if matches!(selector, Selector::Point { .. }) {
            return Ok(true);
        }
        Ok(self.find_element(selector)?.is_some())
    }

    async fn wait_for_element(&self, selector: &Selector, timeout_ms: u64) -> Result<bool> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        while Instant::now() < deadline {
            if self.is_visible(selector).await? {
                return Ok(true);
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        Ok(false)
    }

    async fn wait_for_absence(&self, selector: &Selector, timeout_ms: u64) -> Result<bool> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        while Instant::now() < deadline {
            if !self.is_visible(selector).await? {
                return Ok(true);
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        Ok(false)
    }

    async fn get_element_text(&self, selector: &Selector) -> Result<String> {
        let Some(element) = self.find_element(selector)? else {
            anyhow::bail!("Windows element not found for selector {:?}", selector);
        };

        Ok(first_non_empty([
            element.name.as_str(),
            element.automation_id.as_str(),
            element.help_text.as_str(),
            element.class_name.as_str(),
            element.control_type.as_str(),
        ])
        .unwrap_or_default()
        .to_string())
    }

    async fn open_link(&self, url: &str, _app_id: Option<&str>) -> Result<()> {
        Self::powershell(&format!("Start-Process {}", ps_string(url)))?;
        Ok(())
    }

    async fn compare_screenshot(
        &self,
        reference_path: &Path,
        _tolerance_percent: f64,
    ) -> Result<f64> {
        let temp_path = std::env::temp_dir().join("lumi_tester_windows_compare.png");
        self.take_screenshot(temp_path.to_str().unwrap()).await?;

        let current = image::open(&temp_path)?;
        let reference = image::open(reference_path)?;
        let _ = std::fs::remove_file(&temp_path);

        if current.dimensions() != reference.dimensions() {
            return Ok(100.0);
        }

        let (width, height) = current.dimensions();
        let total_pixels = (width * height) as f64;
        let mut diff_pixels = 0u64;

        for y in 0..height {
            for x in 0..width {
                let c1 = current.get_pixel(x, y);
                let c2 = reference.get_pixel(x, y);
                let channel_diff =
                    c1.0.iter()
                        .zip(c2.0.iter())
                        .any(|(a, b)| (*a as i32 - *b as i32).abs() > 5);
                if channel_diff {
                    diff_pixels += 1;
                }
            }
        }

        Ok((diff_pixels as f64 / total_pixels) * 100.0)
    }

    async fn take_screenshot(&self, path: &str) -> Result<()> {
        let script = format!(
            r#"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bitmap = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
$bitmap.Save({}, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bitmap.Dispose()
"#,
            ps_string(path)
        );
        Self::powershell(&script)?;
        Ok(())
    }

    async fn start_recording(&self, _path: &str) -> Result<()> {
        anyhow::bail!("screen recording is not implemented for the Windows MVP driver")
    }

    async fn stop_recording(&self) -> Result<()> {
        Ok(())
    }

    async fn back(&self) -> Result<()> {
        self.press_key("escape").await
    }

    async fn home(&self) -> Result<()> {
        Self::send_keys("^{ESC}")
    }

    async fn get_screen_size(&self) -> Result<(u32, u32)> {
        let output = Self::powershell(
            "Add-Type -AssemblyName System.Windows.Forms; $b=[System.Windows.Forms.Screen]::PrimaryScreen.Bounds; \"$($b.Width),$($b.Height)\"",
        )?;
        let parts: Vec<u32> = output
            .trim()
            .split(',')
            .filter_map(|part| part.parse::<u32>().ok())
            .collect();
        if parts.len() == 2 {
            Ok((parts[0], parts[1]))
        } else {
            anyhow::bail!("failed to parse Windows screen size: {}", output.trim())
        }
    }

    async fn dump_ui_hierarchy(&self) -> Result<String> {
        let mut lines = vec!["<hierarchy platform=\"windows\">".to_string()];
        for element in self.ui_elements()? {
            lines.push(format!(
                "  <element type=\"{}\" name=\"{}\" id=\"{}\" class=\"{}\" description=\"{}\" offscreen=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
                xml_escape(&element.control_type),
                xml_escape(&element.name),
                xml_escape(&element.automation_id),
                xml_escape(&element.class_name),
                xml_escape(&element.help_text),
                element.is_offscreen,
                element.x.round() as i64,
                element.y.round() as i64,
                element.width.round() as i64,
                element.height.round() as i64,
            ));
        }
        lines.push("</hierarchy>".to_string());
        Ok(lines.join("\n"))
    }

    async fn dump_logs(&self, limit: u32) -> Result<String> {
        Self::powershell(&format!(
            "Get-EventLog -LogName Application -Newest {} | Format-Table -HideTableHeaders -Property TimeGenerated,EntryType,Source,Message | Out-String",
            limit
        ))
    }

    async fn get_pixel_color(&self, x: i32, y: i32) -> Result<(u8, u8, u8)> {
        let temp_path = std::env::temp_dir().join("lumi_tester_windows_pixel.png");
        self.take_screenshot(temp_path.to_str().unwrap()).await?;

        let image = image::open(&temp_path)?;
        let _ = std::fs::remove_file(&temp_path);
        let x = x.max(0) as u32;
        let y = y.max(0) as u32;
        let pixel = image.get_pixel(
            x.min(image.width().saturating_sub(1)),
            y.min(image.height().saturating_sub(1)),
        );

        Ok((pixel[0], pixel[1], pixel[2]))
    }

    async fn press_key(&self, key: &str) -> Result<()> {
        let normalized = key.to_ascii_lowercase();
        let mapped = match normalized.as_str() {
            "return" | "enter" => "{ENTER}".to_string(),
            "tab" => "{TAB}".to_string(),
            "space" => " ".to_string(),
            "delete" => "{DEL}".to_string(),
            "backspace" => "{BACKSPACE}".to_string(),
            "escape" | "esc" => "{ESC}".to_string(),
            "left" | "arrow_left" => "{LEFT}".to_string(),
            "right" | "arrow_right" => "{RIGHT}".to_string(),
            "down" | "arrow_down" => "{DOWN}".to_string(),
            "up" | "arrow_up" => "{UP}".to_string(),
            "home" => "{HOME}".to_string(),
            "end" => "{END}".to_string(),
            "page_up" | "pageup" => "{PGUP}".to_string(),
            "page_down" | "pagedown" => "{PGDN}".to_string(),
            other if other.chars().count() == 1 => other.to_string(),
            other => anyhow::bail!("unsupported Windows key '{}'", other),
        };
        Self::send_keys(&mapped)
    }

    async fn set_clipboard(&self, text: &str) -> Result<()> {
        Self::powershell(&format!("Set-Clipboard -Value {}", ps_string(text)))?;
        Ok(())
    }

    async fn get_clipboard(&self) -> Result<String> {
        Ok(Self::powershell("Get-Clipboard")?
            .trim_end_matches(['\r', '\n'])
            .to_string())
    }

    async fn install_app(&self, path: &str) -> Result<()> {
        Self::powershell(&format!("Start-Process -FilePath {}", ps_string(path)))?;
        Ok(())
    }

    async fn uninstall_app(&self, _app_id: &str) -> Result<()> {
        anyhow::bail!("uninstall_app is not implemented for the Windows MVP driver")
    }

    async fn background_app(&self, _app_id: Option<&str>, duration_ms: u64) -> Result<()> {
        Self::send_keys("%{TAB}")?;
        std::thread::sleep(Duration::from_millis(duration_ms));
        Self::send_keys("%{TAB}")?;
        Ok(())
    }
}

fn ps_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn selector_index(selector: &Selector) -> Option<usize> {
    match selector {
        Selector::Text(_, index, _)
        | Selector::TextRegex(_, index)
        | Selector::Id(_, index)
        | Selector::IdRegex(_, index)
        | Selector::Type(_, index)
        | Selector::Placeholder(_, index)
        | Selector::Role(_, index)
        | Selector::Description(_, index)
        | Selector::DescriptionRegex(_, index)
        | Selector::OCR(_, index, _, _) => Some(*index),
        _ => None,
    }
}

fn element_matches_selector(element: &WindowsUiElement, selector: &Selector) -> Result<bool> {
    if element.is_offscreen || element.width <= 0.0 || element.height <= 0.0 {
        return Ok(false);
    }

    match selector {
        Selector::Text(text, _, exact) => {
            if *exact {
                Ok(element.name == *text)
            } else {
                let needle = text.to_ascii_lowercase();
                Ok(element.name.to_ascii_lowercase().contains(&needle))
            }
        }
        Selector::TextRegex(pattern, _) => {
            let regex = Regex::new(pattern)?;
            Ok(regex.is_match(&element.name))
        }
        Selector::Id(id, _) | Selector::AccessibilityId(id) => Ok(element.automation_id == *id),
        Selector::IdRegex(pattern, _) => Ok(Regex::new(pattern)?.is_match(&element.automation_id)),
        Selector::Type(control_type, _) | Selector::Role(control_type, _) => Ok(
            matches_windows_control_type(&element.control_type, control_type),
        ),
        Selector::Description(description, _) | Selector::Placeholder(description, _) => {
            Ok(element
                .help_text
                .to_ascii_lowercase()
                .contains(&description.to_ascii_lowercase()))
        }
        Selector::DescriptionRegex(pattern, _) => {
            Ok(Regex::new(pattern)?.is_match(&element.help_text))
        }
        Selector::XPath(path) | Selector::Css(path) => Ok(element.automation_id == *path),
        _ => Ok(false),
    }
}

fn matches_windows_control_type(actual: &str, expected: &str) -> bool {
    let normalize = |value: &str| {
        value
            .trim()
            .trim_start_matches("ControlType.")
            .replace([' ', '_', '-'], "")
            .to_ascii_lowercase()
    };
    normalize(actual) == normalize(expected)
}

fn parse_windows_ui_element_line(line: &str) -> Option<WindowsUiElement> {
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() != 10 {
        return None;
    }

    Some(WindowsUiElement {
        control_type: unescape_tsv(parts[0]),
        name: unescape_tsv(parts[1]),
        automation_id: unescape_tsv(parts[2]),
        class_name: unescape_tsv(parts[3]),
        help_text: unescape_tsv(parts[4]),
        is_offscreen: parts[5].eq_ignore_ascii_case("true"),
        x: parts[6].parse().ok()?,
        y: parts[7].parse().ok()?,
        width: parts[8].parse().ok()?,
        height: parts[9].parse().ok()?,
    })
}

fn unescape_tsv(value: &str) -> String {
    value
        .replace("\\t", "\t")
        .replace("\\n", "\n")
        .replace("\\\\", "\\")
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn first_non_empty<'a>(values: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    values.into_iter().find(|value| !value.is_empty())
}

fn windows_uia_dump_script(handle: Option<isize>) -> String {
    WINDOWS_UIA_DUMP_SCRIPT.replace("__LUMI_HANDLE__", &handle.unwrap_or(0).to_string())
}

const WINDOWS_UIA_DUMP_SCRIPT: &str = r#"
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class LumiWin32 {
  [DllImport("user32.dll")]
  public static extern IntPtr GetForegroundWindow();
}
"@

$lumiHandleOverride = [IntPtr]__LUMI_HANDLE__

function Escape-LumiValue([string]$value) {
  if ($null -eq $value) { return "" }
  return $value.Replace("\", "\\").Replace("`t", "\t").Replace("`r", "").Replace("`n", "\n")
}

function String-Property($element, $property) {
  try {
    $value = $element.Current.$property
    if ($null -eq $value) { return "" }
    return [string]$value
  } catch {
    return ""
  }
}

function Element-Key($element) {
  try {
    $runtimeId = $element.GetRuntimeId()
    if ($null -ne $runtimeId) {
      return ($runtimeId -join ".")
    }
  } catch {
  }
  try {
    return [string]$element.Current.NativeWindowHandle
  } catch {
    return ""
  }
}

function Write-Element($element) {
  try {
    $rect = $element.Current.BoundingRectangle
    $type = String-Property $element "ControlType"
    if ($type.StartsWith("ControlType.")) {
      $type = $type.Substring("ControlType.".Length)
    }
    $name = String-Property $element "Name"
    $automationId = String-Property $element "AutomationId"
    $className = String-Property $element "ClassName"
    $helpText = String-Property $element "HelpText"
    $offscreen = String-Property $element "IsOffscreen"

    if ($type -or $name -or $automationId -or $className -or $helpText) {
      @(
        Escape-LumiValue $type,
        Escape-LumiValue $name,
        Escape-LumiValue $automationId,
        Escape-LumiValue $className,
        Escape-LumiValue $helpText,
        Escape-LumiValue $offscreen,
        [string][Math]::Round($rect.X, 2),
        [string][Math]::Round($rect.Y, 2),
        [string][Math]::Round($rect.Width, 2),
        [string][Math]::Round($rect.Height, 2)
      ) -join "`t"
    }
  } catch {
    return
  }
}

function Dump-Tree($root, [int]$maxElements, [int]$maxDepth) {
  if ($null -eq $root) { return }

  $queue = New-Object 'System.Collections.Generic.Queue[object]'
  $seen = New-Object 'System.Collections.Generic.HashSet[string]'
  $queue.Enqueue([pscustomobject]@{ Element = $root; Depth = 0 })
  $emitted = 0

  while ($queue.Count -gt 0 -and $emitted -lt $maxElements) {
    $entry = $queue.Dequeue()
    $element = $entry.Element
    $depth = [int]$entry.Depth
    if ($null -eq $element) { continue }

    $key = Element-Key $element
    if ($key -and -not $seen.Add($key)) { continue }

    Write-Element $element
    $emitted += 1

    if ($depth -ge $maxDepth) { continue }

    try {
      $children = $element.FindAll(
        [System.Windows.Automation.TreeScope]::Children,
        [System.Windows.Automation.Condition]::TrueCondition
      )
      foreach ($child in $children) {
        if ($queue.Count + $emitted -ge $maxElements) { break }
        $queue.Enqueue([pscustomobject]@{ Element = $child; Depth = ($depth + 1) })
      }
    } catch {
      continue
    }
  }
}

$handle = $lumiHandleOverride
if ($handle -eq [IntPtr]::Zero) {
  $handle = [LumiWin32]::GetForegroundWindow()
}
$root = $null
if ($handle -ne [IntPtr]::Zero) {
  $root = [System.Windows.Automation.AutomationElement]::FromHandle($handle)
}
if ($null -eq $root) {
  $root = [System.Windows.Automation.AutomationElement]::RootElement
}
Dump-Tree $root 700 50
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ps_string_escapes_single_quotes() {
        assert_eq!(
            ps_string("C:\\Apps\\Bob's App\\app.exe"),
            "'C:\\Apps\\Bob''s App\\app.exe'"
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn ensure_windows_host_fails_on_non_windows() {
        let error = WindowsDriver::ensure_windows_host()
            .unwrap_err()
            .to_string();
        assert!(error.contains("only supported on Windows hosts"));
    }

    #[test]
    fn parses_windows_uia_element_line() {
        let element = parse_windows_ui_element_line(
            r#"Button	Save	digit\tn	Windows.UI.Button	Helpful	false	10.5	20	30	40"#,
        )
        .unwrap();

        assert_eq!(element.control_type, "Button");
        assert_eq!(element.name, "Save");
        assert_eq!(element.automation_id, "digit\tn");
        assert_eq!(element.class_name, "Windows.UI.Button");
        assert_eq!(element.help_text, "Helpful");
        assert!(!element.is_offscreen);
        assert_eq!(element.x, 10.5);
    }

    #[test]
    fn windows_uia_element_matches_common_selectors() {
        let element = WindowsUiElement {
            control_type: "Button".to_string(),
            name: "Save".to_string(),
            automation_id: "saveButton".to_string(),
            class_name: "Button".to_string(),
            help_text: "Save document".to_string(),
            is_offscreen: false,
            x: 10.0,
            y: 20.0,
            width: 30.0,
            height: 40.0,
        };

        assert!(
            element_matches_selector(&element, &Selector::Text("Save".to_string(), 0, true))
                .unwrap()
        );
        assert!(
            element_matches_selector(&element, &Selector::Id("saveButton".to_string(), 0)).unwrap()
        );
        assert!(element_matches_selector(
            &element,
            &Selector::Type("ControlType.Button".to_string(), 0)
        )
        .unwrap());
        assert!(
            element_matches_selector(&element, &Selector::Role("button".to_string(), 0)).unwrap()
        );
        assert!(element_matches_selector(
            &element,
            &Selector::Description("document".to_string(), 0)
        )
        .unwrap());

        let mut offscreen = element.clone();
        offscreen.is_offscreen = true;
        assert!(!element_matches_selector(
            &offscreen,
            &Selector::Text("Save".to_string(), 0, true)
        )
        .unwrap());
        assert!(!element_matches_selector(
            &element,
            &Selector::Text("saveButton".to_string(), 0, true)
        )
        .unwrap());
    }

    #[test]
    fn windows_uia_dump_script_injects_handle() {
        let script = windows_uia_dump_script(Some(12345));
        assert!(script.contains("$lumiHandleOverride = [IntPtr]12345"));
        assert!(!script.contains("__LUMI_HANDLE__"));
    }
}
