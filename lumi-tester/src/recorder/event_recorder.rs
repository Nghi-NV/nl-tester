//! Event Recorder for capturing user interactions on Android devices
//!
//! This module captures touch events and maps them to UI elements using
//! the UIAutomator hierarchy dump.

use anyhow::{Context, Result};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::driver::android::adb;
use crate::driver::android::uiautomator::{self, UiElement};
use crate::utils::binary_resolver;

use super::selector_scorer::{SelectorCandidate, SelectorScorer};

/// Types of recorded actions
#[derive(Debug, Clone)]
pub enum RecordedAction {
    /// Tap on an element
    Tap {
        element: UiElement,
        selectors: Vec<SelectorCandidate>,
        timestamp: Instant,
    },
    /// Long press on an element
    LongPress {
        element: UiElement,
        selectors: Vec<SelectorCandidate>,
        duration_ms: u64,
        timestamp: Instant,
    },
    /// Text input
    Input {
        element: UiElement,
        selectors: Vec<SelectorCandidate>,
        text: String,
        timestamp: Instant,
    },
    /// Swipe gesture
    Swipe {
        direction: String,
        timestamp: Instant,
    },
    /// Wait/pause
    Wait {
        duration_ms: u64,
        timestamp: Instant,
    },
    /// App opened
    OpenApp { app_id: String, timestamp: Instant },
}

impl RecordedAction {
    /// Get the timestamp of this action
    pub fn timestamp(&self) -> Instant {
        match self {
            RecordedAction::Tap { timestamp, .. } => *timestamp,
            RecordedAction::LongPress { timestamp, .. } => *timestamp,
            RecordedAction::Input { timestamp, .. } => *timestamp,
            RecordedAction::Swipe { timestamp, .. } => *timestamp,
            RecordedAction::Wait { timestamp, .. } => *timestamp,
            RecordedAction::OpenApp { timestamp, .. } => *timestamp,
        }
    }
}

/// Touch event type from getevent
#[derive(Debug, Clone)]
struct TouchEvent {
    event_type: TouchEventType,
    x: i32,
    y: i32,
    timestamp: Instant,
}

#[derive(Debug, Clone, PartialEq)]
enum TouchEventType {
    Down,
    Up,
    Move,
}

/// Event recorder that captures user interactions
pub struct EventRecorder {
    /// Device serial
    serial: Option<String>,
    /// Screen dimensions
    pub screen_width: u32,
    pub screen_height: u32,
    /// Recording state
    is_recording: Arc<Mutex<bool>>,
    /// Recorded actions
    actions: Arc<Mutex<Vec<RecordedAction>>>,
    /// Current UI hierarchy cache
    ui_cache: Arc<Mutex<Option<Vec<UiElement>>>>,
    /// Last hierarchy dump time
    last_dump: Arc<Mutex<Instant>>,
    /// Current foreground app
    current_app: Arc<Mutex<Option<String>>>,
}

impl EventRecorder {
    /// Create a new event recorder for the specified device
    pub async fn new(serial: Option<&str>) -> Result<Self> {
        let (width, height) = adb::get_screen_size(serial).await?;

        Ok(Self {
            serial: serial.map(|s| s.to_string()),
            screen_width: width,
            screen_height: height,
            is_recording: Arc::new(Mutex::new(false)),
            actions: Arc::new(Mutex::new(Vec::new())),
            ui_cache: Arc::new(Mutex::new(None)),
            last_dump: Arc::new(Mutex::new(Instant::now())),
            current_app: Arc::new(Mutex::new(None)),
        })
    }

    /// Start recording user interactions
    pub async fn start_recording(&self) -> Result<()> {
        let mut is_recording = self.is_recording.lock().await;
        if *is_recording {
            anyhow::bail!("Already recording");
        }
        *is_recording = true;
        drop(is_recording);

        // Detect current foreground app
        let app = self.get_foreground_app().await?;
        if let Some(ref app_id) = app {
            let mut current = self.current_app.lock().await;
            *current = Some(app_id.clone());

            let mut actions = self.actions.lock().await;
            actions.push(RecordedAction::OpenApp {
                app_id: app_id.clone(),
                timestamp: Instant::now(),
            });
        }

        // Initial UI dump
        self.refresh_ui_cache().await?;

        println!("🔴 Recording started. Interact with the device...");
        println!("   Press Ctrl+C to stop recording.\n");

        Ok(())
    }

    /// Stop recording and return all recorded actions
    pub async fn stop_recording(&self) -> Result<Vec<RecordedAction>> {
        let mut is_recording = self.is_recording.lock().await;
        *is_recording = false;

        let actions = self.actions.lock().await;
        Ok(actions.clone())
    }

    /// Check if currently recording
    pub async fn is_recording(&self) -> bool {
        *self.is_recording.lock().await
    }

