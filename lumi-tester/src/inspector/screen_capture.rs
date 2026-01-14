//! Screen capture for different platforms
//!
//! Provides screenshot streaming for Android, iOS, and Web platforms.

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::path::Path;

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
            _ => (1080, 1920), // Default for now
        };

        Ok(Self {
            platform: platform.to_string(),
            device_serial: device_serial.map(|s| s.to_string()),
            screen_width: width,
            screen_height: height,
        })
    }

    /// Capture screenshot and return as base64-encoded PNG
    pub async fn capture_base64(&self) -> Result<String> {
        match self.platform.as_str() {
            "android" => self.capture_android_base64().await,
            "ios" => self.capture_ios_base64().await,
            "web" => self.capture_web_base64().await,
            _ => anyhow::bail!("Unsupported platform: {}", self.platform),
        }
    }

    /// Capture screenshot and return as raw PNG bytes
    pub async fn capture_bytes(&self) -> Result<Vec<u8>> {
        match self.platform.as_str() {
            "android" => self.capture_android_bytes().await,
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
            Ok(output.stdout)
        } else {
            anyhow::bail!(
                "Screenshot failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    async fn capture_android_base64(&self) -> Result<String> {
        let png_bytes = self.capture_android_bytes().await?;

        // Convert PNG to WebP for smaller size
        let webp_bytes = convert_png_to_webp(&png_bytes)?;

        Ok(STANDARD.encode(&webp_bytes))
    }

    /// Capture iOS screenshot (placeholder)
    async fn capture_ios_bytes(&self) -> Result<Vec<u8>> {
        // TODO: Implement using idb or xcrun
        anyhow::bail!("iOS capture not yet implemented")
    }

    async fn capture_ios_base64(&self) -> Result<String> {
        let bytes = self.capture_ios_bytes().await?;
        Ok(STANDARD.encode(&bytes))
    }

    /// Capture Web screenshot (placeholder)
    async fn capture_web_bytes(&self) -> Result<Vec<u8>> {
        // TODO: Implement using Playwright CDP
        anyhow::bail!("Web capture not yet implemented")
    }

    async fn capture_web_base64(&self) -> Result<String> {
        let bytes = self.capture_web_bytes().await?;
        Ok(STANDARD.encode(&bytes))
    }
}

/// Convert PNG bytes to JPEG for smaller file size
fn convert_png_to_webp(png_bytes: &[u8]) -> Result<Vec<u8>> {
    use image::codecs::jpeg::JpegEncoder;
    use image::io::Reader as ImageReader;
    use std::io::Cursor;

    // Load PNG
    let img = ImageReader::new(Cursor::new(png_bytes))
        .with_guessed_format()?
        .decode()?;

    // Encode as JPEG with quality 70 (good balance between size and quality)
    let mut jpeg_bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_bytes, 70);
    encoder.encode_image(&img)?;

    Ok(jpeg_bytes)
}

/// Get UI hierarchy for element picking
pub async fn get_hierarchy_android(serial: Option<&str>) -> Result<String> {
    // Dump UI hierarchy
    adb::shell(serial, "uiautomator dump /sdcard/ui.xml").await?;

    // Pull and read
    let temp_path = std::env::temp_dir().join("inspector_ui.xml");
    adb::pull(serial, "/sdcard/ui.xml", temp_path.to_str().unwrap()).await?;

    let xml = std::fs::read_to_string(&temp_path)?;
    Ok(xml)
}
