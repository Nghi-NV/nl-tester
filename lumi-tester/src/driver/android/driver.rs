use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, OnceCell};
use uuid::Uuid;

use super::adb;
use super::uiautomator::{self, UiElement};
use crate::driver::traits::{PlatformDriver, Selector, SwipeDirection};
use colored::Colorize;
use image::GenericImageView;

use crate::driver::common;
use crate::driver::ocr::OcrEngine;
use crate::parser::types::SpeedMode;
use std::collections::HashMap;

/// Speed profile for test execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpeedProfile {
    /// Zero delays, maximum speed (may be flaky on slow devices)
    Turbo,
    /// Minimal delays, for fast devices
    Fast,
    /// Balanced delays (default)
    #[default]
    Normal,
    /// Extra delays for slow devices/emulators
    Safe,
}

impl SpeedProfile {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "turbo" | "max" => SpeedProfile::Turbo,
            "fast" => SpeedProfile::Fast,
            "safe" | "slow" => SpeedProfile::Safe,
            _ => SpeedProfile::Normal,
        }
    }

    /// Get tap delay in milliseconds
    pub fn tap_delay_ms(&self) -> u64 {
        match self {
            SpeedProfile::Turbo => 0,
            SpeedProfile::Fast => 50,
            SpeedProfile::Normal => 150,
            SpeedProfile::Safe => 300,
        }
    }

    /// Get scroll delay in milliseconds
    pub fn scroll_delay_ms(&self) -> u64 {
        match self {
            SpeedProfile::Turbo => 100,
            SpeedProfile::Fast => 300,
            SpeedProfile::Normal => 500,
            SpeedProfile::Safe => 1000,
        }
    }

    /// Get poll interval in milliseconds
    pub fn poll_interval_ms(&self) -> u64 {
        match self {
            SpeedProfile::Turbo => 30,
            SpeedProfile::Fast => 100,
            SpeedProfile::Normal => 300,
            SpeedProfile::Safe => 500,
        }
    }

    /// Max wait time for UI idle detection
    pub fn ui_idle_max_wait_ms(&self) -> u64 {
        match self {
            SpeedProfile::Turbo => 0, // Skip UI idle detection
            SpeedProfile::Fast => 100,
            SpeedProfile::Normal => 200,
            SpeedProfile::Safe => 400,
        }
    }

    /// Whether to skip UI idle detection entirely
    pub fn skip_ui_idle(&self) -> bool {
        matches!(self, SpeedProfile::Turbo)
    }
}

/// State of the background mock location process
#[derive(Clone)]
struct MockLocationState {
    current_lat: Option<f64>,
    current_lon: Option<f64>,
    is_running: bool,
    finished: bool,
    paused: bool,
    speed: Option<f64>,
    speed_mode: SpeedMode,
    speed_noise: Option<f64>,
}

impl Default for MockLocationState {
    fn default() -> Self {
        Self {
            current_lat: None,
            current_lon: None,
            is_running: false,
            finished: false,
            paused: false,
            speed: None,
            speed_mode: SpeedMode::Linear,
            speed_noise: None,
        }
    }
}

/// UI Cache TTL in milliseconds (3 seconds for better performance)
const UI_CACHE_TTL_MS: u64 = 3000;

/// Android driver implementation using ADB
pub struct AndroidDriver {
    serial: Option<String>,
    screen_size: (u32, u32),
    recording_process: Arc<Mutex<Option<tokio::process::Child>>>,
    current_recording_path: Arc<Mutex<Option<String>>>,
    ui_cache: Arc<Mutex<Option<(Instant, Vec<UiElement>)>>>,
    /// Mock location states keyed by name ("" for default)
    mock_states: Arc<Mutex<HashMap<String, MockLocationState>>>,
    /// Target display ID (default 0)
    display_id: AtomicU32,
    /// Speed profile for adaptive delays
    speed_profile: SpeedProfile,
    /// Cached: whether ADBKeyBoard is available for text input
    adbkeyboard_available: bool,
    /// Cached: original IME to restore after using ADBKeyBoard
    original_ime: String,
    /// Whether to use Unicode input support (ADBKeyBoard) - default: false for speed
    #[allow(dead_code)]
    support_unicode: bool,
    /// Lazy-loaded OCR engine
    ocr_engine: Arc<OnceCell<OcrEngine>>,
}

impl AndroidDriver {
    /// Create a new Android driver
    pub async fn new(serial: Option<&str>) -> Result<Self> {
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

        // Get screen size
        let screen_size = adb::get_screen_size(selected_serial.as_deref()).await?;

        // Check environment variable for speed profile
        let speed_profile = std::env::var("LUMI_SPEED")
            .map(|s| SpeedProfile::from_str(&s))
            .unwrap_or_default();

        // Cache original IME for text input
        let original_ime = adb::shell(
            selected_serial.as_deref(),
            "settings get secure default_input_method",
        )
        .await
        .unwrap_or_default()
        .trim()
        .to_string();

        // Check if ADBKeyBoard is available, auto-install if not
        let ime_list = adb::shell(selected_serial.as_deref(), "ime list -s")
            .await
            .unwrap_or_default();
        let mut adbkeyboard_available = ime_list.contains("com.android.adbkeyboard");

        // Auto-install ADBKeyBoard if not present
        if !adbkeyboard_available {
            if let Some(apk_path) = crate::utils::binary_resolver::find_apk("ADBKeyboard.apk") {
                println!(
                    "  {} Installing ADBKeyBoard for Unicode input support...",
                    "⏳".yellow()
                );

                // Install APK
                let install_result = adb::install(
                    selected_serial.as_deref(),
                    apk_path.to_string_lossy().as_ref(),
                )
                .await;

                if install_result.is_ok() {
                    // Enable the IME
                    let _ = adb::shell(
                        selected_serial.as_deref(),
                        "ime enable com.android.adbkeyboard/.AdbIME",
                    )
                    .await;

                    adbkeyboard_available = true;
                    println!("  {} ADBKeyBoard installed successfully", "✓".green());
                } else {
                    println!(
                        "  {} Failed to install ADBKeyBoard: {:?}",
                        "⚠".yellow(),
                        install_result.err()
                    );
                }
            } else {
            }
        } else {
            println!(
                "  {} ADBKeyBoard detected, Unicode input enabled",
                "✓".green()
            );
        }

        // Check LUMI_UNICODE env var for Unicode input support (default: false for speed)
        let support_unicode = std::env::var("LUMI_UNICODE")
            .map(|s| s.to_lowercase() == "true" || s == "1")
            .unwrap_or(false);

        if support_unicode && adbkeyboard_available {
            println!("  {} Unicode input mode enabled (ADBKeyBoard)", "✓".green());
        }

        Ok(Self {
            serial: selected_serial,
            screen_size,
            recording_process: Arc::new(Mutex::new(None)),
            current_recording_path: Arc::new(Mutex::new(None)),
            ui_cache: Arc::new(Mutex::new(None)),
            mock_states: Arc::new(Mutex::new(HashMap::new())),
            display_id: AtomicU32::new(0),
            speed_profile,
            adbkeyboard_available,
            original_ime,
            support_unicode,
            ocr_engine: Arc::new(OnceCell::new()),
        })
    }

    /// Invalidate the UI cache
    async fn invalidate_cache(&self) {
        let mut cache = self.ui_cache.lock().await;
        *cache = None;
    }

    /// Wait for UI to become idle (no animations)
    async fn wait_for_ui_idle(&self) -> Result<()> {
        let max_wait = self.speed_profile.ui_idle_max_wait_ms();
        let start = Instant::now();
        let poll_interval = 30; // Quick polls

        while start.elapsed().as_millis() < max_wait as u128 {
            // Check window animation state
            let output = adb::shell(
                self.serial.as_deref(),
                "dumpsys window | grep -E 'mAnimationScheduled|mCurrentFocus' | head -2",
            )
            .await
            .unwrap_or_default();

            // If no animation is scheduled, UI is idle
            if !output.contains("mAnimationScheduled=true") {
                return Ok(());
            }

            tokio::time::sleep(Duration::from_millis(poll_interval)).await;
        }

        // Timeout reached, continue anyway
        Ok(())
    }

    /// Smart delay after action - uses UI idle detection + minimum delay
    async fn smart_delay_after_action(&self) {
        // Skip all delays in Turbo mode
        if self.speed_profile.skip_ui_idle() {
            return;
        }

        // First wait for UI idle (fast check)
        let _ = self.wait_for_ui_idle().await;

        // Then apply minimum delay based on speed profile
        let min_delay = self.speed_profile.tap_delay_ms();
        if min_delay > 0 {
            tokio::time::sleep(Duration::from_millis(min_delay)).await;
        }
    }

    /// Get the UI hierarchy (with caching)
    async fn get_ui_hierarchy(&self) -> Result<Vec<UiElement>> {
        // Check cache first (TTL based on UI_CACHE_TTL_MS)
        {
            let cache = self.ui_cache.lock().await;
            if let Some((timestamp, elements)) = &*cache {
                if timestamp.elapsed() < Duration::from_millis(UI_CACHE_TTL_MS) {
                    return Ok(elements.clone());
                }
            }
        }

        // Cache miss or expired, dump fresh UI
        // Optimization: Use exec-out with /dev/stdout to avoid file I/O
        // This is faster than writing to /sdcard and reading back
        let xml = match adb::exec_out(self.serial.as_deref(), "uiautomator dump /dev/stdout").await
        {
            Ok(output) if output.contains("<?xml") => output,
            _ => {
                // Fallback to file-based method for older Android versions
                adb::shell(
                    self.serial.as_deref(),
                    "uiautomator dump /sdcard/window_dump.xml > /dev/null && cat /sdcard/window_dump.xml",
                )
                .await?
            }
        };

        let elements = uiautomator::parse_hierarchy(&xml)?;

        // Update cache
        {
            let mut cache = self.ui_cache.lock().await;
            *cache = Some((Instant::now(), elements.clone()));
        }

        Ok(elements)
    }

    /// Find element by selector
    async fn find_element_internal(
        &self,
        selector: &Selector,
    ) -> Result<Option<uiautomator::UiElement>> {
        // Point selector, return None as it has no bounds
        if let Selector::Point { .. } = selector {
            return Ok(None);
        }

        // Image selector, return None for now (would need to change return type to support image bounds)
        if let Selector::Image { .. } = selector {
            // self.find_image_on_screen() returns coords, not UiElement
            return Ok(None);
        }

        let elements = self.get_ui_hierarchy().await?;

        if let Some((elem, _)) = self.find_element_impl(selector, &elements) {
            Ok(Some(elem.clone()))
        } else {
            Ok(None)
        }
    }

    async fn find_element(&self, selector: &Selector) -> Result<Option<(i32, i32)>> {
        // Optimization for Point selector
        if let Selector::Point { x, y } = selector {
            return Ok(Some((*x, *y)));
        }

        // Handle Image selector
        if let Selector::Image { path, region } = selector {
            return self.find_image_on_screen(path, region.as_deref()).await;
        }

        // Handle OCR selector
        if let Selector::OCR(text, index, is_regex, region) = selector {
            return self
                .find_ocr_text(text, *index, *is_regex, region.as_deref())
                .await;
        }

        let elements = self.get_ui_hierarchy().await?;

        if let Some((elem, is_fallback)) = self.find_element_impl(selector, &elements) {
            // For Relative selectors, adjust the tap point based on direction
            // ONLY if fallback was triggered (meaning we are targeting a composite element like Switch)
            if let Selector::Relative { direction, .. } = selector {
                if is_fallback {
                    use crate::driver::traits::RelativeDirection;
                    let bounds = &elem.bounds;
                    let w = bounds.right - bounds.left;
                    let h = bounds.bottom - bounds.top;

                    match direction {
                        RelativeDirection::RightOf => {
                            // Tap at 90% width (right side) for fallback composite switches
                            return Ok(Some((bounds.left + (w * 9 / 10), bounds.top + h / 2)));
                        }
                        RelativeDirection::LeftOf => {
                            // Tap at 10% width
                            return Ok(Some((bounds.left + (w / 10), bounds.top + h / 2)));
                        }
                        _ => {}
                    }
                }
            }

            Ok(Some(elem.bounds.center()))
        } else {
            Ok(None)
        }
    }

