use anyhow::{Context, Result};
use async_trait::async_trait;
use image::GenericImageView;
use regex::Regex;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::driver::traits::{PlatformDriver, Selector, SwipeDirection};

/// Native macOS driver MVP backed by built-in command line tools.
pub struct MacosDriver {
    device_name: Option<String>,
}

#[derive(Debug, Clone)]
struct MacosAxElement {
    role: String,
    title: String,
    description: String,
    value: String,
    identifier: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl MacosDriver {
    pub fn new() -> Self {
        Self {
            device_name: hostname(),
        }
    }

    pub fn with_device_name(device_name: Option<String>) -> Self {
        Self { device_name }
    }

    fn run(program: &str, args: &[&str]) -> Result<String> {
        let mut child = Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to run {}", program))?;

        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if child.try_wait()?.is_some() {
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                anyhow::bail!("{} timed out after 10s", program);
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        let output = child
            .wait_with_output()
            .with_context(|| format!("failed to collect {} output", program))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if program == "osascript" && is_macos_accessibility_error(&stderr) {
                prompt_macos_accessibility_permission();
                anyhow::bail!(
                    "{} failed because macOS Accessibility permission is required. \
Grant permission to your terminal app or lumi-tester in System Settings > Privacy & Security > Accessibility, then run the test again. Original error: {}",
                    program,
                    stderr
                );
            }
            anyhow::bail!(
                "{} failed with status {}{}",
                program,
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

    fn run_with_stdin(program: &str, args: &[&str], input: &str) -> Result<String> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to run {}", program))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input.as_bytes())
                .with_context(|| format!("failed to write {} stdin", program))?;
        }

        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if child.try_wait()?.is_some() {
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                anyhow::bail!("{} timed out after 10s", program);
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        let output = child
            .wait_with_output()
            .with_context(|| format!("failed to collect {} output", program))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            anyhow::bail!(
                "{} failed with status {}{}",
                program,
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

    fn osascript(script: &str) -> Result<String> {
        Self::run("osascript", &["-e", script])
    }

    fn jxa(script: &str) -> Result<String> {
        Self::run("osascript", &["-l", "JavaScript", "-e", script])
    }

    fn swift(script: &str) -> Result<String> {
        Self::run_with_stdin("swift", &["-"], script)
    }

    fn selector_point(selector: &Selector) -> Result<(i32, i32)> {
        match selector {
            Selector::Point { x, y } => Ok((*x, *y)),
            _ => anyhow::bail!(
                "macOS MVP only supports coordinate selectors for pointer input; use point: \"x,y\""
            ),
        }
    }

    fn click_at(x: i32, y: i32) -> Result<()> {
        Self::osascript(&format!(
            "tell application \"System Events\" to click at {{{}, {}}}",
            x, y
        ))?;
        Ok(())
    }

    fn key_code_for(key: &str) -> Option<u16> {
        match key.to_ascii_lowercase().as_str() {
            "return" | "enter" => Some(36),
            "tab" => Some(48),
            "space" => Some(49),
            "delete" | "backspace" => Some(51),
            "escape" | "esc" => Some(53),
            "left" | "arrow_left" => Some(123),
            "right" | "arrow_right" => Some(124),
            "down" | "arrow_down" => Some(125),
            "up" | "arrow_up" => Some(126),
            "home" => Some(115),
            "end" => Some(119),
            "page_up" | "pageup" => Some(116),
            "page_down" | "pagedown" => Some(121),
            _ => None,
        }
    }

    fn modifier_for(key: &str) -> Option<&'static str> {
        match key.to_ascii_lowercase().as_str() {
            "cmd" | "command" | "meta" => Some("command down"),
            "ctrl" | "control" => Some("control down"),
            "option" | "alt" => Some("option down"),
            "shift" => Some("shift down"),
            _ => None,
        }
    }

    fn press_modified_key(key: &str) -> Result<bool> {
        let parts: Vec<&str> = key
            .split('+')
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
            .collect();
        if parts.len() < 2 {
            return Ok(false);
        }

        let Some(target) = parts.last() else {
            return Ok(false);
        };
        let modifiers: Vec<&str> = parts[..parts.len() - 1]
            .iter()
            .map(|part| Self::modifier_for(part))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| anyhow::anyhow!("unsupported macOS modifier in '{}'", key))?;

        let modifier_list = format!("{{{}}}", modifiers.join(", "));
        if let Some(code) = Self::key_code_for(target) {
            Self::osascript(&format!(
                "tell application \"System Events\" to key code {} using {}",
                code, modifier_list
            ))?;
        } else if target.chars().count() == 1 {
            Self::osascript(&format!(
                "tell application \"System Events\" to keystroke {} using {}",
                applescript_string(target),
                modifier_list
            ))?;
        } else {
            anyhow::bail!("unsupported macOS modified key '{}'", key);
        }

        Ok(true)
    }

    fn ax_elements() -> Result<Vec<MacosAxElement>> {
        let output = Self::run_with_stdin("swift", &["-"], MACOS_AX_DUMP_SWIFT)?;
        if output.trim() == "ACCESSIBILITY_DENIED" {
            prompt_macos_accessibility_permission();
            anyhow::bail!(
                "macOS Accessibility permission is required. Grant permission to your terminal app or lumi-tester in System Settings > Privacy & Security > Accessibility, then run the test again."
            );
        }

        Ok(output
            .lines()
            .filter_map(parse_ax_element_line)
            .collect::<Vec<_>>())
    }

    fn find_element(selector: &Selector) -> Result<Option<MacosAxElement>> {
        let elements = Self::ax_elements()?;
        let mut matched = Vec::new();

        for element in elements {
            if element_matches_selector(&element, selector)? {
                matched.push(element);
            }
        }

        let index = selector_index(selector).unwrap_or(0);
        Ok(matched.into_iter().nth(index))
    }
}

impl Default for MacosDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PlatformDriver for MacosDriver {
    fn platform_name(&self) -> &str {
        "macos"
    }

    fn device_serial(&self) -> Option<String> {
        self.device_name.clone()
    }

    async fn launch_app(&self, app_id: &str, clear_state: bool) -> Result<()> {
        if clear_state {
            anyhow::bail!("clear_state is not supported by the macOS MVP driver");
        }

        let args = if Path::new(app_id).exists() {
            vec![app_id]
        } else {
            vec!["-b", app_id]
        };

        let mut last_error = None;
        for _ in 0..3 {
            match Self::run("open", &args) {
                Ok(_) => return Ok(()),
                Err(error) => {
                    last_error = Some(error);
                    std::thread::sleep(Duration::from_millis(250));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("failed to launch macOS app {}", app_id)))
    }

    async fn stop_app(&self, app_id: &str) -> Result<()> {
        if let Some(app_name) = app_name_from_path(app_id) {
            Self::osascript(&format!(
                "tell application {} to quit",
                applescript_string(&app_name)
            ))?;
        } else {
            let escaped = applescript_string(app_id);
            Self::osascript(&format!("tell application id {} to quit", escaped))?;
        }
        Ok(())
    }

    async fn tap(&self, selector: &Selector) -> Result<()> {
        if let Selector::Point { .. } = selector {
            let (x, y) = Self::selector_point(selector)?;
            return Self::click_at(x, y);
        }

        if let Some(element) = Self::find_element(selector)? {
            return Self::click_at(
                (element.x + element.width / 2.0).round() as i32,
                (element.y + element.height / 2.0).round() as i32,
            );
        }

        anyhow::bail!("macOS element not found for selector {:?}", selector)
    }

    async fn long_press(&self, selector: &Selector, duration_ms: u64) -> Result<()> {
        let (x, y) = Self::selector_point(selector)?;
        Self::swift(&format!(
            r#"
import CoreGraphics
import Foundation

let point = CGPoint(x: {x}, y: {y})
let source = CGEventSource(stateID: .hidSystemState)
if let down = CGEvent(mouseEventSource: source, mouseType: .leftMouseDown, mouseCursorPosition: point, mouseButton: .left),
   let up = CGEvent(mouseEventSource: source, mouseType: .leftMouseUp, mouseCursorPosition: point, mouseButton: .left) {{
    down.post(tap: .cghidEventTap)
    Thread.sleep(forTimeInterval: {duration})
    up.post(tap: .cghidEventTap)
}}
"#,
            x = x,
            y = y,
            duration = duration_ms as f64 / 1000.0
        ))?;
        Ok(())
    }

    async fn double_tap(&self, selector: &Selector) -> Result<()> {
        self.tap(selector).await?;
        std::thread::sleep(Duration::from_millis(100));
        self.tap(selector).await
    }

    async fn right_click(&self, selector: &Selector) -> Result<()> {
        let (x, y) = Self::selector_point(selector)?;
        Self::osascript(&format!(
            "tell application \"System Events\" to control click at {{{}, {}}}",
            x, y
        ))?;
        Ok(())
    }

    async fn input_text(&self, text: &str, _unicode: bool) -> Result<()> {
        Self::osascript(&format!(
            "tell application \"System Events\" to keystroke {}",
            applescript_string(text)
        ))?;
        Ok(())
    }

    async fn erase_text(&self, char_count: Option<u32>) -> Result<()> {
        match char_count {
            Some(count) => {
                for _ in 0..count {
                    self.press_key("delete").await?;
                }
            }
            None => {
                Self::osascript(
                    "tell application \"System Events\"\n  keystroke \"a\" using command down\n  key code 51\nend tell",
                )?;
            }
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
        let command = match direction {
            SwipeDirection::Up => (0, -5),
            SwipeDirection::Down => (0, 5),
            SwipeDirection::Left => (-5, 0),
            SwipeDirection::Right => (5, 0),
        };
        Self::swift(&format!(
            r#"
import CoreGraphics

if let event = CGEvent(
    scrollWheelEvent2Source: nil,
    units: .line,
    wheelCount: 2,
    wheel1: {vertical},
    wheel2: {horizontal},
    wheel3: 0
) {{
    event.post(tap: .cghidEventTap)
}}
"#,
            vertical = command.1,
            horizontal = command.0
        ))?;
        Ok(())
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
        if let Selector::Point { .. } = selector {
            return Ok(true);
        }
        Ok(Self::find_element(selector)?.is_some())
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
        let Some(element) = Self::find_element(selector)? else {
            anyhow::bail!("macOS element not found for selector {:?}", selector);
        };

        Ok(first_non_empty([
            element.value.as_str(),
            element.title.as_str(),
            element.description.as_str(),
            element.identifier.as_str(),
        ])
        .unwrap_or_default()
        .to_string())
    }

    async fn open_link(&self, url: &str, app_id: Option<&str>) -> Result<()> {
        if let Some(bundle_id) = app_id {
            Self::run("open", &["-b", bundle_id, url])?;
        } else {
            Self::run("open", &[url])?;
        }
        Ok(())
    }

    async fn compare_screenshot(
        &self,
        reference_path: &Path,
        _tolerance_percent: f64,
    ) -> Result<f64> {
        let temp_path = std::env::temp_dir().join("lumi_tester_macos_compare.png");
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
        Self::run("screencapture", &["-x", path])?;
        Ok(())
    }

    async fn start_recording(&self, _path: &str) -> Result<()> {
        anyhow::bail!("screen recording is not implemented for the macOS MVP driver")
    }

    async fn stop_recording(&self) -> Result<()> {
        Ok(())
    }

    async fn back(&self) -> Result<()> {
        self.press_key("escape").await
    }

    async fn home(&self) -> Result<()> {
        Self::osascript("tell application \"Finder\" to activate")?;
        Ok(())
    }

    async fn get_screen_size(&self) -> Result<(u32, u32)> {
        let output = Self::jxa(
            r#"ObjC.import("AppKit");
var frame = $.NSScreen.mainScreen.frame;
Math.round(frame.size.width) + "," + Math.round(frame.size.height);"#,
        )?;
        let parts: Vec<u32> = output
            .trim()
            .split(',')
            .filter_map(|part| part.parse::<u32>().ok())
            .collect();

        if parts.len() == 2 {
            Ok((parts[0], parts[1]))
        } else {
            anyhow::bail!("failed to parse macOS screen size: {}", output.trim())
        }
    }

    async fn dump_ui_hierarchy(&self) -> Result<String> {
        let mut lines = vec!["<hierarchy platform=\"macos\">".to_string()];
        for element in Self::ax_elements()? {
            lines.push(format!(
                "  <element role=\"{}\" title=\"{}\" description=\"{}\" value=\"{}\" id=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
                xml_escape(&element.role),
                xml_escape(&element.title),
                xml_escape(&element.description),
                xml_escape(&element.value),
                xml_escape(&element.identifier),
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
        let predicate = "process != \"kernel\"";
        Self::run(
            "log",
            &[
                "show",
                "--style",
                "compact",
                "--last",
                "5m",
                "--predicate",
                predicate,
                "--info",
                "--debug",
            ],
        )
        .map(|logs| {
            let lines: Vec<&str> = logs.lines().rev().take(limit as usize).collect();
            lines.into_iter().rev().collect::<Vec<_>>().join("\n")
        })
    }

    async fn get_pixel_color(&self, x: i32, y: i32) -> Result<(u8, u8, u8)> {
        let temp_path = std::env::temp_dir().join("lumi_tester_macos_pixel.png");
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
        if Self::press_modified_key(key)? {
            return Ok(());
        }

        if let Some(code) = Self::key_code_for(key) {
            Self::osascript(&format!(
                "tell application \"System Events\" to key code {}",
                code
            ))?;
            Ok(())
        } else if key.chars().count() == 1 {
            self.input_text(key, true).await
        } else {
            anyhow::bail!("unsupported macOS key '{}'", key)
        }
    }

    async fn set_clipboard(&self, text: &str) -> Result<()> {
        Self::osascript(&format!(
            "set the clipboard to {}",
            applescript_string(text)
        ))?;
        Ok(())
    }

    async fn get_clipboard(&self) -> Result<String> {
        Ok(Self::osascript("the clipboard")?
            .trim_end_matches(['\r', '\n'])
            .to_string())
    }

    async fn install_app(&self, path: &str) -> Result<()> {
        if path.ends_with(".app") {
            Self::run("open", &[path])?;
            Ok(())
        } else {
            anyhow::bail!("macOS MVP install_app only accepts .app paths")
        }
    }

    async fn uninstall_app(&self, _app_id: &str) -> Result<()> {
        anyhow::bail!("uninstall_app is not implemented for the macOS MVP driver")
    }

    async fn background_app(&self, app_id: Option<&str>, duration_ms: u64) -> Result<()> {
        if let Some(bundle_id) = app_id {
            let escaped = applescript_string(bundle_id);
            Self::osascript(&format!("tell application id {} to hide", escaped))?;
        }
        std::thread::sleep(Duration::from_millis(duration_ms));
        if let Some(bundle_id) = app_id {
            let escaped = applescript_string(bundle_id);
            Self::osascript(&format!("tell application id {} to activate", escaped))?;
        }
        Ok(())
    }
}

fn applescript_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
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

fn element_matches_selector(element: &MacosAxElement, selector: &Selector) -> Result<bool> {
    let text_fields = [
        element.title.as_str(),
        element.description.as_str(),
        element.value.as_str(),
    ];
    match selector {
        Selector::Text(text, _, exact) => {
            if *exact {
                Ok(text_fields.iter().any(|value| *value == text))
            } else {
                let needle = text.to_ascii_lowercase();
                Ok(text_fields
                    .iter()
                    .any(|value| value.to_ascii_lowercase().contains(&needle)))
            }
        }
        Selector::TextRegex(pattern, _) => {
            let regex = Regex::new(pattern)?;
            Ok(text_fields.iter().any(|value| regex.is_match(value)))
        }
        Selector::Id(id, _) | Selector::AccessibilityId(id) => Ok(element.identifier == *id),
        Selector::IdRegex(pattern, _) => Ok(Regex::new(pattern)?.is_match(&element.identifier)),
        Selector::Type(role, _) | Selector::Role(role, _) => {
            Ok(element.role.eq_ignore_ascii_case(role)
                || element.role.eq_ignore_ascii_case(&format!("AX{}", role)))
        }
        Selector::Description(description, _) | Selector::Placeholder(description, _) => {
            Ok(element
                .description
                .to_ascii_lowercase()
                .contains(&description.to_ascii_lowercase()))
        }
        Selector::DescriptionRegex(pattern, _) => {
            Ok(Regex::new(pattern)?.is_match(&element.description))
        }
        Selector::XPath(path) | Selector::Css(path) => Ok(element.identifier == *path),
        _ => Ok(false),
    }
}

fn parse_ax_element_line(line: &str) -> Option<MacosAxElement> {
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() != 9 {
        return None;
    }
    Some(MacosAxElement {
        role: unescape_tsv(parts[0]),
        title: unescape_tsv(parts[1]),
        description: unescape_tsv(parts[2]),
        value: unescape_tsv(parts[3]),
        identifier: unescape_tsv(parts[4]),
        x: parts[5].parse().ok()?,
        y: parts[6].parse().ok()?,
        width: parts[7].parse().ok()?,
        height: parts[8].parse().ok()?,
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

fn app_name_from_path(value: &str) -> Option<String> {
    let path = Path::new(value);
    if !path.exists() || path.extension().and_then(|ext| ext.to_str()) != Some("app") {
        return None;
    }

    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_string())
        .filter(|stem| !stem.is_empty())
}

fn is_macos_accessibility_error(stderr: &str) -> bool {
    let normalized = stderr.to_ascii_lowercase();
    normalized.contains("not allowed to send keystrokes")
        || normalized.contains("not allowed assistive access")
        || normalized.contains("not authorized to send apple events")
        || normalized.contains("system events got an error") && normalized.contains("not allowed")
}

fn prompt_macos_accessibility_permission() {
    let _ = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .status();
}

fn hostname() -> Option<String> {
    Command::new("scutil")
        .args(["--get", "ComputerName"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|name| !name.is_empty())
}

const MACOS_AX_DUMP_SWIFT: &str = r#"
import AppKit
import ApplicationServices
import Foundation

func escape(_ value: String) -> String {
    value
        .replacingOccurrences(of: "\\", with: "\\\\")
        .replacingOccurrences(of: "\t", with: "\\t")
        .replacingOccurrences(of: "\n", with: "\\n")
}

func stringAttr(_ element: AXUIElement, _ attr: CFString) -> String {
    var value: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, attr, &value) == .success else {
        return ""
    }
    guard let unwrapped = value else {
        return ""
    }
    if CFGetTypeID(unwrapped) == AXUIElementGetTypeID() {
        return ""
    }
    return "\(unwrapped)"
}

func pointAttr(_ element: AXUIElement, _ attr: CFString) -> CGPoint {
    var value: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, attr, &value) == .success,
          let axValue = value,
          CFGetTypeID(axValue) == AXValueGetTypeID() else {
        return .zero
    }
    var point = CGPoint.zero
    AXValueGetValue((axValue as! AXValue), .cgPoint, &point)
    return point
}

func sizeAttr(_ element: AXUIElement, _ attr: CFString) -> CGSize {
    var value: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, attr, &value) == .success,
          let axValue = value,
          CFGetTypeID(axValue) == AXValueGetTypeID() else {
        return .zero
    }
    var size = CGSize.zero
    AXValueGetValue((axValue as! AXValue), .cgSize, &size)
    return size
}

func children(_ element: AXUIElement) -> [AXUIElement] {
    var value: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &value) == .success,
          let children = value as? [AXUIElement] else {
        return []
    }
    return children
}

func windows(_ element: AXUIElement) -> [AXUIElement] {
    var value: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, kAXWindowsAttribute as CFString, &value) == .success,
          let windows = value as? [AXUIElement] else {
        return []
    }
    return windows
}

var emittedElements = 0
let maxElements = 500

func dump(_ element: AXUIElement, _ depth: Int) {
    if depth > 20 || emittedElements >= maxElements {
        return
    }

    let role = stringAttr(element, kAXRoleAttribute as CFString)
    let title = stringAttr(element, kAXTitleAttribute as CFString)
    let description = stringAttr(element, kAXDescriptionAttribute as CFString)
    let value = stringAttr(element, kAXValueAttribute as CFString)
    let identifier = stringAttr(element, kAXIdentifierAttribute as CFString)
    let position = pointAttr(element, kAXPositionAttribute as CFString)
    let size = sizeAttr(element, kAXSizeAttribute as CFString)

    if !role.isEmpty || !title.isEmpty || !description.isEmpty || !value.isEmpty || !identifier.isEmpty {
        emittedElements += 1
        print([
            escape(role),
            escape(title),
            escape(description),
            escape(value),
            escape(identifier),
            String(format: "%.2f", position.x),
            String(format: "%.2f", position.y),
            String(format: "%.2f", size.width),
            String(format: "%.2f", size.height)
        ].joined(separator: "\t"))
    }

    for child in children(element) {
        dump(child, depth + 1)
        if emittedElements >= maxElements {
            return
        }
    }
}

if !AXIsProcessTrusted() {
    print("ACCESSIBILITY_DENIED")
    exit(0)
}

guard let app = NSWorkspace.shared.frontmostApplication else {
    exit(0)
}

let appElement = AXUIElementCreateApplication(app.processIdentifier)
let rootWindows = windows(appElement)
if rootWindows.isEmpty {
    dump(appElement, 0)
} else {
    for window in rootWindows {
        dump(window, 0)
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applescript_string_escapes_quotes_and_backslashes() {
        assert_eq!(
            applescript_string(r#"say "hi" \ done"#),
            r#""say \"hi\" \\ done""#
        );
    }

    #[test]
    fn app_name_from_path_extracts_app_bundle_name() {
        let app_dir = std::env::temp_dir().join(format!(
            "lumi-tester-example-{}-Example.app",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&app_dir);
        std::fs::create_dir_all(&app_dir).unwrap();

        assert_eq!(
            app_name_from_path(app_dir.to_str().unwrap()),
            Some(format!(
                "lumi-tester-example-{}-Example",
                std::process::id()
            ))
        );

        let _ = std::fs::remove_dir_all(&app_dir);
    }

    #[test]
    fn modifier_for_maps_common_macos_modifiers() {
        assert_eq!(MacosDriver::modifier_for("cmd"), Some("command down"));
        assert_eq!(MacosDriver::modifier_for("control"), Some("control down"));
        assert_eq!(MacosDriver::modifier_for("alt"), Some("option down"));
        assert_eq!(MacosDriver::modifier_for("shift"), Some("shift down"));
    }

    #[test]
    fn detects_accessibility_permission_errors() {
        assert!(is_macos_accessibility_error(
            "osascript is not allowed to send keystrokes. (1002)"
        ));
        assert!(!is_macos_accessibility_error("syntax error"));
    }

    #[test]
    fn macos_ax_element_matches_common_selectors() {
        let element = MacosAxElement {
            role: "AXButton".to_string(),
            title: "Seven".to_string(),
            description: "Calculator digit".to_string(),
            value: String::new(),
            identifier: "digit-7".to_string(),
            x: 10.0,
            y: 20.0,
            width: 30.0,
            height: 40.0,
        };

        assert!(
            element_matches_selector(&element, &Selector::Text("Seven".to_string(), 0, true))
                .unwrap()
        );
        assert!(
            element_matches_selector(&element, &Selector::Id("digit-7".to_string(), 0)).unwrap()
        );
        assert!(
            element_matches_selector(&element, &Selector::Role("button".to_string(), 0)).unwrap()
        );
        assert!(
            element_matches_selector(&element, &Selector::Type("AXButton".to_string(), 0)).unwrap()
        );
        assert!(
            element_matches_selector(&element, &Selector::Description("digit".to_string(), 0))
                .unwrap()
        );
    }
}
