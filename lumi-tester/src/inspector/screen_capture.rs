//! Screen capture for different platforms
//!
//! Provides screenshot streaming for Android, iOS, macOS, and Web platforms.

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};

use crate::driver::android::adb;

/// Platform-agnostic screen capturer
pub struct ScreenCapture {
    platform: String,
    device_serial: Option<String>,
    screen_width: u32,
    screen_height: u32,
}

impl ScreenCapture {
    /// Create a new screen capturer
    pub async fn new(platform: &str, device_serial: Option<&str>) -> Result<Self> {
        let (width, height) = match platform {
            "android" => adb::get_screen_size(device_serial).await?,
            "macos" => {
                #[cfg(target_os = "macos")]
                {
                    crate::driver::macos::MacosBridge::get_main_display_size().unwrap_or((1920, 1080))
                }
                #[cfg(not(target_os = "macos"))]
                {
                    (1920, 1080)
                }
            }
            "windows" => (1920, 1080),
            "ios" => (1170, 2532),
            "web" => (1280, 800),
            _ => (1080, 1920), // Default for mobile / fallback
        };

        Ok(Self {
            platform: platform.to_string(),
            device_serial: device_serial.map(|s| s.to_string()),
            screen_width: width,
            screen_height: height,
        })
    }

    pub fn platform(&self) -> &str {
        &self.platform
    }

    /// Capture screenshot and return as base64-encoded WebP/JPEG along with dimensions (width, height)
    pub async fn capture_base64_with_target(&self, target_app: Option<&str>) -> Result<(String, u32, u32)> {
        match self.platform.as_str() {
            "android" => {
                let bytes = self.capture_android_bytes().await?;
                let webp = convert_png_to_webp(&bytes)?;
                Ok((STANDARD.encode(&webp), self.screen_width, self.screen_height))
            }
            "macos" => {
                let (bytes, w, h) = self.capture_macos_bytes(target_app).await?;
                let webp = convert_png_to_webp(&bytes)?;
                Ok((STANDARD.encode(&webp), w, h))
            }
            "ios" => {
                let bytes = self.capture_ios_bytes().await?;
                let webp = convert_png_to_webp(&bytes)?;
                Ok((STANDARD.encode(&webp), self.screen_width, self.screen_height))
            }
            "windows" => {
                let bytes = self.capture_windows_bytes().await?;
                let webp = convert_png_to_webp(&bytes)?;
                Ok((STANDARD.encode(&webp), self.screen_width, self.screen_height))
            }
            "web" => {
                let bytes = self.capture_web_bytes().await?;
                let webp = convert_png_to_webp(&bytes)?;
                Ok((STANDARD.encode(&webp), self.screen_width, self.screen_height))
            }
            _ => anyhow::bail!("Unsupported platform: {}", self.platform),
        }
    }

    pub async fn capture_base64(&self) -> Result<String> {
        let (data, _, _) = self.capture_base64_with_target(None).await?;
        Ok(data)
    }

    /// Capture screenshot and return as raw PNG bytes
    pub async fn capture_bytes(&self) -> Result<Vec<u8>> {
        match self.platform.as_str() {
            "android" => self.capture_android_bytes().await,
            "macos" => {
                let (bytes, _, _) = self.capture_macos_bytes(None).await?;
                Ok(bytes)
            }
            "ios" => self.capture_ios_bytes().await,
            "windows" => self.capture_windows_bytes().await,
            "web" => self.capture_web_bytes().await,
            _ => anyhow::bail!("Unsupported platform: {}", self.platform),
        }
    }

