//! iOS Driver implementation
//!
//! UI automation (tap/swipe/type/hierarchy/screenshot) goes entirely through the
//! `lm-ios-tester` on-device agent (native XCUITest, see `agent.rs`) - both simulators
//! and real devices. Process/file/lifecycle operations (install/uninstall/push/pull)
//! use native `xcrun devicectl`/`xcrun simctl` (see `devicectl.rs`). idb and
//! WebDriverAgent are not used anywhere in this driver.

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
use super::agent::AgentClient;
use super::devicectl;
use crate::driver::common;
use crate::driver::image_matcher::{find_template, ImageRegion, MatchConfig};
use crate::driver::traits::{PlatformDriver, Selector, SwipeDirection};
use crate::parser::types::SpeedMode;
use colored::Colorize;
use image::GenericImageView;
use std::collections::HashMap as StdHashMap;

/// iOS driver implementation - UI automation via the on-device `lm-ios-tester` agent,
/// process/file/lifecycle operations via native `xcrun devicectl`/`xcrun simctl`.
pub struct IosDriver {
    /// Device UDID
    udid: String,
    /// Device name (used for logging)
    #[allow(dead_code)]
    device_name: String,
    /// Whether this is a simulator (vs physical device)
    is_simulator: bool,
    /// Video recording process
    recording_process: Arc<Mutex<Option<tokio::process::Child>>>,
    /// Current recording output path
    current_recording_path: Arc<Mutex<Option<String>>>,
    /// Screen dimensions
    screen_size: (u32, u32),
    /// Mock location states keyed by name ("" for default)
    mock_states: Arc<Mutex<StdHashMap<String, IosMockLocationState>>>,
    /// Last coordinate a selector-based `tap()` resolved to - lets `input_text`'s
    /// retry loop re-tap the same spot (see its doc comment) without needing to
    /// re-run selector resolution, mirroring the Android driver's identical field.
    last_tap_point: Arc<Mutex<Option<(i32, i32)>>>,
    /// lm-ios-tester agent client - the sole UI-automation path (simulator and real
    /// device alike). `None` only when the agent couldn't be started at all, in which
    /// case UI-automation methods return an explicit error rather than silently no-op.
    /// Previously, `None` here was permanent for the rest of the driver's lifetime -
    /// `ensure_agent_running` was only ever called once, at construction. On a real
    /// device the agent launches via `xcodebuild test-without-building` (an XCTest UI
    /// target), which can take 20-90s and can fail on transient issues (device locked,
    /// signing/auth timeout) unrelated to whether the agent would work if retried a
    /// moment later - a single bad launch attempt shouldn't silently disable UI
    /// automation for an entire test run. See `ensure_agent_ready` below.
    agent_client: Arc<Mutex<Option<AgentClient>>>,
    /// Per-UDID host port for `iproxy`/the agent client (see `agent::agent_port_for`) -
    /// stored so `ensure_agent_ready` can retry `ensure_agent_running` with the same
    /// port later without re-deriving it.
    agent_port: u16,
    /// Wall-clock time of the most recent `ensure_agent_running` retry attempt (`None`
    /// until the first one). Rate-limits retries after `agent_client` is `None`: retrying
    /// immediately on every command would mean hammering `xcodebuild test-without-
    /// building` (an expensive, multi-second-at-best operation) on every single UI
    /// action while the agent is down, but never retrying is the bug described above.
    agent_last_attempt: Arc<Mutex<Option<Instant>>>,
    /// Bundle id of the most recently `launch_app`-ed app - the agent's `hierarchy`
    /// command needs an explicit target bundle id (defaults to SpringBoard otherwise).
    current_app_id: Arc<Mutex<Option<String>>>,
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
        let targets = devicectl::list_targets().await?;

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
        let mut screen_size = (390, 844);

        // The agent (native XCTest) works identically on simulators and real devices -
        // `agent_setup::ensure_agent_running` launches `xcodebuild test-without-building`
        // against the target's UDID either way, and the port-forward step it also tries
        // (`iproxy`, USB-only) simply no-ops harmlessly for a simulator UDID since the
        // agent is already reachable on localhost in that case.
        // Per-UDID host port (not the shared DEFAULT_AGENT_PORT) - a fixed port shared by
        // every device meant a second connected iOS device's `iproxy` could fail to bind
        // or ambiguously share the first device's tunnel, the same class of cross-device
        // bug already found and fixed for Android's `adb forward`. See
        // `agent::agent_port_for`'s doc comment.
        let port = if is_simulator {
            super::agent::DEFAULT_AGENT_PORT
        } else {
            super::agent::agent_port_for(&target.udid)
        };
        let agent_client = if super::agent_setup::ensure_agent_running(&target.udid, port).await {
            Some(AgentClient::new("localhost", port))
        } else {
            eprintln!(
                "{} lm-ios-tester agent could not be started - UI automation (tap/swipe/type/\
                 hierarchy/screenshot) will fail until it is",
                "⚠".yellow()
            );
            None
        };

