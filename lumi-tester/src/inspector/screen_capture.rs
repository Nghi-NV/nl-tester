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
                Ok((STANDARD.encode(&bytes), self.screen_width, self.screen_height))
            }
            "web" => {
                let bytes = self.capture_web_bytes().await?;
                Ok((STANDARD.encode(&bytes), self.screen_width, self.screen_height))
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

    /// Capture iOS screenshot (placeholder)
    async fn capture_ios_bytes(&self) -> Result<Vec<u8>> {
        anyhow::bail!("iOS capture not yet implemented")
    }

    /// Capture Web screenshot (placeholder)
    async fn capture_web_bytes(&self) -> Result<Vec<u8>> {
        anyhow::bail!("Web capture not yet implemented")
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
    adb::shell(serial, "uiautomator dump /sdcard/ui.xml").await?;

    let temp_path = std::env::temp_dir().join("inspector_ui.xml");
    adb::pull(serial, "/sdcard/ui.xml", temp_path.to_str().unwrap()).await?;

    let xml = std::fs::read_to_string(&temp_path)?;
    Ok(xml)
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
            if let Some((_win_id, x, y, _w, _h)) = crate::driver::macos::MacosBridge::get_app_window_info(app_target) {
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