    /// Get screen dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.screen_width, self.screen_height)
    }

    /// Capture Android screenshot
    async fn capture_android_bytes(&self) -> Result<Vec<u8>> {
        let adb_path = crate::utils::binary_resolver::find_adb()?;

        let mut args = Vec::new();
        if let Some(ref serial) = self.device_serial {
            args.push("-s");
            args.push(serial);
        }
        args.push("exec-out");
        args.push("screencap");
        args.push("-p");

        let output = tokio::process::Command::new(&adb_path)
            .args(&args)
            .output()
            .await?;

        if output.status.success() {
            if let Some(pos) = output.stdout.windows(4).position(|w| w == b"\x89PNG") {
                if pos == 0 {
                    return Ok(output.stdout);
                }
                return Ok(output.stdout[pos..].to_vec());
            }
        }

        let device_path = "/sdcard/lumi_screenshot.png";
        let temp_path = std::env::temp_dir().join("lumi_screenshot.png");

        adb::shell(
            self.device_serial.as_deref(),
            &format!("screencap -p {}", device_path),
        )
        .await?;
        adb::pull(
            self.device_serial.as_deref(),
            device_path,
            temp_path.to_str().unwrap(),
        )
        .await?;

        let bytes = tokio::fs::read(&temp_path).await?;
        let _ = tokio::fs::remove_file(&temp_path).await;
        let _ = adb::shell(
            self.device_serial.as_deref(),
            &format!("rm {}", device_path),
        )
        .await;

        if bytes.len() < 8 {
            anyhow::bail!("Screenshot file is too small");
        }

        Ok(bytes)
    }

    /// Capture macOS screenshot: captures specific app window if target_app provided, else full screen
    async fn capture_macos_bytes(&self, target_app: Option<&str>) -> Result<(Vec<u8>, u32, u32)> {
        let temp_path = std::env::temp_dir().join(format!("lumi_inspector_macos_{}.png", uuid::Uuid::new_v4()));
        let mut cmd = tokio::process::Command::new("screencapture");
        cmd.arg("-x");

        let mut actual_w = self.screen_width;
        let mut actual_h = self.screen_height;

        #[cfg(target_os = "macos")]
        if let Some(app) = target_app.filter(|s| !s.trim().is_empty()) {
            if let Some((win_id, _x, _y, w, h)) = crate::driver::macos::MacosBridge::get_app_window_info(app) {
                if w > 0.0 && h > 0.0 {
                    cmd.arg("-l");
                    cmd.arg(win_id.to_string());
                    cmd.arg("-o");
                    actual_w = w as u32;
                    actual_h = h as u32;
                }
            } else {
                let bridge = crate::driver::macos::MacosBridge::new();
                if let Some(bounds) = bridge.get_window_bounds(app) {
                    if bounds.width > 0.0 && bounds.height > 0.0 {
                        let rect_arg = format!(
                            "-R{},{},{},{}",
                            bounds.x as i32, bounds.y as i32, bounds.width as i32, bounds.height as i32
                        );
                        cmd.arg(&rect_arg);
                        actual_w = bounds.width as u32;
                        actual_h = bounds.height as u32;
                    }
                }
            }
        }

        cmd.arg(&temp_path);
        let status = cmd.status().await?;

        if !status.success() {
            anyhow::bail!("screencapture failed with status: {:?}", status);
        }

        let bytes = tokio::fs::read(&temp_path).await?;
        let _ = tokio::fs::remove_file(&temp_path).await;
        Ok((bytes, actual_w, actual_h))
    }

    /// Capture iOS screenshot via the lm-ios-tester agent (idb/WDA removed - see
    /// driver.rs for the full rationale: idb's screenshot service is confirmed broken on
    /// newer iOS versions, WDA's own HTTP round trip is both slower and an extra
    /// subsystem to keep alive).
    async fn capture_ios_bytes(&self) -> Result<Vec<u8>> {
        let client = crate::driver::ios::agent::AgentClient::new(
            "localhost",
            crate::driver::ios::agent::DEFAULT_AGENT_PORT,
        );
        if let Some(base64_data) = client.screenshot_base64().await {
            if let Ok(bytes) = STANDARD.decode(&base64_data) {
                return Ok(bytes);
            }
        }

        anyhow::bail!("Failed to capture iOS screenshot via the lm-ios-tester agent")
    }

    /// Capture Windows desktop screenshot
    async fn capture_windows_bytes(&self) -> Result<Vec<u8>> {
        let temp_path = std::env::temp_dir().join(format!("lumi_inspector_win_{}.png", uuid::Uuid::new_v4()));
        let temp_path_str = temp_path.to_string_lossy().to_string();

        #[cfg(target_os = "windows")]
        {
            let script = format!(
                r#"Add-Type -AssemblyName System.Windows.Forms,System.Drawing; $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds; $bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height; $graphics = [System.Drawing.Graphics]::FromImage($bmp); $graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size); $bmp.Save('{}', [System.Drawing.Imaging.ImageFormat]::Png); $graphics.Dispose(); $bmp.Dispose()"#,
                temp_path_str.replace("'", "''")
            );
            let status = tokio::process::Command::new("powershell")
                .args(&["-NoProfile", "-NonInteractive", "-Command", &script])
                .status()
                .await?;
            if status.success() && temp_path.exists() {
                let bytes = tokio::fs::read(&temp_path).await?;
                let _ = tokio::fs::remove_file(&temp_path).await;
                return Ok(bytes);
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = temp_path_str;
        }

        anyhow::bail!("Windows screenshot capture is only supported on Windows hosts")
    }

    /// Capture Web screenshot via CDP or browser window
    async fn capture_web_bytes(&self) -> Result<Vec<u8>> {
        use futures::StreamExt;

        // 1. Try connecting to active CDP browser on port 9222
        if let Ok((browser, mut handler)) = chromiumoxide::Browser::connect("http://localhost:9222").await {
            tokio::spawn(async move {
                while let Some(_) = handler.next().await {}
            });
            if let Ok(pages) = browser.pages().await {
                if let Some(page) = pages.first() {
                    let screenshot_params = chromiumoxide::page::ScreenshotParams::builder()
                        .format(chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat::Png)
                        .build();
                    if let Ok(png) = page.screenshot(screenshot_params).await {
                        return Ok(png);
                    }
                }
            }
        }

        // 2. Fallback: on macOS, capture active Chrome / Safari / Browser window
        #[cfg(target_os = "macos")]
        {
            if let Ok((bytes, _, _)) = self.capture_macos_bytes(Some("Google Chrome")).await {
                return Ok(bytes);
            }
            if let Ok((bytes, _, _)) = self.capture_macos_bytes(Some("Chromium")).await {
                return Ok(bytes);
            }
            if let Ok((bytes, _, _)) = self.capture_macos_bytes(Some("Safari")).await {
                return Ok(bytes);
            }
            if let Ok((bytes, _, _)) = self.capture_macos_bytes(None).await {
                return Ok(bytes);
            }
        }

        // 3. Fallback: on Windows, capture desktop / browser window
        #[cfg(target_os = "windows")]
        {
            if let Ok(bytes) = self.capture_windows_bytes().await {
                return Ok(bytes);
            }
        }

        // 4. Fallback to temp file
        let temp_path = std::env::temp_dir().join(format!("lumi_inspector_web_{}.png", uuid::Uuid::new_v4()));
        if temp_path.exists() {
            let bytes = tokio::fs::read(&temp_path).await?;
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Ok(bytes);
        }

        anyhow::bail!("Web capture requires an active browser session (Chrome with remote debugging or active browser window)")
    }
}

