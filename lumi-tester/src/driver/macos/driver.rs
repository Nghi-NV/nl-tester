use super::accessibility::{AXNode, MacosAccessibility};
use super::bridge::{MacosBridge, WindowBounds};
use crate::driver::traits::{PlatformDriver, Selector, SwipeDirection};
use crate::parser::types::DesktopState;
use anyhow::Result;
use async_trait::async_trait;
use image::GenericImageView;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct DesktopStateRuntime {
    state: Option<DesktopState>,
    base_dir: PathBuf,
}

/// High-level macOS Desktop Platform Driver implementing PlatformDriver
#[derive(Debug, Clone)]
pub struct MacosDriver {
    bridge: MacosBridge,
    accessibility: MacosAccessibility,
    active_app: Arc<Mutex<Option<String>>>,
    saved_window_bounds: Arc<Mutex<HashMap<String, WindowBounds>>>,
    desktop_state: Arc<Mutex<Option<DesktopStateRuntime>>>,
    device_name: Option<String>,
}

impl MacosDriver {
    pub fn new() -> Self {
        Self {
            bridge: MacosBridge::new(),
            accessibility: MacosAccessibility::new(),
            active_app: Arc::new(Mutex::new(None)),
            saved_window_bounds: Arc::new(Mutex::new(HashMap::new())),
            desktop_state: Arc::new(Mutex::new(None)),
            device_name: None,
        }
    }

    pub fn new_with_device(device_name: &str) -> Self {
        let mut driver = Self::new();
        driver.device_name = Some(device_name.to_string());
        driver
    }

    /// Retrieve active application identifier (from flow header or active app)
    pub fn active_target(&self) -> String {
        self.active_app
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .unwrap_or_default()
    }

    /// Find an element matching a Selector in the active application
    pub fn find_element(&self, selector: &Selector) -> Result<Option<AXNode>> {
        self.accessibility
            .find_element(&self.active_target(), selector)
    }

    fn clear_desktop_state(&self, app_id: &str) -> Result<()> {
        let guard = self
            .desktop_state
            .lock()
            .map_err(|_| anyhow::anyhow!("macOS desktop state lock poisoned"))?;
        let Some(runtime) = guard.as_ref() else {
            return Ok(());
        };
        let Some(state) = runtime.state.as_ref() else {
            return Ok(());
        };

        if let Some(clear) = state.clear.as_ref() {
            let home = std::env::var("HOME").unwrap_or_default();
            let home_path = PathBuf::from(home);
            let app_name = Path::new(app_id)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(app_id);

            let mut targets = Vec::new();
            if clear.paths.is_empty() {
                // AutoSafe default paths
                targets.push(home_path.join("Library/Application Support").join(app_name));
                targets.push(home_path.join("Library/Caches").join(app_name));
                targets.push(home_path.join("Library/Saved Application State").join(format!("{}.savedState", app_name)));
            } else {
                for p in &clear.paths {
                    if p.starts_with('/') {
                        targets.push(PathBuf::from(p));
                    } else if p.starts_with("~/") {
                        targets.push(home_path.join(&p[2..]));
                    } else {
                        targets.push(runtime.base_dir.join(p));
                    }
                }
            }

            for resolved in targets {
                if resolved.exists() {
                    if resolved.is_dir() {
                        let _ = std::fs::remove_dir_all(&resolved);
                    } else {
                        let _ = std::fs::remove_file(&resolved);
                    }
                }
            }
        }
        Ok(())
    }

