//! Web Driver implementation using Chrome DevTools Protocol (CDP) directly.
//!
//! Talks straight to Chrome over CDP via `chromiumoxide` instead of spawning Playwright's
//! Node.js driver subprocess for every session (that subprocess spawn+handshake cost ~10s
//! per run, dwarfing the actual test time - see the rewrite plan for the measurement).

use anyhow::{Context, Result};
use async_trait::async_trait;
use chromiumoxide::cdp::browser_protocol::emulation::{
    ScreenOrientation, ScreenOrientationType, SetCpuThrottlingRateParams,
    SetDeviceMetricsOverrideParams, SetGeolocationOverrideParams,
};
use chromiumoxide::cdp::browser_protocol::input::{
    DispatchKeyEventParams, DispatchKeyEventType, DispatchMouseEventParams,
    DispatchMouseEventType, InsertTextParams, MouseButton,
};
use chromiumoxide::cdp::browser_protocol::network::EmulateNetworkConditionsParams;
use chromiumoxide::cdp::browser_protocol::page::{
    CaptureScreenshotFormat, DialogType, EventJavascriptDialogOpening,
    HandleJavaScriptDialogParams,
};
use chromiumoxide::cdp::js_protocol::runtime::EventConsoleApiCalled;
use chromiumoxide::element::Element;
use chromiumoxide::keys::USKEYBOARD_LAYOUT;
use chromiumoxide::layout::Point;
use chromiumoxide::page::{Page, ScreenshotParams};
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::driver::common;
use crate::driver::image_matcher::{find_template, ImageRegion, MatchConfig};
use crate::driver::traits::{PlatformDriver, RelativeDirection, Selector, SwipeDirection};
use colored::Colorize;

/// Web browser type
#[derive(Debug, Clone, Copy, Default)]
pub enum BrowserType {
    #[default]
    Chromium,
    Firefox,
    Webkit,
}

/// Web Driver configuration
#[derive(Debug, Clone)]
pub struct WebDriverConfig {
    pub browser_type: BrowserType,
    pub headless: bool,
    pub base_url: Option<String>,
    pub viewport_width: u32,
    pub viewport_height: u32,
    /// CDP endpoint to connect to existing browser (e.g. http://localhost:9222)
    pub cdp_endpoint: Option<String>,
    /// Whether to close browser when test finishes (default: true)
    pub close_when_finish: bool,
}

impl Default for WebDriverConfig {
    fn default() -> Self {
        let headless = std::env::var("LUMI_HEADLESS")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        let cdp_endpoint = std::env::var("LUMI_CDP_ENDPOINT").ok();

        Self {
            browser_type: BrowserType::Chromium,
            headless,
            base_url: None,
            viewport_width: 1280,
            viewport_height: 720,
            cdp_endpoint,
            close_when_finish: true,
        }
    }
}

/// Web Driver using Chrome DevTools Protocol directly (no Node.js subprocess in between)
pub struct WebDriver {
    /// Kept alive for its `Drop` behaviour: a `launch()`-ed browser kills its Chrome child
    /// process on drop, while a `connect()`-ed browser (persistent mode) has no child handle
    /// and leaves the externally-owned Chrome process running untouched.
    #[allow(dead_code)]
    browser: Arc<Browser>,
    page: Arc<Mutex<Page>>,
    config: WebDriverConfig,
    current_recording_path: Arc<Mutex<Option<String>>>,
    console_logs: Arc<Mutex<Vec<String>>>,
    ocr_engine: tokio::sync::OnceCell<crate::driver::ocr::OcrEngine>,
    /// Our own logical navigation stack, independent of Chrome's real browser history.
    /// `back()` can't use Chrome's own history/`navigateToHistoryEntry` (that reliably
    /// restores the document from the back-forward cache without the compositor ever
    /// painting a frame in *headless* Chrome - screenshots come back solid black, element
    /// lookups fail), and falling back to a plain `Page.navigate` to the previous URL
    /// instead pushes a *new* history entry rather than truly rewinding, corrupting
    /// Chrome's own history stack for any `back` after the first (reproduced: entries
    /// end up with the same URL duplicated, and the second `back` lands one hop short).
    /// Tracking pushes/pops ourselves sidesteps both bugs entirely.
    nav_stack: Arc<Mutex<Vec<String>>>,
}

/// One point resolved from a `Selector`, in CSS pixels relative to the viewport, plus enough
/// context to decide actionability without a second round-trip.
struct ResolvedPoint {
    x: f64,
    y: f64,
    visible: bool,
}

