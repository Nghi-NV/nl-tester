//! Web Driver implementation using Playwright
//!
//! This driver enables web browser automation testing using the Playwright library.

use anyhow::{Context, Result};
use async_trait::async_trait;
use playwright::api::{Browser, BrowserContext, Page, Viewport};
use playwright::Playwright;
// Import RecordVideo manually if not exported in api prelude
use playwright::api::browser_type::RecordVideo;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::driver::common;
use crate::driver::image_matcher::{find_template, ImageRegion, MatchConfig};
use crate::driver::traits::{PlatformDriver, RelativeDirection, Selector, SwipeDirection};
use colored::Colorize;
use std::sync::Mutex as StdMutex;
use std::sync::OnceLock;

/// Global storage for persistent browser - prevents browser from closing when closeWhenFinish is false
fn get_persistent_browser() -> &'static StdMutex<Option<PersistentBrowserState>> {
    static PERSISTENT_BROWSER: OnceLock<StdMutex<Option<PersistentBrowserState>>> = OnceLock::new();
    PERSISTENT_BROWSER.get_or_init(|| StdMutex::new(None))
}

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

        // Check for CDP endpoint from env
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

/// Web Driver using Playwright
pub struct WebDriver {
    #[allow(dead_code)]
    playwright: Arc<Playwright>,
    #[allow(dead_code)]
    browser: Arc<Browser>,
    #[allow(dead_code)]
    context: Arc<BrowserContext>,
    page: Arc<Mutex<Page>>,
    config: WebDriverConfig,
    /// Current recording output path asked by user
    current_recording_path: Arc<Mutex<Option<String>>>,
    /// Captured console logs
    console_logs: Arc<Mutex<Vec<String>>>,
}

impl WebDriver {
    /// Create a new WebDriver instance
    pub async fn new(config: WebDriverConfig) -> Result<Self> {
        // Set FFmpeg path if found (MUST be set before initialize)
        if let Ok(ffmpeg_path) = crate::utils::binary_resolver::find_ffmpeg() {
            println!("{} Found FFmpeg at: {}", "🎥".blue(), ffmpeg_path.display());
            std::env::set_var("PLAYWRIGHT_FFMPEG_PATH", &ffmpeg_path);

            // Also prepend to PATH just in case
            if let Some(parent) = ffmpeg_path.parent() {
                if let Ok(current_path) = std::env::var("PATH") {
                    let new_path = format!("{}:{}", parent.display(), current_path);
                    std::env::set_var("PATH", new_path);
                }
            }
        } else {
            println!(
                "{} FFmpeg not found, video recording might fail",
                "⚠️".yellow()
            );
        }

        // Initialize Playwright
        let playwright = Playwright::initialize()
            .await
            .context("Failed to initialize Playwright")?;

        // Launch or connect to browser based on config
        let browser = match config.browser_type {
            BrowserType::Chromium => {
                let chromium = playwright.chromium();

                // Try to connect to existing browser via CDP if endpoint provided
                // or if closeWhenFinish is false (persistent mode)
                let cdp_endpoint = config.cdp_endpoint.clone().or_else(|| {
                    if !config.close_when_finish {
                        // Check if browser is running on default port
                        Some("http://localhost:9222".to_string())
                    } else {
                        None
                    }
                });

                if let Some(ref endpoint) = cdp_endpoint {
                    // Try to connect to existing browser
                    println!(
                        "{} Trying to connect to browser at: {}",
                        "🔌".blue(),
                        endpoint
                    );
                    match chromium
                        .connect_over_cdp_builder(endpoint)
                        .connect_over_cdp()
                        .await
                    {
                        Ok(b) => {
                            println!("{} Connected to existing browser!", "✅".green());
                            b
                        }
                        Err(e) => {
                            println!(
                                "{} Could not connect to existing browser: {}",
                                "⚠️".yellow(),
                                e
                            );
                            if !config.close_when_finish {
                                // Launch Chrome externally to keep it running after test
                                println!(
                                    "{} Launching Chrome externally with remote debugging port...",
                                    "🚀".blue()
                                );
                                launch_chrome_externally()?;

                                // Wait a moment for Chrome to start
                                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

                                // Now connect to it via CDP
                                match chromium
                                    .connect_over_cdp_builder(endpoint)
                                    .connect_over_cdp()
                                    .await
                                {
                                    Ok(b) => {
                                        println!(
                                            "{} Connected to externally launched Chrome!",
                                            "✅".green()
                                        );
                                        b
                                    }
                                    Err(e2) => {
                                        anyhow::bail!(
                                            "Failed to connect to Chrome after external launch: {}",
                                            e2
                                        );
                                    }
                                }
                            } else {
                                // Normal launch via Playwright
                                launch_chromium_browser(&chromium, &config).await?
                            }
                        }
                    }
                } else {
                    launch_chromium_browser(&chromium, &config).await?
                }
            }
            BrowserType::Firefox => {
                playwright
                    .firefox()
                    .launcher()
                    .headless(config.headless)
                    .launch()
                    .await?
            }
            BrowserType::Webkit => {
                playwright
                    .webkit()
                    .launcher()
                    .headless(config.headless)
                    .launch()
                    .await?
            }
        };

        // Create browser context
        let record_video = std::env::var("LUMI_VIDEO_RECORD")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        // Try to reuse existing context for persistence
        let reused_context = if !config.close_when_finish {
            let contexts = browser.contexts()?;
            if let Some(ctx) = contexts.into_iter().next() {
                println!("{} Reusing existing browser context", "♻️".green());
                Some(ctx)
            } else {
                None
            }
        } else {
            None
        };

        let context = if let Some(ctx) = reused_context {
            ctx
        } else if record_video {
            let temp_dir = std::env::temp_dir().join("lumi_tester_videos");
            std::fs::create_dir_all(&temp_dir).ok();
            browser
                .context_builder()
                .record_video(RecordVideo {
                    dir: &temp_dir,
                    size: None,
                })
                .build()
                .await?
        } else {
            browser.context_builder().build().await?
        };

        // Create or reuse page
        let page = if !config.close_when_finish {
            // Need a small delay for Playwright to sync pages from CDP
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            let pages = context.pages().unwrap_or_default();

            if let Some(p) = pages.into_iter().next() {
                println!("{} Reusing existing page", "📄".green());
                p.bring_to_front().await.ok();
                p
            } else {
                context.new_page().await?
            }
        } else {
            context.new_page().await?
        };

        // Initialize console logs storage
        let console_logs = Arc::new(Mutex::new(Vec::new()));
        // let logs_clone = console_logs.clone();

        // Attach console listener
        // Note: playwright-rust 0.0.x doesn't expose on_console on Page yet or it has different signature.
        // Disabling for now until crate update or workaround found.
        /*
        page.on_console(move |msg| {
            let logs_clone = logs_clone.clone();
            let text = msg.text().unwrap_or_default();
            tokio::task::spawn(async move {
                let mut logs = logs_clone.lock().await;
                logs.push(text);
            });
        });
        */

        // Set viewport size if configured (Playwright expects it on context or page)
        page.set_viewport_size(Viewport {
            width: config.viewport_width as i32,
            height: config.viewport_height as i32,
        })
        .await?;

        Ok(Self {
            playwright: Arc::new(playwright),
            browser: Arc::new(browser),
            context: Arc::new(context),
            page: Arc::new(Mutex::new(page)),
            config,
            current_recording_path: Arc::new(Mutex::new(None)),
            console_logs,
        })
    }

