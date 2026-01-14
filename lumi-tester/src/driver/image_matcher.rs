//! Image matching algorithms with region-based optimization
//!
//! This module provides fast template matching with optional region constraints
//! to reduce search area and improve matching speed.

use anyhow::Result;
use image::GrayImage;
use imageproc::template_matching::{match_template, MatchTemplateMethod};

/// Supported screen regions for image matching
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageRegion {
    /// Search entire screen (default)
    #[default]
    Full,
    /// Top half of screen
    Top,
    /// Bottom half of screen
    Bottom,
    /// Left half of screen
    Left,
    /// Right half of screen
    Right,
    /// Top-left quadrant
    TopLeft,
    /// Top-right quadrant
    TopRight,
    /// Bottom-left quadrant
    BottomLeft,
    /// Bottom-right quadrant
    BottomRight,
    /// Center region (50% of screen)
    Center,
}

impl ImageRegion {
    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().replace(['-', '_'], "").as_str() {
            "top" => ImageRegion::Top,
            "bottom" => ImageRegion::Bottom,
            "left" => ImageRegion::Left,
            "right" => ImageRegion::Right,
            "topleft" => ImageRegion::TopLeft,
            "topright" => ImageRegion::TopRight,
            "bottomleft" => ImageRegion::BottomLeft,
            "bottomright" => ImageRegion::BottomRight,
            "center" | "middle" => ImageRegion::Center,
            _ => ImageRegion::Full,
        }
    }

    /// Get the crop region as (x, y, width, height) for given screen dimensions
    pub fn get_crop_region(&self, screen_width: u32, screen_height: u32) -> (u32, u32, u32, u32) {
        let half_w = screen_width / 2;
        let half_h = screen_height / 2;
        let quarter_w = screen_width / 4;
        let quarter_h = screen_height / 4;

        match self {
            ImageRegion::Full => (0, 0, screen_width, screen_height),
            ImageRegion::Top => (0, 0, screen_width, half_h),
            ImageRegion::Bottom => (0, half_h, screen_width, half_h),
            ImageRegion::Left => (0, 0, half_w, screen_height),
            ImageRegion::Right => (half_w, 0, half_w, screen_height),
            ImageRegion::TopLeft => (0, 0, half_w, half_h),
            ImageRegion::TopRight => (half_w, 0, half_w, half_h),
            ImageRegion::BottomLeft => (0, half_h, half_w, half_h),
            ImageRegion::BottomRight => (half_w, half_h, half_w, half_h),
            ImageRegion::Center => (quarter_w, quarter_h, half_w, half_h),
        }
    }
}

/// Result of image matching
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// X coordinate of match center
    pub x: i32,
    /// Y coordinate of match center  
    pub y: i32,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
}

/// Image matching configuration
#[derive(Debug, Clone)]
pub struct MatchConfig {
    /// Target width for scaling (default: 220)
    pub target_width: f32,
    /// Minimum confidence threshold (default: 0.7)
    pub threshold: f32,
    /// Region to search in (default: Full)
    pub region: ImageRegion,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            target_width: 220.0,
            threshold: 0.7,
            region: ImageRegion::Full,
        }
    }
}

/// Find template image in screen image
///
/// Returns the center coordinates of the best match, or None if no match found.
pub fn find_template(
    screen: &GrayImage,
    template: &GrayImage,
    config: &MatchConfig,
) -> Result<Option<MatchResult>> {
    let screen_width = screen.width();
    let screen_height = screen.height();

    // Get crop region
    let (crop_x, crop_y, crop_w, crop_h) =
        config.region.get_crop_region(screen_width, screen_height);

    // Crop screen to region
    let cropped_screen = if config.region == ImageRegion::Full {
        screen.clone()
    } else {
        image::imageops::crop_imm(screen, crop_x, crop_y, crop_w, crop_h).to_image()
    };

    // Check template fits in cropped region
    if template.width() > cropped_screen.width() || template.height() > cropped_screen.height() {
        return Ok(None);
    }

    // Calculate scale factor based on ORIGINAL screen width (for consistency)
    // This ensures both full and cropped regions get similar downscaling ratio
    let base_scale = config.target_width / screen_width as f32;
    let scale_factor = if base_scale < 1.0 { base_scale } else { 1.0 };

    let (match_x, match_y, confidence) = if scale_factor < 1.0 {
        // Scale down for faster matching
        use image::imageops::FilterType;

        let new_w = (cropped_screen.width() as f32 * scale_factor) as u32;
        let new_h = (cropped_screen.height() as f32 * scale_factor) as u32;
        let new_tpl_w = (template.width() as f32 * scale_factor).max(3.0) as u32;
        let new_tpl_h = (template.height() as f32 * scale_factor).max(3.0) as u32;

        let scaled_screen =
            image::imageops::resize(&cropped_screen, new_w, new_h, FilterType::Nearest);
        let scaled_template =
            image::imageops::resize(template, new_tpl_w, new_tpl_h, FilterType::Nearest);

        // Template matching
        let result = match_template(
            &scaled_screen,
            &scaled_template,
            MatchTemplateMethod::CrossCorrelationNormalized,
        );

        // Find maximum
        let (max_loc, max_val) = find_max(&result);

        if max_val < config.threshold {
            return Ok(None);
        }

        // Scale back to original coordinates
        let orig_x = (max_loc.0 as f32 / scale_factor) as i32;
        let orig_y = (max_loc.1 as f32 / scale_factor) as i32;

        (orig_x, orig_y, max_val)
    } else {
        // Direct match without scaling
        let result = match_template(
            &cropped_screen,
            template,
            MatchTemplateMethod::CrossCorrelationNormalized,
        );

        let (max_loc, max_val) = find_max(&result);

        if max_val < config.threshold {
            return Ok(None);
        }

        (max_loc.0 as i32, max_loc.1 as i32, max_val)
    };

    // Convert to screen coordinates (add crop offset and template center)
    let center_x = crop_x as i32 + match_x + (template.width() as i32 / 2);
    let center_y = crop_y as i32 + match_y + (template.height() as i32 / 2);

    Ok(Some(MatchResult {
        x: center_x,
        y: center_y,
        confidence,
    }))
}

/// Find maximum value and location in result matrix
fn find_max(result: &image::ImageBuffer<image::Luma<f32>, Vec<f32>>) -> ((u32, u32), f32) {
    let mut max_val = -1.0f32;
    let mut max_loc = (0u32, 0u32);

    for (x, y, pixel) in result.enumerate_pixels() {
        if pixel[0] > max_val {
            max_val = pixel[0];
            max_loc = (x, y);
        }
    }

    (max_loc, max_val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_parsing() {
        assert_eq!(ImageRegion::from_str("top-left"), ImageRegion::TopLeft);
        assert_eq!(ImageRegion::from_str("top_right"), ImageRegion::TopRight);
        assert_eq!(ImageRegion::from_str("bottomleft"), ImageRegion::BottomLeft);
        assert_eq!(ImageRegion::from_str("center"), ImageRegion::Center);
        assert_eq!(ImageRegion::from_str("unknown"), ImageRegion::Full);
    }

    #[test]
    fn test_crop_region() {
        let (x, y, w, h) = ImageRegion::TopRight.get_crop_region(1000, 2000);
        assert_eq!((x, y, w, h), (500, 0, 500, 1000));
    }
}