impl WebDriver {
    /// Create a new WebDriver instance
    pub async fn new(config: WebDriverConfig) -> Result<Self> {
        let cdp_endpoint = config.cdp_endpoint.clone().or_else(|| {
            if !config.close_when_finish {
                Some("http://localhost:9222".to_string())
            } else {
                None
            }
        });

        let (mut browser, mut handler) = if let Some(ref endpoint) = cdp_endpoint {
            println!(
                "{} Trying to connect to browser at: {}",
                "🔌".blue(),
                endpoint
            );
            match Browser::connect(endpoint.as_str()).await {
                Ok(pair) => {
                    println!("{} Connected to existing browser!", "✅".green());
                    pair
                }
                Err(e) => {
                    if !config.close_when_finish {
                        println!(
                            "{} Could not connect to existing browser ({}), launching Chrome externally...",
                            "⚠️".yellow(),
                            e
                        );
                        launch_chrome_externally()?;
                        // A single fixed-delay-then-connect attempt is unreliable - cold
                        // Chrome startup (first launch, no OS-level cache warm yet) can
                        // take longer than 1s to open its CDP port (reproduced: failed
                        // outright on a real run). Retry instead of guessing one delay.
                        let mut connected = None;
                        for _ in 0..15 {
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                            if let Ok(pair) = Browser::connect(endpoint.as_str()).await {
                                connected = Some(pair);
                                break;
                            }
                        }
                        connected.context("Failed to connect to Chrome after external launch")?
                    } else {
                        launch_chromium_browser(&config).await?
                    }
                }
            }
        } else {
            launch_chromium_browser(&config).await?
        };

        tokio::spawn(async move { while (handler.next().await).is_some() {} });

        // Always open a fresh page/tab rather than attaching to one that already existed
        // in the persistent browser (`!config.close_when_finish`): attaching via
        // `fetch_targets()` + `get_page()` does return a `Page` handle, but its CDP
        // session turns out to be unusable for navigation afterward - `page.goto()` on
        // an attached-to-existing-target page hangs indefinitely (reproduced: a second
        // `lumi-tester run` reconnecting to a still-open persistent Chrome hung for a
        // full 60s on the very first `launchApp`). A brand-new page via `new_page()` is
        // the same reliable, well-exercised path every other session already uses, and
        // it still satisfies the actual goal of persistent mode - the browser stays open
        // and visible across runs for manual inspection - just as a new tab each run
        // rather than reusing the previous run's exact tab.
        let page = browser.new_page("about:blank").await?;
        if !config.close_when_finish {
            page.bring_to_front().await.ok();
        }

        page.execute(
            SetDeviceMetricsOverrideParams::builder()
                .width(config.viewport_width as i64)
                .height(config.viewport_height as i64)
                .device_scale_factor(1.0)
                .mobile(false)
                .build()
                .map_err(|e| anyhow::anyhow!(e))?,
        )
        .await?;

        let console_logs = Arc::new(Mutex::new(Vec::new()));
        if let Ok(mut console_events) = page.event_listener::<EventConsoleApiCalled>().await {
            let logs_clone = console_logs.clone();
            tokio::spawn(async move {
                while let Some(event) = console_events.next().await {
                    let text = event
                        .args
                        .iter()
                        .map(|a| {
                            a.value
                                .as_ref()
                                .map(|v| v.to_string())
                                .or_else(|| a.description.clone())
                                .unwrap_or_default()
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    logs_clone
                        .lock()
                        .await
                        .push(format!("[{:?}] {}", event.r#type, text));
                }
            });
        }

        // A native alert()/confirm()/prompt()/beforeunload dialog blocks the renderer's JS
        // execution entirely until answered - unlike Playwright, raw CDP has no default
        // dialog handler, so any real-world page that pops one (common for delete
        // confirmations, unsaved-changes warnings, etc.) would hang every subsequent
        // command forever. Auto-answer the same way Playwright does by default: dismiss
        // (cancel) everything except beforeunload, which is auto-accepted so navigation
        // isn't silently blocked.
        if let Ok(mut dialog_events) = page.event_listener::<EventJavascriptDialogOpening>().await
        {
            let dialog_page = page.clone();
            tokio::spawn(async move {
                while let Some(event) = dialog_events.next().await {
                    let accept = matches!(event.r#type, DialogType::Beforeunload);
                    let _ = dialog_page
                        .execute(
                            HandleJavaScriptDialogParams::builder()
                                .accept(accept)
                                .build()
                                .unwrap(),
                        )
                        .await;
                }
            });
        }

        Ok(Self {
            browser: Arc::new(browser),
            page: Arc::new(Mutex::new(page)),
            config,
            current_recording_path: Arc::new(Mutex::new(None)),
            console_logs,
            ocr_engine: tokio::sync::OnceCell::new(),
            nav_stack: Arc::new(Mutex::new(Vec::new())),
        })
    }

    async fn get_ocr_engine(&self) -> Result<&crate::driver::ocr::OcrEngine> {
        self.ocr_engine
            .get_or_try_init(|| async { crate::driver::ocr::OcrEngine::new().await })
            .await
    }

    /// Polls `window.location.href` (a genuinely live query - unlike `page.url()`, not
    /// routed through chromiumoxide's own internal event-processing cache) until it
    /// reports the same value twice in a row, confirming any in-flight navigation has
    /// settled. For a page that isn't navigating this matches on the first two checks
    /// (near-zero cost, ~50ms); for one that is, it waits exactly as long as the real
    /// transition takes rather than a blind fixed delay - reproduced empirically: a
    /// fixed 100ms wasn't reliably enough after a navigating click, while a large fixed
    /// delay long enough to always cover it (~1s) would be a needless tax on the vastly
    /// more common non-navigating click. Bounded so a page that never quite settles
    /// can't hang the caller forever.
    async fn wait_for_url_settle(page: &Page) {
        let mut last: Option<String> = None;
        for _ in 0..20 {
            let current = page
                .evaluate("window.location.href")
                .await
                .ok()
                .and_then(|r| r.into_value::<String>().ok());
            if current.is_some() && current == last {
                break;
            }
            last = current;
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }

    async fn screenshot_bytes(&self) -> Result<Vec<u8>> {
        let page = self.page.lock().await;
        // `Page.captureScreenshot` grabs whatever's currently in the compositor's frame
        // buffer - right after a history navigation (`back`/`forward`, especially a
        // back-forward-cache restore) that buffer can still hold the previous frame for
        // a moment, producing an all-black screenshot even though the DOM/URL is already
        // correct (reproduced empirically). A double requestAnimationFrame round-trip is
        // the standard way to wait for a real paint to have happened before capturing.
        let _ = page
            .evaluate(
                "new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))",
            )
            .await;
        let bytes = page
            .screenshot(
                ScreenshotParams::builder()
                    .format(CaptureScreenshotFormat::Png)
                    .build(),
            )
            .await?;
        Ok(bytes)
    }

    /// Find text on screen using OCR
    async fn find_ocr_text(
        &self,
        text: &str,
        index: usize,
        is_regex: bool,
        region: Option<&str>,
    ) -> Result<Option<(i32, i32)>> {
        let engine = self.get_ocr_engine().await?;
        let image_region = region.map(ImageRegion::from_str).unwrap_or_default();
        let region_clone = image_region;
        let text = text.to_string();
        let engine_clone = engine.clone();

        let screenshot_bytes = self.screenshot_bytes().await?;

        let result = tokio::task::spawn_blocking(move || {
            let (cropped_data, offset_x, offset_y) = if region_clone != ImageRegion::Full {
                let img = image::load_from_memory(&screenshot_bytes)?;
                let (w, h) = (img.width(), img.height());
                let (x, y, rw, rh) = region_clone.get_crop_region(w, h);
                let cropped = img.crop_imm(x, y, rw, rh);
                let mut buf = std::io::Cursor::new(Vec::new());
                cropped.write_to(&mut buf, image::ImageFormat::Png)?;
                (buf.into_inner(), x as i32, y as i32)
            } else {
                (screenshot_bytes, 0, 0)
            };

            let match_opt =
                engine_clone.find_text_at_index(&cropped_data, &text, is_regex, index)?;

            Ok::<_, anyhow::Error>(match_opt.map(|m| (m.x + offset_x, m.y + offset_y)))
        })
        .await??;

        Ok(result)
    }

    /// Find template image on screen
    async fn find_image_on_screen(
        &self,
        template_path: &str,
        region: Option<&str>,
    ) -> Result<Option<(i32, i32)>> {
        let template_path_buf = Path::new(template_path).to_path_buf();
        if !template_path_buf.exists() {
            anyhow::bail!("Template image not found: {:?}", template_path_buf);
        }

        let image_region = region.map(ImageRegion::from_str).unwrap_or_default();
        let screenshot_bytes = self.screenshot_bytes().await?;

        let result = tokio::task::spawn_blocking(move || -> Result<Option<(i32, i32)>> {
            let img_screen = image::load_from_memory(&screenshot_bytes)?.to_luma8();
            let img_template = image::open(&template_path_buf)?.to_luma8();

            let config = MatchConfig {
                target_width: 220.0,
                threshold: 0.7,
                region: image_region,
            };

            let match_result = find_template(&img_screen, &img_template, &config)?;
            Ok(match_result.map(|r| (r.x, r.y)))
        })
        .await??;

        Ok(result)
    }

    /// Map a `Selector` to an XPath expression, for the kinds where XPath alone can express the
    /// full match (no regex, no cross-element geometry needed). Returns `None` for selector
    /// kinds that need the JS-based resolver below instead.
    fn selector_to_xpath(&self, selector: &Selector) -> Option<String> {
        match selector {
            Selector::Text(text, index, _exact) => Some(format!(
                "(//*[normalize-space(text())={}])[{}]",
                xpath_literal(text),
                index + 1
            )),
            Selector::Id(id, index) => Some(format!(
                "(//*[@id={}])[{}]",
                xpath_literal(id),
                index + 1
            )),
            Selector::Type(t, index) => {
                let tag_xpath = map_web_type_xpath(t);
                Some(format!("({})[{}]", tag_xpath, index + 1))
            }
            Selector::Role(role, index) => Some(format!(
                "(//*[@role={}])[{}]",
                xpath_literal(role),
                index + 1
            )),
            Selector::Css(css) => Some(css_to_xpath_hint(css)),
            Selector::XPath(xpath) => Some(xpath.clone()),
            Selector::Placeholder(p, index) => Some(format!(
                "(//*[@placeholder={}])[{}]",
                xpath_literal(p),
                index + 1
            )),
            Selector::AccessibilityId(id) | Selector::Description(id, _) => Some(format!(
                "//*[@aria-label={}]",
                xpath_literal(id)
            )),
            Selector::AnyClickable(index) => Some(format!(
                "(//button|//a|//*[@onclick]|//*[@role='button'])[{}]",
                index + 1
            )),
            _ => None,
        }
    }

    /// Resolve a `Selector` to an `Element`, for the XPath-expressible kinds. Uses
    /// `Page::find_xpath`, which internally runs `document.evaluate` - matches the original
    /// text-normalizing/index-based semantics without depending on Playwright's own selector
    /// engine.
    async fn find_element(&self, selector: &Selector) -> Result<Option<Element>> {
        if let Some(xpath) = self.selector_to_xpath(selector) {
            let page = self.page.lock().await;
            match page.find_xpath(xpath).await {
                Ok(el) => Ok(Some(el)),
                Err(_) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    /// Resolve any `Selector` (including regex/relative kinds an `Element` handle can't express)
    /// to a clickable point via a single JS round-trip. Used by the selector kinds that need
    /// real `RegExp` matching or cross-element geometry, which XPath 1.0 (what `document.evaluate`
    /// actually supports in a real browser) cannot do.
    async fn resolve_point(&self, selector: &Selector) -> Result<Option<ResolvedPoint>> {
        if let Some(el) = self.find_element(selector).await? {
            return Ok(Some(element_to_point(&el).await?));
        }

        match selector {
            Selector::TextRegex(regex, index) => {
                self.resolve_by_js(
                    &format!(
                        r#"() => {{
                            const re = new RegExp({});
                            const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT);
                            let matches = [];
                            let node;
                            while ((node = walker.nextNode())) {{
                                const own = Array.from(node.childNodes)
                                    .filter(n => n.nodeType === Node.TEXT_NODE)
                                    .map(n => n.textContent).join('').replace(/\s+/g, ' ').trim();
                                if (own && re.test(own)) matches.push(node);
                            }}
                            return matches[{}] || null;
                        }}"#,
                        js_literal(regex),
                        index
                    ),
                )
                .await
            }
            Selector::IdRegex(regex, index) => {
                self.resolve_by_js(&format!(
                    r#"() => {{
                        const re = new RegExp({});
                        const matches = Array.from(document.querySelectorAll('[id]')).filter(el => re.test(el.id));
                        return matches[{}] || null;
                    }}"#,
                    js_literal(regex),
                    index
                ))
                .await
            }
            Selector::DescriptionRegex(regex, index) => {
                self.resolve_by_js(&format!(
                    r#"() => {{
                        const re = new RegExp({});
                        const matches = Array.from(document.querySelectorAll('[aria-label]')).filter(el => re.test(el.getAttribute('aria-label')));
                        return matches[{}] || null;
                    }}"#,
                    js_literal(regex),
                    index
                ))
                .await
            }
            Selector::Relative {
                target,
                anchor,
                direction,
                max_dist,
            } => self.resolve_relative(target, anchor, *direction, *max_dist).await,
            Selector::HasChild { parent, child } => self.resolve_has_child(parent, child).await,
            _ => Ok(None),
        }
    }

    async fn resolve_by_js(&self, expr: &str) -> Result<Option<ResolvedPoint>> {
        // Resolve the node via evaluate rather than find_xpath/find_element - regex-matched
        // nodes don't need the full Element abstraction since we only ever click/read them by
        // point, same as the Point/Image/OCR selectors below.
        let page = self.page.lock().await;
        let js = format!(
            r#"(() => {{
                const el = ({})();
                if (!el) return null;
                el.scrollIntoView({{block: 'center', inline: 'center', behavior: 'instant'}});
                const r = el.getBoundingClientRect();
                const style = getComputedStyle(el);
                const visible = r.width > 0 && r.height > 0 && style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';
                return {{ x: r.x + r.width / 2, y: r.y + r.height / 2, visible }};
            }})()"#,
            expr
        );
        let result: serde_json::Value = page.evaluate(js).await?.into_value()?;
        if result.is_null() {
            return Ok(None);
        }
        Ok(Some(ResolvedPoint {
            x: result.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0),
            y: result.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0),
            visible: result
                .get("visible")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        }))
    }

    async fn resolve_relative(
        &self,
        target: &Selector,
        anchor: &Selector,
        direction: RelativeDirection,
        max_dist: Option<u32>,
    ) -> Result<Option<ResolvedPoint>> {
        let Some(anchor_el) = self.find_element(anchor).await? else {
            return Ok(None);
        };
        let anchor_box = anchor_el.bounding_box().await?;

        let Some(xpath) = self.selector_to_xpath(target).or_else(|| {
            // Relative targets are usually a simple base selector with the index handled here
            // instead, so strip any index suffix from `selector_to_xpath`'s output.
            self.selector_to_xpath(&strip_index(target))
        }) else {
            return Ok(None);
        };

        let page = self.page.lock().await;
        let candidates = page.find_xpaths(xpath).await.unwrap_or_default();
        drop(page);

        let mut matches: Vec<(f64, chromiumoxide::layout::BoundingBox)> = Vec::new();
        for el in &candidates {
            let Ok(bx) = el.bounding_box().await else {
                continue;
            };
            let dx = (bx.x + bx.width / 2.0) - (anchor_box.x + anchor_box.width / 2.0);
            let dy = (bx.y + bx.height / 2.0) - (anchor_box.y + anchor_box.height / 2.0);
            let ok = match direction {
                RelativeDirection::LeftOf => bx.x + bx.width <= anchor_box.x,
                RelativeDirection::RightOf => bx.x >= anchor_box.x + anchor_box.width,
                RelativeDirection::Above => bx.y + bx.height <= anchor_box.y,
                RelativeDirection::Below => bx.y >= anchor_box.y + anchor_box.height,
                RelativeDirection::Near => true,
            };
            let dist = (dx * dx + dy * dy).sqrt();
            if ok && max_dist.map(|d| dist <= d as f64).unwrap_or(true) {
                matches.push((dist, bx));
            }
        }
        matches.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let index = match target {
            Selector::Text(_, i, _)
            | Selector::Id(_, i)
            | Selector::Type(_, i)
            | Selector::Placeholder(_, i)
            | Selector::Role(_, i) => *i,
            _ => 0,
        };

        Ok(matches.get(index).map(|(_, bx)| ResolvedPoint {
            x: bx.x + bx.width / 2.0,
            y: bx.y + bx.height / 2.0,
            visible: true,
        }))
    }

    async fn resolve_has_child(
        &self,
        parent: &Selector,
        child: &Selector,
    ) -> Result<Option<ResolvedPoint>> {
        let Some(xpath) = self.selector_to_xpath(parent) else {
            return Ok(None);
        };
        let page = self.page.lock().await;
        let candidates = page.find_xpaths(xpath).await.unwrap_or_default();
        drop(page);

        let child_css = match child {
            Selector::Css(c) => c.clone(),
            Selector::Type(t, _) => map_web_type(t),
            Selector::Id(id, _) => format!("#{}", id),
            _ => return Ok(None),
        };

        for el in &candidates {
            if el.find_element(child_css.clone()).await.is_ok() {
                return Ok(Some(element_to_point(el).await?));
            }
        }
        Ok(None)
    }

    async fn dispatch_key(&self, key_name: &str) -> Result<()> {
        let def = USKEYBOARD_LAYOUT
            .iter()
            .find(|k| k.key.eq_ignore_ascii_case(key_name))
            .ok_or_else(|| anyhow::anyhow!("Unknown key: {}", key_name))?;

        let page = self.page.lock().await;
        let mut down = DispatchKeyEventParams::builder()
            .r#type(DispatchKeyEventType::KeyDown)
            .key(def.key)
            .code(def.code)
            .windows_virtual_key_code(def.key_code)
            .native_virtual_key_code(def.key_code);
        if let Some(text) = def.text {
            down = down.text(text);
        }
        page.execute(down.build().map_err(|e| anyhow::anyhow!(e))?)
            .await?;

        let up = DispatchKeyEventParams::builder()
            .r#type(DispatchKeyEventType::KeyUp)
            .key(def.key)
            .code(def.code)
            .windows_virtual_key_code(def.key_code)
            .native_virtual_key_code(def.key_code)
            .build()
            .map_err(|e| anyhow::anyhow!(e))?;
        page.execute(up).await?;
        Ok(())
    }

    async fn dispatch_click_at(&self, x: f64, y: f64, button: MouseButton, click_count: i64) -> Result<()> {
        let page = self.page.lock().await;
        let mut down = DispatchMouseEventParams::new(DispatchMouseEventType::MousePressed, x, y);
        down.button = Some(button.clone());
        down.click_count = Some(click_count);
        page.execute(down).await?;

        let mut up = DispatchMouseEventParams::new(DispatchMouseEventType::MouseReleased, x, y);
        up.button = Some(button);
        up.click_count = Some(click_count);
        page.execute(up).await?;
        Ok(())
    }

    async fn wait_actionable(&self, selector: &Selector, timeout_ms: u64) -> Result<Option<ResolvedPoint>> {
        let start = std::time::Instant::now();
        let mut interval = 50u64;
        loop {
            if let Some(p) = self.resolve_point(selector).await? {
                if p.visible {
                    return Ok(Some(p));
                }
            }
            if start.elapsed().as_millis() >= timeout_ms as u128 {
                return Ok(None);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(interval)).await;
            interval = (interval * 3 / 2).min(300);
        }
    }
}