/// Convert PNG bytes to JPEG/WebP for smaller file size
fn convert_png_to_webp(png_bytes: &[u8]) -> Result<Vec<u8>> {
    use image::codecs::jpeg::JpegEncoder;
    use image::io::Reader as ImageReader;
    use std::io::Cursor;

    let img = ImageReader::new(Cursor::new(png_bytes))
        .with_guessed_format()?
        .decode()?;

    let mut jpeg_bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_bytes, 70);
    encoder.encode_image(&img)?;

    Ok(jpeg_bytes)
}

/// Get UI hierarchy for Android element picking
pub async fn get_hierarchy_android(serial: Option<&str>) -> Result<String> {
    use crate::driver::traits::PlatformDriver;
    let driver = crate::driver::android::AndroidDriver::new(serial).await?;
    driver.dump_ui_hierarchy().await
}

/// Get UI hierarchy for macOS element picking
pub async fn get_hierarchy_macos(app_target: Option<&str>) -> Result<String> {
    #[cfg(target_os = "macos")]
    {
        let accessibility = crate::driver::macos::MacosAccessibility::new();
        accessibility.dump_ui_hierarchy(app_target.unwrap_or(""))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app_target;
        anyhow::bail!("macOS is only supported on macOS hosts")
    }
}

/// Get UI hierarchy for iOS element picking
pub async fn get_hierarchy_ios(_udid: Option<&str>) -> Result<String> {
    let client = crate::driver::ios::agent::AgentClient::new(
        "localhost",
        crate::driver::ios::agent::DEFAULT_AGENT_PORT,
    );
    match client.hierarchy("").await {
        Some(data) => Ok(serde_json::to_string(&data)?),
        None => anyhow::bail!("Failed to get iOS hierarchy via the lm-ios-tester agent"),
    }
}