    fn selector_point(selector: &Selector) -> Result<(i32, i32)> {
        match selector {
            Selector::Point { x, y } => Ok((*x, *y)),
            _ => anyhow::bail!("Expected Point selector, found {:?}", selector),
        }
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

    fn set_desktop_state(&self, state: Option<DesktopState>, base_dir: &Path) -> Result<()> {
        *self
            .desktop_state
            .lock()
            .map_err(|_| anyhow::anyhow!("macOS desktop state lock poisoned"))? =
            Some(DesktopStateRuntime {
                state,
                base_dir: base_dir.to_path_buf(),
            });
        Ok(())
    }

    fn set_active_app(&self, app_id: Option<&str>) {
        if let Ok(mut guard) = self.active_app.lock() {
            *guard = app_id.map(|s| s.to_string());
        }
    }

    async fn set_window_size(&self, width: u32, height: u32) -> Result<()> {
        let app_target = self.active_target();
        self.bridge.set_window_size(&app_target, width, height)?;
        if let Some(bounds) = self.bridge.get_window_bounds(&app_target) {
            if let Ok(mut map) = self.saved_window_bounds.lock() {
                map.insert(app_target, bounds);
            }
        }
        Ok(())
    }

    async fn launch_app(&self, app_id: &str, clear_state: bool) -> Result<()> {
        if let Ok(mut guard) = self.active_app.lock() {
            *guard = Some(app_id.to_string());
        }

        // Capture previous window bounds if running before clearing or launching
        if let Some(bounds) = self.bridge.get_window_bounds(app_id) {
            if let Ok(mut map) = self.saved_window_bounds.lock() {
                map.entry(app_id.to_string()).or_insert(bounds);
            }
        }

        if clear_state {
            self.stop_app(app_id).await.ok();
            self.clear_desktop_state(app_id)?;
        }

        let saved_bounds = self
            .saved_window_bounds
            .lock()
            .ok()
            .and_then(|map| map.get(app_id).copied());

        self.bridge.launch_app(app_id, saved_bounds)?;
        Ok(())
    }

    async fn stop_app(&self, app_id: &str) -> Result<()> {
        // Save window bounds before stopping
        if let Some(bounds) = self.bridge.get_window_bounds(app_id) {
            if let Ok(mut map) = self.saved_window_bounds.lock() {
                map.insert(app_id.to_string(), bounds);
            }
        }
        self.bridge.stop_app(app_id)?;
        Ok(())
    }

    async fn tap(&self, selector: &Selector) -> Result<()> {
        if let Selector::Point { .. } = selector {
            let (x, y) = Self::selector_point(selector)?;
            return MacosBridge::click_at(x, y, true);
        }

        // 1. Try non-intrusive Accessibility AXPress action directly on target element
        let app_target = self.active_target();
        if self.accessibility.press_element(&app_target, selector)? {
            return Ok(());
        }

        // 2. Fallback to coordinate-based click with instant cursor restoration
        if let Some(element) = self.find_element(selector)? {
            let (cx, cy) = element.center_point();
            return MacosBridge::click_at(cx, cy, true);
        }

        anyhow::bail!("macOS element not found for selector {:?}", selector);
    }

    async fn resolve_element_point(
        &self,
        selector: &Selector,
        x_pct: f64,
        y_pct: f64,
    ) -> Result<Option<(i32, i32)>> {
        if let Some(element) = self.find_element(selector)? {
            let x = (element.x + element.width * x_pct.clamp(0.0, 1.0)).round() as i32;
            let y = (element.y + element.height * y_pct.clamp(0.0, 1.0)).round() as i32;
            return Ok(Some((x, y)));
        }
        Ok(None)
    }

    async fn long_press(&self, selector: &Selector, duration_ms: u64) -> Result<()> {
        let (x, y) = if let Selector::Point { .. } = selector {
            Self::selector_point(selector)?
        } else if let Some(element) = self.find_element(selector)? {
            element.center_point()
        } else {
            anyhow::bail!("macOS element not found for selector {:?}", selector);
        };
        MacosBridge::long_press_at(x, y, duration_ms)?;
        Ok(())
    }

    async fn double_tap(&self, selector: &Selector) -> Result<()> {
        let (x, y) = if let Selector::Point { .. } = selector {
            Self::selector_point(selector)?
        } else if let Some(element) = self.find_element(selector)? {
            element.center_point()
        } else {
            anyhow::bail!("macOS element not found for selector {:?}", selector);
        };
        MacosBridge::double_click_at(x, y)?;
        Ok(())
    }

    async fn right_click(&self, selector: &Selector) -> Result<()> {
        let (x, y) = if let Selector::Point { .. } = selector {
            Self::selector_point(selector)?
        } else if let Some(element) = self.find_element(selector)? {
            element.center_point()
        } else {
            anyhow::bail!("macOS element not found for selector {:?}", selector);
        };
        MacosBridge::right_click_at(x, y)?;
        Ok(())
    }

    async fn input_text(&self, text: &str, _unicode: bool) -> Result<()> {
        self.accessibility
            .input_text(&self.active_target(), text)?;
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
                MacosBridge::run_osascript(
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
        duration_ms: Option<u64>,
        from: Option<Selector>,
    ) -> Result<()> {
        let start_point = if let Some(ref sel) = from {
            if let Some(el) = self.find_element(sel)? {
                el.center_point()
            } else {
                (600, 400)
            }
        } else {
            (600, 400)
        };

        let distance = 300;
        let end_point = match direction {
            SwipeDirection::Up => (start_point.0, start_point.1 - distance),
            SwipeDirection::Down => (start_point.0, start_point.1 + distance),
            SwipeDirection::Left => (start_point.0 - distance, start_point.1),
            SwipeDirection::Right => (start_point.0 + distance, start_point.1),
        };

        self.bridge
            .swipe(start_point, end_point, duration_ms.unwrap_or(300))?;
        Ok(())
    }

    async fn drag(&self, from: (i32, i32), to: (i32, i32), duration_ms: u64) -> Result<()> {
        self.bridge.swipe(from, to, duration_ms)?;
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
            anyhow::bail!("macOS element not found for selector {:?}", selector);
        };

        let val = if !element.value.is_empty() {
            element.value
        } else if !element.title.is_empty() {
            element.title
        } else if !element.description.is_empty() {
            element.description
        } else {
            element.identifier
        };
        Ok(val)
    }

    async fn open_link(&self, url: &str, _app_id: Option<&str>) -> Result<()> {
        self.bridge.open_url(url)?;
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
        self.bridge.capture_screenshot(path)?;
        Ok(())
    }

    async fn start_recording(&self, _path: &str) -> Result<()> {
        anyhow::bail!("screen recording is not implemented for the macOS driver")
    }

    async fn stop_recording(&self) -> Result<()> {
        Ok(())
    }

    async fn back(&self) -> Result<()> {
        MacosBridge::run_osascript(
            "tell application \"System Events\"\n  keystroke \"[\" using command down\nend tell",
        )?;
        Ok(())
    }

    async fn home(&self) -> Result<()> {
        MacosBridge::run_osascript(
            "tell application \"System Events\"\n  keystroke \"h\" using {command down, option down}\nend tell",
        )?;
        Ok(())
    }

    async fn get_screen_size(&self) -> Result<(u32, u32)> {
        const SCRIPT: &str = r#"
import AppKit
if let screen = NSScreen.main {
    print("\(Int(screen.frame.width)),\(Int(screen.frame.height))")
}
"#;
        let out = MacosBridge::run_swift(SCRIPT, &[])?;
        let parts: Vec<&str> = out.trim().split(',').collect();
        if parts.len() == 2 {
            let w = parts[0].parse::<u32>().unwrap_or(1920);
            let h = parts[1].parse::<u32>().unwrap_or(1080);
            return Ok((w, h));
        }
        Ok((1920, 1080))
    }

    async fn dump_ui_hierarchy(&self) -> Result<String> {
        self.accessibility
            .dump_ui_hierarchy(&self.active_target())
    }

    async fn dump_logs(&self, _limit: u32) -> Result<String> {
        Ok(String::new())
    }

    async fn press_key(&self, key: &str) -> Result<()> {
        self.bridge.post_key(key)?;
        Ok(())
    }

    async fn set_clipboard(&self, text: &str) -> Result<()> {
        const SCRIPT: &str = r#"
import AppKit
guard CommandLine.arguments.count > 1 else { exit(1) }
let text = CommandLine.arguments[1]
let pasteboard = NSPasteboard.general
pasteboard.clearContents()
pasteboard.setString(text, forType: .string)
exit(0)
"#;
        MacosBridge::run_swift(SCRIPT, &[text])?;
        Ok(())
    }

    async fn get_clipboard(&self) -> Result<String> {
        const SCRIPT: &str = r#"
import AppKit
let pasteboard = NSPasteboard.general
if let text = pasteboard.string(forType: .string) {
    print(text)
}
exit(0)
"#;
        MacosBridge::run_swift(SCRIPT, &[])
    }
}
