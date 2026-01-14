//! Common utilities shared across platform drivers
//!
//! This module contains reusable functions for image processing,
//! polling, and text manipulation used by Android, iOS, and Web drivers.

use anyhow::Result;
use image::{DynamicImage, GenericImageView};
use std::future::Future;
use std::time::{Duration, Instant};

// ============================================================================
// Image Utilities
// ============================================================================

/// Extract pixel color from an image at given coordinates
///
/// Returns (r, g, b) tuple. Coordinates are clamped to image bounds.
pub fn get_pixel_from_image(img: &DynamicImage, x: u32, y: u32) -> (u8, u8, u8) {
    let (width, height) = img.dimensions();
    let x = x.min(width.saturating_sub(1));
    let y = y.min(height.saturating_sub(1));

    let pixel = img.get_pixel(x, y);
    (pixel[0], pixel[1], pixel[2])
}

/// Template matching result
pub struct MatchResult {
    pub x: i32,
    pub y: i32,
    pub confidence: f32,
}

/// Find template image within a screenshot using normalized cross-correlation
///
/// Uses a two-pass algorithm:
/// 1. Coarse pass: Downscale for fast initial match
/// 2. Fine pass: Full resolution search in ROI around coarse match
///
/// Returns center coordinates of the best match, or None if no match found.
pub fn find_template_in_image(
    screen_path: &std::path::Path,
    template_path: &std::path::Path,
    threshold: f32,
) -> Result<Option<(i32, i32)>> {
    use imageproc::template_matching::{match_template, MatchTemplateMethod};

    let img_screen = image::open(screen_path)?.to_luma8();
    let img_template = image::open(template_path)?.to_luma8();

    if img_template.width() > img_screen.width() || img_template.height() > img_screen.height() {
        return Ok(None);
    }

    // Helper function to find best match
    let find_best_match = |image: &image::ImageBuffer<image::Luma<f32>, Vec<f32>>,
                           thresh: f32|
     -> Option<(u32, u32)> {
        let mut max_val = -1.0f32;
        let mut max_loc = (0, 0);

        for (x, y, pixel) in image.enumerate_pixels() {
            let val = pixel[0];
            if val > max_val {
                max_val = val;
                max_loc = (x, y);
            }
        }

        if max_val >= thresh {
            Some(max_loc)
        } else {
            None
        }
    };

    // Coarse Pass: Downscale for speed
    let target_width = 360.0;
    let scale_factor = if img_screen.width() as f32 > target_width {
        target_width / img_screen.width() as f32
    } else {
        1.0
    };

    let (coarse_match_x, coarse_match_y) = if scale_factor < 1.0 {
        use image::imageops::FilterType;
        let new_w = (img_screen.width() as f32 * scale_factor) as u32;
        let new_h = (img_screen.height() as f32 * scale_factor) as u32;
        let new_tpl_w = (img_template.width() as f32 * scale_factor) as u32;
        let new_tpl_h = (img_template.height() as f32 * scale_factor) as u32;

        if new_tpl_w < 5 || new_tpl_h < 5 {
            // Too small, do full search
            let result = match_template(
                &img_screen,
                &img_template,
                MatchTemplateMethod::CrossCorrelationNormalized,
            );
            return match find_best_match(&result, threshold) {
                Some((x, y)) => Ok(Some((
                    x as i32 + (img_template.width() as i32 / 2),
                    y as i32 + (img_template.height() as i32 / 2),
                ))),
                None => Ok(None),
            };
        }

        let s = image::imageops::resize(&img_screen, new_w, new_h, FilterType::Triangle);
        let t = image::imageops::resize(&img_template, new_tpl_w, new_tpl_h, FilterType::Triangle);

        let result = match_template(&s, &t, MatchTemplateMethod::CrossCorrelationNormalized);

        match find_best_match(&result, threshold - 0.1) {
            Some((x, y)) => (x as f32 / scale_factor, y as f32 / scale_factor),
            None => return Ok(None),
        }
    } else {
        let result = match_template(
            &img_screen,
            &img_template,
            MatchTemplateMethod::CrossCorrelationNormalized,
        );
        return match find_best_match(&result, threshold) {
            Some((x, y)) => Ok(Some((
                x as i32 + (img_template.width() as i32 / 2),
                y as i32 + (img_template.height() as i32 / 2),
            ))),
            None => Ok(None),
        };
    };

    // Fine Pass: Search in ROI around coarse match
    let roi_padding_w = img_template.width();
    let roi_padding_h = img_template.height();

    let roi_x = (coarse_match_x as u32).saturating_sub(roi_padding_w);
    let roi_y = (coarse_match_y as u32).saturating_sub(roi_padding_h);

    let roi_w = (img_template.width() + roi_padding_w * 2).min(img_screen.width() - roi_x);
    let roi_h = (img_template.height() + roi_padding_h * 2).min(img_screen.height() - roi_y);

    let roi = img_screen.view(roi_x, roi_y, roi_w, roi_h).to_image();

    let result_fine = match_template(
        &roi,
        &img_template,
        MatchTemplateMethod::CrossCorrelationNormalized,
    );

    if let Some((local_x, local_y)) = find_best_match(&result_fine, threshold) {
        let final_x = roi_x + local_x;
        let final_y = roi_y + local_y;

        let center_x = final_x as i32 + (img_template.width() as i32 / 2);
        let center_y = final_y as i32 + (img_template.height() as i32 / 2);

        Ok(Some((center_x, center_y)))
    } else {
        Ok(None)
    }
}