    /// Get the current foreground app package
    async fn get_foreground_app(&self) -> Result<Option<String>> {
        // Don't use grep in shell - it may fail on some devices
        // Instead, get full output and filter in Rust
        let output = adb::shell(self.serial.as_deref(), "dumpsys activity activities")
            .await
            .unwrap_or_default();

        // Parse: mResumedActivity: ActivityRecord{... com.example.app/.MainActivity ...}
        for line in output.lines() {
            if line.contains("mResumedActivity") || line.contains("topResumedActivity") {
                if let Some(start) = line.find("com.") {
                    let rest = &line[start..];
                    if let Some(end) = rest.find('/') {
                        return Ok(Some(rest[..end].to_string()));
                    }
                    // Try finding end by space
                    if let Some(end) = rest.find(' ') {
                        return Ok(Some(rest[..end].to_string()));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Refresh the UI hierarchy cache
    async fn refresh_ui_cache(&self) -> Result<()> {
        let serial = self.serial.as_deref();

        // Dump UI hierarchy
        adb::shell(serial, "uiautomator dump /sdcard/ui.xml").await?;

        // Pull the file
        let temp_path = std::env::temp_dir().join("lumi_ui_dump.xml");
        adb::pull(serial, "/sdcard/ui.xml", temp_path.to_str().unwrap()).await?;

        // Parse hierarchy
        let xml_content = std::fs::read_to_string(&temp_path)?;
        let elements = uiautomator::parse_hierarchy(&xml_content)?;

        let mut cache = self.ui_cache.lock().await;
        *cache = Some(elements);

        let mut last_dump = self.last_dump.lock().await;
        *last_dump = Instant::now();

        Ok(())
    }

    /// Get UI hierarchy, refreshing if stale (> 500ms old)
    async fn get_ui_hierarchy(&self) -> Result<Vec<UiElement>> {
        let last_dump = *self.last_dump.lock().await;
        if last_dump.elapsed() > Duration::from_millis(500) {
            self.refresh_ui_cache().await?;
        }

        let cache = self.ui_cache.lock().await;
        cache
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No UI hierarchy available"))
    }

    /// Find element at coordinates
    fn find_element_at(&self, elements: &[UiElement], x: i32, y: i32) -> Option<UiElement> {
        // Find the smallest (most specific) element containing the point
        let mut best: Option<&UiElement> = None;
        let mut best_area = i64::MAX;

        for el in elements {
            let bounds = &el.bounds;
            if x >= bounds.left && x <= bounds.right && y >= bounds.top && y <= bounds.bottom {
                let area =
                    (bounds.right - bounds.left) as i64 * (bounds.bottom - bounds.top) as i64;
                if area < best_area {
                    best_area = area;
                    best = Some(el);
                }
            }
        }

        best.cloned()
    }

    /// Record a tap at specific coordinates
    pub async fn record_tap(&self, x: i32, y: i32) -> Result<()> {
        let elements = self.get_ui_hierarchy().await?;

        if let Some(element) = self.find_element_at(&elements, x, y) {
            let scorer = SelectorScorer::new(self.screen_width, self.screen_height, elements);
            let selectors = scorer.score_element(&element);

            let best = selectors
                .first()
                .map(|s| s.short_repr())
                .unwrap_or_default();
            println!(
                "  📱 tap: {} (score: {})",
                best,
                selectors.first().map(|s| s.score).unwrap_or(0)
            );

            let mut actions = self.actions.lock().await;
            actions.push(RecordedAction::Tap {
                element,
                selectors,
                timestamp: Instant::now(),
            });
        } else {
            // No element found, record coordinate tap
            println!(
                "  📱 tap: point \"{}%,{}%\" (no element found)",
                (x as f64 / self.screen_width as f64 * 100.0).round() as u32,
                (y as f64 / self.screen_height as f64 * 100.0).round() as u32
            );
        }

        // Refresh UI after tap (state may have changed)
        self.refresh_ui_cache().await?;

        Ok(())
    }

    /// Record text input
    pub async fn record_input(&self, text: &str) -> Result<()> {
        let elements = self.get_ui_hierarchy().await?;

        // Find currently focused element
        let focused = elements.iter().find(|e| e.focusable && e.enabled);

        if let Some(element) = focused {
            let scorer =
                SelectorScorer::new(self.screen_width, self.screen_height, elements.clone());
            let selectors = scorer.score_element(element);

            // Mask sensitive data
            let display_text = if text.len() > 2
                && (text.to_lowercase().contains("pass")
                    || text.chars().all(|c| c.is_ascii_digit()))
            {
                "********".to_string()
            } else {
                text.to_string()
            };

            println!("  ⌨️  inputText: \"{}\"", display_text);

            let mut actions = self.actions.lock().await;
            actions.push(RecordedAction::Input {
                element: element.clone(),
                selectors,
                text: text.to_string(),
                timestamp: Instant::now(),
            });
        }

        Ok(())
    }

    /// Record a swipe gesture
    pub async fn record_swipe(&self, direction: &str) -> Result<()> {
        println!("  👆 swipe: {}", direction);

        let mut actions = self.actions.lock().await;
        actions.push(RecordedAction::Swipe {
            direction: direction.to_string(),
            timestamp: Instant::now(),
        });

        // Refresh UI after swipe
        self.refresh_ui_cache().await?;

        Ok(())
    }

    /// Poll for touch events using getevent
    /// This is a simplified polling approach that reads periodic UI state
    pub async fn poll_events(&self) -> Result<()> {
        let serial = self.serial.clone();
        let is_recording = self.is_recording.clone();
        let _actions = self.actions.clone();
        let screen_width = self.screen_width;
        let screen_height = self.screen_height;

        // Get touch input device
        let devices_output = adb::shell(serial.as_deref(), "getevent -pl").await?;
        let touch_device = Self::find_touch_device(&devices_output);

        if touch_device.is_none() {
            anyhow::bail!("No touch input device found");
        }
        let touch_device = touch_device.unwrap();

        println!("📲 Monitoring touch device: {}", touch_device);

        // Spawn getevent monitor
        let adb_path = binary_resolver::find_adb()?;
        let mut args = Vec::new();
        if let Some(ref s) = serial {
            args.push("-s".to_string());
            args.push(s.clone());
        }
        args.push("shell".to_string());
        args.push(format!("getevent -lt {}", touch_device));

        let mut child = Command::new(&adb_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to start getevent monitor")?;

        let stdout = child.stdout.take().unwrap();
        let reader = tokio::io::BufReader::new(stdout);

        let mut current_x: Option<i32> = None;
        let mut current_y: Option<i32> = None;
        let mut touch_down = false;
        let mut touch_down_time: Option<Instant> = None;

        use tokio::io::AsyncBufReadExt;
        let mut lines = reader.lines();

        loop {
            if !*is_recording.lock().await {
                break;
            }

            tokio::select! {
                line = lines.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            // Parse getevent output
                            // Format: [timestamp] /dev/input/eventX: EV_ABS ABS_MT_POSITION_X value
                            if line.contains("ABS_MT_POSITION_X") {
                                if let Some(val) = Self::parse_getevent_value(&line) {
                                    // Scale to screen coordinates (getevent uses raw touch panel coords)
                                    current_x = Some((val as f64 / 32767.0 * screen_width as f64) as i32);
                                }
                            } else if line.contains("ABS_MT_POSITION_Y") {
                                if let Some(val) = Self::parse_getevent_value(&line) {
                                    current_y = Some((val as f64 / 32767.0 * screen_height as f64) as i32);
                                }
                            } else if line.contains("BTN_TOUCH") && line.contains("DOWN") {
                                touch_down = true;
                                touch_down_time = Some(Instant::now());
                            } else if line.contains("BTN_TOUCH") && line.contains("UP") {
                                if touch_down {
                                    if let (Some(x), Some(y)) = (current_x, current_y) {
                                        // Check if it was a long press
                                        let duration = touch_down_time
                                            .map(|t| t.elapsed().as_millis())
                                            .unwrap_or(0);

                                        if duration > 500 {
                                            println!("  📱 longPress detected at ({}, {})", x, y);
                                        } else {
                                            // This is where we'd call record_tap
                                            // But we can't call async from here easily
                                            // So we'll log it for now
                                            let pct_x = (x as f64 / screen_width as f64 * 100.0).round();
                                            let pct_y = (y as f64 / screen_height as f64 * 100.0).round();
                                            println!("  📱 tap detected at ({}, {}) = {}%,{}%", x, y, pct_x, pct_y);
                                        }
                                    }
                                    touch_down = false;
                                    current_x = None;
                                    current_y = None;
                                }
                            }
                        }
                        Ok(None) => break, // EOF
                        Err(_) => break,
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Periodic check
                }
            }
        }

        // Kill the getevent process
        let _ = child.kill().await;

        Ok(())
    }

    /// Find the primary touch input device
    fn find_touch_device(getevent_output: &str) -> Option<String> {
        let mut current_device: Option<String> = None;

        for line in getevent_output.lines() {
            if line.starts_with("add device") {
                // Extract device path: add device 1: /dev/input/event5
                if let Some(path_start) = line.find("/dev/input/") {
                    current_device = Some(line[path_start..].trim().to_string());
                }
            } else if line.contains("ABS_MT_POSITION_X") || line.contains("ABS_MT_TOUCH") {
                // This device supports multi-touch, it's likely the main touchscreen
                if let Some(ref device) = current_device {
                    return Some(device.clone());
                }
            }
        }

        None
    }

    /// Parse a value from getevent output line
    fn parse_getevent_value(line: &str) -> Option<i32> {
        // Format: [timestamp] device: TYPE CODE value
        let parts: Vec<&str> = line.split_whitespace().collect();
        if let Some(last) = parts.last() {
            // Value might be in hex (0x...) or decimal
            if last.starts_with("0x") || last.starts_with("0X") {
                i32::from_str_radix(&last[2..], 16).ok()
            } else {
                last.parse().ok()
            }
        } else {
            None
        }
    }

    /// Get all recorded actions
    pub async fn get_actions(&self) -> Vec<RecordedAction> {
        self.actions.lock().await.clone()
    }

    /// Get current app ID
    pub async fn get_current_app(&self) -> Option<String> {
        self.current_app.lock().await.clone()
    }
}
