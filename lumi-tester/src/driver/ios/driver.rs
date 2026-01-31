//! iOS Driver implementation
//!
//! This driver enables iOS automation testing:
//! - Simulators: Uses idb CLI tool
//! - Real devices: Uses WebDriverAgent (WDA) via HTTP API

use anyhow::{Context, Result};
use async_trait::async_trait;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::accessibility::{self, IosElement};
use super::idb;
use super::wda::WdaClient;
use crate::driver::common;
use crate::driver::image_matcher::{find_template, ImageRegion, MatchConfig};
use crate::driver::traits::{PlatformDriver, Selector, SwipeDirection};
use crate::parser::types::SpeedMode;
use colored::Colorize;
use image::GenericImageView;
use std::collections::HashMap as StdHashMap;

/// iOS driver implementation
/// - Simulators: Uses idb
/// - Real devices: Uses WebDriverAgent (WDA)
pub struct IosDriver {
    /// Device UDID
    udid: String,
    /// Device name (used for logging)
    #[allow(dead_code)]
    device_name: String,
    /// Whether this is a simulator (vs physical device)
    is_simulator: bool,
    /// Cached UI hierarchy
    ui_cache: Arc<Mutex<Option<Vec<IosElement>>>>,
    /// Cache timestamp
    cache_time: Arc<Mutex<Option<Instant>>>,
    /// Video recording process
    recording_process: Arc<Mutex<Option<tokio::process::Child>>>,
    /// Current recording output path
    current_recording_path: Arc<Mutex<Option<String>>>,
    /// Screen dimensions
    screen_size: (u32, u32),
    /// Mock location states keyed by name ("" for default)
    mock_states: Arc<Mutex<StdHashMap<String, IosMockLocationState>>>,
    /// WDA client for real device UI automation
    wda_client: Arc<Mutex<Option<WdaClient>>>,
    /// OCR engine (lazy-initialized)
    ocr_engine: tokio::sync::OnceCell<crate::driver::ocr::OcrEngine>,
}

/// State of the background mock location process for iOS
#[derive(Clone)]
struct IosMockLocationState {
    current_lat: Option<f64>,
    current_lon: Option<f64>,
    is_running: bool,
    finished: bool,
    paused: bool,
    speed: Option<f64>,
    speed_mode: SpeedMode,
    speed_noise: Option<f64>,
}