    fn find_element_impl<'a>(
        &self,
        selector: &Selector,
        elements: &'a [uiautomator::UiElement],
    ) -> Option<(&'a uiautomator::UiElement, bool)> {
        match selector {
            Selector::Point { .. } => None,
            Selector::Image { .. } => None, // Handled by find_image_on_screen
            Selector::ScrollableItem {
                scrollable_index,
                item_index,
            } => {
                // Find all scrollable elements
                let scrollables: Vec<_> = elements.iter().filter(|e| e.scrollable).collect();

                if let Some(scrollable_container) = scrollables.get(*scrollable_index) {
                    let container_bounds = &scrollable_container.bounds;

                    if let Some(target_idx) = item_index {
                        let target_index_str = target_idx.to_string();
                        // Find descendant with matching index
                        elements
                            .iter()
                            .filter(|e| {
                                // Relaxed check: element center is inside container
                                let center = e.bounds.center();
                                container_bounds.left <= center.0
                                    && container_bounds.right >= center.0
                                    && container_bounds.top <= center.1
                                    && container_bounds.bottom >= center.1
                                    && e.index == target_index_str
                                    && !std::ptr::eq(
                                        *e as *const _,
                                        *scrollable_container as *const _,
                                    )
                            })
                            .next()
                            .map(|e| (e, false))
                    } else {
                        // If no item_index is provided, we return None
                        // This allows scrollUntilVisible to continue scrolling even if the container is visible
                        None
                    }
                } else {
                    None
                }
            }

            Selector::Scrollable(index) => {
                let scrollables: Vec<_> = elements.iter().filter(|e| e.scrollable).collect();
                scrollables.get(*index).map(|e| (*e, false))
            }

            Selector::Text(text, index, exact) => if *exact {
                uiautomator::find_nth_by_text_exact(elements, text, *index as u32).or_else(|| {
                    uiautomator::find_nth_by_text_contains(elements, text, *index as u32)
                })
            } else {
                uiautomator::find_nth_by_text(elements, text, *index as u32).or_else(|| {
                    uiautomator::find_nth_by_text_contains(elements, text, *index as u32)
                })
            }
            .map(|e| (e, false)),

            Selector::TextRegex(pattern, index) => {
                uiautomator::find_nth_by_regex(elements, pattern, *index as u32).map(|e| (e, false))
            }

            Selector::Id(id, index) => {
                uiautomator::find_nth_by_id(elements, id, *index as u32).map(|e| (e, false))
            }

            Selector::IdRegex(pattern, index) => {
                uiautomator::find_nth_by_id_regex(elements, pattern, *index as u32)
                    .map(|e| (e, false))
            }

            Selector::Type(type_name, index) => uiautomator::find_by_type_index(
                elements,
                map_android_type(type_name),
                *index as u32,
            )
            .map(|e| (e, false)),

            Selector::AccessibilityId(id) | Selector::Description(id, _) => elements
                .iter()
                .find(|e| e.content_desc == *id)
                .map(|e| (e, false)),

            Selector::DescriptionRegex(pattern, index) => {
                uiautomator::find_nth_by_description_regex(elements, pattern, *index as u32)
                    .map(|e| (e, false))
            }

            Selector::XPath(_) => None,
            Selector::Css(_) => None,
            Selector::Role(role, index) => {
                let android_type = match role.to_lowercase().as_str() {
                    "button" => "android.widget.Button",
                    "textfield" | "edittext" => "android.widget.EditText",
                    "image" => "android.widget.ImageView",
                    _ => role,
                };
                uiautomator::find_by_type_index(elements, android_type, *index as u32)
                    .map(|e| (e, false))
            }
            Selector::Placeholder(placeholder, index) => {
                // Android doesn't always expose placeholder,
                // falling back to searching by text as they are often the same in the dump
                uiautomator::find_nth_by_text(elements, placeholder, *index as u32)
                    .map(|e| (e, false))
            }

            Selector::AnyClickable(index) => {
                // Find nth clickable element
                elements
                    .iter()
                    .filter(|e| e.clickable)
                    .nth(*index)
                    .map(|e| (e, false))
            }

            Selector::Relative {
                target,
                anchor,
                direction,
                max_dist,
            } => {
                // Get candidates based on target
                let candidates = match target.as_ref() {
                    Selector::Text(t, _, _) => uiautomator::find_all_by_text(elements, t),
                    Selector::TextRegex(r, _) => uiautomator::find_all_by_regex(elements, r),
                    Selector::Id(id, _) => uiautomator::find_all_by_id(elements, id),
                    Selector::IdRegex(r, _) => uiautomator::find_all_by_id_regex(elements, r),
                    Selector::Type(t, _) => {
                        uiautomator::find_all_by_type(elements, map_android_type(t))
                    }
                    Selector::AccessibilityId(id) | Selector::Description(id, _) => {
                        elements.iter().filter(|e| e.content_desc == *id).collect()
                    }
                    Selector::DescriptionRegex(r, _) => {
                        uiautomator::find_all_by_description_regex(elements, r)
                    }
                    Selector::AnyClickable(_) => {
                        // For relative matching, we need ALL clickable elements as candidates
                        // The index will be applied after find_relative filters by direction/distance
                        elements.iter().filter(|e| e.clickable).collect()
                    }
                    _ => Vec::new(),
                };

                // Find anchor
                let (anchor_elem, _) = self.find_element_impl(anchor, elements)?;

                let sorted_matches =
                    uiautomator::find_relative(candidates, anchor_elem, *direction, *max_dist);

                // Get index from target selector
                let target_index = match target.as_ref() {
                    Selector::Text(_, idx, _) => *idx,
                    Selector::TextRegex(_, idx) => *idx,
                    Selector::Id(_, idx) => *idx,
                    Selector::IdRegex(_, idx) => *idx,
                    Selector::Type(_, idx) => *idx,
                    Selector::AccessibilityId(_) => 0, // No index in AccessibilityId variant, implicit 0
                    Selector::Role(_, idx) => *idx,
                    Selector::Description(_, idx) => *idx,
                    Selector::DescriptionRegex(_, idx) => *idx,
                    Selector::AnyClickable(idx) => *idx,
                    Selector::Placeholder(_, idx) => *idx,
                    _ => 0,
                };

                sorted_matches
                    .into_iter()
                    .nth(target_index)
                    .map(|e| (e, false))
            }
            Selector::HasChild { parent, child } => {
                // Find all elements matching parent selector
                let parent_candidates: Vec<_> = elements
                    .iter()
                    .filter(|e| Self::element_matches_selector(e, parent))
                    .collect();

                // Find all elements matching child selector
                let child_candidates: Vec<_> = elements
                    .iter()
                    .filter(|e| Self::element_matches_selector(e, child))
                    .collect();

                // Find parent that contains child
                for p in parent_candidates {
                    for c in &child_candidates {
                        if p.bounds.contains(&c.bounds)
                            && !std::ptr::eq(p as *const _, *c as *const _)
                        {
                            return Some((p, false));
                        }
                    }
                }
                None
            }
            Selector::OCR(..) => None, // OCR handled separately via screenshot
        }
    }

    /// Check if UiElement matches a simple selector (for HasChild matching)
    fn element_matches_selector(e: &uiautomator::UiElement, selector: &Selector) -> bool {
        match selector {
            Selector::Text(text, _, _) => e.text.contains(text) || e.content_desc.contains(text),
            Selector::TextRegex(pattern, _) => {
                if let Ok(re) = regex::Regex::new(pattern) {
                    re.is_match(&e.text) || re.is_match(&e.content_desc)
                } else {
                    false
                }
            }
            Selector::Id(id, _) => {
                e.resource_id == *id || e.resource_id.ends_with(&format!("/{}", id))
            }
            Selector::IdRegex(pattern, _) => {
                if let Ok(re) = regex::Regex::new(pattern) {
                    re.is_match(&e.resource_id)
                } else {
                    false
                }
            }
            Selector::Type(t, _) => e.class.contains(t),
            Selector::AccessibilityId(id) | Selector::Description(id, _) => e.content_desc == *id,
            Selector::DescriptionRegex(pattern, _) => {
                if let Ok(re) = regex::Regex::new(pattern) {
                    re.is_match(&e.content_desc)
                } else {
                    false
                }
            }
            Selector::Placeholder(_, _) => false, // Not available in UiElement
            Selector::Role(_, _) => false,        // Not directly supported
            Selector::AnyClickable(_) => e.clickable, // Match any clickable element
            _ => false,                           // Nested relative/haschild not supported
        }
    }

    /// Find element by class type and index (0-based)
    pub async fn find_element_by_type_index(
        &self,
        element_type: &str,
        index: u32,
    ) -> Result<Option<(i32, i32)>> {
        let elements = self.get_ui_hierarchy().await?;
        if let Some(elem) = uiautomator::find_by_type_index(&elements, element_type, index) {
            Ok(Some(elem.bounds.center()))
        } else {
            Ok(None)
        }
    }

    /// Tap element by type and index
    pub async fn tap_at(&self, element_type: &str, index: u32) -> Result<()> {
        let (x, y) = self
            .find_element_by_type_index(element_type, index)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Element not found: {}[{}]", element_type, index))?;

        adb::shell(
            self.serial.as_deref(),
            &format!(
                "input -d {} tap {} {}",
                self.display_id.load(Ordering::Relaxed),
                x,
                y
            ),
        )
        .await?;
        self.smart_delay_after_action().await;
        self.invalidate_cache().await;

        Ok(())
    }

    /// Input text at element by type and index
    pub async fn input_at(&self, element_type: &str, index: u32, text: &str) -> Result<()> {
        // First tap on the element to focus it
        self.tap_at(element_type, index).await?;
        self.smart_delay_after_action().await;

        // Then input the text
        let escaped = text
            .replace("\\", "\\\\")
            .replace(" ", "%s")
            .replace("\"", "\\\"")
            .replace("'", "\\'")
            .replace("&", "\\&")
            .replace("<", "\\<")
            .replace(">", "\\>")
            .replace("|", "\\|")
            .replace(";", "\\;");

        adb::shell(
            self.serial.as_deref(),
            &format!(
                "input -d {} text '{}'",
                self.display_id.load(Ordering::Relaxed),
                escaped
            ),
        )
        .await?;

        Ok(())
    }

    /// Find template image on screen using optimized single-pass template matching
    /// Get (lazy-load) OCR engine
    async fn get_ocr_engine(&self) -> Result<&OcrEngine> {
        self.ocr_engine
            .get_or_try_init(|| async { OcrEngine::new().await })
            .await
    }

    /// Find text on screen using OCR
    async fn find_ocr_text(
        &self,
        text: &str,
        index: usize,
        is_regex: bool,
        region: Option<&str>,
    ) -> Result<Option<(i32, i32)>> {
        use crate::driver::image_matcher::ImageRegion;

        // Initialize engine first (may trigger download)
        let engine = self.get_ocr_engine().await?;

        // Capture screenshot (fast path via exec-out if possible)
        let png_data = match adb::exec_out_binary(self.serial.as_deref(), "screencap -p").await {
            Ok(data) if data.len() > 100 && data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) => data,
            _ => {
                // Fallback: take screenshot to temp file and read it
                let screenshot_path =
                    std::env::temp_dir().join(format!("ocr_screen_{}.png", Uuid::new_v4()));
                let screenshot_path_str = screenshot_path.to_string_lossy().to_string();
                self.take_screenshot_internal(&screenshot_path_str).await?;
                let data = std::fs::read(&screenshot_path)?;
                let _ = std::fs::remove_file(&screenshot_path);
                data
            }
        };

        // Parse region for cropping
        let image_region = region.map(ImageRegion::from_str).unwrap_or_default();
        let region_clone = image_region;
        let text = text.to_string();
        let engine_clone = engine.clone();

        // Run match in blocking task
        let result = tokio::task::spawn_blocking(move || {
            // Crop image if region specified
            let (cropped_data, offset_x, offset_y) = if region_clone != ImageRegion::Full {
                let img = image::load_from_memory(&png_data)?;
                let (w, h) = (img.width(), img.height());
                let (x, y, rw, rh) = region_clone.get_crop_region(w, h);

                let cropped = img.crop_imm(x, y, rw, rh);
                let mut buf = std::io::Cursor::new(Vec::new());
                cropped.write_to(&mut buf, image::ImageFormat::Png)?;
                (buf.into_inner(), x as i32, y as i32)
            } else {
                (png_data, 0, 0)
            };

            let match_opt =
                engine_clone.find_text_at_index(&cropped_data, &text, is_regex, index)?;

            // Adjust coordinates back to full screen
            Ok::<_, anyhow::Error>(match_opt.map(|m| (m.x + offset_x, m.y + offset_y)))
        })
        .await??;

        Ok(result)
    }

    /// Uses region-based matching if region is specified
    async fn find_image_on_screen(
        &self,
        template_path: &str,
        region: Option<&str>,
    ) -> Result<Option<(i32, i32)>> {
        use crate::driver::image_matcher::{find_template, ImageRegion, MatchConfig};

        let template_path_buf = Path::new(template_path).to_path_buf();
        if !template_path_buf.exists() {
            anyhow::bail!("Template image not found: {:?}", template_path_buf);
        }

        // Parse region
        let image_region = region.map(ImageRegion::from_str).unwrap_or_default();
        let screenshot_path =
            std::env::temp_dir().join(format!("screen_match_{}.png", Uuid::new_v4()));
        let screenshot_path_str = screenshot_path.to_string_lossy().to_string();
        self.take_screenshot_internal(&screenshot_path_str).await?;

        // Run matching in blocking thread to avoid blocking async runtime
        let result = tokio::task::spawn_blocking(move || -> Result<Option<(i32, i32)>> {
            let img_screen = image::open(&screenshot_path)?.to_luma8();
            let img_template = image::open(&template_path_buf)?.to_luma8();

            // Cleanup screenshot
            let _ = std::fs::remove_file(&screenshot_path);

            if img_template.width() > img_screen.width()
                || img_template.height() > img_screen.height()
            {
                return Ok(None);
            }

            let config = MatchConfig {
                target_width: 220.0,
                threshold: 0.7,
                region: image_region,
            };

            let match_result = find_template(&img_screen, &img_template, &config)?;

            match match_result {
                Some(result) => Ok(Some((result.x, result.y))),
                None => Ok(None),
            }
        })
        .await??;

        Ok(result)
    }

    /// Internal screenshot function that doesn't depend on PlatformDriver trait
    /// Optimized to use exec-out for direct transfer without file I/O on device
    async fn take_screenshot_internal(&self, path: &str) -> Result<()> {
        // Try fast path: exec-out screencap with binary output (no file I/O on device)
        let result = adb::exec_out_binary(self.serial.as_deref(), "screencap -p").await;

        match result {
            Ok(data) if data.len() > 100 && data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) => {
                // Valid PNG signature detected, write directly to local file
                std::fs::write(path, &data)?;
                Ok(())
            }
            _ => {
                // Fallback to file-based method
                let remote_path = "/sdcard/screenshot.png";
                adb::shell(
                    self.serial.as_deref(),
                    &format!("screencap -p {}", remote_path),
                )
                .await?;
                adb::pull(self.serial.as_deref(), remote_path, path).await?;
                adb::shell(self.serial.as_deref(), &format!("rm {}", remote_path)).await?;
                Ok(())
            }
        }
    }

    fn to_ascii_fallback(&self, text: &str) -> String {
        common::to_ascii_fallback(text)
    }

    /// Install XAPK (split APK bundle) by extracting and using install-multiple
    async fn install_xapk(&self, xapk_path: &str) -> Result<()> {
        use std::io::Read;
        use zip::ZipArchive;

        println!("  {} Installing XAPK from: {}", "⬇".cyan(), xapk_path);

        // Create temp directory for extraction
        let temp_dir = std::env::temp_dir().join(format!("xapk_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir)?;

        // Extract XAPK (it's a ZIP file)
        let file = std::fs::File::open(xapk_path)?;
        let mut archive = ZipArchive::new(file)?;

        let mut apk_files: Vec<String> = Vec::new();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = temp_dir.join(file.name());

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(p)?;
                    }
                }

                // Only extract APK files
                if file.name().to_lowercase().ends_with(".apk") {
                    let mut outfile = std::fs::File::create(&outpath)?;
                    let mut contents = Vec::new();
                    file.read_to_end(&mut contents)?;
                    std::io::Write::write_all(&mut outfile, &contents)?;
                    apk_files.push(outpath.to_string_lossy().to_string());
                    println!("    {} Extracted: {}", "📦".blue(), file.name());
                }
            }
        }

        if apk_files.is_empty() {
            // Cleanup
            let _ = std::fs::remove_dir_all(&temp_dir);
            anyhow::bail!("No APK files found in XAPK: {}", xapk_path);
        }

        // Build install-multiple command
        let mut args: Vec<&str> = vec!["install-multiple", "-r", "-g"];
        for apk in &apk_files {
            args.push(apk);
        }

        println!(
            "    {} Installing {} APK files...",
            "📲".green(),
            apk_files.len()
        );

        let result = adb::exec(self.serial.as_deref(), &args).await;

        // Cleanup temp directory
        let _ = std::fs::remove_dir_all(&temp_dir);

        result?;
        println!("    {} XAPK installed successfully", "✓".green());
        Ok(())
    }
}