async fn element_to_point(el: &Element) -> Result<ResolvedPoint> {
    let _ = el.scroll_into_view().await;
    match el.clickable_point().await {
        Ok(p) => Ok(ResolvedPoint {
            x: p.x,
            y: p.y,
            visible: true,
        }),
        Err(_) => {
            let (x, y) = match el.bounding_box().await {
                Ok(bx) => (bx.x + bx.width / 2.0, bx.y + bx.height / 2.0),
                Err(_) => (0.0, 0.0),
            };
            Ok(ResolvedPoint {
                x,
                y,
                visible: false,
            })
        }
    }
}

fn strip_index(selector: &Selector) -> Selector {
    match selector {
        Selector::Text(t, _, e) => Selector::Text(t.clone(), 0, *e),
        Selector::Id(id, _) => Selector::Id(id.clone(), 0),
        Selector::Type(t, _) => Selector::Type(t.clone(), 0),
        Selector::Placeholder(p, _) => Selector::Placeholder(p.clone(), 0),
        Selector::Role(r, _) => Selector::Role(r.clone(), 0),
        other => other.clone(),
    }
}

/// Escape a string as an XPath 1.0 string literal (which has no escape syntax of its own -
/// switches quote style, or falls back to `concat()` if the text contains both quote kinds).
fn xpath_literal(s: &str) -> String {
    if !s.contains('"') {
        format!("\"{}\"", s)
    } else if !s.contains('\'') {
        format!("'{}'", s)
    } else {
        let parts: Vec<String> = s
            .split('"')
            .map(|p| format!("\"{}\"", p))
            .collect();
        format!("concat({})", parts.join(", '\"', "))
    }
}