    /// Find template image on screen
    async fn find_image_on_screen(
        &self,
        template_path: &str,
        region: Option<&str>,
    ) -> Result<Option<(i32, i32)>> {
        let total_start = std::time::Instant::now();
        let template_path_buf = Path::new(template_path).to_path_buf();
        if !template_path_buf.exists() {
            anyhow::bail!("Template image not found: {:?}", template_path_buf);
        }

        let image_region = region.map(|r| ImageRegion::from_str(r)).unwrap_or_default();
        if image_region != ImageRegion::Full {
            println!("      📍 Region: {:?}", image_region);
        }

        // Use page.screenshot() for fast in-memory handling
        let page = self.page.lock().await;

        println!("    {} Taking screenshot for image match...", "📷".blue());
        let screenshot_start = std::time::Instant::now();
        let screenshot_bytes = page
            .screenshot_builder()
            .r#type(playwright::api::ScreenshotType::Png)
            .screenshot()
            .await?;
        println!("      ⏱ Screenshot: {:?}", screenshot_start.elapsed());

        drop(page); // Release lock during processing

        // Match
        let match_start = std::time::Instant::now();
        let result = tokio::task::spawn_blocking(move || -> Result<Option<(i32, i32)>> {
            let img_screen = image::load_from_memory(&screenshot_bytes)?.to_luma8();
            let img_template = image::open(&template_path_buf)?.to_luma8();

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

    /// Convert Selector to Playwright selector string
    fn selector_to_playwright(&self, selector: &Selector) -> String {
        match selector {
            Selector::Text(text, index, _) => {
                if *index == 0 {
                    format!("text=\"{}\"", text)
                } else {
                    format!("xpath=(//*[text()=\"{}\"])[{}]", text, index + 1)
                }
            }
            Selector::TextRegex(regex, index) => {
                if *index == 0 {
                    format!("text=/{}/", regex)
                } else {
                    format!("xpath=(//*[matches(text(), \"{}\")])[{}]", regex, index + 1)
                }
            }
            Selector::Id(id, index) => {
                if *index == 0 {
                    format!("#{}", id)
                } else {
                    format!("xpath=(//*[@id=\"{}\"])[{}]", id, index + 1)
                }
            }
            Selector::IdRegex(regex, _index) => {
                println!(
                    "{} IdRegex not implemented for Web yet: {}",
                    "⚠️".yellow(),
                    regex
                );
                "#unsupported_id_regex".to_string()
            }
            Selector::Type(t, index) => {
                let base = map_web_type(t);
                if *index == 0 {
                    base
                } else {
                    format!("xpath=({})[{}]", self.to_xpath(&base), index + 1)
                }
            }
            Selector::Role(role, index) => {
                if *index == 0 {
                    format!("[role=\"{}\"]", role)
                } else {
                    format!("xpath=(//*[@role=\"{}\"])[{}]", role, index + 1)
                }
            }
            Selector::Css(css) => css.clone(),
            Selector::XPath(xpath) => format!("xpath={}", xpath),
            Selector::Placeholder(p, index) => {
                format!("xpath=(//*[@placeholder=\"{}\"])[{}]", p, index + 1)
            }
            Selector::Point { .. } => String::new(), // Handle separately
            Selector::AccessibilityId(id) | Selector::Description(id, _) => {
                format!("[aria-label=\"{}\"]", id)
            }
            Selector::DescriptionRegex(regex, index) => {
                if *index == 0 {
                    format!("text=/{}/", regex) // Fallback to text match for now as aria-label regex needs different strategy or purely xpath
                } else {
                    format!(
                        "xpath=(//*[matches(@aria-label, \"{}\")])[{}]",
                        regex,
                        index + 1
                    )
                }
            }
            Selector::Image { .. } => unimplemented!("Image selector not supported for Web"),
            Selector::Relative {
                target,
                anchor,
                direction,
                max_dist,
            } => {
                let pseudo = match direction {
                    RelativeDirection::LeftOf => "left-of",
                    RelativeDirection::RightOf => "right-of",
                    RelativeDirection::Above => "above",
                    RelativeDirection::Below => "below",
                    RelativeDirection::Near => "near",
                };

                // Get anchor selector that is compatible with CSS pseudo-classes
                let anchor_sel = match anchor.as_ref() {
                    Selector::Text(t, _, _) => format!(":text(\"{}\")", t),
                    Selector::Id(id, _) => format!("#{}", id),
                    Selector::Type(t, _) => map_web_type(t),
                    Selector::Placeholder(p, _) => format!("[placeholder=\"{}\"]", p),
                    Selector::Role(r, _) => format!("[role=\"{}\"]", r),
                    Selector::Image { .. } => unimplemented!("Image anchor not supported"),
                    _ => self.selector_to_playwright(anchor),
                };

                let dist_suffix = max_dist.map(|d| format!(", {}", d)).unwrap_or_default();

                // Get base selector and index from target
                // We prefer simple selectors as the base for layout pseudo-classes
                let (base, index) = match target.as_ref() {
                    Selector::Text(t, idx, _) => (format!(":text(\"{}\")", t), *idx),
                    Selector::Id(id, idx) => (format!("#{}", id), *idx),
                    Selector::Type(t, idx) => (map_web_type(t), *idx),
                    Selector::Placeholder(p, idx) => (format!("[placeholder=\"{}\"]", p), *idx),
                    Selector::Role(r, idx) => (format!("[role=\"{}\"]", r), *idx),
                    Selector::Relative { .. } => {
                        panic!("Relative selectors should be handled by find_relative_element")
                    }
                    Selector::Point { .. } => panic!("Point selectors not supported for web"),
                    Selector::Image { .. } => {
                        unimplemented!("Image selector not supported for Web")
                    }
                    Selector::HasChild { .. } => {
                        panic!("HasChild not supported as relative target base")
                    }
                    _ => {
                        let full = self.selector_to_playwright(target);
                        // If it's a complex selector, try to take the first part
                        let base = full.split(" >> ").next().unwrap_or("*").to_string();
                        (base, 0)
                    }
                };

                // Correct Playwright layout syntax is target:pseudo(anchor)
                if index == 0 {
                    format!("{}:{}({}{})", base, pseudo, anchor_sel, dist_suffix)
                } else {
                    format!(
                        "{}:{}({}{}) >> nth={}",
                        base, pseudo, anchor_sel, dist_suffix, index
                    )
                }
            }
            Selector::AnyClickable(index) => {
                // For web, find any clickable element (buttons, links, elements with onclick)
                if *index == 0 {
                    "button, a, [onclick], [role=\"button\"]".to_string()
                } else {
                    format!(
                        "xpath=(//button|//a|//*[@onclick]|//*[@role='button'])[{}]",
                        index + 1
                    )
                }
            }
            Selector::HasChild { parent, child } => {
                let p = self.selector_to_playwright(parent);
                let c = self.selector_to_playwright(child);
                format!("{} >> :has({})", p, c)
            }
        }
    }

    fn to_xpath(&self, selector: &str) -> String {
        if selector.starts_with("xpath=") {
            selector.trim_start_matches("xpath=").to_string()
        } else if selector.starts_with("*[") {
            format!("//*{}", selector.trim_start_matches("*"))
        } else if selector.contains("[") {
            // Handle tag[attr='val']
            let parts: Vec<&str> = selector.split('[').collect();
            if parts.len() == 2 {
                let tag = if parts[0].is_empty() { "*" } else { parts[0] };
                let attr = parts[1].trim_end_matches(']');
                format!(
                    "//{} [@{}]",
                    tag,
                    attr.replace("='", "=\"").replace("'", "\"")
                )
            } else {
                format!("//{}", selector)
            }
        } else {
            format!("//{}", selector)
        }
    }

    /// Find element handle by regex on ID using JS
    /// Find element handle by regex on ID using JS
    async fn find_element_by_id_regex(
        &self,
        regex: &str,
        index: usize,
    ) -> Result<Option<playwright::api::JsHandle>> {
        let page = self.page.lock().await;

        // JS function to find element by ID regex
        let js = format!(
            r#"
            () => {{
                try {{
                    const pattern = new RegExp("{}");
                    const allElements = document.querySelectorAll('[id]');
                    let count = 0;
                    for (const el of allElements) {{
                        if (pattern.test(el.id)) {{
                            if (count === {}) {{
                                return el;
                            }}
                            count++;
                        }}
                    }}
                    return null;
                }} catch (e) {{
                    return null;
                }}
            }}
        "#,
            regex, index
        );

        let mut handle = page.evaluate_js_handle(&js, Some(())).await?;

        // Check if handle points to null
        // We can't easily check if JsHandle is null without evaluating or getting json value
        // But checking json_value might be expensive if it's a big element?
        // Handles to elements don't serialize to JSON well (circular or just {})?
        // Actually, elements usually serialize to empty object or specific representation.
        // Null serializes to Value::Null.
        // Let's assume if it's an element it won't be Null.

        // Helper to check if null
        let json = handle.json_value::<serde_json::Value>().await?;
        if json == serde_json::Value::Null {
            Ok(None)
        } else {
            Ok(Some(handle))
        }
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

        // If URL is relative and we have a base URL, combine them
        let full_url = if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else if let Some(ref base) = self.config.base_url {
            format!("{}{}", base.trim_end_matches('/'), url)
        } else {
            url.to_string()
        };

        page.goto_builder(&full_url)
            .goto()
            .await
            .context("Failed to navigate to URL")?;

        Ok(())
    }

    async fn stop_app(&self, _app_id: &str) -> Result<()> {
        let page = self.page.lock().await;
        page.goto_builder("about:blank").goto().await?;
        Ok(())
    }

    async fn tap(&self, selector: &Selector) -> Result<()> {
        match selector {
            Selector::IdRegex(regex, index) => {
                if let Some(handle) = self.find_element_by_id_regex(regex, *index).await? {
                    // Click using JS since we have a JsHandle
                    // Use page.evaluate to execute click on the handle
                    let page = self.page.lock().await;
                    page.evaluate::<_, ()>("el => el.click()", handle).await?;
                } else {
                    anyhow::bail!("Element not found for IdRegex: {}", regex);
                }
            }
            Selector::Point { x, y } => {
                let page = self.page.lock().await;
                page.mouse.r#move(*x as f64, *y as f64, None).await?;
                page.mouse.down(None, None).await?;
                page.mouse.up(None, None).await?;
            }
            Selector::Image { path, region } => {
                let pos = self.find_image_on_screen(path, region.as_deref()).await?;
                if let Some((x, y)) = pos {
                    println!(
                        "    {} Tapping on image match at ({}, {})",
                        "👆".cyan(),
                        x,
                        y
                    );
                    let page = self.page.lock().await;
                    page.mouse.r#move(x as f64, y as f64, None).await?;
                    page.mouse.down(None, None).await?;
                    page.mouse.up(None, None).await?;
                } else {
                    anyhow::bail!("Image not found on screen: {}", path);
                }
            }
            _ => {
                let page = self.page.lock().await;
                let sel = self.selector_to_playwright(selector);
                match page.click_builder(&sel).click().await {
                    Ok(_) => {}
                    Err(e) => {
                        println!(
                            "{} Click failed for selector '{}': {:?}",
                            "❌".red(),
                            sel,
                            e
                        );
                        return Err(anyhow::anyhow!("Failed to click: {}. Error: {:?}", sel, e));
                    }
                }
            }
        }

        Ok(())
    }

    async fn long_press(&self, selector: &Selector, duration_ms: u64) -> Result<()> {
        let page = self.page.lock().await;

        let (x, y) = match selector {
            Selector::IdRegex(regex, index) => {
                // Find element using JS
                drop(page); // release lock for helper
                if let Some(handle) = self.find_element_by_id_regex(regex, *index).await? {
                    // Get bounding box via JS
                    let page = self.page.lock().await;
                    let json: serde_json::Value = page
                        .evaluate(
                            "el => {
                            const r = el.getBoundingClientRect();
                            return { x: r.x, y: r.y, width: r.width, height: r.height };
                        }",
                            handle,
                        )
                        .await?;

                    let x = json.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let y = json.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let w = json.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let h = json.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0);

                    (x + w / 2.0, y + h / 2.0)
                } else {
                    anyhow::bail!("Element not found for IdRegex: {}", regex);
                }
            }
            _ => {
                let sel = self.selector_to_playwright(selector);
                if let Some(el) = page.query_selector(&sel).await? {
                    el.scroll_into_view_if_needed(None).await?;
                    if let Some(box_model) = el.bounding_box().await? {
                        (
                            box_model.x + box_model.width / 2.0,
                            box_model.y + box_model.height / 2.0,
                        )
                    } else {
                        anyhow::bail!(
                            "Could not get bounding box implementation for selector: {}",
                            sel
                        );
                    }
                } else {
                    anyhow::bail!("Element not found implementation for selector: {}", sel);
                }
            }
        };

        // Re-acquire lock
        let page = self.page.lock().await;
        // move(x, y, steps)
        page.mouse.r#move(x, y, None).await?;
        // down(button, click_count)
        page.mouse.down(None, None).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
        // up(button, click_count)
        page.mouse.up(None, None).await?;

        Ok(())
    }

    async fn double_tap(&self, selector: &Selector) -> Result<()> {
        match selector {
            Selector::IdRegex(regex, index) => {
                if let Some(handle) = self.find_element_by_id_regex(regex, *index).await? {
                    // Dispatch dblclick event
                    let page = self.page.lock().await;
                    page.evaluate::<_, ()>(
                        "el => el.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))",
                        handle,
                    )
                    .await?;
                } else {
                    anyhow::bail!("Element not found for IdRegex: {}", regex);
                }
            }
            _ => {
                let page = self.page.lock().await;
                let sel = self.selector_to_playwright(selector);
                page.dblclick_builder(&sel).dblclick().await?;
            }
        }
        Ok(())
    }

    async fn right_click(&self, selector: &Selector) -> Result<()> {
        match selector {
            Selector::IdRegex(regex, index) => {
                if let Some(handle) = self.find_element_by_id_regex(regex, *index).await? {
                    // Dispatch contextmenu event for right click simulation
                    let page = self.page.lock().await;
                    page.evaluate::<_, ()>("el => el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, button: 2, buttons: 2 }))", handle).await?;
                } else {
                    anyhow::bail!("Element not found for IdRegex: {}", regex);
                }
            }
            _ => {
                let page = self.page.lock().await;
                let sel = self.selector_to_playwright(selector);
                page.click_builder(&sel)
                    .button(playwright::api::MouseButton::Right)
                    .click()
                    .await?;
            }
        }
        Ok(())
    }

    async fn input_text(&self, text: &str, _unicode: bool) -> Result<()> {
        let page = self.page.lock().await;
        page.keyboard.input_text(text).await?;
        Ok(())
    }

    async fn erase_text(&self, _char_count: Option<u32>) -> Result<()> {
        let page = self.page.lock().await;
        // Select all (Meta+A) manually
        page.keyboard.down("Meta").await?;
        page.keyboard.down("a").await?;
        page.keyboard.up("a").await?;
        page.keyboard.up("Meta").await?;

        // Delete
        page.keyboard.down("Backspace").await?;
        page.keyboard.up("Backspace").await?;
        Ok(())
    }

    async fn hide_keyboard(&self) -> Result<()> {
        // Web doesn't have a keyboard to hide
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
            // If explicit scroll source provided, scroll that element
            match selector {
                Selector::IdRegex(regex, index) => {
                    // Check specific id regex handling
                    if let Some(handle) = self.find_element_by_id_regex(&regex, index).await? {
                        let js = format!("el => el.scrollBy({}, {})", dx, dy);
                        let page = self.page.lock().await;
                        page.evaluate::<_, ()>(&js, handle).await?;
                    }
                }
                _ => {
                    // Use playwright selector
                    let sel = self.selector_to_playwright(&selector);
                    let page = self.page.lock().await;
                    if let Some(handle) = page.query_selector(&sel).await? {
                        let js = format!("el => el.scrollBy({}, {})", dx, dy);
                        page.evaluate::<_, ()>(&js, handle).await?;
                    }
                }
            }
        } else {
            // Default: scroll window
            let page = self.page.lock().await;
            let js = format!("window.scrollBy({}, {})", dx, dy);
            page.evaluate::<_, ()>(&js, ()).await?;
        }

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
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        Ok(false)
    }