/// Get UI hierarchy for Windows element picking
pub async fn get_hierarchy_windows() -> Result<String> {
    #[cfg(target_os = "windows")]
    {
        use crate::driver::traits::PlatformDriver;
        let driver = crate::driver::windows::WindowsDriver::new();
        driver.dump_ui_hierarchy().await
    }
    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("Windows is only supported on Windows hosts")
    }
}

/// Get UI hierarchy for Web element picking
pub async fn get_hierarchy_web() -> Result<String> {
    use futures::StreamExt;

    // 1. Try CDP browser at http://localhost:9222
    if let Ok((browser, mut handler)) = chromiumoxide::Browser::connect("http://localhost:9222").await {
        tokio::spawn(async move {
            while let Some(_) = handler.next().await {}
        });
        if let Ok(pages) = browser.pages().await {
            if let Some(page) = pages.first() {
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
                if let Ok(val) = page.evaluate(SCRIPT).await {
                    if let Ok(xml) = val.into_value::<String>() {
                        return Ok(xml);
                    }
                }
            }
        }
    }

    // 2. Fallback: on macOS, if Chrome or Safari is open, dump macOS AX hierarchy for the browser
    #[cfg(target_os = "macos")]
    {
        if let Ok(xml) = get_hierarchy_macos(Some("Google Chrome")).await {
            if xml.contains("<element") {
                return Ok(xml);
            }
        }
        if let Ok(xml) = get_hierarchy_macos(Some("Safari")).await {
            if xml.contains("<element") {
                return Ok(xml);
            }
        }
    }

    Ok("<hierarchy platform=\"web\"/>".to_string())
}

/// Dispatcher to retrieve raw UI hierarchy for any supported platform
pub async fn get_hierarchy_for_platform(platform: &str, target: Option<&str>) -> Result<String> {
    match platform {
        "android" => get_hierarchy_android(target).await,
        "macos" => get_hierarchy_macos(target).await,
        "ios" => get_hierarchy_ios(target).await,
        "windows" => get_hierarchy_windows().await,
        "web" => get_hierarchy_web().await,
        _ => get_hierarchy_android(target).await,
    }
}

/// Standardize macOS AX roles into universal cross-platform UI types
pub fn normalize_macos_role(role: &str) -> String {
    let stripped = role.strip_prefix("AX").unwrap_or(role);
    match stripped {
        "Window" => "Window".to_string(),
        "TextField" | "TextArea" | "SecureTextField" => "Input".to_string(),
        "StaticText" => "Text".to_string(),
        "Button" => "Button".to_string(),
        "CheckBox" => "CheckBox".to_string(),
        "RadioButton" => "RadioButton".to_string(),
        "Switch" | "Toggle" => "Switch".to_string(),
        "Image" => "Image".to_string(),
        "Link" => "Link".to_string(),
        "PopUpButton" | "ComboBox" | "MenuButton" => "ComboBox".to_string(),
        "Slider" => "Slider".to_string(),
        "List" | "Table" | "Outline" | "CollectionView" => "List".to_string(),
        "Row" => "Row".to_string(),
        "Cell" => "Cell".to_string(),
        "ScrollArea" => "ScrollView".to_string(),
        "Group" => "Group".to_string(),
        "Heading" => "Heading".to_string(),
        other => other.to_string(),
    }
}