fn js_literal(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

fn map_web_type(t: &str) -> String {
    match t.to_lowercase().as_str() {
        "textfield" | "edittext" | "input" => "input".to_string(),
        "button" | "btn" => "button".to_string(),
        "submit" => "*[type='submit']".to_string(),
        "image" | "icon" => "img".to_string(),
        "link" => "a".to_string(),
        "checkbox" => "input[type='checkbox']".to_string(),
        "radio" => "input[type='radio']".to_string(),
        _ => t.to_string(),
    }
}

fn map_web_type_xpath(t: &str) -> String {
    match t.to_lowercase().as_str() {
        "textfield" | "edittext" | "input" => "//input".to_string(),
        "button" | "btn" => "//button".to_string(),
        "submit" => "//*[@type='submit']".to_string(),
        "image" | "icon" => "//img".to_string(),
        "link" => "//a".to_string(),
        "checkbox" => "//input[@type='checkbox']".to_string(),
        "radio" => "//input[@type='radio']".to_string(),
        other => format!("//{}", other),
    }
}

/// Best-effort CSS -> XPath for the simple selector shapes this project actually generates
/// (tag, #id, .class, [attr=val]); anything more complex is passed straight through since
/// `find_xpath` also accepts `document.querySelector`-shaped CSS is NOT valid XPath, so callers
/// that need arbitrary CSS should prefer `Selector::Css` resolved via `page.find_element` instead.
fn css_to_xpath_hint(css: &str) -> String {
    if css.starts_with('#') {
        format!("//*[@id='{}']", &css[1..])
    } else if let Some(stripped) = css.strip_prefix('.') {
        format!("//*[contains(concat(' ', normalize-space(@class), ' '), ' {} ')]", stripped)
    } else if let (Some(open), Some(close)) = (css.find('['), css.rfind(']')) {
        // `tag[attr=val]` / `[attr=val]` (CSS) -> `//tag[@attr='val']` (XPath) - CSS's
        // `[attr=val]` means "has this attribute", XPath's `[attr=val]` means "has a
        // *child element* named attr" instead, so the `@` is required, not cosmetic.
        let tag = if open == 0 { "*" } else { &css[..open] };
        let inner = &css[open + 1..close];
        let (attr, val) = match inner.split_once('=') {
            Some((a, v)) => (a.trim(), v.trim().trim_matches(|c| c == '\'' || c == '"')),
            None => (inner.trim(), ""),
        };
        if val.is_empty() {
            format!("//{}[@{}]", tag, attr)
        } else {
            format!("//{}[@{}='{}']", tag, attr, val)
        }
    } else {
        format!("//{}", css)
    }
}

#[async_trait]
impl PlatformDriver for WebDriver {
    fn platform_name(&self) -> &str {
        "web"
    }

    fn device_serial(&self) -> Option<String> {
        Some(format!("{:?}", self.config.browser_type))
    }

    async fn launch_app(&self, url: &str, _clear_state: bool) -> Result<()> {
        let page = self.page.lock().await;

        let full_url = if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else if let Some(ref base) = self.config.base_url {
            format!("{}{}", base.trim_end_matches('/'), url)
        } else {
            url.to_string()
        };

        page.goto(full_url.as_str())
            .await
            .context("Failed to navigate to URL")?;
        let live_url = page
            .evaluate("window.location.href")
            .await
            .ok()
            .and_then(|r| r.into_value::<String>().ok())
            .unwrap_or(full_url);
        self.nav_stack.lock().await.push(live_url);

        Ok(())
    }

    async fn stop_app(&self, _app_id: &str) -> Result<()> {
        let page = self.page.lock().await;
        page.goto("about:blank").await?;
        Ok(())
    }

    async fn resolve_element_point(
        &self,
        selector: &Selector,
        x_pct: f64,
        y_pct: f64,
    ) -> Result<Option<(i32, i32)>> {
        // The trait's default impl always returns `None`, so unless a driver overrides
        // it, anything routed through it (e.g. `drag`'s from/to selectors, `tapAt` with
        // an align/offset) silently fails with "element not found" no matter how valid
        // the selector is - reproduced: a real `id`-based drag on a real page failed here
        // 100% of the time before this override existed.
        if let Some(el) = self.find_element(selector).await? {
            let _ = el.scroll_into_view().await;
            if let Ok(bx) = el.bounding_box().await {
                return Ok(Some((
                    (bx.x + bx.width * x_pct) as i32,
                    (bx.y + bx.height * y_pct) as i32,
                )));
            }
        }
        // Selector kinds `find_element`'s XPath path can't express (regex/relative) still
        // resolve via `resolve_point` - it only ever yields a single JS-computed point
        // (not a bounding box), so an x/y offset isn't meaningful for them anyway.
        if let Some(p) = self.resolve_point(selector).await? {
            return Ok(Some((p.x as i32, p.y as i32)));
        }
        Ok(None)
    }

    async fn tap(&self, selector: &Selector) -> Result<()> {
        let url_before = {
            let page = self.page.lock().await;
            page.evaluate("window.location.href")
                .await
                .ok()
                .and_then(|r| r.into_value::<String>().ok())
        };
        match selector {
            Selector::Point { x, y } => {
                let page = self.page.lock().await;
                page.click(Point::new(*x as f64, *y as f64)).await?;
            }
            Selector::Image { path, region } => {
                let pos = self.find_image_on_screen(path, region.as_deref()).await?;
                if let Some((x, y)) = pos {
                    println!("    {} Tapping on image match at ({}, {})", "👆".cyan(), x, y);
                    let page = self.page.lock().await;
                    page.click(Point::new(x as f64, y as f64)).await?;
                } else {
                    anyhow::bail!("Image not found on screen: {}", path);
                }
            }
            Selector::OCR(text, index, is_regex, region) => {
                let pos = self
                    .find_ocr_text(text, *index, *is_regex, region.as_deref())
                    .await?;
                if let Some((x, y)) = pos {
                    println!("    {} Tapping on OCR match at ({}, {})", "👆".cyan(), x, y);
                    let page = self.page.lock().await;
                    page.click(Point::new(x as f64, y as f64)).await?;
                } else {
                    anyhow::bail!("Text not found on screen via OCR: {}", text);
                }
            }
            _ => {
                if let Some(el) = self.find_element(selector).await? {
                    el.click().await.map_err(|e| {
                        anyhow::anyhow!("Failed to click element for {:?}: {}", selector, e)
                    })?;
                } else if let Some(p) = self.resolve_point(selector).await? {
                    let page = self.page.lock().await;
                    page.click(Point::new(p.x, p.y)).await?;
                } else {
                    anyhow::bail!("Element not found for selector: {:?}", selector);
                }
            }
        }
        // If this click triggered a navigation, the live document can be mid-transition
        // for a while afterward - wait for it to settle (see `wait_for_url_settle`).
        let page = self.page.lock().await;
        Self::wait_for_url_settle(&page).await;
        let url_after = page
            .evaluate("window.location.href")
            .await
            .ok()
            .and_then(|r| r.into_value::<String>().ok());
        if let Some(url) = url_after {
            if Some(&url) != url_before.as_ref() {
                self.nav_stack.lock().await.push(url);
            }
        }
        Ok(())
    }

    async fn long_press(&self, selector: &Selector, duration_ms: u64) -> Result<()> {
        let point = if let Some(el) = self.find_element(selector).await? {
            element_to_point(&el).await?
        } else if let Some(p) = self.resolve_point(selector).await? {
            p
        } else {
            anyhow::bail!("Element not found for selector: {:?}", selector);
        };

        let page = self.page.lock().await;
        let mut down = DispatchMouseEventParams::new(DispatchMouseEventType::MousePressed, point.x, point.y);
        down.button = Some(MouseButton::Left);
        down.click_count = Some(1);
        page.execute(down).await?;
        drop(page);

        tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;

        let page = self.page.lock().await;
        let mut up = DispatchMouseEventParams::new(DispatchMouseEventType::MouseReleased, point.x, point.y);
        up.button = Some(MouseButton::Left);
        up.click_count = Some(1);
        page.execute(up).await?;
        Ok(())
    }

    async fn double_tap(&self, selector: &Selector) -> Result<()> {
        let point = if let Some(el) = self.find_element(selector).await? {
            element_to_point(&el).await?
        } else if let Some(p) = self.resolve_point(selector).await? {
            p
        } else {
            anyhow::bail!("Element not found for selector: {:?}", selector);
        };
        self.dispatch_click_at(point.x, point.y, MouseButton::Left, 1).await?;
        self.dispatch_click_at(point.x, point.y, MouseButton::Left, 2).await?;
        Ok(())
    }

    async fn right_click(&self, selector: &Selector) -> Result<()> {
        let point = if let Some(el) = self.find_element(selector).await? {
            element_to_point(&el).await?
        } else if let Some(p) = self.resolve_point(selector).await? {
            p
        } else {
            anyhow::bail!("Element not found for selector: {:?}", selector);
        };
        self.dispatch_click_at(point.x, point.y, MouseButton::Right, 1).await?;
        Ok(())
    }

    async fn input_text(&self, text: &str, _unicode: bool) -> Result<()> {
        let page = self.page.lock().await;
        page.execute(
            InsertTextParams::builder()
                .text(text)
                .build()
                .map_err(|e| anyhow::anyhow!(e))?,
        )
        .await?;
        Ok(())
    }

    async fn erase_text(&self, _char_count: Option<u32>) -> Result<()> {
        // Select-all via raw Meta+A/Ctrl+A modifier bits is unreliable over CDP (the browser's
        // native accelerator handling doesn't always fire from synthetic events) - the
        // `commands` field is CDP's documented mechanism for triggering editing commands like
        // `selectAll` directly, bypassing that ambiguity.
        {
            let page = self.page.lock().await;
            let down = DispatchKeyEventParams::builder()
                .r#type(DispatchKeyEventType::RawKeyDown)
                .commands(vec!["selectAll".to_string()])
                .build()
                .map_err(|e| anyhow::anyhow!(e))?;
            page.execute(down).await?;
        }
        self.dispatch_key("Backspace").await?;
        Ok(())
    }

    async fn hide_keyboard(&self) -> Result<()> {
        Ok(())
    }

    async fn swipe(
        &self,
        direction: SwipeDirection,
        _duration_ms: Option<u64>,
        from: Option<Selector>,
    ) -> Result<()> {
        let (dx, dy) = match direction {
            SwipeDirection::Up => (0, -300),
            SwipeDirection::Down => (0, 300),
            SwipeDirection::Left => (-300, 0),
            SwipeDirection::Right => (300, 0),
        };

        if let Some(selector) = from {
            if let Some(el) = self.find_element(&selector).await? {
                let js = format!("function() {{ this.scrollBy({}, {}); }}", dx, dy);
                el.call_js_fn(js, false).await?;
            }
        } else {
            let page = self.page.lock().await;
            let js = format!("window.scrollBy({}, {})", dx, dy);
            page.evaluate(js).await?;
        }

        Ok(())
    }

    async fn drag(&self, from: (i32, i32), to: (i32, i32), duration_ms: u64) -> Result<()> {
        let page = self.page.lock().await;
        page.execute(DispatchMouseEventParams::new(
            DispatchMouseEventType::MouseMoved,
            from.0 as f64,
            from.1 as f64,
        ))
        .await?;
        let mut down =
            DispatchMouseEventParams::new(DispatchMouseEventType::MousePressed, from.0 as f64, from.1 as f64);
        down.button = Some(MouseButton::Left);
        down.click_count = Some(1);
        page.execute(down).await?;

        let steps = (duration_ms / 25).max(5) as i32;
        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            let cx = from.0 as f64 + (to.0 as f64 - from.0 as f64) * t;
            let cy = from.1 as f64 + (to.1 as f64 - from.1 as f64) * t;
            page.execute(DispatchMouseEventParams::new(
                DispatchMouseEventType::MouseMoved,
                cx,
                cy,
            ))
            .await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(
                (duration_ms / steps as u64).max(10),
            ))
            .await;
        }

        let mut up = DispatchMouseEventParams::new(DispatchMouseEventType::MouseReleased, to.0 as f64, to.1 as f64);
        up.button = Some(MouseButton::Left);
        up.click_count = Some(1);
        page.execute(up).await?;
        Ok(())
    }

    async fn scroll_until_visible(
        &self,
        selector: &Selector,
        max_scrolls: u32,
        direction: Option<SwipeDirection>,
        from: Option<Selector>,
    ) -> Result<bool> {
        let swipe_dir = direction.unwrap_or(SwipeDirection::Down);
        for _ in 0..max_scrolls {
            if self.is_visible(selector).await? {
                return Ok(true);
            }
            self.swipe(swipe_dir.clone(), None, from.clone()).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }
        Ok(false)
    }

    async fn is_visible(&self, selector: &Selector) -> Result<bool> {
        match selector {
            Selector::Image { path, region } => {
                Ok(self.find_image_on_screen(path, region.as_deref()).await?.is_some())
            }
            Selector::OCR(text, index, is_regex, region) => Ok(self
                .find_ocr_text(text, *index, *is_regex, region.as_deref())
                .await?
                .is_some()),
            _ => Ok(self
                .resolve_point(selector)
                .await?
                .map(|p| p.visible)
                .unwrap_or(false)),
        }
    }

    async fn tap_by_type_index(&self, element_type: &str, index: u32) -> Result<()> {
        self.tap(&Selector::Type(element_type.to_string(), index as usize))
            .await
    }

    async fn input_by_type_index(&self, element_type: &str, index: u32, text: &str) -> Result<()> {
        self.tap(&Selector::Type(element_type.to_string(), index as usize))
            .await?;
        self.input_text(text, false).await
    }

    async fn wait_for_element(&self, selector: &Selector, timeout_ms: u64) -> Result<bool> {
        match selector {
            Selector::Image { .. } | Selector::OCR(..) => {
                let start = std::time::Instant::now();
                while start.elapsed().as_millis() < timeout_ms as u128 {
                    if self.is_visible(selector).await? {
                        return Ok(true);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                }
                Ok(false)
            }
            _ => Ok(self.wait_actionable(selector, timeout_ms).await?.is_some()),
        }
    }

    async fn wait_for_absence(&self, selector: &Selector, timeout_ms: u64) -> Result<bool> {
        let start = std::time::Instant::now();
        while start.elapsed().as_millis() < timeout_ms as u128 {
            if !self.is_visible(selector).await? {
                return Ok(true);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
        Ok(false)
    }

    async fn get_element_text(&self, selector: &Selector) -> Result<String> {
        if let Some(el) = self.find_element(selector).await? {
            let js = "function() { return this.value || this.innerText || this.textContent || ''; }";
            let result = el.call_js_fn(js, false).await?;
            Ok(result
                .result
                .value
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default())
        } else {
            Ok(String::new())
        }
    }

    async fn open_link(&self, url: &str, _app_id: Option<&str>) -> Result<()> {
        self.launch_app(url, false).await
    }

    async fn compare_screenshot(&self, reference_path: &Path, tolerance_percent: f64) -> Result<f64> {
        use image::GenericImageView;

        let temp_path = std::env::temp_dir().join("lumi_tester_compare.png");
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
                let channel_diff = c1
                    .0
                    .iter()
                    .zip(c2.0.iter())
                    .any(|(a, b)| (*a as i32 - *b as i32).abs() > 5);
                if channel_diff {
                    diff_pixels += 1;
                }
            }
        }

        let diff_percent = (diff_pixels as f64 / total_pixels) * 100.0;
        if diff_percent > tolerance_percent {
            Ok(diff_percent)
        } else {
            Ok(0.0)
        }
    }

    async fn take_screenshot(&self, path: &str) -> Result<()> {
        let path_buf = std::path::PathBuf::from(path);
        if let Some(parent) = path_buf.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = self.screenshot_bytes().await?;
        std::fs::write(&path_buf, bytes)?;
        Ok(())
    }

    async fn back(&self) -> Result<()> {
        // Uses our own `nav_stack` (see its doc comment on the struct field) rather than
        // Chrome's real browser history: `Page.navigateToHistoryEntry`, the correct CDP
        // primitive, restores the document from the back-forward cache without the
        // compositor ever painting a visible frame in *headless* Chrome (screenshots come
        // back solid black, every element lookup fails - reproduced repeatedly). The
        // workaround, a plain `Page.navigate` to the target URL, renders correctly but
        // pushes a *new* history entry instead of truly rewinding, so relying on Chrome's
        // history for a *second* `back` computes against a self-corrupted, duplicated
        // stack (reproduced: after one `goto`-based back, `GetNavigationHistoryParams`
        // entries had the same URL twice and the next `back` landed one hop short).
        // Tracking our own push/pop stack sidesteps both bugs entirely - `back` always
        // pops exactly what a preceding `tapOn`/`open` pushed.
        let mut stack = self.nav_stack.lock().await;
        if stack.len() < 2 {
            return Ok(());
        }
        stack.pop();
        let target_url = stack.last().cloned();
        drop(stack);
        if let Some(target_url) = target_url {
            let page = self.page.lock().await;
            page.goto(target_url.as_str()).await?;
            Self::wait_for_url_settle(&page).await;
        }
        Ok(())
    }

    async fn home(&self) -> Result<()> {
        if let Some(ref base) = self.config.base_url {
            self.launch_app(base, false).await?;
        }
        Ok(())
    }

    async fn get_screen_size(&self) -> Result<(u32, u32)> {
        Ok((self.config.viewport_width, self.config.viewport_height))
    }

    async fn dump_ui_hierarchy(&self) -> Result<String> {
        let page = self.page.lock().await;
        const SCRIPT: &str = r#"
            (() => {
                function xmlEscape(s) {
                    if (!s) return '';
                    return String(s)
                        .replace(/&/g, '&amp;')
                        .replace(/</g, '&lt;')
                        .replace(/>/g, '&gt;')
                        .replace(/"/g, '&quot;')
                        .replace(/'/g, '&apos;');
                }
                const elements = Array.from(document.querySelectorAll('*'));
                let out = '<hierarchy platform="web">\n';
                for (const el of elements) {
                    const rect = el.getBoundingClientRect();
                    if (rect.width <= 0 || rect.height <= 0) continue;
                    const style = window.getComputedStyle(el);
                    if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') continue;
                    const tag = el.tagName ? el.tagName.toLowerCase() : '';
                    const id = el.id || '';
                    const cls = typeof el.className === 'string' ? el.className : '';
                    let text = '';
                    for (const node of el.childNodes) {
                        if (node.nodeType === Node.TEXT_NODE) {
                            text += node.textContent || '';
                        }
                    }
                    text = text.trim().substring(0, 120);
                    if (!text && (tag === 'input' || tag === 'textarea')) {
                        text = (el.value || el.placeholder || '').trim().substring(0, 120);
                    }
                    const clickable = ['a', 'button', 'input', 'select', 'textarea'].includes(tag) ||
                        el.hasAttribute('onclick') ||
                        el.getAttribute('role') === 'button' ||
                        style.cursor === 'pointer';
                    out += `  <element tag="${xmlEscape(tag)}" id="${xmlEscape(id)}" class="${xmlEscape(cls)}" text="${xmlEscape(text)}" x="${Math.round(rect.left)}" y="${Math.round(rect.top)}" width="${Math.round(rect.width)}" height="${Math.round(rect.height)}" clickable="${clickable}"/>\n`;
                }
                out += '</hierarchy>';
                return out;
            })()
        "#;
        match page.evaluate(SCRIPT).await {
            Ok(val) => match val.into_value::<String>() {
                Ok(xml) => Ok(xml),
                Err(_) => Ok(page.content().await?),
            },
            Err(_) => Ok(page.content().await?),
        }
    }

    async fn dump_logs(&self, limit: u32) -> Result<String> {
        let logs = self.console_logs.lock().await;
        let count = logs.len();
        let start = if count > limit as usize {
            count - limit as usize
        } else {
            0
        };
        Ok(logs[start..].join("\n"))
    }

    async fn get_pixel_color(&self, x: i32, y: i32) -> Result<(u8, u8, u8)> {
        let screenshot_data = self.screenshot_bytes().await?;
        let img = image::load_from_memory(&screenshot_data)?;
        Ok(common::get_pixel_from_image(&img, x as u32, y as u32))
    }

    async fn rotate_screen(&self, mode: &str) -> Result<()> {
        let (w, h) = (self.config.viewport_width, self.config.viewport_height);
        let (new_w, new_h) = if mode.eq_ignore_ascii_case("landscape") {
            (w.max(h), w.min(h))
        } else {
            (w.min(h), w.max(h))
        };
        let page = self.page.lock().await;
        page.execute(
            SetDeviceMetricsOverrideParams::builder()
                .width(new_w as i64)
                .height(new_h as i64)
                .device_scale_factor(1.0)
                .mobile(false)
                .build()
                .map_err(|e| anyhow::anyhow!(e))?,
        )
        .await?;
        Ok(())
    }

    async fn start_recording(&self, path: &str) -> Result<()> {
        self.current_recording_path
            .lock()
            .await
            .replace(path.to_string());
        println!(
            "  {} Web recording via CDP video capture not implemented; screenshot-based evidence only.",
            "⚠️".yellow()
        );
        Ok(())
    }

    async fn stop_recording(&self) -> Result<()> {
        Ok(())
    }

    async fn press_key(&self, key: &str) -> Result<()> {
        self.dispatch_key(key).await
    }

    async fn push_file(&self, _source: &str, _dest: &str) -> Result<()> {
        Err(anyhow::anyhow!(
            "push_file not supported on Web. Use dedicated upload command (future)."
        ))
    }

    async fn pull_file(&self, _source: &str, _dest: &str) -> Result<()> {
        Err(anyhow::anyhow!("pull_file not supported on Web."))
    }

    async fn clear_app_data(&self, _app_id: &str) -> Result<()> {
        let page = self.page.lock().await;
        page.evaluate("() => { localStorage.clear(); sessionStorage.clear(); }")
            .await?;
        Ok(())
    }

    async fn set_clipboard(&self, text: &str) -> Result<()> {
        let page = self.page.lock().await;
        page.evaluate(format!(
            "navigator.clipboard.writeText({})",
            js_literal(text)
        ))
        .await?;
        Ok(())
    }

    async fn get_clipboard(&self) -> Result<String> {
        let page = self.page.lock().await;
        let result = page.evaluate("navigator.clipboard.readText()").await?;
        let value: serde_json::Value = result.into_value()?;
        Ok(value.as_str().unwrap_or_default().to_string())
    }

    async fn set_network_connection(&self, wifi: Option<bool>, data: Option<bool>) -> Result<()> {
        let offline = wifi == Some(false) || data == Some(false);
        let page = self.page.lock().await;
        page.execute(
            EmulateNetworkConditionsParams::builder()
                .offline(offline)
                .latency(0.0)
                .download_throughput(-1.0)
                .upload_throughput(-1.0)
                .build()
                .map_err(|e| anyhow::anyhow!(e))?,
        )
        .await?;
        println!("  {} Set Web Connection Offline: {}", "🌐".cyan(), offline);
        Ok(())
    }

    async fn toggle_airplane_mode(&self) -> Result<()> {
        println!(
            "  {} toggle_airplane_mode on web strictly sets offline=true (limitation)",
            "⚠️".yellow()
        );
        let page = self.page.lock().await;
        page.execute(
            EmulateNetworkConditionsParams::builder()
                .offline(true)
                .latency(0.0)
                .download_throughput(-1.0)
                .upload_throughput(-1.0)
                .build()
                .map_err(|e| anyhow::anyhow!(e))?,
        )
        .await?;
        Ok(())
    }

    async fn start_mock_location(
        &self,
        _name: Option<String>,
        points: Vec<crate::parser::gps::GpsPoint>,
        _speed_kmh: Option<f64>,
        _speed_mode: crate::parser::types::SpeedMode,
        _speed_noise: Option<f64>,
        _interval_ms: u64,
        _loop_route: bool,
    ) -> Result<()> {
        if points.is_empty() {
            return Ok(());
        }
        let point = &points[0];
        // Without an explicit permission grant, navigator.geolocation.getCurrentPosition()
        // never settles in headless Chrome (no UI to auto-deny it), so any page JS awaiting it
        // hangs forever - grant it before overriding the position.
        {
            use chromiumoxide::cdp::browser_protocol::browser::{
                PermissionDescriptor, PermissionSetting, SetPermissionParams,
            };
            self.browser
                .execute(
                    SetPermissionParams::builder()
                        .permission(
                            PermissionDescriptor::builder()
                                .name("geolocation")
                                .build()
                                .map_err(|e| anyhow::anyhow!(e))?,
                        )
                        .setting(PermissionSetting::Granted)
                        .build()
                        .map_err(|e| anyhow::anyhow!(e))?,
                )
                .await?;
        }
        let page = self.page.lock().await;
        page.execute(
            SetGeolocationOverrideParams::builder()
                .latitude(point.lat)
                .longitude(point.lon)
                .accuracy(10.0)
                .build(),
        )
        .await?;
        println!(
            "  {} Web Mock Location set to: {}, {}",
            "📍".cyan(),
            point.lat,
            point.lon
        );
        Ok(())
    }

    async fn wait_for_location(
        &self,
        _name: Option<String>,
        lat: f64,
        lon: f64,
        tolerance: f64,
        timeout: u64,
    ) -> Result<()> {
        let page = self.page.lock().await;
        let start = std::time::Instant::now();
        let js = r#"() => new Promise((resolve) => {
            if (!navigator.geolocation) { resolve(null); return; }
            navigator.geolocation.getCurrentPosition(
                (pos) => resolve({ lat: pos.coords.latitude, lon: pos.coords.longitude }),
                (err) => resolve(null),
                { enableHighAccuracy: true, timeout: 5000, maximumAge: 0 }
            );
        })"#;

        while start.elapsed().as_millis() < timeout as u128 {
            let result: serde_json::Value = page.evaluate(js).await?.into_value()?;
            if let Some(obj) = result.as_object() {
                if let (Some(c_lat), Some(c_lon)) = (
                    obj.get("lat").and_then(|v| v.as_f64()),
                    obj.get("lon").and_then(|v| v.as_f64()),
                ) {
                    if (c_lat - lat).abs() <= tolerance && (c_lon - lon).abs() <= tolerance {
                        return Ok(());
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        Err(anyhow::anyhow!("Timeout waiting for location {}, {}", lat, lon))
    }

    async fn open_quick_settings(&self) -> Result<()> {
        println!("  {} open_quick_settings not supported on Web", "⚠️".yellow());
        Ok(())
    }

    async fn set_volume(&self, _level: u8) -> Result<()> {
        println!("  {} set_volume not supported on Web", "⚠️".yellow());
        Ok(())
    }

    async fn lock_device(&self) -> Result<()> {
        println!("  {} lock_device not supported on Web", "⚠️".yellow());
        Ok(())
    }

    async fn unlock_device(&self) -> Result<()> {
        println!("  {} unlock_device not supported on Web", "⚠️".yellow());
        Ok(())
    }

    async fn install_app(&self, _path: &str) -> Result<()> {
        println!("  {} install_app not supported on Web", "⚠️".yellow());
        Ok(())
    }

    async fn uninstall_app(&self, _app_id: &str) -> Result<()> {
        println!("  {} uninstall_app not supported on Web", "⚠️".yellow());
        Ok(())
    }

    async fn background_app(&self, _app_id_opt: Option<&str>, duration_ms: u64) -> Result<()> {
        println!("  {} background_app: waiting {}ms (fake)", "⏳".blue(), duration_ms);
        tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
        Ok(())
    }

    async fn set_orientation(&self, mode: crate::parser::types::Orientation) -> Result<()> {
        use crate::parser::types::Orientation;

        let w = self.config.viewport_width;
        let h = self.config.viewport_height;
        let (new_w, new_h, angle, orientation_type) = match mode {
            Orientation::Portrait => (w.min(h), w.max(h), 0, ScreenOrientationType::PortraitPrimary),
            Orientation::UpsideDown => (w.min(h), w.max(h), 180, ScreenOrientationType::PortraitSecondary),
            Orientation::Landscape | Orientation::LandscapeLeft => {
                (w.max(h), w.min(h), 90, ScreenOrientationType::LandscapePrimary)
            }
            Orientation::LandscapeRight => {
                (w.max(h), w.min(h), -90, ScreenOrientationType::LandscapeSecondary)
            }
        };

        let page = self.page.lock().await;
        page.execute(
            SetDeviceMetricsOverrideParams::builder()
                .width(new_w as i64)
                .height(new_h as i64)
                .device_scale_factor(1.0)
                .mobile(false)
                .screen_orientation(ScreenOrientation::new(orientation_type, angle))
                .build()
                .map_err(|e| anyhow::anyhow!(e))?,
        )
        .await?;

        println!("  {} Set Viewport: {}x{}", "📐".cyan(), new_w, new_h);
        Ok(())
    }

    async fn start_profiling(
        &self,
        _params: Option<crate::parser::types::StartProfilingParams>,
    ) -> Result<()> {
        let page = self.page.lock().await;
        page.evaluate(
            "window.performance.clearResourceTimings(); window.performance.clearMarks(); window.performance.clearMeasures();",
        )
        .await?;
        Ok(())
    }

    async fn stop_profiling(&self) -> Result<()> {
        Ok(())
    }

    async fn get_performance_metrics(&self) -> Result<std::collections::HashMap<String, f64>> {
        let page = self.page.lock().await;
        let js = r#"() => {
             const nav = performance.getEntriesByType('navigation')[0] || {};
             const paint = performance.getEntriesByType('paint') || [];
             let fcp = 0;
             const fcpEntry = paint.find(p => p.name === 'first-contentful-paint');
             if (fcpEntry) fcp = fcpEntry.startTime;
             let memory = 0;
             if (performance.memory) { memory = performance.memory.usedJSHeapSize; }
             return { loadTime: nav.loadEventEnd - nav.loadEventStart, duration: nav.duration, fcp: fcp, jsHeapSize: memory };
        }"#;
        let json: serde_json::Value = page.evaluate(js).await?.into_value()?;

        let mut metrics = std::collections::HashMap::new();
        if let Some(val) = json.get("duration").and_then(|v| v.as_f64()) {
            metrics.insert("load_time_ms".to_string(), val);
        }
        if let Some(val) = json.get("fcp").and_then(|v| v.as_f64()) {
            metrics.insert("fcp_ms".to_string(), val);
        }
        if let Some(mem_bytes) = json.get("jsHeapSize").and_then(|v| v.as_f64()) {
            if mem_bytes > 0.0 {
                metrics.insert("memory_heap_mb".to_string(), mem_bytes / 1024.0 / 1024.0);
            }
        }
        Ok(metrics)
    }

    async fn set_cpu_throttling(&self, rate: f64) -> Result<()> {
        let page = self.page.lock().await;
        page.execute(SetCpuThrottlingRateParams::new(rate)).await?;
        println!("  {} CPU throttling set to {}x", "🐢".cyan(), rate);
        Ok(())
    }

    async fn set_network_conditions(&self, profile: &str) -> Result<()> {
        let (latency, download, upload) = match profile.to_lowercase().as_str() {
            "slow-3g" | "slow3g" => (400.0, 400.0 * 1024.0 / 8.0, 400.0 * 1024.0 / 8.0),
            "fast-3g" | "fast3g" => (150.0, 1.6 * 1024.0 * 1024.0 / 8.0, 750.0 * 1024.0 / 8.0),
            "4g" => (20.0, 4.0 * 1024.0 * 1024.0 / 8.0, 3.0 * 1024.0 * 1024.0 / 8.0),
            "offline" => {
                let page = self.page.lock().await;
                page.execute(
                    EmulateNetworkConditionsParams::builder()
                        .offline(true)
                        .latency(0.0)
                        .download_throughput(-1.0)
                        .upload_throughput(-1.0)
                        .build()
                        .map_err(|e| anyhow::anyhow!(e))?,
                )
                .await?;
                return Ok(());
            }
            _ => {
                println!("  {} Unknown network profile '{}', ignoring", "⚠️".yellow(), profile);
                return Ok(());
            }
        };

        let page = self.page.lock().await;
        page.execute(
            EmulateNetworkConditionsParams::builder()
                .offline(false)
                .latency(latency)
                .download_throughput(download)
                .upload_throughput(upload)
                .build()
                .map_err(|e| anyhow::anyhow!(e))?,
        )
        .await?;
        println!("  {} Network emulation set to '{}'", "🌐".cyan(), profile);
        Ok(())
    }
}

async fn launch_chromium_browser(config: &WebDriverConfig) -> Result<(Browser, chromiumoxide::Handler)> {
    let mut builder = BrowserConfig::builder();
    builder = if config.headless {
        builder.new_headless_mode()
    } else {
        builder.with_head()
    };

    let env_path = std::env::var("PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH")
        .ok()
        .map(std::path::PathBuf::from);
    let system_path = find_system_browser();
    let chrome_path = find_chrome_explicitly();

    if let Some(ref path) = env_path {
        println!("{} Using browser from env: {}", "🌐".blue(), path.display());
        builder = builder.chrome_executable(path);
    } else if let Some(ref path) = system_path {
        println!("{} Using discovered browser: {}", "🌐".blue(), path.display());
        builder = builder.chrome_executable(path);
    } else if let Some(ref path) = chrome_path {
        println!("{} Using explicitly found Chrome: {}", "🌐".blue(), path.display());
        builder = builder.chrome_executable(path);
    } else {
        println!(
            "{} No browser executable found. Attempting default launch if possible...",
            "ℹ".blue()
        );
    }

    builder = builder.args([
        "--no-sandbox",
        "--disable-setuid-sandbox",
        "--disable-dev-shm-usage",
        "--disable-gpu",
        "--ignore-certificate-errors",
    ]);

    if !config.close_when_finish {
        builder = builder.port(9222);
        println!(
            "{} Browser will stay open for reuse (closeWhenFinish: false)",
            "📌".cyan()
        );
    }

    let browser_config = builder.build().map_err(|e| anyhow::anyhow!(e))?;
    Ok(Browser::launch(browser_config).await?)
}

fn find_system_browser() -> Option<std::path::PathBuf> {
    let common_paths = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
    ];

    for path in common_paths {
        let p = std::path::Path::new(path);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }
    None
}

/// Launch Chrome externally as a detached process (not managed by chromiumoxide)
/// This allows the browser to stay open after the test ends
fn launch_chrome_externally() -> Result<()> {
    let chrome_path = find_chrome_explicitly()
        .ok_or_else(|| anyhow::anyhow!("Could not find Chrome/Chromium browser"))?;

    #[cfg(target_os = "macos")]
    {
        let path_str = chrome_path.to_string_lossy();
        if let Some(app_idx) = path_str.rfind(".app/") {
            let app_path = &path_str[..app_idx + 4];
            println!("{} Launching Chrome via 'open' command from: {}", "🍎".blue(), app_path);

            let status = std::process::Command::new("open")
                .args(["-n", "-a", app_path, "--args"])
                .args([
                    "--remote-debugging-port=9222",
                    "--no-first-run",
                    "--no-default-browser-check",
                    "--disable-default-apps",
                    "--user-data-dir=/tmp/lumi-chrome-profile",
                ])
                .status()
                .context("Failed to run 'open' command")?;

            if !status.success() {
                anyhow::bail!("'open' command failed");
            }
            println!("{} Chrome launched via open command", "✅".green());
            return Ok(());
        }
    }

    println!("{} Launching detached Chrome binary from: {}", "🌐".blue(), chrome_path.display());

    let mut cmd = std::process::Command::new(&chrome_path);
    cmd.args([
        "--remote-debugging-port=9222",
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-default-apps",
        "--user-data-dir=/tmp/lumi-chrome-profile",
    ]);

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
    }

    let child = cmd.spawn().context("Failed to spawn Chrome process")?;
    println!("{} Chrome launched with PID: {}", "✅".green(), child.id());
    Ok(())
}

fn find_chrome_explicitly() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("mdfind")
            .args(["kMDItemCFBundleIdentifier", "com.google.Chrome"])
            .output()
        {
            if let Ok(path_str) = String::from_utf8(output.stdout) {
                for line in path_str.lines() {
                    let chrome_path = std::path::Path::new(line).join("Contents/MacOS/Google Chrome");
                    if chrome_path.exists() {
                        return Some(chrome_path);
                    }
                }
            }
        }
    }

    let chrome_paths = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
        "/snap/bin/chromium",
    ];

    for path_str in &chrome_paths {
        let p = std::path::Path::new(path_str);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let chrome_path = std::path::Path::new(&local_app_data).join("Google/Chrome/Application/chrome.exe");
            if chrome_path.exists() {
                return Some(chrome_path);
            }
        }
    }

    None
}