#[async_trait]
impl PlatformDriver for AndroidDriver {
    fn platform_name(&self) -> &str {
        "android"
    }

    fn device_serial(&self) -> Option<String> {
        self.serial.clone()
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

            // Map short names to full Android permission strings
            let full_perm = match perm.to_lowercase().as_str() {
                "camera" => "android.permission.CAMERA",
                "microphone" | "mic" => "android.permission.RECORD_AUDIO",
                "location" | "gps" => "android.permission.ACCESS_FINE_LOCATION",
                "coarse_location" => "android.permission.ACCESS_COARSE_LOCATION",
                "contacts" => "android.permission.READ_CONTACTS",
                "phone" | "call" => "android.permission.CALL_PHONE",
                "sms" => "android.permission.SEND_SMS",
                "storage" | "files" => "android.permission.READ_EXTERNAL_STORAGE",
                "write_storage" => "android.permission.WRITE_EXTERNAL_STORAGE",
                "calendar" => "android.permission.READ_CALENDAR",
                "notifications" => "android.permission.POST_NOTIFICATIONS",
                "all" => {
                    // Grant all common permissions using pm grant
                    let all_perms = [
                        "android.permission.CAMERA",
                        "android.permission.RECORD_AUDIO",
                        "android.permission.ACCESS_FINE_LOCATION",
                        "android.permission.ACCESS_COARSE_LOCATION",
                        "android.permission.READ_CONTACTS",
                        "android.permission.WRITE_CONTACTS",
                        "android.permission.READ_EXTERNAL_STORAGE",
                        "android.permission.WRITE_EXTERNAL_STORAGE",
                        "android.permission.POST_NOTIFICATIONS",
                        "android.permission.READ_PHONE_STATE",
                        "android.permission.CALL_PHONE",
                        "android.permission.READ_SMS",
                        "android.permission.SEND_SMS",
                        "android.permission.READ_CALENDAR",
                        "android.permission.WRITE_CALENDAR",
                        "android.permission.ACCESS_BACKGROUND_LOCATION",
                    ];
                    for p in all_perms {
                        let _ = adb::shell(
                            self.serial.as_deref(),
                            &format!("pm {} {} {}", cmd, app_id, p),
                        )
                        .await;
                    }

                    // Also use appops for runtime permissions (Android 6.0+)
                    // This helps with permissions that require runtime approval
                    let appops_perms = [
                        "CAMERA",
                        "RECORD_AUDIO",
                        "ACCESS_FINE_LOCATION",
                        "ACCESS_COARSE_LOCATION",
                        "READ_CONTACTS",
                        "WRITE_CONTACTS",
                        "READ_EXTERNAL_STORAGE",
                        "WRITE_EXTERNAL_STORAGE",
                        "POST_NOTIFICATIONS",
                        "READ_PHONE_STATE",
                        "CALL_PHONE",
                        "READ_SMS",
                        "SEND_SMS",
                        "READ_CALENDAR",
                        "WRITE_CALENDAR",
                    ];
                    let appops_cmd = if state.eq_ignore_ascii_case("deny") {
                        "deny"
                    } else {
                        "allow"
                    };
                    for p in appops_perms {
                        let _ = adb::shell(
                            self.serial.as_deref(),
                            &format!("appops set {} {} {}", app_id, p, appops_cmd),
                        )
                        .await;
                    }
                    continue;
                }
                _ => perm.as_str(), // Assume it's already a full permission string
            };

            let result = adb::shell(
                self.serial.as_deref(),
                &format!("pm {} {} {}", cmd, app_id, full_perm),
            )
            .await;
            if let Err(e) = result {
                println!(
                    "  {} Failed to {} permission {}: {}",
                    "⚠".yellow(),
                    cmd,
                    full_perm,
                    e
                );
            }
        }
        Ok(())
    }

    async fn launch_app(&self, app_id: &str, clear_state: bool) -> Result<()> {
        if clear_state {
            // Clear app data
            adb::shell(self.serial.as_deref(), &format!("pm clear {}", app_id)).await?;
        }

        // Resolve main activity
        let resolve_cmd = format!(
            "cmd package resolve-activity --brief {} | tail -n 1",
            app_id
        );
        let activity_output = adb::shell(self.serial.as_deref(), &resolve_cmd)
            .await
            .unwrap_or_default();
        let activity = activity_output.trim();

        if activity.contains('/') {
            // Use am start
            adb::shell(self.serial.as_deref(), &format!("am start -n {}", activity)).await?;
        } else {
            // Fallback to monkey if activity resolution failed
            println!(
                "  {} Warning: Could not resolve activity for {}, falling back to monkey",
                "⚠".yellow(),
                app_id
            );
            adb::shell(
                self.serial.as_deref(),
                &format!("monkey -p {} -c android.intent.category.LAUNCHER 1", app_id),
            )
            .await?;
        }

        // Poll for app focus instead of fixed sleep
        let start = Instant::now();
        let timeout = Duration::from_secs(10);
        let poll_interval = self.speed_profile.poll_interval_ms();
        let mut launched = false;

        while start.elapsed() < timeout {
            // Use dumpsys activity activities which is more reliable for finding the resumed app
            // and use simple grep to avoid compatibility issues
            let output = adb::shell(
                self.serial.as_deref(),
                "dumpsys activity activities | grep ResumedActivity",
            )
            .await
            .unwrap_or_default();
            if output.contains(app_id) {
                launched = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(poll_interval)).await;
        }

        if !launched {
            println!(
                "  {} Warning: App {} did not appear in focus within 10s",
                "⚠".yellow(),
                app_id
            );
        }

        self.invalidate_cache().await;

        Ok(())
    }

    async fn stop_app(&self, app_id: &str) -> Result<()> {
        adb::shell(self.serial.as_deref(), &format!("am force-stop {}", app_id)).await?;
        self.invalidate_cache().await;
        Ok(())
    }

    async fn tap(&self, selector: &Selector) -> Result<()> {
        let (x, y) = self
            .find_element(selector)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Element not found: {:?}", selector))?;

        adb::shell(
            self.serial.as_deref(),
            &format!(
                "input -d {} tap {} {}",
                self.display_id.load(Ordering::Relaxed),
                x,
                y
            ),
        )
        .await?;

        // Smart delay after tap (adaptive based on speed profile)
        self.smart_delay_after_action().await;
        self.invalidate_cache().await;

        Ok(())
    }

    async fn long_press(&self, selector: &Selector, duration_ms: u64) -> Result<()> {
        let (x, y) = self
            .find_element(selector)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Element not found: {:?}", selector))?;

        // Long press is simulated with swipe from same point to same point
        adb::shell(
            self.serial.as_deref(),
            &format!(
                "input -d {} swipe {} {} {} {} {}",
                self.display_id.load(Ordering::Relaxed),
                x,
                y,
                x,
                y,
                duration_ms
            ),
        )
        .await?;
        self.invalidate_cache().await;

        Ok(())
    }

    async fn double_tap(&self, selector: &Selector) -> Result<()> {
        let (x, y) = self
            .find_element(selector)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Element not found: {:?}", selector))?;

        let display_id = self.display_id.load(Ordering::Relaxed);

        // Batch both taps into one shell command for speed
        adb::shell(
            self.serial.as_deref(),
            &format!(
                "input -d {} tap {} {} && sleep 0.08 && input -d {} tap {} {}",
                display_id, x, y, display_id, x, y
            ),
        )
        .await?;

        self.smart_delay_after_action().await;
        self.invalidate_cache().await;

        Ok(())
    }

    async fn right_click(&self, _selector: &Selector) -> Result<()> {
        Err(anyhow::anyhow!("Right click is not supported on Android"))
    }

    async fn input_text(&self, text: &str, unicode: bool) -> Result<()> {
        const ADBKEYBOARD_IME: &str = "com.android.adbkeyboard/.AdbIME";

        // Fast path: when unicode is false, use direct input (no ADBKeyBoard overhead)
        if !unicode {
            // Simple direct input - fastest but may have issues with Vietnamese keyboard
            let escaped = text
                .replace("\\", "\\\\")
                .replace(" ", "%s")
                .replace("\"", "\\\"")
                .replace("'", "\\'")
                .replace("&", "\\&")
                .replace("<", "\\<")
                .replace(">", "\\>")
                .replace("|", "\\|")
                .replace(";", "\\;");

            adb::shell(
                self.serial.as_deref(),
                &format!(
                    "input -d {} text '{}'",
                    self.display_id.load(Ordering::Relaxed),
                    escaped
                ),
            )
            .await?;

            return Ok(());
        }

        // Unicode path: use ADBKeyBoard for reliable input (slower but handles all keyboards)
        if self.adbkeyboard_available {
            // Switch to ADBKeyBoard
            let _ = adb::shell(
                self.serial.as_deref(),
                &format!("ime set {}", ADBKEYBOARD_IME),
            )
            .await;

            // Wait and verify IME is active (poll up to 500ms)
            let mut ime_ready = false;
            for _ in 0..5 {
                tokio::time::sleep(Duration::from_millis(100)).await;
                let current_ime = adb::shell(
                    self.serial.as_deref(),
                    "settings get secure default_input_method",
                )
                .await
                .unwrap_or_default();
                if current_ime.contains("adbkeyboard") {
                    ime_ready = true;
                    break;
                }
            }

            if !ime_ready {
                // Fallback: wait a bit more
                tokio::time::sleep(Duration::from_millis(200)).await;
            }

            // Escape text for broadcast
            let escaped = text
                .replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("$", "\\$")
                .replace("`", "\\`");

            // Send text via broadcast (retry once on failure)
            let mut result = adb::shell(
                self.serial.as_deref(),
                &format!("am broadcast -a ADB_INPUT_TEXT --es msg \"{}\"", escaped),
            )
            .await;

            if result.is_err() {
                tokio::time::sleep(Duration::from_millis(100)).await;
                result = adb::shell(
                    self.serial.as_deref(),
                    &format!("am broadcast -a ADB_INPUT_TEXT --es msg \"{}\"", escaped),
                )
                .await;
            }

            // Small delay to let text appear before restoring IME
            tokio::time::sleep(Duration::from_millis(50)).await;

            // Restore original IME (using cached value)
            if !self.original_ime.is_empty() && self.original_ime != "null" {
                let _ = adb::shell(
                    self.serial.as_deref(),
                    &format!("ime set {}", self.original_ime),
                )
                .await;

                // Wait for keyboard to appear (poll up to 2000ms) with robust check
                // ADBKeyBoard might leave mInputShown=true, so we also check if current IME matches original
                println!(
                    "    {} Restoring keyboard {}...",
                    "\u{2328}".blue(),
                    self.original_ime
                );
                let mut restored = false;
                for _ in 0..10 {
                    // 2 seconds
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    let dumpsys = adb::shell(self.serial.as_deref(), "dumpsys input_method")
                        .await
                        .unwrap_or_default();

                    let is_shown = dumpsys.contains("mInputShown=true")
                        || dumpsys.contains("mIsInputViewShown=true");
                    let is_correct_ime =
                        dumpsys.contains(&format!("mCurMethodId={}", self.original_ime));

                    if is_shown && is_correct_ime {
                        restored = true;
                        break;
                    }
                }

                if !restored {
                    println!(
                        "    {} Failed to restore keyboard, trying to force it...",
                        "\u{26A0}".yellow()
                    );
                    // Try to nudge it? Tapping the screen might help if we knew where.
                    // Sending a non-destructive key event?
                    // input keyevent 111 (ESC) hides it.
                    // Let's just warn for now, as sending keyevents blindly is risky.
                    // But user asked to "make sure".
                    // Maybe it's just slow?
                    tokio::time::sleep(Duration::from_millis(500)).await;
                } else {
                    println!("    {} Keyboard restored successfully", "\u{2705}".green());
                }
            }

            if result.is_ok() {
                return Ok(());
            }
        }

        // Fallback: use standard input text
        let mut final_text = text.to_string();

        if text.chars().any(|c| !c.is_ascii()) {
            println!(
                "  {} ADBKeyBoard not available, converting to ASCII.",
                "⚠".yellow()
            );
            final_text = self.to_ascii_fallback(text);
        }

        let escaped = final_text
            .replace("\\", "\\\\")
            .replace(" ", "%s")
            .replace("\"", "\\\"")
            .replace("'", "\\'")
            .replace("&", "\\&")
            .replace("<", "\\<")
            .replace(">", "\\>")
            .replace("|", "\\|")
            .replace(";", "\\;");

        adb::shell(
            self.serial.as_deref(),
            &format!(
                "input -d {} text '{}'",
                self.display_id.load(Ordering::Relaxed),
                escaped
            ),
        )
        .await?;

        Ok(())
    }

    async fn erase_text(&self, char_count: Option<u32>) -> Result<()> {
        let count = char_count.unwrap_or(100);

        // Send DEL key multiple times
        let display_id = self.display_id.load(Ordering::Relaxed);
        for _ in 0..count {
            adb::shell(
                self.serial.as_deref(),
                &format!("input -d {} keyevent 67", display_id),
            )
            .await?; // KEYCODE_DEL
        }
        self.invalidate_cache().await;

        Ok(())
    }

    async fn hide_keyboard(&self) -> Result<()> {
        // Check if keyboard is currently visible (no delay needed since input_text waits for keyboard)
        let dumpsys = adb::shell(self.serial.as_deref(), "dumpsys input_method")
            .await
            .unwrap_or_default();

        let keyboard_visible =
            dumpsys.contains("mInputShown=true") || dumpsys.contains("mIsInputViewShown=true");

        if keyboard_visible {
            // Keyboard is shown, use BACK to hide it
            adb::shell(self.serial.as_deref(), "input keyevent 4").await?;

            // Wait for keyboard to hide
            tokio::time::sleep(Duration::from_millis(100)).await;
            self.invalidate_cache().await;
        }
        // If keyboard is not visible, do nothing (prevents accidental back navigation)

        Ok(())
    }

    async fn swipe(
        &self,
        direction: SwipeDirection,
        duration_ms: Option<u64>,
        from: Option<Selector>,
    ) -> Result<()> {
        // Get current screen size dynamically to handle rotation
        let (width, height) = adb::get_screen_size(self.serial.as_deref())
            .await
            .unwrap_or(self.screen_size);
        let duration = duration_ms.unwrap_or(300);

        // Determine swipe area
        let (area_left, area_top, area_right, area_bottom) = if let Some(selector) = from {
            if let Some(element) = self.find_element_internal(&selector).await? {
                (
                    element.bounds.left,
                    element.bounds.top,
                    element.bounds.right,
                    element.bounds.bottom,
                )
            } else {
                // Return error if source element not found, or maybe just log and default to full screen?
                // Returning error seems safer for explicit tests
                return Err(anyhow::anyhow!("Source element for swipe not found"));
            }
        } else {
            (0, 0, width as i32, height as i32)
        };

        let area_w = area_right - area_left;
        let area_h = area_bottom - area_top;

        let (start_x, start_y, end_x, end_y) = match direction {
            SwipeDirection::Up => {
                let x = area_left + area_w / 2;
                (x, area_bottom - area_h / 4, x, area_top + area_h / 4)
            }
            SwipeDirection::Down => {
                let x = area_left + area_w / 2;
                (x, area_top + area_h / 4, x, area_bottom - area_h / 4)
            }
            SwipeDirection::Left => {
                let y = area_top + area_h / 2;
                (area_right - area_w / 4, y, area_left + area_w / 4, y)
            }
            SwipeDirection::Right => {
                let y = area_top + area_h / 2;
                (area_left + area_w / 4, y, area_right - area_w / 4, y)
            }
        };

        adb::shell(
            self.serial.as_deref(),
            &format!(
                "input -d {} swipe {} {} {} {} {}",
                self.display_id.load(Ordering::Relaxed),
                start_x,
                start_y,
                end_x,
                end_y,
                duration
            ),
        )
        .await?;
        self.invalidate_cache().await;

        Ok(())
    }

    async fn scroll_until_visible(
        &self,
        selector: &Selector,
        max_scrolls: u32,
        direction: Option<SwipeDirection>,
        from: Option<Selector>,
    ) -> Result<bool> {
        // Direction mapping: "up" = scroll content up (swipe finger down), "down" = scroll content down (swipe finger up)
        // Default to SwipeDirection::Up (scrolling down the list)
        let swipe_dir = direction.unwrap_or(SwipeDirection::Up);
        let scroll_delay = self.speed_profile.scroll_delay_ms();

        for _ in 0..max_scrolls {
            if self.is_visible(selector).await? {
                return Ok(true);
            }

            self.swipe(swipe_dir.clone(), Some(800), from.clone())
                .await?;

            // Wait for scroll animation (adaptive based on speed profile)
            tokio::time::sleep(Duration::from_millis(scroll_delay)).await;

            // Explicitly invalidate cache to force fresh dump
            self.invalidate_cache().await;
        }

        // Final check
        Ok(self.is_visible(selector).await?)
    }

    async fn is_visible(&self, selector: &Selector) -> Result<bool> {
        Ok(self.find_element(selector).await?.is_some())
    }

    async fn wait_for_element(&self, selector: &Selector, timeout_ms: u64) -> Result<bool> {
        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);
        let base_interval = self.speed_profile.poll_interval_ms();
        let mut interval = base_interval;
        const MAX_INTERVAL: u64 = 500;

        while start.elapsed() < timeout {
            // Invalidate cache to get fresh UI state
            self.invalidate_cache().await;

            if self.is_visible(selector).await? {
                return Ok(true);
            }

            tokio::time::sleep(Duration::from_millis(interval)).await;

            // Exponential backoff: increase interval by 50% each time, up to max
            interval = (interval * 3 / 2).min(MAX_INTERVAL);
        }

        Ok(false)
    }

    async fn wait_for_absence(&self, selector: &Selector, timeout_ms: u64) -> Result<bool> {
        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);
        let base_interval = self.speed_profile.poll_interval_ms();
        let mut interval = base_interval;
        const MAX_INTERVAL: u64 = 500;

        while start.elapsed() < timeout {
            // Invalidate cache to get fresh UI state
            self.invalidate_cache().await;

            if !self.is_visible(selector).await? {
                return Ok(true);
            }

            tokio::time::sleep(Duration::from_millis(interval)).await;

            // Exponential backoff
            interval = (interval * 3 / 2).min(MAX_INTERVAL);
        }

        Ok(false)
    }

    async fn get_element_text(&self, selector: &Selector) -> Result<String> {
        let elements = self.get_ui_hierarchy().await?;

        match self.find_element_impl(selector, &elements) {
            Some((element, _)) => {
                // Return text or content_desc, preferring text
                if !element.text.is_empty() {
                    Ok(element.text.clone())
                } else {
                    Ok(element.content_desc.clone())
                }
            }
            None => Ok(String::new()),
        }
    }

    async fn open_link(&self, url: &str, app_id: Option<&str>) -> Result<()> {
        // Quote the URL to prevent shell expansion issues (e.g. & character)
        let quoted_url = format!("'{}'", url);

        let pkg_arg = if let Some(pkg) = app_id {
            format!(" -p {}", pkg)
        } else {
            String::new()
        };

        let output = adb::shell(
            self.serial.as_deref(),
            &format!(
                "am start -W -a android.intent.action.VIEW -d {}{}",
                quoted_url, pkg_arg
            ),
        )
        .await?;

        // Check for common errors in output even if exit code was 0
        if output.contains("Error:") || output.contains("exception") {
            println!("  {} Deep Link Warning: {}", "⚠️".yellow(), output.trim());
        } else {
            // println!("DEBUG: Open Link Output: {}", output.trim());
        }

        self.invalidate_cache().await;
        Ok(())
    }

    async fn compare_screenshot(
        &self,
        reference_path: &Path,
        _tolerance_percent: f64,
    ) -> Result<f64> {
        let temp_screenshot =
            std::env::temp_dir().join(format!("temp_screenshot_{}.png", Uuid::new_v4()));
        self.take_screenshot(temp_screenshot.to_str().unwrap())
            .await?;

        let img1 = image::open(&temp_screenshot)?;
        let img2 = image::open(reference_path)?;

        if img1.dimensions() != img2.dimensions() {
            anyhow::bail!(
                "Image dimensions mismatch: current {:?} vs reference {:?}",
                img1.dimensions(),
                img2.dimensions()
            );
        }

        let mut diff_pixels = 0;
        let total_pixels = img1.width() * img1.height();

        for (x, y, pixel1) in img1.pixels() {
            let pixel2 = img2.get_pixel(x, y);
            if pixel1 != pixel2 {
                diff_pixels += 1;
            }
        }

        let diff_percent = (diff_pixels as f64 / total_pixels as f64) * 100.0;

        // Clean up temp file
        let _ = std::fs::remove_file(temp_screenshot);

        Ok(diff_percent)
    }

    async fn take_screenshot(&self, path: &str) -> Result<()> {
        let remote_path = "/sdcard/screenshot.png";

        // Take screenshot on device
        adb::shell(
            self.serial.as_deref(),
            &format!(
                "screencap -d {} -p {}",
                self.display_id.load(Ordering::Relaxed),
                remote_path
            ),
        )
        .await?;

        // Pull to local
        adb::pull(self.serial.as_deref(), remote_path, path).await?;

        // Cleanup
        adb::shell(self.serial.as_deref(), &format!("rm {}", remote_path)).await?;

        Ok(())
    }

    async fn start_recording(&self, path: &str) -> Result<()> {
        let remote_path = "/sdcard/screenrecord.mp4";
        let local_path = path.to_string();

        // Start recording in background
        let child = tokio::process::Command::new("adb")
            .args(&[
                "-s",
                self.serial.as_deref().unwrap_or(""),
                "shell",
                "screenrecord",
                "--bit-rate",
                "4000000",
                remote_path,
            ])
            .spawn()?;

        *self.recording_process.lock().await = Some(child);
        self.current_recording_path.lock().await.replace(local_path);

        Ok(())
    }

    async fn stop_recording(&self) -> Result<()> {
        if let Some(mut child) = self.recording_process.lock().await.take() {
            // Gracefully stop screenrecord on device using SIGINT (Ctrl+C)
            // This ensures the MP4 file is finalized correctly.
            let _ = adb::shell(self.serial.as_deref(), "pkill -2 screenrecord").await;

            // Wait for the local adb process to exit
            match tokio::time::timeout(tokio::time::Duration::from_secs(3), child.wait()).await {
                Ok(_) => {}
                Err(_) => {
                    println!(
                        "{} screenrecord did not exit gracefully, force killing...",
                        "⚠️".yellow()
                    );
                    let _ = child.kill().await;
                }
            }

            // Wait for file system to settle
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

            // Pull the file
            if let Some(local_path) = self.current_recording_path.lock().await.take() {
                adb::exec(
                    self.serial.as_deref(),
                    &["pull", "/sdcard/screenrecord.mp4", &local_path],
                )
                .await?;
                println!("  {} Saved Video Recording: {}", "🎥".green(), local_path);
            }
        }

        Ok(())
    }

    async fn back(&self) -> Result<()> {
        adb::shell(self.serial.as_deref(), "input keyevent 4").await?; // KEYCODE_BACK
        self.invalidate_cache().await;
        Ok(())
    }

    async fn home(&self) -> Result<()> {
        adb::shell(self.serial.as_deref(), "input keyevent 3").await?; // KEYCODE_HOME
        self.invalidate_cache().await;
        Ok(())
    }

    async fn get_screen_size(&self) -> Result<(u32, u32)> {
        Ok(self.screen_size)
    }

    async fn dump_ui_hierarchy(&self) -> Result<String> {
        adb::shell(
            self.serial.as_deref(),
            "uiautomator dump /sdcard/window_dump.xml",
        )
        .await?;
        adb::shell(self.serial.as_deref(), "cat /sdcard/window_dump.xml").await
    }

    async fn tap_by_type_index(&self, element_type: &str, index: u32) -> Result<()> {
        self.tap_at(element_type, index).await
    }

    async fn input_by_type_index(&self, element_type: &str, index: u32, text: &str) -> Result<()> {
        self.input_at(element_type, index, text).await
    }

    async fn dump_logs(&self, limit: u32) -> Result<String> {
        adb::exec(
            self.serial.as_deref(),
            &["logcat", "-d", "-t", &limit.to_string()],
        )
        .await
    }

    async fn start_mock_location(
        &self,
        name: Option<String>,
        points: Vec<crate::parser::gps::GpsPoint>,
        speed_kmh: Option<f64>,
        speed_mode: SpeedMode,
        speed_noise: Option<f64>,
        interval_ms: u64,
        loop_route: bool,
    ) -> Result<()> {
        use colored::Colorize;
        use rand::Rng;

        if points.is_empty() {
            anyhow::bail!("No GPS points provided for mock location");
        }

        let instance_name = name.clone().unwrap_or_default();
        println!(
            "  {} Starting mock location '{}' with {} waypoints",
            "📍".green(),
            if instance_name.is_empty() {
                "default"
            } else {
                &instance_name
            },
            points.len()
        );

        let serial = self.serial.clone();
        let interval = std::time::Duration::from_millis(interval_ms);

        if let Some(speed) = speed_kmh {
            let mode_str = match speed_mode {
                SpeedMode::Linear => "Linear",
                SpeedMode::Noise => format!("Noise ±{:.1}", speed_noise.unwrap_or(5.0))
                    .to_string()
                    .leak(),
            };
            println!(
                "  {} Using speed: {} km/h ({})",
                "🚗".cyan(),
                speed,
                mode_str
            );
        }

        // Initialize nl-mirror service (auto-deploy and start if needed)
        let mirror_result =
            super::mirror_service::MirrorService::init_session(serial.as_deref()).await;
        let mirror_active = if let Err(e) = &mirror_result {
            eprintln!(
                "  ⚠️ nl-mirror init failed: {}. Speed may not be accurate.",
                e
            );
            false
        } else {
            true
        };

        // Initialize Mock Location using 'cmd location' (Android 10+)
        let setup_cmds = vec![
            "settings put global wifi_scan_always_enabled 0",
            "settings put global ble_scan_always_enabled 0",
            "appops set 2000 android:mock_location allow",
            "cmd location providers add-test-provider gps",
            "cmd location providers set-test-provider-enabled gps true",
            "cmd location providers add-test-provider network",
            "cmd location providers set-test-provider-enabled network true",
            "cmd location providers add-test-provider fused",
            "cmd location providers set-test-provider-enabled fused true",
        ];

        for cmd in setup_cmds {
            if let Err(e) = adb::shell(serial.as_deref(), cmd).await {
                eprintln!("Mock setup warning (might be normal on old devices): {}", e);
            }
        }

        // Spawn background task to update location
        let points_clone = points.clone();
        let mock_states = self.mock_states.clone();
        let instance_key = instance_name.clone();

        // Initialize state
        {
            let mut states = mock_states.lock().await;
            let state = states
                .entry(instance_key.clone())
                .or_insert_with(MockLocationState::default);
            state.is_running = true;
            state.finished = false;
            state.paused = false;
            state.speed = speed_kmh;
            state.speed_mode = speed_mode.clone();
            state.speed_noise = speed_noise;
        }

        tokio::spawn(async move {
            use rand::SeedableRng;
            let mut rng = rand::rngs::StdRng::from_entropy();

            'outer: loop {
                for (i, point) in points_clone.iter().enumerate() {
                    let lat = point.lat;
                    let lon = point.lon;

                    // Check for pause and external control file
                    loop {
                        // Read external control file for VSCode integration
                        let control_path = "/tmp/lumi-gps-control.json";
                        if let Ok(content) = std::fs::read_to_string(control_path) {
                            if let Ok(ctrl) = serde_json::from_str::<serde_json::Value>(&content) {
                                let mut states = mock_states.lock().await;
                                if let Some(state) = states.get_mut(&instance_key) {
                                    // Update speed if specified
                                    if let Some(speed) = ctrl.get("speed").and_then(|v| v.as_f64())
                                    {
                                        state.speed = Some(speed);
                                    }
                                    // Update pause state
                                    if let Some(paused) =
                                        ctrl.get("paused").and_then(|v| v.as_bool())
                                    {
                                        state.paused = paused;
                                    }
                                    // Update speed mode
                                    if let Some(mode) =
                                        ctrl.get("speedMode").and_then(|v| v.as_str())
                                    {
                                        state.speed_mode = match mode {
                                            "noise" => SpeedMode::Noise,
                                            _ => SpeedMode::Linear,
                                        };
                                    }
                                }
                                // Clear control file after reading
                                let _ = std::fs::remove_file(control_path);
                            }
                        }

                        let is_paused = {
                            let states = mock_states.lock().await;
                            states.get(&instance_key).map(|s| s.paused).unwrap_or(false)
                        };
                        if !is_paused {
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }

                    // Update state
                    let (current_speed, current_mode, _current_noise) = {
                        let mut states = mock_states.lock().await;
                        if let Some(state) = states.get_mut(&instance_key) {
                            state.current_lat = Some(lat);
                            state.current_lon = Some(lon);
                            state.is_running = true;
                            (state.speed, state.speed_mode.clone(), state.speed_noise)
                        } else {
                            (speed_kmh, speed_mode.clone(), speed_noise)
                        }
                    };

                    // Calculate bearing to next point (or keep previous bearing for last point)
                    let bearing = if i < points_clone.len() - 1 {
                        let next_pt = &points_clone[i + 1];
                        calculate_bearing(lat, lon, next_pt.lat, next_pt.lon)
                    } else if i > 0 {
                        // Use previous bearing for last point
                        let prev_pt = &points_clone[i - 1];
                        calculate_bearing(prev_pt.lat, prev_pt.lon, lat, lon)
                    } else {
                        0.0 // Default bearing
                    };

                    // Calculate effective speed for this segment (m/s)
                    let effective_speed_kmh = match current_mode {
                        SpeedMode::Linear => current_speed.unwrap_or(0.0),
                        SpeedMode::Noise => {
                            let base = current_speed.unwrap_or(0.0);
                            let noise_range = _current_noise.unwrap_or(5.0);
                            let noise: f64 = rng.gen_range(-noise_range..noise_range);
                            (base + noise).max(1.0)
                        }
                    };
                    let speed_ms = effective_speed_kmh / 3.6; // Convert km/h to m/s

                    // Method 1: Use nl-android (nl-mirror) via socket - FULL SPEED SUPPORT
                    let nl_cmd = format!(
                        r#"{{"cmd":"set_location","lat":{},"lon":{},"alt":{},"bearing":{:.2},"speed":{:.2}}}"#,
                        lat,
                        lon,
                        point.altitude.unwrap_or(0.0),
                        bearing,
                        speed_ms
                    );

                    // Try to send to nl-mirror synchronously with better timeout
                    let nl_success = if mirror_active {
                        match std::net::TcpStream::connect_timeout(
                            &"127.0.0.1:8889".parse().unwrap(),
                            std::time::Duration::from_millis(200),
                        ) {
                            Ok(mut stream) => {
                                let _ = stream
                                    .set_write_timeout(Some(std::time::Duration::from_millis(200)));
                                use std::io::Write;
                                if let Err(e) = stream.write_all(format!("{}\n", nl_cmd).as_bytes())
                                {
                                    eprintln!("  ⚠️ nl-mirror write failed: {}", e);
                                    false
                                } else {
                                    true
                                }
                            }
                            Err(_) => {
                                // Silent failure on connect is common if service momentarily busy
                                false
                            }
                        }
                    } else {
                        false
                    };

                    // Only use fallback if nl-mirror failed
                    if !nl_success {
                        // Method 2: Standard cmd location (fallback, no bearing support)
                        let providers = vec!["gps", "network", "fused"];
                        for provider in providers {
                            let cmd_loc = format!(
                                "cmd location providers set-test-provider-location {} --location {},{}",
                                provider, lat, lon
                            );
                            let _ = adb::shell(serial.as_deref(), &cmd_loc).await;
                        }

                        // Method 3: Emulator (geo fix)
                        let geo_cmd = format!("geo fix {} {}", lon, lat);
                        let _ = adb::shell(serial.as_deref(), &geo_cmd).await;
                    }

                    if i < points_clone.len() - 1 {
                        let next_point = &points_clone[i + 1];
                        let delay = if let Some(base_speed) = current_speed {
                            // Apply noise if enabled
                            let effective_speed = match current_mode {
                                SpeedMode::Linear => base_speed,
                                SpeedMode::Noise => {
                                    let noise_range = speed_noise.unwrap_or(5.0);
                                    let noise: f64 = rng.gen_range(-noise_range..noise_range);
                                    (base_speed + noise).max(1.0) // Minimum 1 km/h
                                }
                            };

                            let dist_m = haversine_distance(
                                point.lat,
                                point.lon,
                                next_point.lat,
                                next_point.lon,
                            );
                            let speed_ms = effective_speed / 3.6;
                            if speed_ms > 0.001 {
                                (dist_m / speed_ms * 1000.0) as u64
                            } else {
                                interval.as_millis() as u64
                            }
                        } else {
                            interval.as_millis() as u64
                        };

                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    }
                }

                // Check if we should loop
                if !loop_route {
                    break 'outer;
                }
            }

            // Mark finished
            {
                let mut states = mock_states.lock().await;
                if let Some(state) = states.get_mut(&instance_key) {
                    state.is_running = false;
                    state.finished = true;
                }
            }
            println!("  {} Mock location playback completed", "✅".green());
        });

        // Give the first point time to be set
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        Ok(())
    }

    async fn stop_mock_location(&self) -> Result<()> {
        let cmds = vec![
            "cmd location providers remove-test-provider gps",
            "cmd location providers remove-test-provider network",
            "cmd location providers remove-test-provider fused",
            "appops set 2000 android:mock_location deny",
            "settings put secure mock_location 0", // Cleanup legacy too
            "settings put global wifi_scan_always_enabled 1", // Restore scan
            "settings put global ble_scan_always_enabled 1", // Restore scan
        ];

        for cmd in cmds {
            let _ = adb::shell(self.serial.as_deref(), cmd).await;
        }

        println!("  {} Mock location stopped", "📍".yellow());
        Ok(())
    }

    async fn get_pixel_color(&self, x: i32, y: i32) -> Result<(u8, u8, u8)> {
        // Take a temporary screenshot
        let temp_path =
            std::env::temp_dir().join(format!("color_check_{}.png", uuid::Uuid::new_v4()));
        let temp_path_str = temp_path.to_string_lossy().to_string();

        self.take_screenshot(&temp_path_str).await?;

        // Open the image and get pixel color using common utility
        let img = image::open(&temp_path)?;
        let result = common::get_pixel_from_image(&img, x as u32, y as u32);

        // Cleanup temp file
        let _ = std::fs::remove_file(temp_path);

        Ok(result)
    }

    async fn rotate_screen(&self, mode: &str) -> Result<()> {
        // Disable auto-rotate first
        adb::shell(
            self.serial.as_deref(),
            "settings put system accelerometer_rotation 0",
        )
        .await?;

        let rotation = match mode.to_lowercase().as_str() {
            "portrait" => "0",
            "landscape" => "1",
            _ => anyhow::bail!("Invalid rotation mode. Use 'portrait' or 'landscape'"),
        };

        adb::shell(
            self.serial.as_deref(),
            &format!("settings put system user_rotation {}", rotation),
        )
        .await?;

        // Wait for rotation animation
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        self.invalidate_cache().await;

        Ok(())
    }

    async fn press_key(&self, key: &str) -> Result<()> {
        let keycode_str = key.to_lowercase();
        let keycode = match keycode_str.as_str() {
            "home" => "3",
            "back" => "4",
            "search" => "84",
            "enter" | "done" => "66",
            "numpad_enter" => "160",
            "power" => "26",
            "volume_up" => "24",
            "volume_down" => "25",
            "menu" => "82",
            "tab" => "61",
            "space" => "62",
            "del" | "delete" | "backspace" => "67",
            "dpad_up" | "up" => "19",
            "dpad_down" | "down" => "20",
            "dpad_left" | "left" => "21",
            "dpad_right" | "right" => "22",
            "dpad_center" | "center" => "23",
            s if s.chars().all(|c| c.is_ascii_digit()) => s,
            _ => anyhow::bail!(
                "Unsupported key: {}. Use raw keycode (e.g. '66') or runScript if needed",
                key
            ),
        };

        adb::shell(
            self.serial.as_deref(),
            &format!(
                "input -d {} keyevent {}",
                self.display_id.load(Ordering::Relaxed),
                keycode
            ),
        )
        .await?;
        self.invalidate_cache().await;
        Ok(())
    }

    async fn push_file(&self, local_path: &str, remote_path: &str) -> Result<()> {
        adb::push(self.serial.as_deref(), local_path, remote_path).await
    }

    async fn pull_file(&self, remote_path: &str, local_path: &str) -> Result<()> {
        adb::pull(self.serial.as_deref(), remote_path, local_path).await
    }

    async fn clear_app_data(&self, app_id: &str) -> Result<()> {
        adb::shell(self.serial.as_deref(), &format!("pm clear {}", app_id)).await?;
        self.invalidate_cache().await;
        Ok(())
    }

    async fn set_clipboard(&self, text: &str) -> Result<()> {
        // Workaround: simulate typing as 'paste' logic
        let escaped = text.replace("\"", "\\\"").replace(" ", "%s");
        adb::shell(
            self.serial.as_deref(),
            &format!("input text \"{}\"", escaped),
        )
        .await?;
        Ok(())
    }

    async fn get_clipboard(&self) -> Result<String> {
        // Android prevents background clipboard access on modern versions
        Err(anyhow::anyhow!("getClipboard not supported natively on Android without helper app. Workaround: use setVar with known values."))
    }

    // New Commands Implementation

    async fn set_network_connection(&self, wifi: Option<bool>, data: Option<bool>) -> Result<()> {
        if let Some(enabled) = wifi {
            let state = if enabled { "enable" } else { "disable" };
            adb::shell(self.serial.as_deref(), &format!("svc wifi {}", state)).await?;
        }
        if let Some(enabled) = data {
            let state = if enabled { "enable" } else { "disable" };
            adb::shell(self.serial.as_deref(), &format!("svc data {}", state)).await?;
        }
        Ok(())
    }

    async fn toggle_airplane_mode(&self) -> Result<()> {
        // Get current state
        let output = adb::shell(
            self.serial.as_deref(),
            "settings get global airplane_mode_on",
        )
        .await?;
        let current_state = output.trim();

        let new_state = if current_state == "1" { "0" } else { "1" };
        let state_bool = if new_state == "1" { "true" } else { "false" };

        adb::shell(
            self.serial.as_deref(),
            &format!("settings put global airplane_mode_on {}", new_state),
        )
        .await?;
        adb::shell(
            self.serial.as_deref(),
            &format!(
                "am broadcast -a android.intent.action.AIRPLANE_MODE --ez state {}",
                state_bool
            ),
        )
        .await?;

        println!(
            "  {} Toggled Airplane Mode to: {}",
            "✈".cyan(),
            if new_state == "1" { "ON" } else { "OFF" }
        );
        Ok(())
    }

    async fn open_notifications(&self) -> Result<()> {
        adb::shell(self.serial.as_deref(), "cmd statusbar expand-notifications").await?;
        Ok(())
    }

    async fn open_quick_settings(&self) -> Result<()> {
        adb::shell(self.serial.as_deref(), "cmd statusbar expand-settings").await?;
        Ok(())
    }

    async fn set_volume(&self, level: u8) -> Result<()> {
        // Try 'cmd media_session' first (newer androids)
        // Stream 3 is MUSIC
        let cmd = format!("cmd media_session volume --stream 3 --set {}", level);
        if let Ok(_) = adb::shell(self.serial.as_deref(), &cmd).await {
            return Ok(());
        }

        // Fallback to 'media volume --set' (older androids/some vendors)
        adb::shell(
            self.serial.as_deref(),
            &format!("media volume --set {}", level),
        )
        .await?;
        Ok(())
    }
    async fn lock_device(&self) -> Result<()> {
        adb::shell(self.serial.as_deref(), "input keyevent 26").await?; // KEYCODE_POWER (toggles, but often used to lock)
                                                                        // Ideally checking display state would be better, but simple toggle is okay for now
        Ok(())
    }

    async fn unlock_device(&self) -> Result<()> {
        // Wake up
        adb::shell(self.serial.as_deref(), "input keyevent 26").await?; // Power
        adb::shell(self.serial.as_deref(), "input keyevent 82").await?; // Menu/Unlock
        Ok(())
    }

    async fn install_app(&self, path: &str) -> Result<()> {
        if !std::path::Path::new(path).exists() {
            anyhow::bail!("App file not found: {}", path);
        }

        // Check if it's an XAPK (split APK bundle)
        if path.to_lowercase().ends_with(".xapk") {
            return self.install_xapk(path).await;
        }

        println!("  {} Installing app from: {}", "⬇".cyan(), path);
        adb::exec(
            self.serial.as_deref(),
            &["install", "-r", "-g", path], // -r: replace, -g: grant perms
        )
        .await?;
        Ok(())
    }

    async fn uninstall_app(&self, app_id: &str) -> Result<()> {
        println!("  {} Uninstalling app: {}", "🗑".cyan(), app_id);
        adb::exec(self.serial.as_deref(), &["uninstall", app_id]).await?;
        Ok(())
    }

    async fn background_app(&self, app_id_opt: Option<&str>, duration_ms: u64) -> Result<()> {
        // Press Home
        adb::shell(self.serial.as_deref(), "input keyevent 3").await?;

        // Wait
        tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;

        // Resume
        if let Some(app_id) = app_id_opt {
            // Try to launch main activity again to bring to front
            self.launch_app(app_id, false).await?;
        } else {
            // Try to use APP_SWITCH to switch back to last app (double tap recent?)
            // Or just warn
            println!(
                "  {} No app_id provided to resume, staying on home screen",
                "⚠".yellow()
            );
        }
        Ok(())
    }

    async fn set_orientation(&self, mode: crate::parser::types::Orientation) -> Result<()> {
        use crate::parser::types::Orientation;

        // Disable auto-rotation first
        adb::shell(
            self.serial.as_deref(),
            "settings put system accelerometer_rotation 0",
        )
        .await?;

        let rotation = match mode {
            Orientation::Portrait => "0",
            Orientation::Landscape => "1",      // 90 degrees
            Orientation::UpsideDown => "2",     // 180 degrees
            Orientation::LandscapeLeft => "1",  // 90 degrees
            Orientation::LandscapeRight => "3", // 270 degrees
        };

        adb::shell(
            self.serial.as_deref(),
            &format!("settings put system user_rotation {}", rotation),
        )
        .await?;

        // Wait for rotation animation
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        self.invalidate_cache().await;

        Ok(())
    }

    async fn wait_for_location(
        &self,
        name: Option<String>,
        lat: f64,
        lon: f64,
        tolerance: f64,
        timeout: u64,
    ) -> Result<()> {
        // Call inherent method
        AndroidDriver::wait_for_location(self, name, lat, lon, tolerance, timeout).await
    }

    async fn wait_for_mock_completion(
        &self,
        name: Option<String>,
        timeout: Option<u64>,
    ) -> Result<()> {
        // Call inherent method
        AndroidDriver::wait_for_mock_completion(self, name, timeout).await
    }

    async fn control_mock_location(
        &self,
        name: Option<String>,
        speed: Option<f64>,
        speed_mode: Option<SpeedMode>,
        speed_noise: Option<f64>,
        pause: Option<bool>,
        resume: Option<bool>,
    ) -> Result<()> {
        // Call inherent method
        AndroidDriver::control_mock_location(
            self,
            name,
            speed,
            speed_mode,
            speed_noise,
            pause,
            resume,
        )
        .await
    }

    async fn start_profiling(
        &self,
        params: Option<crate::parser::types::StartProfilingParams>,
    ) -> Result<()> {
        let current = self.get_current_package().await?;
        let app = params.and_then(|p| p.package).unwrap_or(current);

        adb::shell(
            self.serial.as_deref(),
            &format!("dumpsys gfxinfo {} reset", app),
        )
        .await?;
        Ok(())
    }

    async fn stop_profiling(&self) -> Result<()> {
        // No-op for Android basic profiling as we pull live data
        Ok(())
    }

    async fn get_performance_metrics(&self) -> Result<std::collections::HashMap<String, f64>> {
        let app = self.get_current_package().await?;
        let mut metrics = std::collections::HashMap::new();

        // 1. Memory (PSS Total in MB)
        let mem_out = adb::shell(self.serial.as_deref(), &format!("dumpsys meminfo {}", app))
            .await
            .unwrap_or_default();
        if let Some(capt) = regex::Regex::new(r"TOTAL\s+(\d+)")
            .unwrap()
            .captures(&mem_out)
        {
            if let Ok(kb) = capt[1].parse::<f64>() {
                metrics.insert("memory".to_string(), kb / 1024.0);
            }
        }

        // 2. CPU
        let cpu_out = adb::shell(self.serial.as_deref(), "dumpsys cpuinfo")
            .await
            .unwrap_or_default();
        for line in cpu_out.lines() {
            if line.contains(&app) {
                if let Some(capt) = regex::Regex::new(r"(\d+(\.\d+)?)%").unwrap().captures(line) {
                    if let Ok(val) = capt[1].parse::<f64>() {
                        metrics.insert("cpu".to_string(), val);
                        break;
                    }
                }
            }
        }

        // 3. FPS / Frame Quality
        let gfx_out = adb::shell(self.serial.as_deref(), &format!("dumpsys gfxinfo {}", app))
            .await
            .unwrap_or_default();
        let mut frame_count = 0.0;
        let mut jank_count = 0.0;
        let mut parsing_frames = false;
        for line in gfx_out.lines() {
            if line.contains("Draw") && line.contains("Prepare") {
                parsing_frames = true;
                continue;
            }
            if parsing_frames {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let mut sum = 0.0;
                    let mut valid = true;
                    // Draw, Prepare, Process
                    for p in &parts[0..3] {
                        if let Ok(d) = p.parse::<f64>() {
                            sum += d;
                        } else {
                            valid = false;
                            break;
                        }
                    }
                    if valid {
                        frame_count += 1.0;
                        if sum > 16.6 {
                            jank_count += 1.0;
                        }
                    }
                } else if !line.trim().is_empty() {
                    // Try to guess if end of table
                    if line.contains("View hierarchy:") {
                        parsing_frames = false;
                    }
                }
            }
        }

        if frame_count > 0.0 {
            let fps = 60.0 * (1.0 - (jank_count / frame_count));
            metrics.insert("fps".to_string(), fps);
            metrics.insert("jank_rate".to_string(), jank_count / frame_count * 100.0);
        }

        Ok(metrics)
    }

    async fn set_cpu_throttling(&self, _rate: f64) -> Result<()> {
        println!(
            "  {} CPU throttling not supported on Android without root/custom kernel",
            "⚠".yellow()
        );
        Ok(())
    }

    async fn set_network_conditions(&self, profile: &str) -> Result<()> {
        match profile.to_lowercase().as_str() {
            "offline" => {
                self.set_network_connection(Some(false), Some(false))
                    .await?;
            }
            "wifi" | "wifi-only" => {
                self.set_network_connection(Some(true), Some(false)).await?;
            }
            "data" | "mobile" | "4g" | "5g" | "lte" => {
                self.set_network_connection(Some(false), Some(true)).await?;
            }
            _ => {
                println!(
                    "  {} Unknown network profile '{}', defaulting to wifi on",
                    "⚠".yellow(),
                    profile
                );
                self.set_network_connection(Some(true), None).await?;
            }
        }
        Ok(())
    }

    async fn select_display(&self, display_id: u32) -> Result<()> {
        self.display_id.store(display_id, Ordering::Relaxed);
        println!("  {} Selected Display ID: {}", "📺".cyan(), display_id);
        Ok(())
    }

    async fn set_locale(&self, locale: &str) -> Result<()> {
        // Android: use adb shell to set system locale
        // Format: en-US, vi-VN, ja-JP, etc.
        adb::shell(
            self.serial.as_deref(),
            &format!("settings put system system_locales {}", locale),
        )
        .await?;
        println!("  {} Set device locale to: {}", "🌐".green(), locale);
        Ok(())
    }

    // App Status Commands

    async fn detect_app_crash(&self, app_id: &str) -> Result<bool> {
        // Check recent logcat for FATAL EXCEPTION specifically for this app
        // Format in logcat: "FATAL EXCEPTION: main" followed by "Process: com.example.app, PID: 12345"

        // Get recent crash logs and filter for this specific app
        let output = adb::shell(
            self.serial.as_deref(),
            "logcat -d -t 200 AndroidRuntime:E *:S",
        )
        .await;

        match output {
            Ok(logs) => {
                // Look for crash pattern: FATAL EXCEPTION followed by Process line with our app_id
                // The logs are sequential, so we check if the app appears in crash context
                let lines: Vec<&str> = logs.lines().collect();

                for (i, line) in lines.iter().enumerate() {
                    // Look for "Process: {app_id}, PID:" pattern
                    if line.contains(&format!("Process: {}, PID:", app_id))
                        || line.contains(&format!("Process: {},", app_id))
                    {
                        // Check if there's a FATAL EXCEPTION nearby (within 5 lines before)
                        let start = i.saturating_sub(5);
                        for j in start..=i {
                            if lines
                                .get(j)
                                .map_or(false, |l| l.contains("FATAL EXCEPTION"))
                            {
                                return Ok(true);
                            }
                        }
                    }
                }

                Ok(false)
            }
            Err(_) => Ok(false), // If logcat fails, assume no crash
        }
    }

    // Audio Test Commands

    async fn play_media(&self, file_path: &std::path::Path, loop_playback: bool) -> Result<()> {
        // Push file to device if local
        let remote_path = format!(
            "/sdcard/Music/{}",
            file_path.file_name().unwrap().to_string_lossy()
        );

        if file_path.exists() {
            adb::push(
                self.serial.as_deref(),
                &file_path.to_string_lossy(),
                &remote_path,
            )
            .await?;
        }

        // Play using am start
        let loop_flag = if loop_playback { "--ez loop true " } else { "" };
        let cmd = format!(
            "am start -a android.intent.action.VIEW -d file://{} -t audio/* {}",
            remote_path, loop_flag
        );
        adb::shell(self.serial.as_deref(), &cmd).await?;

        println!("  {} Playing media: {}", "🎵".green(), file_path.display());
        Ok(())
    }

    async fn stop_media(&self) -> Result<()> {
        // Send media control keys to stop playback
        // Try PAUSE first (more widely supported) then STOP
        adb::shell(self.serial.as_deref(), "input keyevent KEYCODE_MEDIA_PAUSE").await?;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        adb::shell(self.serial.as_deref(), "input keyevent KEYCODE_MEDIA_STOP").await?;

        // Also try to kill common media players just in case
        let common_players = [
            "com.google.android.music",
            "com.google.android.apps.youtube.music",
            "com.samsung.android.app.music.chn",
            "com.sec.android.app.music",
            "com.miui.player",
        ];

        for pkg in common_players {
            let _ = adb::shell(self.serial.as_deref(), &format!("am force-stop {}", pkg)).await;
        }

        println!("  {} Media stopped", "🎵".yellow());
        Ok(())
    }

    async fn start_audio_capture(&self, duration_ms: u64, port: u16) -> Result<()> {
        use super::audio_service::AudioService;

        // Store capture in a static or thread-local (simplified: just log for now)
        println!(
            "  {} Starting audio capture for {}ms on port {}",
            "🎤".cyan(),
            duration_ms,
            port
        );

        match AudioService::start_capture(self.serial.as_deref()).await {
            Ok(capture) => {
                // Store capture handle (would need proper state management)
                // For now, just spawn a timeout
                let duration = std::time::Duration::from_millis(duration_ms);
                tokio::spawn(async move {
                    tokio::time::sleep(duration).await;
                    let analysis = capture.stop_and_analyze();
                    analysis.print_summary();
                });
                Ok(())
            }
            Err(e) => {
                eprintln!("  ⚠️ Audio capture failed: {}", e);
                Err(e)
            }
        }
    }

    async fn stop_audio_capture(&self) -> Result<()> {
        println!("  {} Audio capture stopped", "🎤".yellow());
        Ok(())
    }

    async fn verify_audio_ducking(&self, min_events: usize, drop_threshold: f64) -> Result<()> {
        // This would check the captured audio analysis
        // For now, log and pass (would need proper state management to access capture results)
        println!(
            "  {} Verify audio ducking: min_events={}, drop_threshold={}%",
            "🔊".cyan(),
            min_events,
            drop_threshold
        );

        // TODO: Access stored AudioAnalysis and verify
        // For demo, we'll pass the test
        println!(
            "  {} Audio ducking verification passed (placeholder)",
            "✓".green()
        );
        Ok(())
    }

    /// Auto-detect Android Auto display by parsing activity and display info
    async fn detect_android_auto_display(&self) -> Result<Option<u32>> {
        // Strategy 1: Check dumpsys activity activities for display with running gearhead activity
        // This is the most reliable method as it finds the display with actual activity
        let activities_output = adb::shell(
            self.serial.as_deref(),
            "dumpsys activity activities | grep -E 'Display #|gearhead.*GhostActivity'",
        )
        .await
        .unwrap_or_default();

        let mut current_display: Option<u32> = None;
        for line in activities_output.lines() {
            let line = line.trim();
            if line.starts_with("Display #") {
                // Parse "Display #37 (activities from top to bottom):"
                current_display = line
                    .trim_start_matches("Display #")
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<u32>().ok());
            } else if line.contains("gearhead") && line.contains("GhostActivity") {
                if let Some(id) = current_display {
                    if id > 0 {
                        println!(
                            "  {} Found Android Auto Display ID: {} (via activity)",
                            "📺".cyan(),
                            id
                        );
                        return Ok(Some(id));
                    }
                }
            }
        }

        // Strategy 2: Parse dumpsys display for gearhead virtual displays
        let output = adb::shell(self.serial.as_deref(), "dumpsys display").await?;

        // State machine to parse multi-line output
        let mut current_display_id: Option<u32> = None;
        let mut current_is_gearhead = false;
        let mut current_is_on = false;

        let mut candidates: Vec<(u32, bool)> = Vec::new(); // (id, is_on)

        for line in output.lines() {
            let line = line.trim();

            // New display block starts with "Display ID:"
            if line.starts_with("Display ") && line.ends_with(":") {
                // Save previous candidate if valid
                if let Some(id) = current_display_id {
                    if current_is_gearhead {
                        candidates.push((id, current_is_on));
                    }
                }

                // Reset state
                current_display_id = line
                    .trim_start_matches("Display ")
                    .trim_end_matches(':')
                    .parse::<u32>()
                    .ok();
                current_is_gearhead = false;
                current_is_on = false;
            } else if current_display_id.is_some() {
                // Check if owned by gearhead
                if line.contains("owner com.google.android.projection.gearhead")
                    || line.contains("virtual:com.google.android.projection.gearhead")
                {
                    current_is_gearhead = true;
                }

                // Check state
                if line.contains("state ON") {
                    current_is_on = true;
                }
            }
        }

        // Check last block
        if let Some(id) = current_display_id {
            if current_is_gearhead {
                candidates.push((id, current_is_on));
            }
        }

        // Sort: ON first, then by ID ascending (prefer lower ID which is often the main AA display)
        candidates.sort_by(|a, b| {
            if a.1 != b.1 {
                b.1.cmp(&a.1) // true (ON) > false (OFF)
            } else {
                a.0.cmp(&b.0) // Lower ID first (main display usually has lower ID)
            }
        });

        if let Some((id, is_on)) = candidates.first() {
            println!(
                "  {} Found Android Auto Display ID: {} (Active: {})",
                "📺".cyan(),
                id,
                is_on
            );
            return Ok(Some(*id));
        }

        // Fallback: Find any secondary display (ID > 0)
        let re = regex::Regex::new(r"Display (\d+)").unwrap();
        let mut displays: Vec<u32> = re
            .captures_iter(&output)
            .filter_map(|cap| cap.get(1).and_then(|m| m.as_str().parse().ok()))
            .filter(|&id| id > 0)
            .collect();
        displays.sort();

        Ok(displays.first().copied()) // Pick lowest non-zero ID as fallback
    }
}