// ============================================================================
// Polling Utilities
// ============================================================================

/// Configuration for polling operations
#[derive(Clone)]
pub struct PollConfig {
    pub timeout_ms: u64,
    pub initial_interval_ms: u64,
    pub max_interval_ms: u64,
    pub use_exponential_backoff: bool,
}

impl Default for PollConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 10000,
            initial_interval_ms: 100,
            max_interval_ms: 500,
            use_exponential_backoff: true,
        }
    }
}

/// Generic polling function with optional exponential backoff
///
/// Calls `check_fn` repeatedly until it returns `true` or timeout is reached.
/// Returns `true` if condition was met, `false` if timed out.
pub async fn wait_until<F, Fut>(check_fn: F, config: PollConfig) -> bool
where
    F: Fn() -> Fut,
    Fut: Future<Output = bool>,
{
    let start = Instant::now();
    let timeout = Duration::from_millis(config.timeout_ms);
    let mut interval = config.initial_interval_ms;

    while start.elapsed() < timeout {
        if check_fn().await {
            return true;
        }

        tokio::time::sleep(Duration::from_millis(interval)).await;

        if config.use_exponential_backoff {
            interval = (interval * 3 / 2).min(config.max_interval_ms);
        }
    }

    false
}

// ============================================================================
// Text Utilities
// ============================================================================

/// Escape text for Android shell input command
pub fn escape_for_android_shell(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(' ', "%s")
        .replace('"', "\\\"")
        .replace('\'', "\\'")
        .replace('&', "\\&")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('|', "\\|")
        .replace(';', "\\;")
}

/// Convert Vietnamese diacritics to ASCII fallback
///
/// Useful when input methods don't support Unicode.
pub fn to_ascii_fallback(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'à' | 'á' | 'ạ' | 'ả' | 'ã' | 'â' | 'ầ' | 'ấ' | 'ậ' | 'ẩ' | 'ẫ' | 'ă' | 'ằ' | 'ắ'
            | 'ặ' | 'ẳ' | 'ẵ' => 'a',
            'À' | 'Á' | 'Ạ' | 'Ả' | 'Ã' | 'Â' | 'Ầ' | 'Ấ' | 'Ậ' | 'Ẩ' | 'Ẫ' | 'Ă' | 'Ằ' | 'Ắ'
            | 'Ặ' | 'Ẳ' | 'Ẵ' => 'A',
            'è' | 'é' | 'ẹ' | 'ẻ' | 'ẽ' | 'ê' | 'ề' | 'ế' | 'ệ' | 'ể' | 'ễ' => {
                'e'
            }
            'È' | 'É' | 'Ẹ' | 'Ẻ' | 'Ẽ' | 'Ê' | 'Ề' | 'Ế' | 'Ệ' | 'Ể' | 'Ễ' => {
                'E'
            }
            'ì' | 'í' | 'ị' | 'ỉ' | 'ĩ' => 'i',
            'Ì' | 'Í' | 'Ị' | 'Ỉ' | 'Ĩ' => 'I',
            'ò' | 'ó' | 'ọ' | 'ỏ' | 'õ' | 'ô' | 'ồ' | 'ố' | 'ộ' | 'ổ' | 'ỗ' | 'ơ' | 'ờ' | 'ớ'
            | 'ợ' | 'ở' | 'ỡ' => 'o',
            'Ò' | 'Ó' | 'Ọ' | 'Ỏ' | 'Õ' | 'Ô' | 'Ồ' | 'Ố' | 'Ộ' | 'Ổ' | 'Ỗ' | 'Ơ' | 'Ờ' | 'Ớ'
            | 'Ợ' | 'Ở' | 'Ỡ' => 'O',
            'ù' | 'ú' | 'ụ' | 'ủ' | 'ũ' | 'ư' | 'ừ' | 'ứ' | 'ự' | 'ử' | 'ữ' => {
                'u'
            }
            'Ù' | 'Ú' | 'Ụ' | 'Ủ' | 'Ũ' | 'Ư' | 'Ừ' | 'Ứ' | 'Ự' | 'Ử' | 'Ữ' => {
                'U'
            }
            'ỳ' | 'ý' | 'ỵ' | 'ỷ' | 'ỹ' => 'y',
            'Ỳ' | 'Ý' | 'Ỵ' | 'Ỷ' | 'Ỹ' => 'Y',
            'đ' => 'd',
            'Đ' => 'D',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_fallback() {
        assert_eq!(to_ascii_fallback("Việt Nam"), "Viet Nam");
        assert_eq!(to_ascii_fallback("Đường"), "Duong");
        assert_eq!(to_ascii_fallback("Hello"), "Hello");
    }

    #[test]
    fn test_escape_android_shell() {
        assert_eq!(escape_for_android_shell("hello world"), "hello%sworld");
        assert_eq!(escape_for_android_shell("a&b"), "a\\&b");
    }

    #[test]
    fn test_pixel_extraction() {
        // Create a simple 2x2 red image
        let img =
            DynamicImage::ImageRgb8(image::RgbImage::from_pixel(2, 2, image::Rgb([255, 0, 0])));
        let (r, g, b) = get_pixel_from_image(&img, 0, 0);
        assert_eq!((r, g, b), (255, 0, 0));
    }
}