/// Parse iOS accessibility JSON to UiElement structures for Inspector
pub fn parse_ios_hierarchy_to_ui_elements(
    json_str: &str,
    screen_width: u32,
    screen_height: u32,
) -> Vec<crate::driver::android::uiautomator::UiElement> {
    if let Ok(elements) = crate::driver::ios::accessibility::parse_ui_hierarchy(json_str) {
        let flat = crate::driver::ios::accessibility::flatten_elements(&elements);

        // Compute logical screen dimensions from root Application or Window elements (or max bounds <= 600 pt)
        let mut root_w = 0.0f64;
        let mut root_h = 0.0f64;
        for el in &flat {
            let class = el.element_type.as_deref().unwrap_or("");
            if class == "Application" || class == "Window" {
                if el.frame.width > root_w && el.frame.width <= 600.0 {
                    root_w = el.frame.width;
                    root_h = el.frame.height;
                }
            }
        }
        if root_w <= 50.0 {
            for el in &flat {
                if el.frame.width > root_w && el.frame.width <= 600.0 {
                    root_w = el.frame.width;
                    root_h = el.frame.height;
                }
            }
        }
        if root_w <= 50.0 {
            root_w = 390.0;
            root_h = 844.0;
        }

        // Scale factor between logical point coordinates (e.g. 390x844) and screenshot pixel resolution (e.g. 1170x2532)
        let (scale_x, scale_y) = if screen_width > 0 && root_w > 50.0 && (screen_width as f64) > root_w * 1.2 {
            (
                screen_width as f64 / root_w,
                if root_h > 50.0 && screen_height > 0 {
                    screen_height as f64 / root_h
                } else {
                    screen_width as f64 / root_w
                },
            )
        } else {
            (1.0, 1.0)
        };

        flat.into_iter()
            .map(|el| {
                let left = (el.frame.x * scale_x).round() as i32;
                let top = (el.frame.y * scale_y).round() as i32;
                let right = ((el.frame.x + el.frame.width) * scale_x).round() as i32;
                let bottom = ((el.frame.y + el.frame.height) * scale_y).round() as i32;
                let class = el.element_type.clone().unwrap_or_else(|| "View".to_string());
                let text = el.display_text().unwrap_or_default().to_string();
                let resource_id = el.identifier.clone().unwrap_or_default();
                let hint = el.placeholder.clone().unwrap_or_default();
                let clickable = el.enabled
                    && (class.contains("Button")
                        || class.contains("TextField")
                        || class.contains("Cell")
                        || class.contains("Switch")
                        || class.contains("Link")
                        || class.contains("SegmentedControl"));

                crate::driver::android::uiautomator::UiElement {
                    class: class.clone(),
                    text,
                    resource_id,
                    content_desc: hint.clone(),
                    bounds: crate::driver::android::uiautomator::Bounds {
                        left,
                        top,
                        right,
                        bottom,
                    },
                    clickable,
                    enabled: el.enabled,
                    focusable: class.contains("TextField") || class.contains("Button"),
                    focused: false,
                    hint,
                    scrollable: class.contains("ScrollView")
                        || class.contains("Table")
                        || class.contains("Collection"),
                    password: class.contains("Secure"),
                    index: "0".to_string(),
                    package: "ios".to_string(),
                }
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// Generic XML parser converting `<element ...>` / `<node ...>` into UiElement structures
pub fn parse_xml_elements_to_ui_elements(
    xml: &str,
    platform: &str,
) -> Vec<crate::driver::android::uiautomator::UiElement> {
    let mut list = Vec::new();
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.trim_text(true);
    let mut buf = Vec::new();

    while let Ok(event) = reader.read_event_into(&mut buf) {
        match event {
            quick_xml::events::Event::Start(ref e) | quick_xml::events::Event::Empty(ref e) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag_name == "element" || tag_name == "node" || tag_name == "item" {
                    let mut x: Option<i32> = None;
                    let mut y: Option<i32> = None;
                    let mut width: Option<i32> = None;
                    let mut height: Option<i32> = None;
                    let mut left: Option<i32> = None;
                    let mut top: Option<i32> = None;
                    let mut right: Option<i32> = None;
                    let mut bottom: Option<i32> = None;
                    let mut text = String::new();
                    let mut id = String::new();
                    let mut class = String::new();
                    let mut desc = String::new();
                    let mut clickable = false;
                    let mut enabled = true;
                    let mut scrollable = false;
                    let mut password = false;

                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        let val = String::from_utf8_lossy(&attr.value).to_string();

                        match key.as_str() {
                            "x" => x = val.parse::<f64>().ok().map(|v| v.round() as i32),
                            "y" => y = val.parse::<f64>().ok().map(|v| v.round() as i32),
                            "width" | "w" => {
                                width = val.parse::<f64>().ok().map(|v| v.round() as i32)
                            }
                            "height" | "h" => {
                                height = val.parse::<f64>().ok().map(|v| v.round() as i32)
                            }
                            "left" => left = val.parse::<f64>().ok().map(|v| v.round() as i32),
                            "top" => top = val.parse::<f64>().ok().map(|v| v.round() as i32),
                            "right" => right = val.parse::<f64>().ok().map(|v| v.round() as i32),
                            "bottom" => bottom = val.parse::<f64>().ok().map(|v| v.round() as i32),
                            "bounds" => {
                                let parts: Vec<&str> = val
                                    .trim_matches(|c| c == '[' || c == ']')
                                    .split("][")
                                    .collect();
                                if parts.len() == 2 {
                                    let p1: Vec<&str> = parts[0].split(',').collect();
                                    let p2: Vec<&str> = parts[1].split(',').collect();
                                    if p1.len() == 2 && p2.len() == 2 {
                                        left = p1[0].trim().parse::<i32>().ok();
                                        top = p1[1].trim().parse::<i32>().ok();
                                        right = p2[0].trim().parse::<i32>().ok();
                                        bottom = p2[1].trim().parse::<i32>().ok();
                                    }
                                }
                            }
                            "text" | "name" | "title" | "value" | "label" => {
                                if text.is_empty() && !val.trim().is_empty() {
                                    text = val;
                                }
                            }
                            "id" | "resource-id" | "automation_id" | "identifier" => {
                                if id.is_empty() && !val.trim().is_empty() {
                                    id = val;
                                }
                            }
                            "class" | "type" | "role" | "tag" | "control_type" => {
                                if class.is_empty() && !val.trim().is_empty() {
                                    class = val;
                                }
                            }
                            "description" | "desc" | "content-desc" | "help_text"
                            | "placeholder" => {
                                if desc.is_empty() && !val.trim().is_empty() {
                                    desc = val;
                                }
                            }
                            "clickable" => clickable = val == "true" || val == "1",
                            "enabled" => enabled = val != "false" && val != "0",
                            "scrollable" => scrollable = val == "true" || val == "1",
                            "password" => password = val == "true" || val == "1",
                            _ => {}
                        }
                    }

                    let (final_l, final_t, final_r, final_b) =
                        match (left, top, right, bottom, x, y, width, height) {
                            (Some(l), Some(t), Some(r), Some(b), ..) => (l, t, r, b),
                            (_, _, _, _, Some(px), Some(py), Some(pw), Some(ph)) => {
                                (px, py, px + pw, py + ph)
                            }
                            _ => (0, 0, 0, 0),
                        };

                    if final_r > final_l && final_b > final_t {
                        if !clickable {
                            let lc = class.to_lowercase();
                            if lc.contains("button")
                                || lc.contains("input")
                                || lc.contains("link")
                                || lc.contains("select")
                                || lc.contains("checkbox")
                                || lc.contains("switch")
                            {
                                clickable = true;
                            }
                        }

                        list.push(crate::driver::android::uiautomator::UiElement {
                            class: if class.is_empty() {
                                "View".to_string()
                            } else {
                                class
                            },
                            text,
                            resource_id: id,
                            content_desc: desc.clone(),
                            bounds: crate::driver::android::uiautomator::Bounds {
                                left: final_l,
                                top: final_t,
                                right: final_r,
                                bottom: final_b,
                            },
                            clickable,
                            enabled,
                            focusable: clickable,
                            focused: false,
                            hint: desc,
                            scrollable,
                            password,
                            index: "0".to_string(),
                            package: platform.to_string(),
                        });
                    }
                }
            }
            quick_xml::events::Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    list
}

/// Parse raw UI hierarchy into unified UiElement list for any platform
pub fn parse_hierarchy_for_platform(
    platform: &str,
    raw_data: &str,
    target_app: &str,
    screen_width: u32,
    screen_height: u32,
) -> Vec<crate::driver::android::uiautomator::UiElement> {
    let trimmed = raw_data.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    match platform {
        "macos" => parse_macos_hierarchy_to_ui_elements(trimmed, target_app),
        "ios" => {
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                parse_ios_hierarchy_to_ui_elements(trimmed, screen_width, screen_height)
            } else {
                parse_xml_elements_to_ui_elements(trimmed, "ios")
            }
        }
        "windows" => parse_xml_elements_to_ui_elements(trimmed, "windows"),
        "web" => parse_xml_elements_to_ui_elements(trimmed, "web"),
        _ => {
            if let Ok(elements) = crate::driver::android::uiautomator::parse_hierarchy(trimmed) {
                if !elements.is_empty() {
                    return elements;
                }
            }
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                parse_ios_hierarchy_to_ui_elements(trimmed, screen_width, screen_height)
            } else {
                parse_xml_elements_to_ui_elements(trimmed, platform)
            }
        }
    }
}

/// Parse macOS accessibility XML hierarchy to UiElement structures for Inspector
pub fn parse_macos_hierarchy_to_ui_elements(
    xml: &str,
    app_target: &str,
) -> Vec<crate::driver::android::uiautomator::UiElement> {
    #[cfg(target_os = "macos")]
    {
        let accessibility = crate::driver::macos::MacosAccessibility::new();
        let nodes = accessibility.parse_hierarchy_nodes(xml);

        // If target app is specified and has window bounds, offset coordinates relative to window
        let (window_offset_x, window_offset_y) = if !app_target.trim().is_empty() {
            if let Some((_win_id, x, y, _w, _h)) =
                crate::driver::macos::MacosBridge::get_app_window_info(app_target)
            {
                (x, y)
            } else {
                let bridge = crate::driver::macos::MacosBridge::new();
                if let Some(bounds) = bridge.get_window_bounds(app_target) {
                    if bounds.width > 0.0 && bounds.height > 0.0 {
                        (bounds.x, bounds.y)
                    } else {
                        (0.0, 0.0)
                    }
                } else {
                    (0.0, 0.0)
                }
            }
        } else {
            (0.0, 0.0)
        };

        nodes
            .into_iter()
            .map(|n| {
                let text = if !n.title.is_empty() {
                    n.title
                } else if !n.value.is_empty() {
                    n.value
                } else if !n.placeholder.is_empty() {
                    n.placeholder.clone()
                } else {
                    String::new()
                };
                let hint = if !n.placeholder.is_empty() {
                    n.placeholder
                } else {
                    n.description
                };
                let norm_role = normalize_macos_role(&n.role);
                let clickable = norm_role == "Button"
                    || norm_role == "Input"
                    || norm_role == "Switch"
                    || norm_role == "CheckBox"
                    || norm_role == "RadioButton"
                    || norm_role == "Row"
                    || norm_role == "Cell"
                    || norm_role == "ComboBox"
                    || norm_role == "Link";

                let left = (n.x - window_offset_x) as i32;
                let top = (n.y - window_offset_y) as i32;
                let right = (n.x + n.width - window_offset_x) as i32;
                let bottom = (n.y + n.height - window_offset_y) as i32;

                crate::driver::android::uiautomator::UiElement {
                    class: norm_role.clone(),
                    text,
                    resource_id: n.identifier,
                    content_desc: hint.clone(),
                    bounds: crate::driver::android::uiautomator::Bounds {
                        left,
                        top,
                        right,
                        bottom,
                    },
                    clickable,
                    enabled: true,
                    focusable: norm_role == "Input" || norm_role == "Button" || norm_role == "Text",
                    focused: false,
                    hint,
                    scrollable: norm_role == "ScrollView" || norm_role == "List",
                    password: n.role.contains("Secure"),
                    index: "0".to_string(),
                    package: app_target.to_string(),
                }
            })
            .collect()
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (xml, app_target);
        Vec::new()
    }
}