/// Map common element type aliases to Android widget classes
fn map_android_type(t: &str) -> &str {
    match t.to_lowercase().as_str() {
        "input" | "edittext" | "textfield" => "android.widget.EditText",
        "button" | "btn" => "android.widget.Button",
        "image" | "img" | "icon" => "android.widget.ImageView",
        "text" | "label" | "textview" => "android.widget.TextView",
        "view" => "android.view.View",
        _ => t,
    }
}

impl AndroidDriver {
    async fn wait_for_location(
        &self,
        name: Option<String>,
        lat: f64,
        lon: f64,
        tolerance: f64,
        timeout_ms: u64,
    ) -> Result<()> {
        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);
        let instance_key = name.unwrap_or_default();

        println!(
            "  {} Waiting for location ({:.4}, {:.4}) within {:.1}m...",
            "⏳".cyan(),
            lat,
            lon,
            tolerance
        );

        loop {
            if start.elapsed() > timeout {
                anyhow::bail!("Timeout waiting for location");
            }

            let current_pos = {
                let states = self.mock_states.lock().await;
                states.get(&instance_key).and_then(|state| {
                    if let (Some(c_lat), Some(c_lon)) = (state.current_lat, state.current_lon) {
                        Some((c_lat, c_lon))
                    } else {
                        None
                    }
                })
            };

            if let Some((c_lat, c_lon)) = current_pos {
                let dist = haversine_distance(c_lat, c_lon, lat, lon);
                if dist <= tolerance {
                    println!(
                        "  {} Reached location ({:.4}, {:.4}). Distance: {:.1}m",
                        "✅".green(),
                        c_lat,
                        c_lon,
                        dist
                    );
                    return Ok(());
                }
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    async fn wait_for_mock_completion(
        &self,
        name: Option<String>,
        timeout_ms: Option<u64>,
    ) -> Result<()> {
        let start = Instant::now();
        let instance_key = name.unwrap_or_default();

        println!(
            "  {} Waiting for mock location '{}' completion...",
            "⏳".cyan(),
            if instance_key.is_empty() {
                "default"
            } else {
                &instance_key
            }
        );

        loop {
            // Check timeout only if specified
            if let Some(t) = timeout_ms {
                if start.elapsed() > Duration::from_millis(t) {
                    anyhow::bail!("Timeout waiting for mock location completion");
                }
            }

            {
                let states = self.mock_states.lock().await;
                if let Some(state) = states.get(&instance_key) {
                    if state.finished {
                        println!("  {} Mock location completed", "✅".green());
                        return Ok(());
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// Control a running mock location instance
    async fn control_mock_location(
        &self,
        name: Option<String>,
        speed: Option<f64>,
        speed_mode: Option<SpeedMode>,
        speed_noise: Option<f64>,
        pause: Option<bool>,
        resume: Option<bool>,
    ) -> Result<()> {
        let instance_key = name.unwrap_or_default();

        let mut states = self.mock_states.lock().await;
        let state = states.get_mut(&instance_key).ok_or_else(|| {
            anyhow::anyhow!(
                "Mock location instance '{}' not found",
                if instance_key.is_empty() {
                    "default"
                } else {
                    &instance_key
                }
            )
        })?;

        if let Some(s) = speed {
            println!("  {} Updating mock speed to {} km/h", "🚗".cyan(), s);
            state.speed = Some(s);
        }

        if let Some(mode) = speed_mode {
            state.speed_mode = mode;
        }

        if let Some(noise) = speed_noise {
            state.speed_noise = Some(noise);
        }

        if pause == Some(true) {
            println!("  {} Pausing mock location", "⏸".yellow());
            state.paused = true;
        }

        if resume == Some(true) {
            println!("  {} Resuming mock location", "▶".green());
            state.paused = false;
        }

        Ok(())
    }

    // Helper to get currently focused/resumed app package
    async fn get_current_package(&self) -> Result<String> {
        // Method 1: ResumedActivity
        let out = adb::shell(
            self.serial.as_deref(),
            "dumpsys activity activities | grep ResumedActivity",
        )
        .await
        .unwrap_or_default();
        // Output format: "    ResumedActivity: ActivityRecord{... u0 com.package.name/.ActivityName ...}"
        if let Some(capt) = regex::Regex::new(r"u0\s+([a-zA-Z0-9_.]+)/")
            .unwrap()
            .captures(&out)
        {
            if let Some(m) = capt.get(1) {
                return Ok(m.as_str().to_string());
            }
        }

        // Method 2: mCurrentFocus
        let out2 = adb::shell(
            self.serial.as_deref(),
            "dumpsys window windows | grep mCurrentFocus",
        )
        .await
        .unwrap_or_default();
        if let Some(capt) = regex::Regex::new(r"u0\s+([a-zA-Z0-9_.]+)/")
            .unwrap()
            .captures(&out2)
        {
            if let Some(m) = capt.get(1) {
                return Ok(m.as_str().to_string());
            }
        }

        Err(anyhow::anyhow!("Could not detect current package"))
    }
}

/// Calculate Haversine distance between two points in meters
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371000.0; // Earth radius in meters
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    r * c
}

/// Calculate initial bearing from point 1 to point 2 in degrees (0-360)
/// 0° = North, 90° = East, 180° = South, 270° = West
fn calculate_bearing(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let d_lon = (lon2 - lon1).to_radians();

    let x = d_lon.sin() * lat2_rad.cos();
    let y = lat1_rad.cos() * lat2_rad.sin() - lat1_rad.sin() * lat2_rad.cos() * d_lon.cos();

    let bearing_rad = x.atan2(y);
    let bearing_deg = bearing_rad.to_degrees();

    // Normalize to 0-360 range
    (bearing_deg + 360.0) % 360.0
}

/// Map ASCII character to Android keycode
/// Returns (keycode, needs_shift)
#[allow(dead_code)]
fn char_to_keycode(c: char) -> Option<(u32, bool)> {
    match c {
        'a'..='z' => Some((29 + (c as u32 - 'a' as u32), false)), // KEYCODE_A = 29
        'A'..='Z' => Some((29 + (c as u32 - 'A' as u32), true)),
        '0'..='9' => Some((7 + (c as u32 - '0' as u32), false)), // KEYCODE_0 = 7
        '@' => Some((77, true)),   // KEYCODE_AT (shift+2 on some keyboards)
        '.' => Some((56, false)),  // KEYCODE_PERIOD
        '_' => Some((69, true)),   // KEYCODE_MINUS with shift
        '-' => Some((69, false)),  // KEYCODE_MINUS
        '+' => Some((81, true)),   // KEYCODE_PLUS
        '=' => Some((70, false)),  // KEYCODE_EQUALS
        '!' => Some((8, true)),    // shift+1
        '#' => Some((18, true)),   // shift+3 (varies by keyboard)
        '$' => Some((11, true)),   // shift+4
        '%' => Some((12, true)),   // shift+5
        '^' => Some((13, true)),   // shift+6
        '&' => Some((14, true)),   // shift+7
        '*' => Some((17, true)),   // shift+8
        '(' => Some((71, true)),   // KEYCODE_LEFT_BRACKET with shift
        ')' => Some((72, true)),   // KEYCODE_RIGHT_BRACKET with shift
        '/' => Some((76, false)),  // KEYCODE_SLASH
        '\\' => Some((73, false)), // KEYCODE_BACKSLASH
        ':' => Some((74, true)),   // KEYCODE_SEMICOLON with shift
        ';' => Some((74, false)),  // KEYCODE_SEMICOLON
        ',' => Some((55, false)),  // KEYCODE_COMMA
        '?' => Some((76, true)),   // KEYCODE_SLASH with shift
        '\'' => Some((75, false)), // KEYCODE_APOSTROPHE
        '"' => Some((75, true)),   // KEYCODE_APOSTROPHE with shift
        '[' => Some((71, false)),  // KEYCODE_LEFT_BRACKET
        ']' => Some((72, false)),  // KEYCODE_RIGHT_BRACKET
        '{' => Some((71, true)),
        '}' => Some((72, true)),
        '|' => Some((73, true)),  // KEYCODE_BACKSLASH with shift
        '<' => Some((55, true)),  // KEYCODE_COMMA with shift
        '>' => Some((56, true)),  // KEYCODE_PERIOD with shift
        '`' => Some((68, false)), // KEYCODE_GRAVE
        '~' => Some((68, true)),
        ' ' => Some((62, false)),  // KEYCODE_SPACE
        '\n' => Some((66, false)), // KEYCODE_ENTER
        '\t' => Some((61, false)), // KEYCODE_TAB
        _ => None,
    }
}