impl Default for IosMockLocationState {
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

impl IosDriver {
    /// Create a new iOS driver
    pub async fn new(udid: Option<&str>) -> Result<Self> {
        let targets = idb::list_targets().await?;

        let target = if let Some(id) = udid.filter(|s| !s.is_empty()) {
            targets
                .iter()
                .find(|t| t.udid == id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Device with UDID {} not found", id))?
        } else {
            // Pick the first booted target
            targets
                .iter()
                .find(|t| t.state.eq_ignore_ascii_case("Booted"))
                .cloned()
                .or_else(|| targets.first().cloned())
                .ok_or_else(|| anyhow::anyhow!("No iOS devices or simulators found"))?
        };

        println!(
            "{} Connected to iOS {}: {} ({})",
            "✓".green(),
            if target.target_type.eq_ignore_ascii_case("simulator") {
                "simulator"
            } else {
                "device"
            },
            target.name,
            target.udid
        );

        let is_simulator = target.target_type.eq_ignore_ascii_case("simulator");
        let screen_size = idb::get_screen_size(&target.udid)
            .await
            .unwrap_or((390, 844));

        // Initialize WDA client for real devices
        let wda_client = if !is_simulator {
            // Try to ensure WDA is running (auto-start if possible)
            let port = super::wda::DEFAULT_WDA_PORT;
            let _ = super::wda_setup::ensure_wda_running(&target.udid, port).await;

            // Check if WDA host was found (stored in env by wda_setup)
            let wda_host = std::env::var("WDA_HOST").unwrap_or_else(|_| "localhost".to_string());
            let client = WdaClient::with_host(&wda_host, port);

            if client.is_ready().await.unwrap_or(false) {
                println!(
                    "{} WebDriverAgent ready at {}:{}",
                    "✓".green(),
                    wda_host,
                    port
                );
                Some(client)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            udid: target.udid,
            device_name: target.name,
            is_simulator,
            ui_cache: Arc::new(Mutex::new(None)),
            cache_time: Arc::new(Mutex::new(None)),
            recording_process: Arc::new(Mutex::new(None)),
            current_recording_path: Arc::new(Mutex::new(None)),
            screen_size,
            mock_states: Arc::new(Mutex::new(StdHashMap::new())),
            wda_client: Arc::new(Mutex::new(wda_client)),
            ocr_engine: tokio::sync::OnceCell::new(),
        })
    }

    /// Invalidate the UI cache
    pub async fn invalidate_cache(&self) {
        let mut cache = self.ui_cache.lock().await;
        *cache = None;
        let mut time = self.cache_time.lock().await;
        *time = None;
    }

    /// Get the UI hierarchy (with caching)
    async fn get_ui_hierarchy(&self) -> Result<Vec<IosElement>> {
        const CACHE_DURATION: Duration = Duration::from_millis(500);

        // Check cache
        let cache_time = self.cache_time.lock().await;
        if let Some(time) = *cache_time {
            if time.elapsed() < CACHE_DURATION {
                let cache = self.ui_cache.lock().await;
                if let Some(elements) = cache.as_ref() {
                    return Ok(elements.clone());
                }
            }
        }
        drop(cache_time);

        // Fetch fresh hierarchy
        let json_output = idb::describe_ui(&self.udid).await?;
        let elements = accessibility::parse_ui_hierarchy(&json_output)?;

        // Update cache
        let mut cache = self.ui_cache.lock().await;
        *cache = Some(elements.clone());
        let mut time = self.cache_time.lock().await;
        *time = Some(Instant::now());

        Ok(elements)
    }

    /// Get OCR engine (lazy-initialized)
    async fn get_ocr_engine(&self) -> Result<&crate::driver::ocr::OcrEngine> {
        self.ocr_engine
            .get_or_try_init(|| async { crate::driver::ocr::OcrEngine::new().await })
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

        // Initialize engine first
        let engine = self.get_ocr_engine().await?;

        // Capture screenshot
        let screenshot_path = std::env::temp_dir().join(format!("ios_ocr_{}.png", Uuid::new_v4()));
        let screenshot_path_str = screenshot_path.to_string_lossy().to_string();
        idb::screenshot(&self.udid, &screenshot_path_str).await?;
        let png_data = std::fs::read(&screenshot_path)?;
        let _ = std::fs::remove_file(&screenshot_path);

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

    /// Find template image on screen using optimized single-pass template matching
    /// Uses region-based matching if region is specified
    async fn find_image_on_screen(
        &self,
        template_path: &str,
        region: Option<&str>,
    ) -> Result<Option<(i32, i32)>> {
        let total_start = Instant::now();
        let template_path_buf = Path::new(template_path).to_path_buf();
        if !template_path_buf.exists() {
            anyhow::bail!("Template image not found: {:?}", template_path_buf);
        }

        // Parse region
        let image_region = region.map(|r| ImageRegion::from_str(r)).unwrap_or_default();
        if image_region != ImageRegion::Full {
            println!("      📍 Region: {:?}", image_region);
        }

        // Screenshot
        // Use temp file for screenshot
        let screenshot_path =
            std::env::temp_dir().join(format!("ios_match_{}.png", Uuid::new_v4()));
        let screenshot_path_str = screenshot_path.to_string_lossy().to_string();

        let screenshot_start = Instant::now();
        idb::screenshot(&self.udid, &screenshot_path_str).await?;
        println!("      ⏱ Screenshot: {:?}", screenshot_start.elapsed());

        // Match
        let match_start = Instant::now();
        let result = tokio::task::spawn_blocking(move || -> Result<Option<(i32, i32)>> {
            let img_screen = image::open(&screenshot_path)?.to_luma8();
            let img_template = image::open(&template_path_buf)?.to_luma8();

            // Cleanup
            let _ = std::fs::remove_file(&screenshot_path);

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

        println!("      ⏱ Match: {:?}", match_start.elapsed());
        let total_time = total_start.elapsed();
        println!("      ⏱ Total image match: {:?}", total_time);
        Ok(result)
    }

    /// Find element by selector
    async fn find_element_internal(
        &self,
        selector: &Selector,
    ) -> Result<Option<accessibility::IosElement>> {
        // Point selector has no dimensions/element structure, return None
        if let Selector::Point { .. } = selector {
            return Ok(None);
        }

        let elements = self.get_ui_hierarchy().await?;

        let element = match selector {
            Selector::Text(text, index, _) => accessibility::find_by_text(&elements, text, *index),
            Selector::TextRegex(pattern, index) => {
                let regex = Regex::new(pattern).context("Invalid regex pattern")?;
                accessibility::find_by_text_regex(&elements, &regex, *index)
            }
            Selector::Id(id, index) => accessibility::find_by_id(&elements, id, *index),
            Selector::IdRegex(pattern, index) => {
                let regex = Regex::new(pattern).context("Invalid regex pattern")?;
                accessibility::find_by_id_regex(&elements, &regex, *index)
            }
            Selector::Type(element_type, index) => {
                accessibility::find_by_type(&elements, element_type, *index)
            }
            Selector::Placeholder(placeholder, index) => {
                accessibility::find_by_placeholder(&elements, placeholder, *index)
            }
            Selector::AccessibilityId(id) => accessibility::find_by_id(&elements, id, 0),
            Selector::XPath(_) => None,
            Selector::Css(_) => None,
            Selector::Role(role, index) => accessibility::find_by_type(&elements, role, *index),
            Selector::Description(desc, index) => {
                accessibility::find_by_accessibility_id(&elements, desc, *index)
            }
            Selector::DescriptionRegex(pattern, index) => {
                let regex = Regex::new(pattern).context("Invalid regex pattern")?;
                accessibility::find_by_accessibility_id_regex(&elements, &regex, *index)
            }
            Selector::AnyClickable(index) => {
                // On iOS, we look for elements that are enabled and have actions
                let flat = accessibility::flatten_elements(&elements);
                let clickables: Vec<_> = flat
                    .into_iter()
                    .filter(|e| e.visible && e.enabled)
                    .collect();
                clickables.get(*index).copied()
            }
            Selector::Relative {
                target,
                anchor,
                direction,
                max_dist,
            } => self.find_relative_element(&elements, target, anchor, direction, max_dist),
            Selector::Point { .. } => unreachable!(),
            Selector::Image { .. } => None,
            Selector::OCR(..) => None, // OCR handled separately via screenshot
            Selector::ScrollableItem { .. } | Selector::Scrollable(_) => None,
            Selector::HasChild { parent, child } => {
                let flat = accessibility::flatten_elements(&elements);
                let parent_candidates: Vec<_> = flat
                    .iter()
                    .filter(|e| e.visible && self.element_matches_selector(e, parent))
                    .collect();
                let child_candidates: Vec<_> = flat
                    .iter()
                    .filter(|e| e.visible && self.element_matches_selector(e, child))
                    .collect();

                let mut found = None;
                for p in parent_candidates {
                    for c in &child_candidates {
                        if p.frame.contains(&c.frame)
                            && !std::ptr::eq(*p as *const _, **c as *const _)
                        {
                            found = Some(p); // Found parent
                            break;
                        }
                    }
                    if found.is_some() {
                        break;
                    }
                }
                match found {
                    Some(p) => Some(*p),
                    None => None,
                }
            }
        };

        Ok(element.cloned())
    }

    async fn find_element(&self, selector: &Selector) -> Result<Option<(i32, i32)>> {
        // Handle Point selector directly
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

        let el = self.find_element_internal(selector).await?;
        Ok(el.map(|e| e.frame.center()))
    }

    /// Find element relative to an anchor (non-async to avoid recursion)
    fn find_relative_element<'a>(
        &self,
        elements: &'a [IosElement],
        target: &Selector,
        anchor: &Selector,
        direction: &crate::driver::traits::RelativeDirection,
        max_dist: &Option<u32>,
    ) -> Option<&'a IosElement> {
        // Find anchor element directly (inline logic to avoid async recursion)
        let anchor_element = match anchor {
            Selector::Text(text, index, _) => accessibility::find_by_text(elements, text, *index),
            Selector::Id(id, index) => accessibility::find_by_id(elements, id, *index),
            Selector::Type(t, index) => accessibility::find_by_type(elements, t, *index),
            Selector::Placeholder(p, index) => {
                accessibility::find_by_placeholder(elements, p, *index)
            }
            Selector::Point { x, y } => accessibility::find_at_point(elements, *x, *y),
            Selector::Image { .. } => None,
            _ => None,
        };

        let anchor_element = anchor_element?;
        let (ax, ay) = anchor_element.center();

        let flat = accessibility::flatten_elements(elements);
        let max_distance = max_dist.unwrap_or(500) as i32;

        // Find target matching the base selector
        let target_base_matches: Vec<_> = flat
            .into_iter()
            .filter(|e| e.visible && self.element_matches_selector(e, target))
            .collect();

        // Filter by direction and distance
        // Calculate scores and collect matches
        let mut scored_matches: Vec<(&IosElement, f64)> = Vec::new();

        use crate::driver::traits::RelativeDirection::*;
        for element in target_base_matches {
            // Filter out large container elements (>95% width or >80% height)
            // Screen size assumption: typical iOS device ~390x844 or larger
            let screen_width = 430.0; // Max common iPhone width
            let screen_height = 932.0; // Max common iPhone height
            let width_ratio = element.frame.width / screen_width;
            let height_ratio = element.frame.height / screen_height;

            // Skip if element covers >95% width OR >80% height (container)
            if width_ratio > 0.95 || height_ratio > 0.8 || (width_ratio > 0.8 && height_ratio > 0.5)
            {
                continue;
            }

            let (ex, ey) = element.center();
            let dx = ex - ax;
            let dy = ey - ay;

            let matches = match direction {
                LeftOf => {
                    (dx <= 0 && dx.abs() < max_distance)
                        || (anchor_element.frame.contains(&element.frame)
                            && element.frame.center().0 <= ax)
                }
                RightOf => {
                    (dx >= 0 && dx < max_distance)
                        || (anchor_element.frame.contains(&element.frame)
                            && element.frame.center().0 >= ax)
                }
                Above => {
                    (dy <= 0 && dy.abs() < max_distance)
                        || (anchor_element.frame.contains(&element.frame)
                            && element.frame.center().1 <= ay)
                }
                Below => {
                    (dy >= 0 && dy < max_distance)
                        || (anchor_element.frame.contains(&element.frame)
                            && element.frame.center().1 >= ay)
                }
                Near => (dx.abs() + dy.abs()) < max_distance,
            };

            if matches {
                // Overlap bonus: prioritize elements that overlap on the orthogonal axis
                let overlap_bonus = match direction {
                    RightOf | LeftOf => {
                        // Revised logic using proper f64 comparisons
                        let cy = ey as f64;
                        let cy_anchor = ay as f64;

                        let candidate_top = cy - element.frame.height / 2.0;
                        let candidate_bottom = cy + element.frame.height / 2.0;
                        let anchor_top = cy_anchor - anchor_element.frame.height / 2.0;
                        let anchor_bottom = cy_anchor + anchor_element.frame.height / 2.0;

                        let overlap_start = candidate_top.max(anchor_top);
                        let overlap_end = candidate_bottom.min(anchor_bottom);

                        // If significant overlap (more than 50% of smaller height)
                        let min_height = element.frame.height.min(anchor_element.frame.height);
                        if overlap_end > overlap_start
                            && (overlap_end - overlap_start) > min_height * 0.5
                        {
                            -100.0
                        } else {
                            0.0
                        }
                    }
                    Below | Above => {
                        let cx = ex as f64;
                        let cx_anchor = ax as f64;

                        let candidate_left = cx - element.frame.width / 2.0;
                        let candidate_right = cx + element.frame.width / 2.0;
                        let anchor_left = cx_anchor - anchor_element.frame.width / 2.0;
                        let anchor_right = cx_anchor + anchor_element.frame.width / 2.0;

                        let overlap_start = candidate_left.max(anchor_left);
                        let overlap_end = candidate_right.min(anchor_right);

                        // If significant overlap
                        let min_width = element.frame.width.min(anchor_element.frame.width);
                        if overlap_end > overlap_start
                            && (overlap_end - overlap_start) > min_width * 0.5
                        {
                            -100.0
                        } else {
                            0.0
                        }
                    }
                    Near => 0.0,
                };

                // Calculate edge distance for scoring (logic adapted from Android)
                // We want to prioritize elements closest to the reference edge
                let edge_dist = match direction {
                    RightOf => {
                        element.frame.x - (anchor_element.frame.x + anchor_element.frame.width)
                    }
                    LeftOf => (element.frame.x + element.frame.width) - anchor_element.frame.x,
                    Below => {
                        element.frame.y - (anchor_element.frame.y + anchor_element.frame.height)
                    }
                    Above => (element.frame.y + element.frame.height) - anchor_element.frame.y,
                    Near => (((ex - ax).pow(2) + (ey - ay).pow(2)) as f64).sqrt(),
                };

                // Use abs() to prioritize elements closer to the edge (whether inside or outside)
                // This matches the Android logic fix
                let score = edge_dist.abs() + overlap_bonus;

                scored_matches.push((element, score));
            }
        }

        // Sort by score
        scored_matches.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Get index from target selector
        let target_index = match target {
            Selector::Text(_, idx, _) => *idx,
            Selector::TextRegex(_, idx) => *idx,
            Selector::Id(_, idx) => *idx,
            Selector::IdRegex(_, idx) => *idx,
            Selector::Type(_, idx) => *idx,
            Selector::Role(_, idx) => *idx,
            Selector::Placeholder(_, idx) => *idx,
            Selector::AccessibilityId(_) => 0,
            Selector::Description(_, idx) => *idx,
            Selector::DescriptionRegex(_, idx) => *idx,
            Selector::AnyClickable(idx) => *idx,
            _ => 0,
        };

        scored_matches.into_iter().nth(target_index).map(|(e, _)| e)
    }

    /// Check if element matches a selector (for relative finding)
    fn element_matches_selector(&self, element: &IosElement, selector: &Selector) -> bool {
        match selector {
            Selector::Text(text, _, _) => element.matches_text(text),
            Selector::Id(id, _) => element.matches_id(id),
            Selector::Type(t, _) => element.matches_type(t),
            Selector::Placeholder(p, _) => element.matches_placeholder(p),
            Selector::Image { .. } => false,
            Selector::IdRegex(pattern, _) => {
                if let Ok(regex) = Regex::new(pattern) {
                    element.matches_id_regex(&regex)
                } else {
                    false
                }
            }
            Selector::AnyClickable(_) => element.visible && element.enabled,
            Selector::AccessibilityId(id) | Selector::Description(id, _) => {
                element.matches_label(id)
            }
            Selector::DescriptionRegex(pattern, _) => {
                if let Ok(regex) = Regex::new(pattern) {
                    element
                        .label
                        .as_ref()
                        .map(|l| regex.is_match(l))
                        .unwrap_or(false)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Input text via clipboard (simulator only)
    async fn input_text_clipboard(&self, text: &str) -> Result<()> {
        // 1. Set clipboard
        let mut child = tokio::process::Command::new("xcrun")
            .args(&["simctl", "pbcopy", &self.udid])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn pbcopy")?;

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(text.as_bytes()).await?;
        }
        child.wait().await?;

        // 2. Find target element to tap (TextField)
        // We reuse the logic from erase_text to find generic text field if we don't know where to tap
        let ui_json = idb::describe_ui(&self.udid).await?;
        let mut tap_x = (self.screen_size.0 / 2) as i32;
        let mut tap_y = (self.screen_size.1 / 4) as i32;

        if let Ok(elements) = crate::driver::ios::accessibility::parse_ui_hierarchy(&ui_json) {
            for el in crate::driver::ios::accessibility::flatten_elements(&elements) {
                if let Some(el_type) = &el.element_type {
                    if el_type == "TextField"
                        || el_type == "TextArea"
                        || el_type == "SecureTextField"
                    {
                        // Ideally checking for "hasKeyboardFocus" but valid JSON doesn't always have it exposed nicely
                        // We assume the first visible text field is the one we want or the one focused
                        let (cx, cy) = el.center();
                        tap_x = cx;
                        tap_y = cy;
                        break;
                    }
                }
            }
        }

        // 3. Tap to ensure focus / Bring up menu
        // We tap once. If menu doesn't appear, we try tapping cursor again.
        println!("    {} Tapping to focus text field...", "ℹ".blue());
        idb::tap(&self.udid, tap_x, tap_y).await?;
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Check if "Paste" is visible immediately
        if self.find_paste_button().await?.is_none() {
            // Tap again (sometimes toggle menu)
            println!("    {} Tapping again to reveal menu...", "ℹ".blue());
            idb::tap(&self.udid, tap_x, tap_y).await?;
            tokio::time::sleep(Duration::from_millis(700)).await;
        }

        // If still not visible, try long press
        if self.find_paste_button().await?.is_none() {
            println!("    {} Long pressing to reveal menu...", "ℹ".blue());
            idb::long_press(&self.udid, tap_x, tap_y, 1000).await?;
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }

        // 4. Tap Paste
        if let Some(paste_btn) = self.find_paste_button().await? {
            let (px, py) = paste_btn.center();
            println!(
                "    {} Tapping Paste button at ({}, {})...",
                "ℹ".blue(),
                px,
                py
            );
            idb::tap(&self.udid, px, py).await?;
        } else {
            println!(
                "{} Could not find 'Paste' menu item. Trying blind tap near cursor...",
                "⚠️".yellow()
            );
            // Last resort: If menu appeared but "Paste" wasn't found (maybe it's icons?),
            // or if we just want to try typing blindly.
            // But for non-ASCII, typing blindly via HID crashes.
            // So we abort here or try to send Ctrl+V equivalent if possible (simulator doesn't always support cmd+v via hid)

            return Err(anyhow::anyhow!(
                "Failed to paste text: 'Paste' menu not found on screen."
            ));
        }

        Ok(())
    }

    async fn find_paste_button(&self) -> Result<Option<IosElement>> {
        let ui_json = idb::describe_ui(&self.udid).await?;
        if let Ok(elements) = crate::driver::ios::accessibility::parse_ui_hierarchy(&ui_json) {
            let flat = crate::driver::ios::accessibility::flatten_elements(&elements);

            // Search for any element that looks like a Paste button
            for el in flat {
                if !el.visible {
                    continue;
                }

                let label = el.label.as_deref().unwrap_or("");
                let value = el.value.as_deref().unwrap_or("");
                let _name = el.element_type.as_deref().unwrap_or(""); // Sometimes name is in type? No, type is type.

                if label == "Paste" || value == "Paste" || label == "Dán" {
                    return Ok(Some(el.clone()));
                }

                // Check for MenuItem type specifically
                if let Some(t) = &el.element_type {
                    if t == "MenuItem" && (label.contains("Paste") || label.contains("Dán")) {
                        return Ok(Some(el.clone()));
                    }
                }
            }
        }
        Ok(None)
    }
}

#[async_trait]
impl PlatformDriver for IosDriver {
    fn platform_name(&self) -> &str {
        "ios"
    }

    fn device_serial(&self) -> Option<String> {
        Some(self.udid.clone())
    }

    async fn launch_app(&self, bundle_id: &str, clear_state: bool) -> Result<()> {
        self.invalidate_cache().await;

        // Always terminate first if running
        let _ = idb::terminate_app(&self.udid, bundle_id, self.is_simulator).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        if clear_state {
            // Silently clear app data

            // Get app data container path using simctl
            let container_result = tokio::process::Command::new("xcrun")
                .args(&["simctl", "get_app_container", &self.udid, bundle_id, "data"])
                .output()
                .await;

            if let Ok(output) = container_result {
                if output.status.success() {
                    let container_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !container_path.is_empty() && std::path::Path::new(&container_path).exists()
                    {
                        // Delete contents of Documents, Library, tmp folders (keep the folders)
                        let subfolders = ["Documents", "Library", "tmp"];
                        for folder in &subfolders {
                            let folder_path = format!("{}/{}", container_path, folder);
                            if std::path::Path::new(&folder_path).exists() {
                                // Remove all contents inside the folder
                                let _ = tokio::process::Command::new("rm")
                                    .args(&["-rf", &format!("{}/*", folder_path)])
                                    .output()
                                    .await;

                                // Use shell to expand glob
                                let _ = tokio::process::Command::new("sh")
                                    .args(&[
                                        "-c",
                                        &format!("rm -rf {}/* 2>/dev/null || true", folder_path),
                                    ])
                                    .output()
                                    .await;
                            }
                        }
                        // Cleared app container silently
                    }
                }
            }

            // Also reset privacy permissions
            let _ = tokio::process::Command::new("xcrun")
                .args(&["simctl", "privacy", &self.udid, "reset", "all", bundle_id])
                .output()
                .await;

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        // Launch the app (silently)
        idb::launch_app(&self.udid, bundle_id, self.is_simulator).await?;

        // Wait longer for app to fully stabilize (especially after clear state)
        let wait_time = if clear_state { 2000 } else { 1000 };
        tokio::time::sleep(Duration::from_millis(wait_time)).await;
        self.invalidate_cache().await;

        Ok(())
    }

    async fn stop_app(&self, bundle_id: &str) -> Result<()> {
        idb::terminate_app(&self.udid, bundle_id, self.is_simulator).await?;
        self.invalidate_cache().await;
        Ok(())
    }

    async fn tap(&self, selector: &Selector) -> Result<()> {
        let pos = self
            .find_element(selector)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Element not found for tap: {:?}", selector))?;

        if self.is_simulator {
            idb::tap(&self.udid, pos.0, pos.1).await?;
        } else {
            // Use WDA for real devices
            let mut wda = self.wda_client.lock().await;
            if let Some(ref mut client) = *wda {
                client.tap(pos.0, pos.1).await?;
            } else {
                // Fallback to idb (will likely fail)
                idb::tap(&self.udid, pos.0, pos.1).await?;
            }
        }
        self.invalidate_cache().await;
        Ok(())
    }

    async fn long_press(&self, selector: &Selector, duration_ms: u64) -> Result<()> {
        let pos = self
            .find_element(selector)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Element not found for long_press: {:?}", selector))?;

        if self.is_simulator {
            idb::long_press(&self.udid, pos.0, pos.1, duration_ms).await?;
        } else {
            let mut wda = self.wda_client.lock().await;
            if let Some(ref mut client) = *wda {
                client.long_press(pos.0, pos.1, duration_ms).await?;
            } else {
                idb::long_press(&self.udid, pos.0, pos.1, duration_ms).await?;
            }
        }
        self.invalidate_cache().await;
        Ok(())
    }

    async fn double_tap(&self, selector: &Selector) -> Result<()> {
        let pos = self
            .find_element(selector)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Element not found for double_tap: {:?}", selector))?;

        if self.is_simulator {
            // Perform two rapid taps
            idb::tap(&self.udid, pos.0, pos.1).await?;
            tokio::time::sleep(Duration::from_millis(50)).await;
            idb::tap(&self.udid, pos.0, pos.1).await?;
        } else {
            let mut wda = self.wda_client.lock().await;
            if let Some(ref mut client) = *wda {
                client.double_tap(pos.0, pos.1).await?;
            } else {
                idb::tap(&self.udid, pos.0, pos.1).await?;
                tokio::time::sleep(Duration::from_millis(50)).await;
                idb::tap(&self.udid, pos.0, pos.1).await?;
            }
        }

        self.invalidate_cache().await;
        Ok(())
    }

    async fn right_click(&self, _selector: &Selector) -> Result<()> {
        anyhow::bail!("Right click is not supported on iOS")
    }

    async fn input_text(&self, text: &str, _unicode: bool) -> Result<()> {
        if self.is_simulator {
            if text.chars().all(|c| c.is_ascii()) {
                idb::input_text(&self.udid, text).await?;
            } else {
                // Fallback for non-ASCII characters (e.g. Vietnamese)
                self.input_text_clipboard(text).await?;
            }
        } else {
            // Use WDA for real devices - supports both ASCII and Unicode
            let mut wda = self.wda_client.lock().await;
            if let Some(ref mut client) = *wda {
                client.input_text(text).await?;
            } else {
                // Fallback to idb (will likely fail for real devices)
                idb::input_text(&self.udid, text).await?;
            }
        }
        self.invalidate_cache().await;
        Ok(())
    }

    async fn erase_text(&self, _char_count: Option<u32>) -> Result<()> {
        // For iOS, find text field and select all via triple-tap then replace
        let ui_json = idb::describe_ui(&self.udid).await?;

        // Look for TextField/SearchField in the UI to get correct coordinates
        let mut tap_x = (self.screen_size.0 / 2) as i32;
        let mut tap_y = 80i32; // Default to top area

        if let Ok(elements) = crate::driver::ios::accessibility::parse_ui_hierarchy(&ui_json) {
            for el in crate::driver::ios::accessibility::flatten_elements(&elements) {
                if let Some(el_type) = &el.element_type {
                    if el_type == "TextField" || el_type == "TextArea" {
                        let (cx, cy) = el.center();
                        tap_x = cx;
                        tap_y = cy;
                        break;
                    }
                }
            }
        }

        // Triple-tap to select all text
        for _ in 0..3 {
            idb::tap(&self.udid, tap_x, tap_y).await?;
            tokio::time::sleep(Duration::from_millis(80)).await;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Type space to replace selected text
        idb::input_text(&self.udid, " ").await?;
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Triple-tap again to select the space
        for _ in 0..3 {
            idb::tap(&self.udid, tap_x, tap_y).await?;
            tokio::time::sleep(Duration::from_millis(80)).await;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;

        self.invalidate_cache().await;
        Ok(())
    }

    async fn hide_keyboard(&self) -> Result<()> {
        if self.is_simulator {
            // Try pressing return to dismiss, common pattern
            let _ = idb::press_key(&self.udid, "XCUIKeyboardKeyReturn").await;

            // Alternative: tap outside the keyboard area
            let (_, height) = self.screen_size;
            let _ = idb::tap(&self.udid, 50, (height / 4) as i32).await;
        } else {
            let mut wda = self.wda_client.lock().await;
            if let Some(ref mut client) = *wda {
                // Press return key to dismiss keyboard
                let _ = client.press_key("RETURN").await;
            } else {
                let _ = idb::press_key(&self.udid, "XCUIKeyboardKeyReturn").await;
            }
        }

        self.invalidate_cache().await;
        Ok(())
    }

    async fn swipe(
        &self,
        direction: SwipeDirection,
        duration_ms: Option<u64>,
        from: Option<Selector>,
    ) -> Result<()> {
        let (width, height) = self.screen_size;

        // Determine swipe area
        let (area_left, area_top, area_w, area_h) = if let Some(selector) = from {
            if let Some(element) = self.find_element_internal(&selector).await? {
                let frame = &element.frame;
                (
                    frame.x as i32,
                    frame.y as i32,
                    frame.width as i32,
                    frame.height as i32,
                )
            } else {
                return Err(anyhow::anyhow!("Source element for swipe not found"));
            }
        } else {
            (0, 0, width as i32, height as i32)
        };

        // Calculate center of area
        let center_x = area_left + area_w / 2;
        let center_y = area_top + area_h / 2;

        // Use 15% margin relative to the AREA
        let margin_x = (area_w as f64 * 0.15) as i32;
        let margin_y = (area_h as f64 * 0.15) as i32;

        let (x1, y1, x2, y2) = match direction {
            SwipeDirection::Up => (
                center_x,
                area_top + area_h - margin_y,
                center_x,
                area_top + margin_y,
            ),
            SwipeDirection::Down => (
                center_x,
                area_top + margin_y,
                center_x,
                area_top + area_h - margin_y,
            ),
            SwipeDirection::Left => (
                area_left + area_w - margin_x,
                center_y,
                area_left + margin_x,
                center_y,
            ),
            SwipeDirection::Right => (
                area_left + margin_x,
                center_y,
                area_left + area_w - margin_x,
                center_y,
            ),
        };

        println!(
            "    {} Swiping {:?}: ({}, {}) -> ({}, {})",
            "ℹ".blue(),
            direction,
            x1,
            y1,
            x2,
            y2
        );

        if self.is_simulator {
            idb::swipe(&self.udid, x1, y1, x2, y2, duration_ms).await?;
        } else {
            let mut wda = self.wda_client.lock().await;
            if let Some(ref mut client) = *wda {
                client.swipe(x1, y1, x2, y2, duration_ms).await?;
            } else {
                idb::swipe(&self.udid, x1, y1, x2, y2, duration_ms).await?;
            }
        }
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
        let swipe_dir = direction.unwrap_or(SwipeDirection::Up);
        for _ in 0..max_scrolls {
            if self.is_visible(selector).await? {
                return Ok(true);
            }
            self.swipe(swipe_dir.clone(), Some(300), from.clone())
                .await?;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        Ok(false)
    }

    async fn is_visible(&self, selector: &Selector) -> Result<bool> {
        self.invalidate_cache().await;
        Ok(self.find_element(selector).await?.is_some())
    }

    async fn wait_for_element(&self, selector: &Selector, timeout_ms: u64) -> Result<bool> {
        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        while start.elapsed() < timeout {
            self.invalidate_cache().await;
            if self.find_element(selector).await?.is_some() {
                return Ok(true);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        Ok(false)
    }

    async fn wait_for_absence(&self, selector: &Selector, timeout_ms: u64) -> Result<bool> {
        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        while start.elapsed() < timeout {
            self.invalidate_cache().await;
            if self.find_element(selector).await?.is_none() {
                return Ok(true);
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        Ok(false)
    }

    async fn get_element_text(&self, selector: &Selector) -> Result<String> {
        let elements = self.get_ui_hierarchy().await?;

        // Helper to extract text from element
        let extract_text = |e: &IosElement| -> String {
            if let Some(val) = &e.value {
                if !val.is_empty() {
                    return val.clone();
                }
            }
            if let Some(lbl) = &e.label {
                if !lbl.is_empty() {
                    return lbl.clone();
                }
            }
            if let Some(ph) = &e.placeholder {
                if !ph.is_empty() {
                    return ph.clone();
                }
            }
            String::new()
        };

        if let Selector::Point { .. } = selector {
            return Ok(String::new());
        }

        let element = match selector {
            Selector::Text(text, index, _) => accessibility::find_by_text(&elements, text, *index),
            Selector::TextRegex(pattern, index) => {
                let regex = Regex::new(pattern).context("Invalid regex pattern")?;
                accessibility::find_by_text_regex(&elements, &regex, *index)
            }
            Selector::Id(id, index) => accessibility::find_by_id(&elements, id, *index),
            Selector::Type(element_type, index) => {
                accessibility::find_by_type(&elements, element_type, *index)
            }
            Selector::Placeholder(placeholder, index) => {
                accessibility::find_by_placeholder(&elements, placeholder, *index)
            }
            Selector::AccessibilityId(id) => accessibility::find_by_id(&elements, id, 0),
            Selector::Role(role, index) => accessibility::find_by_type(&elements, role, *index),
            Selector::Relative {
                target,
                anchor,
                direction,
                max_dist,
            } => self.find_relative_element(&elements, target, anchor, direction, max_dist),
            Selector::Image { .. } => None,
            Selector::HasChild { parent, child } => {
                let flat = accessibility::flatten_elements(&elements);
                let parent_candidates: Vec<_> = flat
                    .iter()
                    .filter(|e| e.visible && self.element_matches_selector(*e, parent))
                    .copied()
                    .collect();
                let child_candidates: Vec<_> = flat
                    .iter()
                    .filter(|e| e.visible && self.element_matches_selector(*e, child))
                    .copied()
                    .collect();

                let mut found = None;
                for p in parent_candidates {
                    for c in &child_candidates {
                        if p.frame.contains(&c.frame)
                            // p is &IosElement, c is &&IosElement
                            && !std::ptr::eq(p, *c)
                        {
                            found = Some(p);
                            break;
                        }
                    }
                    if found.is_some() {
                        break;
                    }
                }
                found
            }
            Selector::IdRegex(pattern, index) => {
                let regex = Regex::new(pattern).context("Invalid regex pattern")?;
                accessibility::find_by_id_regex(&elements, &regex, *index)
            }
            Selector::AnyClickable(index) => {
                let flat = accessibility::flatten_elements(&elements);
                let clickables: Vec<_> = flat
                    .into_iter()
                    .filter(|e| e.visible && e.enabled)
                    .collect();
                clickables.get(*index).copied()
            }
            _ => None,
        };

        if let Some(e) = element {
            Ok(extract_text(e))
        } else {
            Ok(String::new())
        }
    }

    async fn open_link(&self, url: &str, _app_id: Option<&str>) -> Result<()> {
        let _output = idb::open_url(&self.udid, url).await?;
        // Check output?
        self.invalidate_cache().await;
        Ok(())
    }

    async fn compare_screenshot(
        &self,
        reference_path: &Path,
        _tolerance_percent: f64,
    ) -> Result<f64> {
        // Take current screenshot
        let temp_path = format!("/tmp/ios_screenshot_{}.png", Uuid::new_v4());
        idb::screenshot(&self.udid, &temp_path).await?;

        // Load both images
        let current = image::open(&temp_path).context("Failed to open current screenshot")?;
        let reference = image::open(reference_path).context("Failed to open reference image")?;

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_path);

        // Compare dimensions
        if current.dimensions() != reference.dimensions() {
            return Ok(100.0); // 100% different if sizes don't match
        }

        // Pixel comparison
        let (width, height) = current.dimensions();
        let mut diff_count = 0u64;
        let total = (width * height) as u64;

        for y in 0..height {
            for x in 0..width {
                let p1 = current.get_pixel(x, y);
                let p2 = reference.get_pixel(x, y);
                if p1 != p2 {
                    diff_count += 1;
                }
            }
        }

        Ok((diff_count as f64 / total as f64) * 100.0)
    }

    async fn take_screenshot(&self, path: &str) -> Result<()> {
        idb::screenshot(&self.udid, path).await?;
        println!("{} Screenshot saved to: {}", "✓".green(), path);
        Ok(())
    }

    async fn back(&self) -> Result<()> {
        // iOS back gesture: swipe from left edge
        let (_, height) = self.screen_size;
        let center_y = height as i32 / 2;

        if self.is_simulator {
            idb::swipe(&self.udid, 5, center_y, 200, center_y, Some(200)).await?;
        } else {
            let mut wda = self.wda_client.lock().await;
            if let Some(ref mut client) = *wda {
                client.swipe(5, center_y, 200, center_y, Some(200)).await?;
            } else {
                idb::swipe(&self.udid, 5, center_y, 200, center_y, Some(200)).await?;
            }
        }
        self.invalidate_cache().await;
        Ok(())
    }

    async fn home(&self) -> Result<()> {
        if self.is_simulator {
            idb::press_button(&self.udid, "HOME").await?;
        } else {
            let mut wda = self.wda_client.lock().await;
            if let Some(ref mut client) = *wda {
                client.press_button("home").await?;
            } else {
                idb::press_button(&self.udid, "HOME").await?;
            }
        }
        self.invalidate_cache().await;
        Ok(())
    }

    async fn get_screen_size(&self) -> Result<(u32, u32)> {
        Ok(self.screen_size)
    }

    async fn dump_ui_hierarchy(&self) -> Result<String> {
        idb::describe_ui(&self.udid).await
    }

    async fn dump_logs(&self, limit: u32) -> Result<String> {
        idb::get_logs(&self.udid, limit).await
    }

    async fn tap_by_type_index(&self, element_type: &str, index: u32) -> Result<()> {
        let selector = Selector::Type(element_type.to_string(), index as usize);
        self.tap(&selector).await
    }

    async fn input_by_type_index(&self, element_type: &str, index: u32, text: &str) -> Result<()> {
        let selector = Selector::Type(element_type.to_string(), index as usize);
        self.tap(&selector).await?;
        tokio::time::sleep(Duration::from_millis(200)).await;
        self.input_text(text, false).await
    }

    async fn start_recording(&self, path: &str) -> Result<()> {
        let child = tokio::process::Command::new("idb")
            .args(&["record", "video", "--udid", &self.udid, path])
            .spawn()?;

        *self.recording_process.lock().await = Some(child);
        self.current_recording_path
            .lock()
            .await
            .replace(path.to_string());
        Ok(())
    }

    async fn stop_recording(&self) -> Result<()> {
        if let Some(mut child) = self.recording_process.lock().await.take() {
            // idb record video stops on SIGINT
            // Since child.kill() is SIGKILL, we should try to send SIGINT if possible.
            // On MacOS/Linux we can use kill -2
            if let Some(pid) = child.id() {
                let _ = std::process::Command::new("kill")
                    .args(&["-2", &pid.to_string()])
                    .output();
            } else {
                let _ = child.kill().await;
            }

            let _ = child.wait().await;

            if let Some(path) = self.current_recording_path.lock().await.take() {
                println!("  {} Saved iOS Recording: {}", "🎥".green(), path);
            }
        }
        Ok(())
    }

    async fn rotate_screen(&self, _mode: &str) -> Result<()> {
        if self.is_simulator {
            // Use AppleScript to rotate simulator
            // Requires Simulator app to be running
            println!(
                "      {} Rotating simulator via AppleScript...",
                "🔄".blue()
            );

            // Script to rotate left (Cmd+Left Arrow equivalent via menu)
            let script = r#"
                tell application "Simulator" to activate
                tell application "System Events" 
                    tell process "Simulator"
                        click menu item "Rotate Left" of menu "Device" of menu bar 1
                    end tell
                end tell
            "#;

            use std::process::Command;
            let output = Command::new("osascript").arg("-e").arg(script).output()?;

            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                // Don't fail the test, just warn, as this is brittle
                println!(
                    "      {} Failed to rotate simulator: {}",
                    "⚠️".yellow(),
                    err.trim()
                );
            } else {
                // Wait a bit for rotation animation
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
            Ok(())
        } else {
            // Physical device rotation via WDA is complex (needs orientation endpoint)
            // For now return error to be explicit
            anyhow::bail!(
                "rotate_screen not yet supported on physical iOS devices (requires WDA update)"
            );
        }
    }

    async fn press_key(&self, key: &str) -> Result<()> {
        match key.to_uppercase().as_str() {
            "HOME" | "VOLUME_UP" | "VOLUME_DOWN" | "LOCK" | "SIRI" => {
                idb::press_button(&self.udid, &key.to_uppercase()).await
            }
            _ => idb::press_key(&self.udid, key).await,
        }
    }

    async fn push_file(&self, source: &str, dest: &str) -> Result<()> {
        idb::push_file(&self.udid, source, dest).await
    }

    async fn pull_file(&self, source: &str, dest: &str) -> Result<()> {
        idb::pull_file(&self.udid, source, dest).await
    }

    async fn clear_app_data(&self, app_id: &str) -> Result<()> {
        // Just terminate for now
        idb::terminate_app(&self.udid, app_id, self.is_simulator).await
    }

    async fn set_clipboard(&self, text: &str) -> Result<()> {
        // Workaround: type text
        idb::input_text(&self.udid, text).await
    }

    async fn get_clipboard(&self) -> Result<String> {
        Err(anyhow::anyhow!("get_clipboard not supported on iOS"))
    }

    async fn get_pixel_color(&self, x: i32, y: i32) -> Result<(u8, u8, u8)> {
        // Take screenshot and extract pixel using common utility
        let temp_path = format!("/tmp/ios_pixel_{}.png", Uuid::new_v4());
        idb::screenshot(&self.udid, &temp_path).await?;

        let img = image::open(&temp_path).context("Failed to open screenshot for pixel color")?;
        let _ = std::fs::remove_file(&temp_path);

        Ok(common::get_pixel_from_image(&img, x as u32, y as u32))
    }

    async fn set_permissions(
        &self,
        app_id: &str,
        permissions: &HashMap<String, String>,
    ) -> Result<()> {
        if self.is_simulator {
            for (service, state) in permissions {
                let action = if state.eq_ignore_ascii_case("deny") {
                    "revoke"
                } else {
                    "grant"
                };

                let service_name = map_ios_permission(service);
                if service_name == "unknown" {
                    println!(
                        "  {} Warning: Unknown permission '{}', skipping",
                        "⚠".yellow(),
                        service
                    );
                    continue;
                }

                let status = std::process::Command::new("xcrun")
                    .args(&[
                        "simctl",
                        "privacy",
                        &self.udid,
                        action,
                        service_name,
                        app_id,
                    ])
                    .status()?;

                if !status.success() {
                    println!(
                        "  {} Failed to {} permission {}",
                        "⚠".yellow(),
                        action,
                        service
                    );
                }
            }
        } else {
            println!(
                "  {} Warning: setPermissions not supported on physical iOS devices",
                "⚠".yellow()
            );
        }
        Ok(())
    }

    async fn clear_keychain(&self) -> Result<()> {
        if self.is_simulator {
            // For simulator: delete keychain database files directly
            let keychain_path = format!(
                "{}/Library/Developer/CoreSimulator/Devices/{}/data/Library/Keychains",
                std::env::var("HOME").unwrap_or_else(|_| "/Users".to_string()),
                self.udid
            );

            if std::path::Path::new(&keychain_path).exists() {
                // Delete keychain database files
                let _ = tokio::process::Command::new("sh")
                    .args(&[
                        "-c",
                        &format!("rm -f {}/*.db* 2>/dev/null || true", keychain_path),
                    ])
                    .output()
                    .await;
            }
        }
        // For physical device: clearKeychain is a no-op (would require app reinstall)
        // Handled silently - the display_name shows clearKeychain was requested

        Ok(())
    }

    // New Commands Implementation

    async fn set_network_connection(&self, _wifi: Option<bool>, _data: Option<bool>) -> Result<()> {
        println!(
            "  {} set_network_connection not supported on iOS directly. Use standard Library/Network Link Conditioner manually.",
            "⚠️".yellow()
        );
        Ok(())
    }

    async fn toggle_airplane_mode(&self) -> Result<()> {
        println!(
            "  {} toggle_airplane_mode not supported on iOS simulators/devices via public API.",
            "⚠️".yellow()
        );
        Ok(())
    }

    async fn open_notifications(&self) -> Result<()> {
        // Swipe down from top center
        let (w, _h) = self.screen_size;
        let center_x = (w / 2) as i32;
        // Start very top (0) to 500
        idb::swipe(&self.udid, center_x, 0, center_x, 500, Some(300)).await
    }

    async fn open_quick_settings(&self) -> Result<()> {
        // Control Center: Swipe down from top-right
        let (w, _h) = self.screen_size;
        let start_x = (w as i32) - 10;
        idb::swipe(&self.udid, start_x, 0, start_x, 400, Some(400)).await
    }

    async fn set_volume(&self, _level: u8) -> Result<()> {
        println!("  {} set_volume not supported on iOS", "⚠️".yellow());
        Ok(())
    }

    async fn lock_device(&self) -> Result<()> {
        idb::press_button(&self.udid, "LOCK").await
    }

    async fn unlock_device(&self) -> Result<()> {
        // Wake up
        idb::press_button(&self.udid, "HOME").await?;
        // If it was locked, this might wake it. If on lock screen, might need swipe up?
        // Let's try to swipe up from bottom just in case
        let (w, h) = self.screen_size;
        let center_x = (w / 2) as i32;
        let bottom_y = (h as i32) - 10;
        let mid_y = (h / 2) as i32;
        idb::swipe(&self.udid, center_x, bottom_y, center_x, mid_y, Some(300)).await?;
        Ok(())
    }

    async fn install_app(&self, path: &str) -> Result<()> {
        // Resolve relative path if needed? Context usually resolves it.
        // But driver receives path string.
        if !std::path::Path::new(path).exists() {
            anyhow::bail!("App file not found: {}", path);
        }
        println!("  {} Installing app: {}", "⬇".cyan(), path);
        idb::install_app(&self.udid, path).await
    }

    async fn uninstall_app(&self, app_id: &str) -> Result<()> {
        println!("  {} Uninstalling app: {}", "🗑".cyan(), app_id);
        idb::uninstall_app(&self.udid, app_id).await
    }

    async fn background_app(&self, app_id_opt: Option<&str>, duration_ms: u64) -> Result<()> {
        // Press Home
        idb::press_button(&self.udid, "HOME").await?;

        // Wait
        tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;

        // Resume
        if let Some(app_id) = app_id_opt {
            self.launch_app(app_id, false).await?;
        } else {
            println!("  {} No app_id provided to resume", "⚠".yellow());
        }
        Ok(())
    }

    async fn set_orientation(&self, _mode: crate::parser::types::Orientation) -> Result<()> {
        println!(
             "  {} set_orientation not reliably supported on iOS Simulators via idb (requires private APIs or XCUI)", 
             "⚠️".yellow()
        );
        Ok(())
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
        use rand::Rng;
        use rand::SeedableRng;

        if !self.is_simulator {
            println!(
                "  {} Mock location is only supported on iOS Simulator",
                "⚠".yellow()
            );
            return Ok(());
        }

        if points.is_empty() {
            anyhow::bail!("No GPS points provided for mock location");
        }

        let instance_name = name.clone().unwrap_or_default();
        println!(
            "  {} Starting iOS mock location '{}' with {} waypoints",
            "📍".green(),
            if instance_name.is_empty() {
                "default"
            } else {
                &instance_name
            },
            points.len()
        );

        let udid = self.udid.clone();
        let interval = std::time::Duration::from_millis(interval_ms);

        if let Some(speed) = speed_kmh {
            let mode_str = match speed_mode {
                SpeedMode::Linear => "Linear",
                SpeedMode::Noise => &format!("Noise ±{:.1}", speed_noise.unwrap_or(5.0)),
            };
            println!(
                "  {} Using speed: {} km/h ({})",
                "🚗".cyan(),
                speed,
                mode_str
            );
        }

        let points_clone = points.clone();
        let mock_states = self.mock_states.clone();
        let instance_key = instance_name.clone();

        // Initialize state
        {
            let mut states = mock_states.lock().await;
            let state = states
                .entry(instance_key.clone())
                .or_insert_with(IosMockLocationState::default);
            state.is_running = true;
            state.finished = false;
            state.paused = false;
            state.speed = speed_kmh;
            state.speed_mode = speed_mode.clone();
            state.speed_noise = speed_noise;
        }

        tokio::spawn(async move {
            let mut rng = rand::rngs::StdRng::from_entropy();

            'outer: loop {
                for (i, point) in points_clone.iter().enumerate() {
                    let lat = point.lat;
                    let lon = point.lon;

                    // Check for pause
                    loop {
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
                    let (current_speed, current_mode, current_noise) = {
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

                    // Set location using simctl
                    let _ = tokio::process::Command::new("xcrun")
                        .args(&[
                            "simctl",
                            "location",
                            &udid,
                            "set",
                            &format!("{},{}", lat, lon),
                        ])
                        .output()
                        .await;

                    if i < points_clone.len() - 1 {
                        let next_point = &points_clone[i + 1];
                        let delay = if let Some(base_speed) = current_speed {
                            // Apply noise if enabled
                            let effective_speed = match current_mode {
                                SpeedMode::Linear => base_speed,
                                SpeedMode::Noise => {
                                    let noise_range = current_noise.unwrap_or(5.0);
                                    let noise: f64 = rng.gen_range(-noise_range..noise_range);
                                    (base_speed + noise).max(1.0)
                                }
                            };

                            let dist_m =
                                haversine_distance_ios(lat, lon, next_point.lat, next_point.lon);
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
            println!("  {} iOS mock location playback completed", "✅".green());
        });

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        Ok(())
    }

    async fn stop_mock_location(&self) -> Result<()> {
        if self.is_simulator {
            let _ = tokio::process::Command::new("xcrun")
                .args(&["simctl", "location", &self.udid, "clear"])
                .output()
                .await;
        }
        println!("  {} iOS mock location stopped", "📍".yellow());
        Ok(())
    }

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
                let dist = haversine_distance_ios(c_lat, c_lon, lat, lon);
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
            "  {} Waiting for iOS mock location '{}' completion...",
            "⏳".cyan(),
            if instance_key.is_empty() {
                "default"
            } else {
                &instance_key
            }
        );

        loop {
            if let Some(t) = timeout_ms {
                if start.elapsed() > Duration::from_millis(t) {
                    anyhow::bail!("Timeout waiting for mock location completion");
                }
            }

            {
                let states = self.mock_states.lock().await;
                if let Some(state) = states.get(&instance_key) {
                    if state.finished {
                        println!("  {} iOS mock location completed", "✅".green());
                        return Ok(());
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
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
            println!("  {} Updating iOS mock speed to {} km/h", "🚗".cyan(), s);
            state.speed = Some(s);
        }

        if let Some(mode) = speed_mode {
            state.speed_mode = mode;
        }

        if let Some(noise) = speed_noise {
            state.speed_noise = Some(noise);
        }

        if pause == Some(true) {
            println!("  {} Pausing iOS mock location", "⏸".yellow());
            state.paused = true;
        }

        if resume == Some(true) {
            println!("  {} Resuming iOS mock location", "▶".green());
            state.paused = false;
        }

        Ok(())
    }

    async fn start_profiling(
        &self,
        _params: Option<crate::parser::types::StartProfilingParams>,
    ) -> Result<()> {
        // For Simulator: No setup needed for basic ps sampling
        Ok(())
    }

    async fn stop_profiling(&self) -> Result<()> {
        Ok(())
    }

    async fn get_performance_metrics(&self) -> Result<std::collections::HashMap<String, f64>> {
        let mut metrics = std::collections::HashMap::new();

        if self.is_simulator {
            // Use xcrun simctl spawn <udid> ps -o %cpu,%mem,comm
            // We need to identify the app process. We don't track the PID, so we search by name?
            // Usually the app process name matches the executable inside the bundle.
            // For now, let's try to get all processes and find the one consuming most CPU that isn't system?
            // Or better: users usually only run one app under test.
            // Let's rely on StartProfilingParams containing the package/process name, or infer it?
            // PlatformDriver doesn't store current app ID.
            // Let's implement a heuristic: get highest CPU user process.

            let output = tokio::process::Command::new("xcrun")
                .args(&["simctl", "spawn", &self.udid, "ps", "aux"])
                .output()
                .await?;

            let stdout = String::from_utf8_lossy(&output.stdout);

            // Output format: USER PID %CPU %MEM VSZ RSS TT STAT STARTED TIME COMMAND
            // We look for the line with highest CPU that is not a system process
            // Or we assume the app under test is the last launched?
            // Let's look for known app directory path in COMMAND? /Containers/Bundle/Application/...

            let mut max_cpu = 0.0;
            let mut found_mem = 0.0;
            let mut found_cmd = String::new();

            for line in stdout.lines().skip(1) {
                // Skip header
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 11 {
                    continue;
                }

                // USER, PID, %CPU, %MEM
                let cpu: f64 = parts[2].parse().unwrap_or(0.0);
                let _mem: f64 = parts[3].parse().unwrap_or(0.0); // %MEM
                                                                 // RSS is parts[5] (in KB usually)
                let rss: f64 = parts[5].parse().unwrap_or(0.0);

                let cmd = parts[10..].join(" ");

                if cmd.contains("/Containers/Bundle/Application/") && !cmd.contains("xctest") {
                    // Likely our app
                    if cpu >= max_cpu {
                        max_cpu = cpu;
                        found_mem = rss / 1024.0; // KB -> MB
                        found_cmd = cmd.clone();
                    }
                }
            }

            if !found_cmd.is_empty() {
                metrics.insert("cpu".to_string(), max_cpu);
                metrics.insert("memory".to_string(), found_mem);
                // metrics.insert("process".to_string(), ...); // Metric values must be f64
            }
        } else {
            // Real Devicce: idb doesn't expose metrics easily via CLI
            // TODO: integrate with instruments
        }

        Ok(metrics)
    }

    async fn set_cpu_throttling(&self, _rate: f64) -> Result<()> {
        // Not supported on iOS simulators/devices easily
        println!("  {} CPU throttling not supported on iOS", "⚠️".yellow());
        Ok(())
    }

    async fn set_network_conditions(&self, _profile: &str) -> Result<()> {
        // Network link conditioner is system-wide, hard to control via CLI without external tools
        println!(
            "  {} Network emulation not supported on iOS directly",
            "⚠️".yellow()
        );
        Ok(())
    }

    async fn set_locale(&self, locale: &str) -> Result<()> {
        if self.is_simulator {
            // iOS Simulator: use simctl to set AppleLanguages
            let output = std::process::Command::new("xcrun")
                .args(&[
                    "simctl",
                    "spawn",
                    &self.udid,
                    "defaults",
                    "write",
                    "Apple Global Domain",
                    "AppleLanguages",
                    &format!("({})", locale),
                ])
                .output();

            match output {
                Ok(o) if o.status.success() => {
                    println!(
                        "  {} Set iOS simulator locale to: {} (restart app for effect)",
                        "🌐".green(),
                        locale
                    );
                }
                _ => {
                    println!(
                        "  {} iOS locale change may require app restart",
                        "⚠".yellow()
                    );
                }
            }
            Ok(())
        } else {
            anyhow::bail!("set_locale only works on iOS Simulator, not physical devices")
        }
    }
}

/// Map iOS permissions to simctl service names
fn map_ios_permission(p: &str) -> &str {
    match p.to_lowercase().as_str() {
        "all" => "all",
        "calendar" => "calendar",
        "contacts" => "contacts",
        "contacts-limited" => "contacts-limited",
        "location" | "gps" | "fine_location" | "coarse_location" => "location",
        "location-always" | "background_location" => "location-always",
        "photos" | "gallery" | "read_external_storage" => "photos",
        "photos-add" | "write_external_storage" => "photos-add",
        "microphone" | "record_audio" => "microphone",
        "camera" => "camera",
        "media-library" | "medialibrary" => "media-library",
        "motion" | "sensors" => "motion",
        "reminders" => "reminders",
        "siri" => "siri",
        "faceid" | "face-id" => "faceid",
        "homekit" => "homekit",
        "health" => "health",
        _ => "unknown",
    }
}

/// Calculate Haversine distance between two points in meters (for iOS)
fn haversine_distance_ios(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371000.0;
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    r * c
}

/// Calculate initial bearing from point 1 to point 2 in degrees (0-360)
#[allow(dead_code)]
fn calculate_bearing_ios(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let d_lon = (lon2 - lon1).to_radians();

    let x = d_lon.sin() * lat2_rad.cos();
    let y = lat1_rad.cos() * lat2_rad.sin() - lat1_rad.sin() * lat2_rad.cos() * d_lon.cos();

    let bearing_rad = x.atan2(y);
    let bearing_deg = bearing_rad.to_degrees();

    (bearing_deg + 360.0) % 360.0
}