        if let Some(ref agent) = agent_client {
            if let Some(size) = agent.get_screen_size().await {
                screen_size = size;
            }
        }

        Ok(Self {
            udid: target.udid,
            device_name: target.name,
            is_simulator,
            recording_process: Arc::new(Mutex::new(None)),
            current_recording_path: Arc::new(Mutex::new(None)),
            screen_size,
            mock_states: Arc::new(Mutex::new(StdHashMap::new())),
            last_tap_point: Arc::new(Mutex::new(None)),
            agent_client: Arc::new(Mutex::new(agent_client)),
            agent_port: port,
            // Seed with "now" (not None) - we just attempted a launch above, so the
            // cooldown in `ensure_agent_ready` should count from this attempt, not allow
            // an immediate second attempt on the very next command.
            agent_last_attempt: Arc::new(Mutex::new(Some(Instant::now()))),
            current_app_id: Arc::new(Mutex::new(None)),
            ocr_engine: tokio::sync::OnceCell::new(),
        })
    }

    /// Ensure `agent_client` is populated, retrying `ensure_agent_running` (bounded by
    /// `AGENT_RETRY_COOLDOWN`) if a previous attempt failed - see `agent_client`'s doc
    /// comment for why this exists. Returns whether an agent is available after this
    /// call. Mirrors the equivalent fix in the Android driver
    /// (`AndroidDriver::send_mirror_command`'s cooldown-gated `init_session` retry).
    async fn ensure_agent_ready(&self) -> bool {
        {
            let guard = self.agent_client.lock().await;
            if guard.is_some() {
                return true;
            }
        }

        // Longer cooldown than Android's (5s): a failed iOS agent launch means retrying
        // `xcodebuild test-without-building` end to end, which is itself already a
        // multi-second-to-tens-of-seconds operation (`ensure_agent_running` waits up to
        // 60s for it) - retrying that too eagerly would make a genuinely-down agent add
        // significant latency to every command in the meantime.
        const AGENT_RETRY_COOLDOWN: Duration = Duration::from_secs(20);
        let should_attempt = {
            let last = *self.agent_last_attempt.lock().await;
            match last {
                Some(t) if t.elapsed() < AGENT_RETRY_COOLDOWN => false,
                _ => true,
            }
        };
        if !should_attempt {
            return false;
        }
        *self.agent_last_attempt.lock().await = Some(Instant::now());

        if super::agent_setup::ensure_agent_running(&self.udid, self.agent_port).await {
            let mut guard = self.agent_client.lock().await;
            *guard = Some(AgentClient::new("localhost", self.agent_port));
            true
        } else {
            false
        }
    }

    /// Clear an app's on-disk state and privacy permissions. Simulator-only:
    /// `xcrun simctl ...` (Simulator Control) only ever targets simulators - on a real
    /// device these calls fail immediately, so this used to be silently swallowed and
    /// `clearState: true` did nothing on physical hardware (confirmed: an already-logged-
    /// in app stayed logged in across `launchApp(clearState: true)`). No idb-level
    /// equivalent exists for a real device short of a full app uninstall/reinstall
    /// (needs the app's .ipa/.app bundle on hand, not available here) - warn instead of
    /// silently no-op'ing, so a flow relying on this doesn't fail confusingly several
    /// steps later with no indication why. Shared by both `launch_app`'s `clearState`
    /// path and the separate `clear_app_data` trait method (the executor calls the
    /// latter, not `launch_app`'s clear-state branch, whenever a flow's `launchApp` step
    /// combines `clearState` with `permissions` - both call paths need the real logic).
    async fn clear_app_state_impl(&self, bundle_id: &str) {
        if self.is_simulator {
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
                                let _ = tokio::process::Command::new("sh")
                                    .args(&[
                                        "-c",
                                        &format!("rm -rf {}/* 2>/dev/null || true", folder_path),
                                    ])
                                    .output()
                                    .await;
                            }
                        }
                    }
                }
            }

            let _ = tokio::process::Command::new("xcrun")
                .args(&["simctl", "privacy", &self.udid, "reset", "all", bundle_id])
                .output()
                .await;
        } else {
            println!(
                "  {} clearState not supported on physical iOS devices (simctl is \
                 simulator-only; a real device needs a full app uninstall/reinstall \
                 instead)",
                "⚠".yellow()
            );
        }
    }

    /// Get the UI hierarchy - always a fresh dump, never cached.
    ///
    /// This used to cache the last dump for up to 500ms. Removed for the same reason
    /// as the equivalent Android driver cache (see that `get_ui_hierarchy`'s doc
    /// comment for the full, live-confirmed root cause on Flutter apps): a hierarchy
    /// dumped for one check can land inside a cache's TTL window while still
    /// reflecting stale semantics from before a navigation/state change, so a
    /// subsequent selector resolution (e.g. a tap's coordinates) can silently use
    /// wrong data. Not independently reproduced against a physical iOS device in this
    /// pass (no device was connected), but the same caching assumption
    /// ("invalidate-on-mutating-action keeps it honest") applies here too, and a
    /// single round trip via the agent is already fast (~100ms) - not worth caching.
    async fn get_ui_hierarchy(&self) -> Result<Vec<IosElement>> {
        // Single round trip via the lm-ios-tester agent (`-[XCUIElement
        // snapshotWithError:]`, ~100ms measured) - the only hierarchy source now that
        // idb/WDA are gone. No fallback: if the agent isn't reachable, error out
        // explicitly instead of returning a stale/empty hierarchy.
        let json_output = self
            .try_agent_hierarchy_dump()
            .await
            .ok_or_else(|| anyhow::anyhow!("lm-ios-tester agent is not reachable - cannot read UI hierarchy"))?;
        accessibility::parse_ui_hierarchy(&json_output)
    }

    /// Try the lm-ios-tester agent's `hierarchy` command. Returns `None` if there's no
    /// agent client (simulator, or the agent failed to start) or the request fails.
    async fn try_agent_hierarchy_dump(&self) -> Option<String> {
        self.ensure_agent_ready().await;
        let guard = self.agent_client.lock().await;
        let agent = guard.as_ref()?;
        let bundle_id = self.current_app_id.lock().await.clone().unwrap_or_default();
        let data = agent.hierarchy(&bundle_id).await?;
        serde_json::to_string(&data).ok()
    }

    async fn try_agent_tap(&self, x: i32, y: i32) -> bool {
        self.ensure_agent_ready().await;
        let guard = self.agent_client.lock().await;
        match guard.as_ref() {
            Some(agent) => agent.tap(x as f64, y as f64).await,
            None => false,
        }
    }

    async fn try_agent_long_press(&self, x: i32, y: i32, duration_ms: u64) -> bool {
        self.ensure_agent_ready().await;
        let guard = self.agent_client.lock().await;
        match guard.as_ref() {
            Some(agent) => agent.long_press(x as f64, y as f64, duration_ms).await,
            None => false,
        }
    }

    async fn try_agent_double_tap(&self, x: i32, y: i32) -> bool {
        self.ensure_agent_ready().await;
        let guard = self.agent_client.lock().await;
        match guard.as_ref() {
            Some(agent) => agent.double_tap(x as f64, y as f64).await,
            None => false,
        }
    }

    async fn try_agent_swipe(&self, x1: i32, y1: i32, x2: i32, y2: i32, duration_ms: u64) -> bool {
        self.ensure_agent_ready().await;
        let guard = self.agent_client.lock().await;
        match guard.as_ref() {
            Some(agent) => {
                agent
                    .swipe(x1 as f64, y1 as f64, x2 as f64, y2 as f64, duration_ms)
                    .await
            }
            None => false,
        }
    }

    async fn try_agent_type_text(&self, text: &str) -> bool {
        self.ensure_agent_ready().await;
        let guard = self.agent_client.lock().await;
        match guard.as_ref() {
            Some(agent) => agent.type_text(text).await,
            None => false,
        }
    }

    async fn try_agent_erase_text(&self, count: u32) -> bool {
        self.ensure_agent_ready().await;
        let guard = self.agent_client.lock().await;
        match guard.as_ref() {
            Some(agent) => agent.erase_text(count).await,
            None => false,
        }
    }

    async fn try_agent_press_key(&self, key: &str) -> bool {
        self.ensure_agent_ready().await;
        let guard = self.agent_client.lock().await;
        match guard.as_ref() {
            Some(agent) => agent.press_key(key).await,
            None => false,
        }
    }

    async fn try_agent_press_button(&self, name: &str) -> bool {
        self.ensure_agent_ready().await;
        let guard = self.agent_client.lock().await;
        match guard.as_ref() {
            Some(agent) => agent.press_button(name).await,
            None => false,
        }
    }

    async fn try_agent_screenshot(&self, path: &str) -> bool {
        self.ensure_agent_ready().await;
        let guard = self.agent_client.lock().await;
        let Some(agent) = guard.as_ref() else {
            return false;
        };
        let Some(b64) = agent.screenshot_base64().await else {
            return false;
        };
        use base64::Engine;
        let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&b64) else {
            return false;
        };
        std::fs::write(path, bytes).is_ok()
    }

    /// Screenshot capture used throughout the driver (OCR, image matching, pixel color,
    /// screenshot comparison, `take_screenshot`). Agent first (works on simulator and
    /// real device alike); `xcrun simctl io screenshot` as a simulator-only fallback
    /// (CoreSimulator talks straight to the simulator's window server, no XCTest agent
    /// needed) - no equivalent native fallback exists for a real device.
    async fn capture_screenshot(&self, path: &str) -> Result<()> {
        if self.try_agent_screenshot(path).await {
            return Ok(());
        }
        if self.is_simulator {
            devicectl::screenshot_simulator(&self.udid, path).await?;
            return Ok(());
        }
        anyhow::bail!("lm-ios-tester agent is not reachable - cannot capture screenshot")
    }

    /// Raw hierarchy JSON via the agent, erroring out (not silently degrading) when the
    /// agent isn't reachable - used by helpers that need to inspect the tree directly
    /// (paste-menu detection, text-field discovery) rather than through `get_ui_hierarchy`'s
    /// cache.
    async fn hierarchy_json(&self) -> Result<String> {
        self.try_agent_hierarchy_dump()
            .await
            .ok_or_else(|| anyhow::anyhow!("lm-ios-tester agent is not reachable - cannot read UI hierarchy"))
    }

    async fn agent_tap(&self, x: i32, y: i32) -> Result<()> {
        if self.try_agent_tap(x, y).await {
            Ok(())
        } else {
            anyhow::bail!("lm-ios-tester agent is not reachable - cannot tap")
        }
    }

    async fn agent_long_press(&self, x: i32, y: i32, duration_ms: u64) -> Result<()> {
        if self.try_agent_long_press(x, y, duration_ms).await {
            Ok(())
        } else {
            anyhow::bail!("lm-ios-tester agent is not reachable - cannot long-press")
        }
    }

    async fn agent_swipe(&self, x1: i32, y1: i32, x2: i32, y2: i32, duration_ms: u64) -> Result<()> {
        if self.try_agent_swipe(x1, y1, x2, y2, duration_ms).await {
            Ok(())
        } else {
            anyhow::bail!("lm-ios-tester agent is not reachable - cannot swipe")
        }
    }

    async fn agent_press_button(&self, name: &str) -> Result<()> {
        if self.try_agent_press_button(name).await {
            Ok(())
        } else {
            anyhow::bail!("lm-ios-tester agent is not reachable - cannot press '{}'", name)
        }
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
        self.capture_screenshot(&screenshot_path_str).await?;
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
        self.capture_screenshot(&screenshot_path_str).await?;
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
        let ui_json = self.hierarchy_json().await?;
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
        self.agent_tap(tap_x, tap_y).await?;
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Check if "Paste" is visible immediately
        if self.find_paste_button().await?.is_none() {
            // Tap again (sometimes toggle menu)
            println!("    {} Tapping again to reveal menu...", "ℹ".blue());
            self.agent_tap(tap_x, tap_y).await?;
            tokio::time::sleep(Duration::from_millis(700)).await;
        }

        // If still not visible, try long press
        if self.find_paste_button().await?.is_none() {
            println!("    {} Long pressing to reveal menu...", "ℹ".blue());
            self.agent_long_press(tap_x, tap_y, 1000).await?;
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
            self.agent_tap(px, py).await?;
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
        let ui_json = self.hierarchy_json().await?;
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

    /// Best-effort: does any element in a fresh hierarchy snapshot report a `value`
    /// reflecting the text just typed? There's no explicit "is this element focused"
    /// flag in the agent's hierarchy schema (`hierarchyDictForSnapshot` in
    /// `LumiCommandHandler.m` doesn't expose one) to check *just* the focused field
    /// the way the equivalent Android verification does, so this scans every
    /// element's value instead - looser, but still catches the actual failure mode
    /// (nothing anywhere reflects the typed text) without needing an agent-side
    /// schema change to fix live.
    ///
    /// `SecureTextField`s need separate handling: their `value` is a run of masking
    /// characters (`•`, confirmed live - a real device reported
    /// `"••••••••••••••••••••••••••••••••••••••••••"` for a password field), never
    /// the literal typed text, so `contains(text)` can never match one - every write
    /// into a secure field was failing this check even when it had genuinely
    /// succeeded (confirmed live: same real device, same field - `type_text`
    /// reported success AND the masked value's length grew, but this function still
    /// said "not landed" and the retry loop above kept re-typing on top of what was
    /// already there before finally timing out). Length is the only signal a masked
    /// value can give, so for those elements specifically, accept "the masked value
    /// is at least as long as what we just typed" as landed.
    async fn verify_text_landed(&self, text: &str) -> bool {
        let Ok(json) = self.hierarchy_json().await else {
            return true; // Inconclusive (e.g. a transient dump failure) - don't fail the command over that.
        };
        let Ok(elements) = accessibility::parse_ui_hierarchy(&json) else {
            return true;
        };
        accessibility::flatten_elements(&elements).iter().any(|el| {
            let Some(v) = el.value.as_deref() else { return false };
            if v.contains(text) {
                return true;
            }
            let is_secure = el
                .element_type
                .as_deref()
                .map(|t| t.to_lowercase().contains("secure"))
                .unwrap_or(false);
            is_secure && !text.is_empty() && v.chars().count() >= text.chars().count()
        })
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

    async fn set_current_app_id(&self, app_id: &str) {
        // Only fill in if unset - don't clobber the real bundle id a `launchApp`
        // (this session or a prior one, e.g. a device left on-screen between test
        // runs) already recorded with just the flow header's declared appId, which
        // could be stale if the flow ever switches apps mid-run.
        let mut current = self.current_app_id.lock().await;
        if current.is_none() {
            *current = Some(app_id.to_string());
        }
    }

    async fn launch_app(&self, bundle_id: &str, clear_state: bool) -> Result<()> {
        *self.current_app_id.lock().await = Some(bundle_id.to_string());

        // The agent's `launch_app`/`terminate_app` use `[XCUIApplication terminate]`/
        // `launch` (native XCTest) - the reliable path on both simulator and real
        // device. `devicectl`/`simctl process launch` is the fallback when the agent
        // itself isn't reachable (agent not started) - it can restart the process but
        // can't drive the app afterward, so UI automation will still fail until the
        // agent comes back.
        self.ensure_agent_ready().await;
        let agent_launched = {
            let guard = self.agent_client.lock().await;
            match guard.as_ref() {
                Some(agent) => {
                    agent.terminate_app(bundle_id).await;
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    Some(agent.launch_app(bundle_id).await)
                }
                None => None,
            }
        };

        if agent_launched.is_none() {
            let _ = devicectl::terminate_app(&self.udid, bundle_id, self.is_simulator).await;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        if clear_state {
            self.clear_app_state_impl(bundle_id).await;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        if agent_launched.is_none() {
            devicectl::launch_app(&self.udid, bundle_id, self.is_simulator).await?;
        }

        // Wait longer for app to fully stabilize (especially after clear state)
        let wait_time = if clear_state { 2000 } else { 1000 };
        tokio::time::sleep(Duration::from_millis(wait_time)).await;

        Ok(())
    }

    async fn stop_app(&self, bundle_id: &str) -> Result<()> {
        self.ensure_agent_ready().await;
        let agent_ok = {
            let guard = self.agent_client.lock().await;
            match guard.as_ref() {
                Some(agent) => Some(agent.terminate_app(bundle_id).await),
                None => None,
            }
        };
        if agent_ok.is_none() {
            devicectl::terminate_app(&self.udid, bundle_id, self.is_simulator).await?;
        }
        Ok(())
    }

    async fn tap(&self, selector: &Selector) -> Result<()> {
        let pos = self
            .find_element(selector)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Element not found for tap: {:?}", selector))?;

        *self.last_tap_point.lock().await = Some(pos);
        self.agent_tap(pos.0, pos.1).await?;
        Ok(())
    }

    async fn long_press(&self, selector: &Selector, duration_ms: u64) -> Result<()> {
        let pos = self
            .find_element(selector)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Element not found for long_press: {:?}", selector))?;

        self.agent_long_press(pos.0, pos.1, duration_ms).await?;
        Ok(())
    }

    async fn double_tap(&self, selector: &Selector) -> Result<()> {
        let pos = self
            .find_element(selector)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Element not found for double_tap: {:?}", selector))?;

        if !self.try_agent_double_tap(pos.0, pos.1).await {
            anyhow::bail!("lm-ios-tester agent is not reachable - cannot double-tap");
        }

        Ok(())
    }

    async fn right_click(&self, _selector: &Selector) -> Result<()> {
        anyhow::bail!("Right click is not supported on iOS")
    }

    async fn input_text(&self, text: &str, _unicode: bool) -> Result<()> {
        if !self.try_agent_type_text(text).await {
            if self.is_simulator {
                // Clipboard-paste fallback (simulator only: uses `xcrun simctl pbcopy`,
                // which has no real-device equivalent) for when the agent can't type
                // directly - e.g. non-ASCII text the keyboard-event path chokes on.
                return self.input_text_clipboard(text).await;
            } else {
                anyhow::bail!("lm-ios-tester agent is not reachable - cannot type text");
            }
        }

        // The agent's `type_text` synthesizes raw keyboard events via
        // `XCSynthesizedEventRecord`/`XCPointerEventPath` (see `synthesizeTypeText:` in
        // `LumiCommandHandler.m`) - a low-level event stream delivered to whatever
        // currently has keyboard focus, entirely bypassing XCUITest's own public
        // gesture APIs (and their built-in app-quiescence wait, by design - that wait
        // is the ~650ms/call overhead this whole agent exists to avoid). It reports
        // success as soon as the OS accepted the event stream, not once any field
        // actually displays the characters. Confirmed live on a real device, and it's
        // the SAME root cause as the Android tap-before-focus-settles race this
        // codebase already hardened `input_text` against: a `tap()` immediately
        // followed by `type_text` right after a screen *navigation* transition (not
        // just a field gaining focus on an already-settled screen) can resolve/land
        // the tap while the destination screen's transition animation is still
        // playing - the tap event is delivered and "succeeds", but doesn't actually
        // focus the field, so nothing is listening when the keystrokes arrive right
        // after. A screenshot at the exact failure point showed the field with NO
        // cursor/keyboard at all (not "focused but not yet receiving input" as
        // initially assumed) - so recovery has to re-tap, not just retry typing;
        // `last_tap_point` is the same coordinate `tap()` just resolved, so re-tapping
        // it after a beat (letting the transition animation finish) re-attempts the
        // exact same focus action rather than guessing at a new one.
        for backoff_ms in [300u64, 600, 900, 1500] {
            if self.verify_text_landed(text).await {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            if let Some((x, y)) = *self.last_tap_point.lock().await {
                self.try_agent_tap(x, y).await;
                tokio::time::sleep(Duration::from_millis(80)).await;
            }
            self.try_agent_type_text(text).await;
        }
        if self.verify_text_landed(text).await {
            return Ok(());
        }

        anyhow::bail!(
            "Failed to enter text: \"{}\" doesn't appear in any element's value after \
             typing + 4 retries - the target field was likely still settling (e.g. \
             keyboard entrance animation) when the agent sent the keystrokes and never \
             actually received them",
            text
        )
    }

    async fn erase_text(&self, char_count: Option<u32>) -> Result<()> {
        if self.try_agent_erase_text(char_count.unwrap_or(60)).await {
            return Ok(());
        }

        // Fallback: find text field and select all via triple-tap then replace
        let ui_json = self.hierarchy_json().await?;

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
            self.agent_tap(tap_x, tap_y).await?;
            tokio::time::sleep(Duration::from_millis(80)).await;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Type space to replace selected text
        if !self.try_agent_type_text(" ").await {
            anyhow::bail!("lm-ios-tester agent is not reachable - cannot erase text");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Triple-tap again to select the space
        for _ in 0..3 {
            self.agent_tap(tap_x, tap_y).await?;
            tokio::time::sleep(Duration::from_millis(80)).await;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;

        Ok(())
    }

    async fn hide_keyboard(&self) -> Result<()> {
        // RETURN used to be the *primary* strategy here, with tap-outside only as a
        // fallback for when the agent couldn't even send the keypress. That's
        // backwards: RETURN dismisses the keyboard only for a single-line field
        // whose keyboard happens to have a "Done"/"Return" action wired to resign
        // first responder - for most fields (numeric keypads with no return key at
        // all, multi-line fields, fields wired to submit/navigate on return) it
        // either does nothing to the keyboard or triggers an unintended action
        // (form submission, navigating to the next field) instead. Sending the
        // keystroke itself almost always "succeeds" at the transport level even
        // when it has none of the intended dismiss effect, so the old code reported
        // success while the keyboard visibly stayed up - confirmed live (this
        // app's phone-number field uses a numeric keypad with no return key).
        // Tapping outside the keyboard/any field is the actual OS-level dismiss
        // gesture (resigns first responder) and works regardless of keyboard type,
        // so it's the primary path now. Near the top-center of the screen: high in
        // the view (keyboards occupy roughly the bottom half+), and centered rather
        // than left/right-aligned to avoid a corner back/action button that a
        // header commonly has.
        let (width, height) = self.screen_size;
        self.agent_tap((width / 2) as i32, (height as f64 * 0.06) as i32)
            .await?;

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

        self.agent_swipe(x1, y1, x2, y2, duration_ms.unwrap_or(300)).await?;
        Ok(())
    }

    async fn drag(&self, from: (i32, i32), to: (i32, i32), duration_ms: u64) -> Result<()> {
        let (x1, y1) = from;
        let (x2, y2) = to;
        println!(
            "    {} Dragging from ({}, {}) to ({}, {}) [{}ms]",
            "ℹ".blue(),
            x1,
            y1,
            x2,
            y2,
            duration_ms
        );

        self.agent_swipe(x1, y1, x2, y2, duration_ms).await?;
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
        Ok(self.find_element(selector).await?.is_some())
    }

    async fn wait_for_element(&self, selector: &Selector, timeout_ms: u64) -> Result<bool> {
        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        while start.elapsed() < timeout {
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
        // `xcrun simctl openurl` covers simulators cleanly. No equivalent exists for a
        // real device without idb: `devicectl device process launch` only passes plain
        // command-line arguments to the target process, not a URL through
        // `application(_:open:options:)`, so a real device has no native way to open a
        // deep link short of idb's own `open` command - a genuine, documented gap.
        devicectl::open_url(&self.udid, url, self.is_simulator).await?;
        Ok(())
    }

    async fn compare_screenshot(
        &self,
        reference_path: &Path,
        _tolerance_percent: f64,
    ) -> Result<f64> {
        // Take current screenshot
        let temp_path = format!("/tmp/ios_screenshot_{}.png", Uuid::new_v4());
        self.capture_screenshot(&temp_path).await?;

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
        self.capture_screenshot(path).await?;
        println!("{} Screenshot saved to: {}", "✓".green(), path);
        Ok(())
    }

    async fn back(&self) -> Result<()> {
        // iOS back gesture: swipe from left edge
        let (_, height) = self.screen_size;
        let center_y = height as i32 / 2;
        self.agent_swipe(5, center_y, 200, center_y, 200).await?;
        Ok(())
    }

    async fn home(&self) -> Result<()> {
        self.agent_press_button("home").await?;
        Ok(())
    }

    async fn get_screen_size(&self) -> Result<(u32, u32)> {
        Ok(self.screen_size)
    }

    async fn dump_ui_hierarchy(&self) -> Result<String> {
        self.hierarchy_json().await
    }

    async fn dump_logs(&self, limit: u32) -> Result<String> {
        // Simulators are ordinary macOS processes under CoreSimulator, so `simctl spawn`
        // + the system `log` tool reads the simulator's own unified log directly - a
        // genuine native replacement for idb's `log --style compact`. Real devices have
        // no devicectl/simctl equivalent (no log-streaming subcommand exists there) - a
        // documented gap, not a faked replacement.
        if !self.is_simulator {
            anyhow::bail!(
                "dump_logs is not supported on physical iOS devices without idb (no \
                 xcrun devicectl equivalent for streaming device logs exists)"
            );
        }
        let output = tokio::process::Command::new("xcrun")
            .args(&["simctl", "spawn", &self.udid, "log", "show", "--style", "compact", "--last", "2m"])
            .output()
            .await
            .context("Failed to run xcrun simctl spawn log show")?;
        let logs = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = logs.lines().rev().take(limit as usize).collect();
        Ok(lines.into_iter().rev().collect::<Vec<_>>().join("\n"))
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
        // `xcrun simctl io recordVideo` is a clean native replacement for idb's `record
        // video` on simulators (stops the same way, on SIGINT). No devicectl/simctl
        // equivalent exists for real devices - a genuine, documented gap.
        if !self.is_simulator {
            anyhow::bail!(
                "start_recording is not supported on physical iOS devices without idb \
                 (no xcrun devicectl equivalent for screen recording exists)"
            );
        }
        let child = tokio::process::Command::new("xcrun")
            .args(&["simctl", "io", &self.udid, "recordVideo", path])
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
            // `simctl io recordVideo` stops on SIGINT, same as idb's `record video` did.
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
        let agent_button_name = match key.to_uppercase().as_str() {
            "HOME" => Some("home"),
            "VOLUME_UP" => Some("volumeup"),
            "VOLUME_DOWN" => Some("volumedown"),
            _ => None,
        };
        let ok = if let Some(name) = agent_button_name {
            self.try_agent_press_button(name).await
        } else {
            self.try_agent_press_key(&key.to_uppercase()).await
        };
        if ok {
            Ok(())
        } else {
            anyhow::bail!("lm-ios-tester agent is not reachable - cannot press '{}'", key)
        }
    }

    async fn push_file(&self, source: &str, dest: &str) -> Result<()> {
        let bundle_id = self.current_app_id.lock().await.clone().ok_or_else(|| {
            anyhow::anyhow!("push_file needs a launched app's bundle id (no app has been launched yet)")
        })?;
        devicectl::push_file(&self.udid, &bundle_id, source, dest, self.is_simulator).await
    }

    async fn pull_file(&self, source: &str, dest: &str) -> Result<()> {
        let bundle_id = self.current_app_id.lock().await.clone().ok_or_else(|| {
            anyhow::anyhow!("pull_file needs a launched app's bundle id (no app has been launched yet)")
        })?;
        devicectl::pull_file(&self.udid, &bundle_id, source, dest, self.is_simulator).await
    }

    async fn clear_app_data(&self, app_id: &str) -> Result<()> {
        // Was previously just a terminate - never actually cleared any data on either
        // simulator or real device (comment said "Just terminate for now"). The executor
        // calls this method, not `launch_app`'s own clear-state branch, whenever a
        // flow's `launchApp` step combines `clearState: true` with `permissions` (see
        // `TestCommand::LaunchApp` in executor.rs) - a flow like that was silently never
        // getting its app data cleared at all.
        let _ = devicectl::terminate_app(&self.udid, app_id, self.is_simulator).await;
        self.clear_app_state_impl(app_id).await;
        Ok(())
    }

    async fn set_clipboard(&self, text: &str) -> Result<()> {
        // Workaround: type text
        if self.try_agent_type_text(text).await {
            Ok(())
        } else {
            anyhow::bail!("lm-ios-tester agent is not reachable - cannot set clipboard")
        }
    }

    async fn get_clipboard(&self) -> Result<String> {
        Err(anyhow::anyhow!("get_clipboard not supported on iOS"))
    }

    async fn get_pixel_color(&self, x: i32, y: i32) -> Result<(u8, u8, u8)> {
        // Take screenshot and extract pixel using common utility
        let temp_path = format!("/tmp/ios_pixel_{}.png", Uuid::new_v4());
        self.capture_screenshot(&temp_path).await?;

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
        } else {
            // `idb clear-keychain` was worth trying (confirmed on a physical iOS 26.5.2
            // device that it actually rejects real targets: "Target doesn't conform to
            // FBSimulatorKeychainCommands protocol" - the name makes clear it's
            // simulator-only, contrary to what its own `--help` output suggested). No
            // idb-level equivalent exists for a real device short of a full app
            // uninstall/reinstall (which needs the app's .ipa/.app bundle on hand, not
            // available here) - warn instead of silently no-op'ing as before, so a flow
            // relying on `clearKeychain: true` to force a logged-out state doesn't fail
            // confusingly several steps later with no indication why.
            println!(
                "  {} clearKeychain not supported on physical iOS devices (no XCTest/ \
                 devicectl API for it exists; a real device needs a full app \
                 uninstall/reinstall instead)",
                "⚠".yellow()
            );
        }

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
        self.agent_swipe(center_x, 0, center_x, 500, 300).await
    }

    async fn open_quick_settings(&self) -> Result<()> {
        // Control Center: Swipe down from top-right
        let (w, _h) = self.screen_size;
        let start_x = (w as i32) - 10;
        self.agent_swipe(start_x, 0, start_x, 400, 400).await
    }

    async fn set_volume(&self, _level: u8) -> Result<()> {
        println!("  {} set_volume not supported on iOS", "⚠️".yellow());
        Ok(())
    }

    async fn lock_device(&self) -> Result<()> {
        // Neither XCTest's public `XCUIDevice.Button` (home/volumeUp/volumeDown only,
        // no lock case) nor devicectl/simctl expose a way to lock the screen - idb's
        // `press-button LOCK` relied on a private mechanism with no native equivalent.
        // A genuine, documented gap rather than a faked no-op.
        anyhow::bail!(
            "lock_device is not supported without idb (no public XCTest API or xcrun \
             devicectl/simctl equivalent for locking the screen exists)"
        )
    }

    async fn unlock_device(&self) -> Result<()> {
        // Wake up
        self.agent_press_button("home").await?;
        // If it was locked, this might wake it. If on lock screen, might need swipe up?
        // Let's try to swipe up from bottom just in case
        let (w, h) = self.screen_size;
        let center_x = (w / 2) as i32;
        let bottom_y = (h as i32) - 10;
        let mid_y = (h / 2) as i32;
        self.agent_swipe(center_x, bottom_y, center_x, mid_y, 300).await?;
        Ok(())
    }

    async fn install_app(&self, path: &str) -> Result<()> {
        // Resolve relative path if needed? Context usually resolves it.
        // But driver receives path string.
        if !std::path::Path::new(path).exists() {
            anyhow::bail!("App file not found: {}", path);
        }
        println!("  {} Installing app: {}", "⬇".cyan(), path);
        devicectl::install_app(&self.udid, path, self.is_simulator).await
    }

    async fn uninstall_app(&self, app_id: &str) -> Result<()> {
        println!("  {} Uninstalling app: {}", "🗑".cyan(), app_id);
        devicectl::uninstall_app(&self.udid, app_id, self.is_simulator).await
    }

    async fn background_app(&self, app_id_opt: Option<&str>, duration_ms: u64) -> Result<()> {
        // Press Home
        self.agent_press_button("home").await?;

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

    async fn set_orientation(&self, mode: crate::parser::types::Orientation) -> Result<()> {
        use crate::parser::types::Orientation;
        if !self.is_simulator {
            let agent_mode = match mode {
                Orientation::Portrait => "portrait",
                Orientation::UpsideDown => "upsideDown",
                Orientation::Landscape | Orientation::LandscapeLeft => "landscapeLeft",
                Orientation::LandscapeRight => "landscapeRight",
            };
            self.ensure_agent_ready().await;
            let guard = self.agent_client.lock().await;
            if let Some(agent) = guard.as_ref() {
                if agent.set_orientation(agent_mode).await {
                    return Ok(());
                }
            }
        }
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
        let is_simulator = self.is_simulator;
        let agent_client = self.agent_client.clone();
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

                    // Simulators only understand `simctl location`; real devices use the
                    // agent's `set_location` (XCTRunnerDaemonSession-based, see agent.rs).
                    if is_simulator {
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
                    } else {
                        let guard = agent_client.lock().await;
                        if let Some(agent) = guard.as_ref() {
                            agent.set_location(lat, lon, 0.0).await;
                        }
                    }

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
        } else {
            self.ensure_agent_ready().await;
            let guard = self.agent_client.lock().await;
            if let Some(agent) = guard.as_ref() {
                agent.clear_location().await;
            }
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