    async fn is_visible(&self, selector: &Selector) -> Result<bool> {
        match selector {
            Selector::IdRegex(regex, index) => {
                let handle = self.find_element_by_id_regex(regex, *index).await?;
                if let Some(h) = handle {
                    // Check visibility using JS
                    let page = self.page.lock().await;
                    let visible: bool = page.evaluate(
                        "el => {
                            if (!el.isConnected) return false;
                            const style = window.getComputedStyle(el);
                            return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';
                        }",
                        h,
                    )
                    .await?;
                    Ok(visible)
                } else {
                    Ok(false)
                }
            }
            Selector::Image { path, region } => {
                let found = self.find_image_on_screen(path, region.as_deref()).await?;
                Ok(found.is_some())
            }
            _ => {
                let page = self.page.lock().await;
                let sel = self.selector_to_playwright(selector);
                let element = page.query_selector(&sel).await?;
                if let Some(el) = element {
                    Ok(el.is_visible().await?)
                } else {
                    Ok(false)
                }
            }
        }
    }

    async fn tap_by_type_index(&self, element_type: &str, index: u32) -> Result<()> {
        let page = self.page.lock().await;
        let elements = page.query_selector_all(element_type).await?;
        if let Some(el) = elements.get(index as usize) {
            el.click_builder().click().await?;
            Ok(())
        } else {
            anyhow::bail!("Element not found: {} at index {}", element_type, index)
        }
    }

    async fn input_by_type_index(&self, element_type: &str, index: u32, text: &str) -> Result<()> {
        let page = self.page.lock().await;
        let elements = page.query_selector_all(element_type).await?;
        if let Some(el) = elements.get(index as usize) {
            el.fill_builder(text).fill().await?;
            Ok(())
        } else {
            anyhow::bail!("Element not found: {} at index {}", element_type, index)
        }
    }

    async fn wait_for_element(&self, selector: &Selector, timeout_ms: u64) -> Result<bool> {
        match selector {
            Selector::IdRegex(regex, index) => {
                let start = std::time::Instant::now();
                while start.elapsed().as_millis() < timeout_ms as u128 {
                    if let Some(_) = self.find_element_by_id_regex(regex, *index).await? {
                        return Ok(true);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                Ok(false)
            }
            Selector::Image { .. } => {
                let start = std::time::Instant::now();
                while start.elapsed().as_millis() < timeout_ms as u128 {
                    if self.is_visible(selector).await? {
                        return Ok(true);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
                Ok(false)
            }
            _ => {
                let page = self.page.lock().await;
                let sel = self.selector_to_playwright(selector);

                let result = page
                    .wait_for_selector_builder(&sel)
                    .timeout(timeout_ms as f64)
                    .wait_for_selector()
                    .await;

                Ok(result.is_ok())
            }
        }
    }

    async fn wait_for_absence(&self, selector: &Selector, timeout_ms: u64) -> Result<bool> {
        let start = std::time::Instant::now();

        // Polling loop
        while start.elapsed().as_millis() < timeout_ms as u128 {
            if !self.is_visible(selector).await? {
                return Ok(true);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        Ok(false)
    }

    async fn get_element_text(&self, selector: &Selector) -> Result<String> {
        match selector {
            Selector::IdRegex(regex, index) => {
                if let Some(handle) = self.find_element_by_id_regex(regex, *index).await? {
                    let page = self.page.lock().await;
                    let js = "el => el.value || el.innerText || el.textContent || ''";
                    let text: String = page.evaluate(js, handle).await?;
                    Ok(text)
                } else {
                    Ok(String::new())
                }
            }
            _ => {
                let page = self.page.lock().await;
                let sel = self.selector_to_playwright(selector);

                // Use eval_on_selector to get value or text
                let js = "el => el.value || el.innerText || el.textContent || ''";

                // Use None for argument, seems Playwright might expect Option or infer it
                match page
                    .evaluate_on_selector::<String, _>(&sel, js, None::<String>)
                    .await
                {
                    Ok(text) => Ok(text),
                    Err(_) => Ok(String::new()),
                }
            }
        }
    }

    async fn open_link(&self, url: &str, _app_id: Option<&str>) -> Result<()> {
        self.launch_app(url, false).await
    }

    async fn compare_screenshot(
        &self,
        reference_path: &Path,
        tolerance_percent: f64,
    ) -> Result<f64> {
        use image::GenericImageView;

        // Take current screenshot to temp file
        let temp_path = std::env::temp_dir().join("lumi_tester_compare.png");
        self.take_screenshot(temp_path.to_str().unwrap()).await?;

        // Load both images
        let current = image::open(&temp_path)?;
        let reference = image::open(reference_path)?;

        // Cleanup temp file
        let _ = std::fs::remove_file(&temp_path);

        // Check dimensions
        if current.dimensions() != reference.dimensions() {
            return Ok(100.0); // 100% different if dimensions don't match
        }

        let (width, height) = current.dimensions();
        let total_pixels = (width * height) as f64;
        let mut diff_pixels = 0u64;

        // Compare pixels
        for y in 0..height {
            for x in 0..width {
                let c1 = current.get_pixel(x, y);
                let c2 = reference.get_pixel(x, y);

                // Check if pixels are different (allowing some tolerance per channel)
                let channel_diff =
                    c1.0.iter()
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
            Ok(0.0) // Within tolerance
        }
    }

    async fn take_screenshot(&self, path: &str) -> Result<()> {
        let page = self.page.lock().await;
        let path_buf = std::path::PathBuf::from(path);

        if let Some(parent) = path_buf.parent() {
            std::fs::create_dir_all(parent)?;
        }

        page.screenshot_builder()
            .path(path_buf)
            .screenshot()
            .await?;
        Ok(())
    }

    async fn back(&self) -> Result<()> {
        let page = self.page.lock().await;
        // Use JavaScript for back navigation
        page.evaluate::<(), ()>("window.history.back()", ()).await?;
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
        let html = page.content().await?;
        Ok(html)
    }

    async fn dump_logs(&self, limit: u32) -> Result<String> {
        let logs = self.console_logs.lock().await;
        // Return up to `limit` last logs
        let count = logs.len();
        let start = if count > limit as usize {
            count - limit as usize
        } else {
            0
        };

        let output = logs[start..].join("\n");
        Ok(output)
    }

    async fn get_pixel_color(&self, x: i32, y: i32) -> Result<(u8, u8, u8)> {
        let page = self.page.lock().await;

        // Take screenshot and read pixel using common utility
        let screenshot_data = page.screenshot_builder().screenshot().await?;
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
        page.set_viewport_size(Viewport {
            width: new_w as i32,
            height: new_h as i32,
        })
        .await?;
        Ok(())
    }

    async fn start_recording(&self, path: &str) -> Result<()> {
        // Playwright records continuously if cached. We mark the current request.
        // In valid implementation, we might clear previous videos or mark start time.
        // For now, we just store the path where we want to save the video at the end.
        self.current_recording_path
            .lock()
            .await
            .replace(path.to_string());
        Ok(())
    }

    async fn stop_recording(&self) -> Result<()> {
        let page = self.page.lock().await;
        if let Ok(Some(video)) = page.video() {
            // Wait for video to be available/saved
            // Page closing often triggers save, but here we are mid-session.
            // video.save_as(path) copies it.
            if let Some(path) = self.current_recording_path.lock().await.take() {
                // Ensure directory exists
                if let Some(parent) = Path::new(&path).parent() {
                    std::fs::create_dir_all(parent).ok();
                }

                // Save video manually since save_as is private
                // Check if video.path() is public
                let src_path = video.path()?;
                std::fs::copy(&src_path, &path)?;
                // Optionally delete original?
                // std::fs::remove_file(src_path)?;

                println!("  {} Saved Web Recording: {}", "🎥".green(), path);

                // Note: This only saves the video up to now? Or does it?
                // Playwright "video" object represents the recording of the page.
                // It continues recording until page close.
                // save_as takes a snapshot or waits?
                // Docs say: "Saves the video to a user-specified path."
                // It might block until page closes if we want FULL video, but here we just want what we have?
                // Actually `save_as` might throw if video not finished?
                // Playwright Rust wrapper map: `video.save_as(path)`.
            }
        } else {
            println!(
                "  {} No video recording available (check context config)",
                "⚠️".yellow()
            );
        }
        Ok(())
    }

    async fn press_key(&self, key: &str) -> Result<()> {
        let page = self.page.lock().await;
        // Workaround for potential binding issue with press()
        page.keyboard.down(key).await?;
        page.keyboard.up(key).await?;
        Ok(())
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
        page.context().clear_cookies().await?;
        page.evaluate::<_, ()>(
            "() => { localStorage.clear(); sessionStorage.clear(); }",
            (),
        )
        .await?;
        Ok(())
    }

    async fn set_clipboard(&self, text: &str) -> Result<()> {
        let page = self.page.lock().await;
        // Note: Requires permissions in some environments
        page.evaluate::<_, ()>("txt => navigator.clipboard.writeText(txt)", text)
            .await?;
        Ok(())
    }

    async fn get_clipboard(&self) -> Result<String> {
        // Read via JS
        let page = self.page.lock().await;
        let text: String = page
            .evaluate("() => navigator.clipboard.readText()", ())
            .await?;
        Ok(text)
    }

    // New Commands Implementation

    async fn set_network_connection(&self, _wifi: Option<bool>, _data: Option<bool>) -> Result<()> {
        // Web only supports generic offline mode via CDP/Context
        // We will treat any "disable" as setting offline=true
        let offline = _wifi == Some(false) || _data == Some(false);

        self.context.set_offline(offline).await?;
        println!("  {} Set Web Connection Offline: {}", "🌐".cyan(), offline);
        Ok(())
    }

    async fn toggle_airplane_mode(&self) -> Result<()> {
        // Use a dirty check to toggle? Playwright doesn't easily expose current offline state getter on Context
        // We might need to track it in struct if we want true toggle.
        // For now, let's just assume we want to Toggle ON (offline) then OFF?
        // Or just warn that toggle is generic.
        // Better: let's try to assume it's ONLINE by default, so toggle requests OFFLINE.
        // Actually without state, toggle is dangerous. Let's just set offline=true for now or warn.
        println!(
            "  {} toggle_airplane_mode on web strictly sets offline=true (limitation)",
            "⚠️".yellow()
        );
        self.context.set_offline(true).await?;
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

        // For V1, we just take the first point and set it as static location
        // Animating/moving requires a background task which is more complex for now.
        let point = &points[0];

        let permissions = vec!["geolocation".to_string()];
        // Grant permissions for all origins (or just current)
        // Playwright Rust signature might differ slightly, let's try granting for all.
        // Actually, we should probably get current origin.

        // Grant permissions
        self.context.grant_permissions(&permissions, None).await?;

        // Set Geolocation
        self.context
            .set_geolocation(Some(&playwright::api::Geolocation {
                latitude: point.lat,
                longitude: point.lon,
                // GpsPoint currently doesn't carry accuracy info, defaulting to 10m
                accuracy: Some(10.0),
            }))
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
        let js = format!(
            r#"
            () => new Promise((resolve) => {{
                if (!navigator.geolocation) {{
                    resolve(null);
                    return;
                }}
                navigator.geolocation.getCurrentPosition(
                    (pos) => {{
                        resolve({{
                            lat: pos.coords.latitude,
                            lon: pos.coords.longitude
                        }});
                    }},
                    (err) => resolve(null),
                    {{ enableHighAccuracy: true, timeout: 5000, maximumAge: 0 }}
                );
            }})
            "#
        );

        while start.elapsed().as_millis() < timeout as u128 {
            let result: serde_json::Value = page.evaluate(&js, ()).await?;
            // println!("DEBUG: Location result: {:?}", result);

            if let Some(obj) = result.as_object() {
                if let (Some(c_lat), Some(c_lon)) = (
                    obj.get("lat").and_then(|v| v.as_f64()),
                    obj.get("lon").and_then(|v| v.as_f64()),
                ) {
                    let d_lat = (c_lat - lat).abs();
                    let d_lon = (c_lon - lon).abs();
                    if d_lat <= tolerance && d_lon <= tolerance {
                        return Ok(());
                    }
                }
            } else {
                // println!("DEBUG: Location invalid: {:?}", result);
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        Err(anyhow::anyhow!(
            "Timeout waiting for location {}, {}",
            lat,
            lon
        ))
    }

    async fn open_quick_settings(&self) -> Result<()> {
        println!(
            "  {} open_quick_settings not supported on Web",
            "⚠️".yellow()
        );
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
        // Simulate visibility change?
        let _page = self.page.lock().await;
        // TODO: Use CDP to set visibility state hidden?
        // For now, simple wait
        println!(
            "  {} background_app: waiting {}ms (fake)",
            "⏳".blue(),
            duration_ms
        );
        tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
        Ok(())
    }

    async fn set_orientation(&self, mode: crate::parser::types::Orientation) -> Result<()> {
        use crate::parser::types::Orientation;

        let w = self.config.viewport_width;
        let h = self.config.viewport_height;

        let (new_w, new_h) = match mode {
            Orientation::Portrait | Orientation::UpsideDown => (w, h),
            Orientation::Landscape | Orientation::LandscapeLeft | Orientation::LandscapeRight => {
                (h, w)
            } // Swap
        };

        let page = self.page.lock().await;
        page.set_viewport_size(playwright::api::Viewport {
            width: new_w as i32,
            height: new_h as i32,
        })
        .await?;

        println!("  {} Set Viewport: {}x{}", "📐".cyan(), new_w, new_h);
        Ok(())
    }

    async fn start_profiling(
        &self,
        _params: Option<crate::parser::types::StartProfilingParams>,
    ) -> Result<()> {
        // Clear performance timeline
        let page = self.page.lock().await;
        page.evaluate::<_, ()>("window.performance.clearResourceTimings(); window.performance.clearMarks(); window.performance.clearMeasures();", ()).await?;
        Ok(())
    }

    async fn stop_profiling(&self) -> Result<()> {
        // No-op for web, we read current state
        Ok(())
    }

    async fn get_performance_metrics(&self) -> Result<std::collections::HashMap<String, f64>> {
        let page = self.page.lock().await;

        // 1. Navigation Timing (Load, DOMContentLoaded) & Web Vitals (FCP, etc. if available via PO)
        let json: serde_json::Value = page
            .evaluate(
                "() => {
             const nav = performance.getEntriesByType('navigation')[0] || {};
             const paint = performance.getEntriesByType('paint') || [];
             let fcp = 0;
             const fcpEntry = paint.find(p => p.name === 'first-contentful-paint');
             if (fcpEntry) fcp = fcpEntry.startTime;

             let memory = 0;
             if (performance.memory) {
                 memory = performance.memory.usedJSHeapSize;
             }

             return {
                 loadTime: nav.loadEventEnd - nav.loadEventStart,
                 domContentLoadTime: nav.domContentLoadedEventEnd - nav.domContentLoadedEventStart,
                 duration: nav.duration, // Total load time
                 fcp: fcp,
                 jsHeapSize: memory
             };
         }",
                (),
            )
            .await?;

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
        // Slow down capability via CDP if available (Chromium only)
        // Playwright doesn't expose this easily in high level API
        println!(
            "  {} CPU throttling not available via standard Playwright API yet. (Target: {}x)",
            "⚠️".yellow(),
            rate
        );
        Ok(())
    }

    async fn set_network_conditions(&self, profile: &str) -> Result<()> {
        // Chromium only - emulate network
        if matches!(self.config.browser_type, BrowserType::Chromium) {
            let _context = &self.context;
            // Offline
            // slow 3g, fast 3g

            // Playwright doesn't have a direct 'emulateNetwork' on context at crate level yet in all versions
            // But let's check if we can simply warn for now as this requires CDP session access
            println!("  {} Network emulation '{}' only supported if underlying driver exposes CDP session. Skipping.", "⚠️".yellow(), profile);
        } else {
            println!(
                "  {} Network emulation only supported on Chromium",
                "⚠️".yellow()
            );
        }
        Ok(())
    }
}

/// Map common element type aliases to HTML tags
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

/// Launch a new Chromium browser with optional remote debugging support
async fn launch_chromium_browser(
    chromium: &playwright::api::BrowserType,
    config: &WebDriverConfig,
) -> Result<playwright::api::Browser> {
    let mut launcher = chromium.launcher();
    launcher = launcher.headless(config.headless);

    let env_path = std::env::var("PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH")
        .ok()
        .map(std::path::PathBuf::from);

    let system_path = find_system_browser();
    let chrome_path = find_chrome_explicitly();

    if let Some(ref path) = env_path {
        println!("{} Using browser from env: {}", "🌐".blue(), path.display());
        launcher = launcher.executable(path);
    } else if let Some(ref path) = system_path {
        println!(
            "{} Using discovered browser: {}",
            "🌐".blue(),
            path.display()
        );
        launcher = launcher.executable(path);
    } else if let Some(ref path) = chrome_path {
        println!(
            "{} Using explicitly found Chrome: {}",
            "🌐".blue(),
            path.display()
        );
        launcher = launcher.executable(path);
    } else {
        println!(
            "{} No browser executable found. Attempting default launch if possible...",
            "ℹ".blue()
        );
    }

    // Build args - add remote debugging port if persistence is enabled
    let mut args: Vec<String> = vec![
        "--no-sandbox",
        "--disable-setuid-sandbox",
        "--disable-dev-shm-usage",
        "--disable-gpu",
        "--ignore-certificate-errors",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    // Enable remote debugging for browser persistence
    if !config.close_when_finish {
        args.push("--remote-debugging-port=9222".to_string());
        println!(
            "{} Browser will stay open for reuse (closeWhenFinish: false)",
            "📌".cyan()
        );
    }

    launcher = launcher.args(&args);

    Ok(launcher.launch().await?)
}

fn find_system_browser() -> Option<std::path::PathBuf> {
    let common_paths = [
        // macOS System - Prioritize Google Chrome first
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        // Linux - Prioritize Google Chrome first
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
        // Fallback to Chromium and other browsers
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        // Playwright defaults (local to user) - Check these LAST
        "Library/Caches/ms-playwright/chromium-1091/chrome-mac/Chromium.app/Contents/MacOS/Chromium",
        "Library/Caches/ms-playwright/chromium-1084/chrome-mac/Chromium.app/Contents/MacOS/Chromium",
    ];

    for path in common_paths {
        if path.starts_with('/') {
            let p = std::path::Path::new(path);
            if p.exists() {
                return Some(p.to_path_buf());
            }
        }
    }
    None
}

/// Explicitly find Google Chrome with additional paths and methods
struct PersistentBrowserState {
    #[allow(dead_code)]
    playwright: Arc<Playwright>,
    #[allow(dead_code)]
    browser: Arc<Browser>,
    #[allow(dead_code)]
    context: Arc<BrowserContext>,
    #[allow(dead_code)]
    page: Arc<Mutex<Page>>,
}

/// Drop implementation to handle browser lifecycle
impl Drop for WebDriver {
    fn drop(&mut self) {
        // If close_when_finish is false, we should NOT close the browser
        // Browser will remain open for subsequent test runs
        if !self.config.close_when_finish {
            println!(
                "{} Detaching from browser (closeWhenFinish: false) - browser stays open",
                "📌".cyan()
            );

            // Store references in global static to keep them alive
            // This prevents the browser from being closed when WebDriver is dropped
            // MUST save page as well, otherwise dropping the only page closes the window
            let state = PersistentBrowserState {
                playwright: self.playwright.clone(),
                browser: self.browser.clone(),
                context: self.context.clone(),
                page: self.page.clone(),
            };

            if let Ok(mut guard) = get_persistent_browser().lock() {
                *guard = Some(state);
            }
        }
        // If close_when_finish is true (default), browser closes normally via Arc drop
    }
}

/// Launch Chrome externally as a detached process (not managed by Playwright)
/// This allows the browser to stay open after the test ends
fn launch_chrome_externally() -> Result<()> {
    // Find Chrome executable
    let chrome_path = find_chrome_explicitly()
        .ok_or_else(|| anyhow::anyhow!("Could not find Chrome/Chromium browser"))?;

    // On macOS, try to find .app to use 'open' command for UI interaction
    #[cfg(target_os = "macos")]
    {
        let path_str = chrome_path.to_string_lossy();
        // If we found the binary inside .app, go up to .app
        // Common path: .../Google Chrome.app/Contents/MacOS/Google Chrome
        if let Some(app_idx) = path_str.rfind(".app/") {
            let app_path = &path_str[..app_idx + 4]; // Include .app
            println!(
                "{} Launching Chrome via 'open' command from: {}",
                "🍎".blue(),
                app_path
            );

            // open -n -a "app_path" --args ...
            let status = std::process::Command::new("open")
                .args(&["-n", "-a", app_path, "--args"])
                .args(&[
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

    println!(
        "{} Launching detached Chrome binary from: {}",
        "🌐".blue(),
        chrome_path.display()
    );

    let mut cmd = std::process::Command::new(&chrome_path);
    cmd.args(&[
        "--remote-debugging-port=9222",
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-default-apps",
        "--user-data-dir=/tmp/lumi-chrome-profile",
    ]);

    // Linux/Unix specific detachment (setsid)
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

/// Explicitly find Google Chrome with additional paths and methods
fn find_chrome_explicitly() -> Option<std::path::PathBuf> {
    // Try to find via mdfind on macOS
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("mdfind")
            .args(&["kMDItemCFBundleIdentifier", "com.google.Chrome"])
            .output()
        {
            if let Ok(path_str) = String::from_utf8(output.stdout) {
                for line in path_str.lines() {
                    let chrome_path =
                        std::path::Path::new(line).join("Contents/MacOS/Google Chrome");
                    if chrome_path.exists() {
                        return Some(chrome_path);
                    }
                }
            }
        }
    }

    // Check standard paths
    let chrome_paths = [
        // macOS
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        // Linux
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
            let chrome_path =
                std::path::Path::new(&local_app_data).join("Google/Chrome/Application/chrome.exe");
            if chrome_path.exists() {
                return Some(chrome_path);
            }
        }
    }

    None
}
