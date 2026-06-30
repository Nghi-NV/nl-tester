//! Computer-vision core: perspective rectification + HSV LED state detection.
//!
//! Ported from the OpenCV reference (`hardware_auto_test`), implemented purely
//! on top of the `image` + `imageproc` crates (no OpenCV / FFI):
//!   1. Warp the 4 device corners onto a flat rectangle to cancel perspective
//!      and small camera drift.
//!   2. For each button ROI, classify the dominant LED color against the
//!      profile's data-driven state rules.

use crate::camera::profile::{ButtonRoi, CameraProfile, HsvRange, StateRule};
use image::{Rgb, RgbImage};
use imageproc::geometric_transformations::{warp_into, Interpolation, Projection};

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DetectionStatus {
    Match,
    Unknown,
    Ambiguous,
    Misaligned,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateCandidate {
    pub state: String,
    pub confidence: f32,
}

/// Detection result for a single button/LED.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonReading {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    pub label: String,
    /// Resolved state name (matched rule) or "UNKNOWN".
    pub state: String,
    pub status: DetectionStatus,
    /// Fraction of ROI pixels that matched the winning state (0.0..1.0).
    pub confidence: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second_best: Option<StateCandidate>,
    pub margin: f32,
    /// Warped-image center of the matched blob, if any: [x, y].
    pub position: Option<[u32; 2]>,
}

/// Detection result for the whole device.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeviceState {
    pub buttons: Vec<ButtonReading>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedLedRoi {
    pub id: Option<String>,
    pub label: String,
    pub roi: [u32; 4],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_roi: Option<[u32; 4]>,
    pub mask: String,
    pub confidence: f32,
    pub found: bool,
}

#[derive(Debug, Clone, Copy)]
struct BlobStats {
    count: u32,
    cx: u32,
    cy: u32,
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

impl DeviceState {
    pub fn get(&self, label: &str) -> Option<&ButtonReading> {
        let want = label.trim().to_lowercase();
        self.buttons.iter().find(|b| {
            let qualified = b
                .device_id
                .as_deref()
                .zip(b.id.as_deref())
                .map(|(device_id, id)| format!("{}.{}", device_id.trim(), id.trim()));
            b.label.trim().to_lowercase() == want
                || b.id
                    .as_deref()
                    .map(|id| id.trim().to_lowercase() == want)
                    .unwrap_or(false)
                || qualified
                    .as_deref()
                    .map(|id| id.eq_ignore_ascii_case(label.trim()))
                    .unwrap_or(false)
        })
    }
}

/// Convert an RGB pixel to HSV in OpenCV convention: H 0..=179, S/V 0..=255.
pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> [u8; 3] {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;

    let mut h = if delta <= f32::EPSILON {
        0.0
    } else if (max - rf).abs() < f32::EPSILON {
        60.0 * (((gf - bf) / delta) % 6.0)
    } else if (max - gf).abs() < f32::EPSILON {
        60.0 * (((bf - rf) / delta) + 2.0)
    } else {
        60.0 * (((rf - gf) / delta) + 4.0)
    };
    if h < 0.0 {
        h += 360.0;
    }
    let s = if max <= f32::EPSILON {
        0.0
    } else {
        delta / max
    };

    // OpenCV packs hue into 0..179 (degrees / 2).
    [
        (h / 2.0).round().clamp(0.0, 179.0) as u8,
        (s * 255.0).round().clamp(0.0, 255.0) as u8,
        (max * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// Build the projection mapping source-frame corners onto the warped rectangle.
/// Returns `None` if the 4 corners are degenerate.
fn projection(profile: &CameraProfile) -> Option<Projection> {
    let c = &profile.geometry.corners;
    let [w, h] = profile.geometry.warp;
    let from = [
        (c[0][0], c[0][1]),
        (c[1][0], c[1][1]),
        (c[2][0], c[2][1]),
        (c[3][0], c[3][1]),
    ];
    // Rectangle corners in the same TL, TR, BR, BL order.
    let to = [
        (0.0, 0.0),
        (w as f32, 0.0),
        (w as f32, h as f32),
        (0.0, h as f32),
    ];
    Projection::from_control_points(from, to)
}

/// Rectify the device face into a flat `warp` image.
pub fn warp_device(frame: &RgbImage, profile: &CameraProfile) -> RgbImage {
    let [w, h] = profile.geometry.warp;
    let mut out = RgbImage::new(w, h);
    if let Some(proj) = projection(profile) {
        warp_into(
            frame,
            &proj,
            Interpolation::Bilinear,
            Rgb([0, 0, 0]),
            &mut out,
        );
    }
    out
}

fn uses_ellipse_mask(roi: &ButtonRoi) -> bool {
    roi.mask
        .as_deref()
        .map(|m| m.trim().eq_ignore_ascii_case("ellipse"))
        .unwrap_or(false)
}

fn point_in_roi_mask(x: u32, y: u32, roi: &ButtonRoi) -> bool {
    if !uses_ellipse_mask(roi) {
        return true;
    }
    let [rx, ry, rw, rh] = roi.roi;
    if rw == 0 || rh == 0 {
        return false;
    }
    let cx = rx as f32 + rw as f32 / 2.0;
    let cy = ry as f32 + rh as f32 / 2.0;
    let nx = (x as f32 + 0.5 - cx) / (rw as f32 / 2.0);
    let ny = (y as f32 + 0.5 - cy) / (rh as f32 / 2.0);
    nx * nx + ny * ny <= 1.0
}

fn state_allowed(roi: &ButtonRoi, rule: &StateRule) -> bool {
    roi.allowed_states.is_empty()
        || roi
            .allowed_states
            .iter()
            .any(|s| s.trim().eq_ignore_ascii_case(rule.name.trim()))
}

fn is_dark_rule(rule: &StateRule) -> bool {
    rule.rule_type
        .as_deref()
        .map(|t| t.trim().eq_ignore_ascii_case("dark"))
        .unwrap_or(false)
}

fn is_signal_pixel(hsv: [u8; 3]) -> bool {
    (hsv[1] >= 45 && hsv[2] >= 55) || hsv[2] >= 150
}

fn is_color_signal_pixel(hsv: [u8; 3]) -> bool {
    hsv[1] >= 70 && hsv[2] >= 70
}

fn is_white_rule(rule: &StateRule) -> bool {
    rule.rule_type
        .as_deref()
        .map(|t| t.trim().eq_ignore_ascii_case("white"))
        .unwrap_or(false)
}

fn largest_blob_for_rule(
    warped: &RgbImage,
    roi: &ButtonRoi,
    rule: &StateRule,
) -> Option<BlobStats> {
    let (iw, ih) = warped.dimensions();
    let x0 = roi.roi[0].min(iw.saturating_sub(1));
    let y0 = roi.roi[1].min(ih.saturating_sub(1));
    let x1 = (roi.roi[0] + roi.roi[2]).min(iw);
    let y1 = (roi.roi[1] + roi.roi[3]).min(ih);
    let width = x1.saturating_sub(x0);
    let height = y1.saturating_sub(y0);
    if width == 0 || height == 0 {
        return None;
    }

    let len = (width as usize) * (height as usize);
    let mut matches = vec![false; len];
    let mut visited = vec![false; len];
    let idx = |x: u32, y: u32| -> usize { ((y - y0) * width + (x - x0)) as usize };

    for y in y0..y1 {
        for x in x0..x1 {
            if !point_in_roi_mask(x, y, roi) {
                continue;
            }
            let px = warped.get_pixel(x, y);
            if rule.matches(rgb_to_hsv(px[0], px[1], px[2])) {
                matches[idx(x, y)] = true;
            }
        }
    }

    let mut best: Option<(u32, u64, u64, u32, u32, u32, u32)> = None;
    let mut stack = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let start_idx = idx(x, y);
            if visited[start_idx] || !matches[start_idx] {
                continue;
            }

            visited[start_idx] = true;
            stack.clear();
            stack.push((x, y));
            let mut count = 0u32;
            let mut sum_x = 0u64;
            let mut sum_y = 0u64;
            let mut min_x = x;
            let mut max_x = x;
            let mut min_y = y;
            let mut max_y = y;

            while let Some((cx, cy)) = stack.pop() {
                count += 1;
                sum_x += cx as u64;
                sum_y += cy as u64;
                min_x = min_x.min(cx);
                max_x = max_x.max(cx);
                min_y = min_y.min(cy);
                max_y = max_y.max(cy);

                let neighbors = [
                    (cx.wrapping_sub(1), cy, cx > x0),
                    (cx + 1, cy, cx + 1 < x1),
                    (cx, cy.wrapping_sub(1), cy > y0),
                    (cx, cy + 1, cy + 1 < y1),
                ];
                for (nx, ny, valid) in neighbors {
                    if !valid {
                        continue;
                    }
                    let ni = idx(nx, ny);
                    if !visited[ni] && matches[ni] {
                        visited[ni] = true;
                        stack.push((nx, ny));
                    }
                }
            }

            if best
                .map(|(best_count, _, _, _, _, _, _)| count > best_count)
                .unwrap_or(true)
            {
                best = Some((count, sum_x, sum_y, min_x, min_y, max_x, max_y));
            }
        }
    }

    best.map(|(count, sum_x, sum_y, min_x, min_y, max_x, max_y)| {
        BlobStats {
            count,
            cx: (sum_x / count.max(1) as u64) as u32,
            cy: (sum_y / count.max(1) as u64) as u32,
            min_x,
            min_y,
            max_x,
            max_y,
        }
    })
}

fn led_blob_quality(blob: BlobStats, roi: &ButtonRoi) -> f32 {
    if blob.count < 6 {
        return 0.0;
    }
    let w = blob.max_x.saturating_sub(blob.min_x) + 1;
    let h = blob.max_y.saturating_sub(blob.min_y) + 1;
    let bbox_min = w.min(h).max(1);
    let bbox_max = w.max(h);
    let aspect = bbox_max as f32 / bbox_min as f32;
    if aspect > 2.2 {
        return 0.0;
    }
    let density = blob.count as f32 / w.saturating_mul(h).max(1) as f32;
    if density < 0.18 {
        return 0.0;
    }
    let roi_min = roi.roi[2].min(roi.roi[3]).max(1);
    let max_blob_side = (roi_min as f32 * 1.10).max(18.0) as u32;
    if bbox_max > max_blob_side {
        return 0.0;
    }
    let touches_edge = blob.min_x <= roi.roi[0] + 1
        || blob.min_y <= roi.roi[1] + 1
        || blob.max_x + 2 >= roi.roi[0] + roi.roi[2]
        || blob.max_y + 2 >= roi.roi[1] + roi.roi[3];
    let near_expected = roi
        .expected_center
        .map(|expected| {
            let dx = blob.cx as f32 - expected[0] as f32;
            let dy = blob.cy as f32 - expected[1] as f32;
            (dx * dx + dy * dy).sqrt() <= roi_min as f32 * 0.75
        })
        .unwrap_or(false);
    let edge_factor = if touches_edge {
        if near_expected {
            0.85
        } else if bbox_max as f32 >= roi_min as f32 * 0.90 {
            0.25
        } else {
            0.45
        }
    } else {
        1.0
    };
    let roundness = (1.0 / aspect).clamp(0.0, 1.0);
    let size_score = (bbox_max as f32 / max_blob_side.max(1) as f32).clamp(0.0, 1.0);
    ((density * 0.45 + roundness * 0.35 + size_score * 0.20) * edge_factor).clamp(0.0, 1.0)
}

/// Classify a single ROI in the warped image against the state rules.
fn classify_roi(warped: &RgbImage, roi: &ButtonRoi, profile: &CameraProfile) -> ButtonReading {
    let (iw, ih) = warped.dimensions();
    let x0 = roi.roi[0].min(iw.saturating_sub(1));
    let y0 = roi.roi[1].min(ih.saturating_sub(1));
    let x1 = (roi.roi[0] + roi.roi[2]).min(iw);
    let y1 = (roi.roi[1] + roi.roi[3]).min(ih);
    let rules = profile.effective_state_rules(roi);

    // Per-state pixel counts + centroid accumulation for the winner.
    let mut counts = vec![0u32; rules.len()];
    let mut sum_xy = vec![(0u64, 0u64); rules.len()];
    let mut total = 0u32;
    let mut signal_total = 0u32;
    let mut color_signal_total = 0u32;

    for y in y0..y1 {
        for x in x0..x1 {
            if !point_in_roi_mask(x, y, roi) {
                continue;
            }
            total += 1;
            let px = warped.get_pixel(x, y);
            let hsv = rgb_to_hsv(px[0], px[1], px[2]);
            if is_signal_pixel(hsv) {
                signal_total += 1;
            }
            if is_color_signal_pixel(hsv) {
                color_signal_total += 1;
            }
            for (i, rule) in rules.iter().enumerate() {
                if state_allowed(roi, rule) && rule.matches(hsv) {
                    counts[i] += 1;
                    sum_xy[i].0 += x as u64;
                    sum_xy[i].1 += y as u64;
                }
            }
        }
    }

    let total = total.max(1) as f32;
    let raw_color_signal_total = color_signal_total;
    let signal_total = signal_total.max(1) as f32;
    let color_signal_total = color_signal_total.max(1) as f32;
    let has_allowed_white_rule = rules
        .iter()
        .any(|rule| is_white_rule(rule) && state_allowed(roi, rule));
    // Winning state = highest pixel count above the min ratio.
    let mut best_idx: Option<usize> = None;
    let mut best_score = 0.0f32;
    let mut best_count = 0u32;
    let mut second_idx: Option<usize> = None;
    let mut second_score = 0.0f32;
    let mut second_count = 0u32;
    for (i, &c) in counts.iter().enumerate() {
        let denominator = if is_dark_rule(&rules[i]) {
            total
        } else if is_white_rule(&rules[i]) {
            signal_total
        } else {
            color_signal_total
        };
        let score = if is_dark_rule(&rules[i]) {
            let color_ratio = raw_color_signal_total as f32 / total;
            if !has_allowed_white_rule && color_ratio <= 0.03 {
                (1.0 - color_ratio).clamp(0.0, 1.0)
            } else {
                (c as f32 / denominator).min(1.0)
            }
        } else {
            (c as f32 / denominator).min(1.0)
        };
        if score > best_score {
            second_score = best_score;
            second_count = best_count;
            second_idx = best_idx;
            best_score = score;
            best_count = c;
            best_idx = Some(i);
        } else if score > second_score {
            second_score = score;
            second_count = c;
            second_idx = Some(i);
        }
    }

    let ratio = best_score;
    let second_ratio = second_score;
    let margin = (ratio - second_ratio).max(0.0);
    let second_best = second_idx.map(|i| StateCandidate {
        state: rules[i].name.clone(),
        confidence: second_ratio.min(1.0),
    });

    let unknown = || ButtonReading {
        id: roi.id.clone(),
        device_id: None,
        label: String::new(),
        state: "UNKNOWN".to_string(),
        status: DetectionStatus::Unknown,
        confidence: ratio.min(1.0),
        second_best: second_best.clone(),
        margin,
        position: None,
    };

    match best_idx {
        Some(i) if ratio >= profile.min_ratio => {
            let blob = largest_blob_for_rule(warped, roi, &rules[i]);
            let (cx, cy, confidence) = if is_dark_rule(&rules[i]) {
                (
                    blob.map(|b| b.cx).unwrap_or_else(|| {
                        (sum_xy[i].0 / best_count.max(1) as u64) as u32
                    }),
                    blob.map(|b| b.cy).unwrap_or_else(|| {
                        (sum_xy[i].1 / best_count.max(1) as u64) as u32
                    }),
                    ratio.min(1.0),
                )
            } else {
                let Some(blob) = blob else {
                    return unknown();
                };
                let quality = led_blob_quality(blob, roi);
                if quality < 0.25 {
                    return ButtonReading {
                        id: roi.id.clone(),
                        device_id: None,
                        label: String::new(),
                        state: "UNKNOWN".to_string(),
                        status: DetectionStatus::Unknown,
                        confidence: (ratio * quality).min(1.0),
                        second_best: second_best.clone(),
                        margin,
                        position: Some([blob.cx, blob.cy]),
                    };
                }
                (blob.cx, blob.cy, (ratio * quality).min(1.0))
            };
            let position = Some([cx, cy]);
            if let (Some(expected), Some(max_drift)) = (roi.expected_center, roi.max_center_drift) {
                let dx = cx as f32 - expected[0] as f32;
                let dy = cy as f32 - expected[1] as f32;
                let drift_limit = if roi.roi[2].max(roi.roi[3]) <= 64 {
                    (max_drift as f32).max(roi.roi[2].min(roi.roi[3]) as f32 * 0.55)
                } else {
                    max_drift as f32
                };
                if (dx * dx + dy * dy).sqrt() > drift_limit {
                    return ButtonReading {
                        id: roi.id.clone(),
                        device_id: None,
                        label: String::new(),
                        state: "MISALIGNED".to_string(),
                        status: DetectionStatus::Misaligned,
                        confidence,
                        second_best,
                        margin,
                        position,
                    };
                }
            }
            if second_count > 0 && margin < profile.min_margin {
                return ButtonReading {
                    id: roi.id.clone(),
                    device_id: None,
                    label: String::new(),
                    state: "AMBIGUOUS".to_string(),
                    status: DetectionStatus::Ambiguous,
                    confidence,
                    second_best,
                    margin,
                    position,
                };
            }
            ButtonReading {
                id: roi.id.clone(),
                device_id: None,
                label: String::new(),
                state: rules[i].name.clone(),
                status: DetectionStatus::Match,
                confidence,
                second_best,
                margin,
                position,
            }
        }
        _ => unknown(),
    }
}

/// Read all button states from a raw frame.
pub fn read_device(frame: &RgbImage, profile: &CameraProfile) -> DeviceState {
    let warped = warp_device(frame, profile);
    read_device_warped(&warped, profile)
}

/// Read all button states from an already-warped frame (avoids re-warping when
/// the caller also needs the warped image, e.g. for annotation).
pub fn read_device_warped(warped: &RgbImage, profile: &CameraProfile) -> DeviceState {
    let buttons = profile
        .buttons
        .iter()
        .map(|b| {
            let mut reading = classify_roi(warped, b, profile);
            reading.label = b.label.clone();
            reading.device_id = b.device_id.clone();
            reading
        })
        .collect();
    DeviceState { buttons }
}

/// Learn HSV ranges for a state by sampling pixels inside the given ROIs of the
/// warped image. Returns one or two ranges (two when the hue wraps around red).
///
/// Strategy: prefer bright, saturated pixels (the lit LED); if a region is dark
/// (an off / unlit LED) fall back to all pixels so the OFF state can also be
/// learned. Ranges are robust percentiles widened by a tolerance margin.
pub fn learn_ranges(warped: &RgbImage, rois: &[[u32; 4]]) -> Vec<HsvRange> {
    let (iw, ih) = warped.dimensions();
    let mut bright: Vec<[u8; 3]> = Vec::new();
    let mut all: Vec<[u8; 3]> = Vec::new();

    for roi in rois {
        let x0 = roi[0].min(iw.saturating_sub(1));
        let y0 = roi[1].min(ih.saturating_sub(1));
        let x1 = (roi[0] + roi[2]).min(iw);
        let y1 = (roi[1] + roi[3]).min(ih);
        for y in y0..y1 {
            for x in x0..x1 {
                let px = warped.get_pixel(x, y);
                let hsv = rgb_to_hsv(px[0], px[1], px[2]);
                all.push(hsv);
                if hsv[1] >= 60 && hsv[2] >= 70 {
                    bright.push(hsv);
                }
            }
        }
    }

    // Use bright/colored pixels if there are enough; otherwise the LED is off
    // (dark) and we characterize the whole region instead.
    let bright_ratio = bright.len() as f32 / all.len().max(1) as f32;
    let use_bright = bright.len() >= 12 && (bright_ratio >= 0.005 || bright.len() >= 20);
    let samples = if use_bright { &bright } else { &all };
    if samples.is_empty() {
        return vec![HsvRange::new([0, 0, 0], [179, 255, 255])];
    }

    // Detect red hue wrap: significant mass near both ends of the hue circle.
    let near_low = samples.iter().filter(|h| h[0] <= 15).count();
    let near_high = samples.iter().filter(|h| h[0] >= 165).count();
    let wraps = near_low > samples.len() / 10 && near_high > samples.len() / 10;

    if wraps {
        // Two ranges: [0..hi_low] and [lo_high..179], sharing S/V bounds.
        let (smin, smax) = percentile_channel(samples, 1);
        let (vmin, vmax) = percentile_channel(samples, 2);
        let low: Vec<[u8; 3]> = samples.iter().filter(|h| h[0] <= 90).copied().collect();
        let high: Vec<[u8; 3]> = samples.iter().filter(|h| h[0] > 90).copied().collect();
        let (_, lh) = percentile_channel(&low, 0);
        let (hl, _) = percentile_channel(&high, 0);
        vec![
            HsvRange::new(
                [0, sat_floor(smin), val_floor(vmin)],
                [(lh + 8).min(179), smax, vmax],
            ),
            HsvRange::new(
                [hl.saturating_sub(8), sat_floor(smin), val_floor(vmin)],
                [179, smax, vmax],
            ),
        ]
    } else {
        let (hmin, hmax) = percentile_channel(samples, 0);
        let (smin, smax) = percentile_channel(samples, 1);
        let (vmin, vmax) = percentile_channel(samples, 2);
        vec![HsvRange::new(
            [hmin.saturating_sub(8), sat_floor(smin), val_floor(vmin)],
            [(hmax + 8).min(179), smax, vmax],
        )]
    }
}

/// Propose tight LED ROIs by finding the strongest bright/saturated blob inside
/// each current button region. This is intentionally conservative: if a LED is
/// off or no reliable blob is found, the original ROI is kept and `found=false`.
pub fn propose_led_rois(warped: &RgbImage, profile: &CameraProfile) -> Vec<ProposedLedRoi> {
    profile
        .buttons
        .iter()
        .map(|button| propose_led_roi(warped, button, profile))
        .collect()
}

fn search_roi_for_button(button: &ButtonRoi, iw: u32, ih: u32) -> [u32; 4] {
    if let Some(search) = button.search_roi {
        return search;
    }
    if button.roi[2].max(button.roi[3]) <= 80 {
        let side = button
            .roi[2]
            .max(button.roi[3])
            .saturating_mul(3)
            .max(120)
            .min(180)
            .min(iw)
            .min(ih);
        let cx = button.roi[0] + button.roi[2] / 2;
        let cy = button.roi[1] + button.roi[3] / 2;
        return [
            cx.saturating_sub(side / 2).min(iw.saturating_sub(side)),
            cy.saturating_sub(side / 2).min(ih.saturating_sub(side)),
            side,
            side,
        ];
    }
    button.roi
}

fn propose_led_roi(warped: &RgbImage, button: &ButtonRoi, profile: &CameraProfile) -> ProposedLedRoi {
    let (iw, ih) = warped.dimensions();
    let derived_search_roi = search_roi_for_button(button, iw, ih);
    let reading = classify_roi(warped, button, profile);
    if let Some([cx, cy]) = reading.position {
        if reading.confidence >= 0.08 {
            let side = button.roi[2].min(button.roi[3]).clamp(24, 64) as i32;
            let half = side / 2;
            let x = (cx as i32 - half).clamp(0, iw.saturating_sub(side as u32) as i32) as u32;
            let y = (cy as i32 - half).clamp(0, ih.saturating_sub(side as u32) as i32) as u32;
            return ProposedLedRoi {
                id: button.id.clone(),
                label: button.label.clone(),
                roi: [x, y, side as u32, side as u32],
                search_roi: Some(derived_search_roi),
                mask: "ellipse".to_string(),
                confidence: reading.confidence.clamp(0.0, 1.0),
                found: true,
            };
        }
    }
    let search = derived_search_roi;
    let mut expanded_search = button.search_roi.is_some();
    if button.search_roi.is_none() && search != button.roi {
        expanded_search = true;
    }
    let x0 = search[0].min(iw.saturating_sub(1));
    let y0 = search[1].min(ih.saturating_sub(1));
    let x1 = (search[0] + search[2]).min(iw);
    let y1 = (search[1] + search[3]).min(ih);
    let width = x1.saturating_sub(x0);
    let height = y1.saturating_sub(y0);
    if width == 0 || height == 0 {
        return proposed_original(button, false, 0.0);
    }

    let len = width as usize * height as usize;
    let mut hot = vec![false; len];
    let mut visited = vec![false; len];
    let idx = |x: u32, y: u32| -> usize { ((y - y0) * width + (x - x0)) as usize };
    let mut hot_count = 0u32;

    for y in y0..y1 {
        for x in x0..x1 {
            // Auto-proposal searches the full button rectangle. The final LED
            // ROI is an ellipse, but the initial button ROI may itself be an
            // ellipse centered in the button while the real LED sits near a
            // corner. Applying the old mask here would skip valid LEDs.
            let px = warped.get_pixel(x, y);
            let hsv = rgb_to_hsv(px[0], px[1], px[2]);
            let saturated_led = hsv[1] >= 60 && hsv[2] >= 65;
            let white_led = hsv[1] <= 70 && hsv[2] >= 210;
            let is_hot = saturated_led || white_led;
            if is_hot {
                hot[idx(x, y)] = true;
                hot_count += 1;
            }
        }
    }

    if hot_count < 4 {
        return proposed_original(button, false, 0.0);
    }

    let prior_cx = button
        .expected_center
        .map(|center| center[0] as f32)
        .unwrap_or(button.roi[0] as f32 + button.roi[2] as f32 / 2.0);
    let prior_cy = button
        .expected_center
        .map(|center| center[1] as f32)
        .unwrap_or(button.roi[1] as f32 + button.roi[3] as f32 / 2.0);
    let mut best: Option<(u32, u64, u64, i64, u32, u32)> = None;
    let mut stack = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let start = idx(x, y);
            if visited[start] || !hot[start] {
                continue;
            }
            visited[start] = true;
            stack.clear();
            stack.push((x, y));
            let mut count = 0u32;
            let mut sum_x = 0u64;
            let mut sum_y = 0u64;
            let mut brightness = 0u32;
            let mut min_x = x;
            let mut max_x = x;
            let mut min_y = y;
            let mut max_y = y;

            while let Some((cx, cy)) = stack.pop() {
                count += 1;
                sum_x += cx as u64;
                sum_y += cy as u64;
                min_x = min_x.min(cx);
                max_x = max_x.max(cx);
                min_y = min_y.min(cy);
                max_y = max_y.max(cy);
                let px = warped.get_pixel(cx, cy);
                brightness += px[0].max(px[1]).max(px[2]) as u32;
                let neighbors = [
                    (cx.wrapping_sub(1), cy, cx > x0),
                    (cx + 1, cy, cx + 1 < x1),
                    (cx, cy.wrapping_sub(1), cy > y0),
                    (cx, cy + 1, cy + 1 < y1),
                ];
                for (nx, ny, valid) in neighbors {
                    if !valid {
                        continue;
                    }
                    let ni = idx(nx, ny);
                    if !visited[ni] && hot[ni] {
                        visited[ni] = true;
                        stack.push((nx, ny));
                    }
                }
            }

            let cx = sum_x as f32 / count.max(1) as f32;
            let cy = sum_y as f32 / count.max(1) as f32;
            let avg_brightness = brightness / count.max(1);
            let bbox_w = max_x - min_x + 1;
            let bbox_h = max_y - min_y + 1;
            let bbox_min = bbox_w.min(bbox_h).max(1);
            let bbox_max = bbox_w.max(bbox_h);
            let bbox_area = bbox_w.saturating_mul(bbox_h).max(1);
            let density = count as f32 / bbox_area as f32;
            let aspect = bbox_max as f32 / bbox_min as f32;
            if count < 6 || bbox_w > 48 || bbox_h > 48 || aspect > 2.0 || density < 0.22 {
                continue;
            }
            let distance = (cx - prior_cx).hypot(cy - prior_cy);
            let distance_penalty = if expanded_search && button.expected_center.is_some() {
                distance.round() as i64 * 12
            } else {
                0
            };
            let large_blob_penalty = count.saturating_sub(500) as i64 * 2;
            let tiny_blob_penalty = 12u32.saturating_sub(count) as i64 * 20;
            let circularity_bonus = (density * 80.0).round() as i64 - ((aspect - 1.0) * 80.0).round() as i64;
            let score = avg_brightness as i64
                + count.min(300) as i64
                + circularity_bonus
                - distance_penalty
                - large_blob_penalty
                - tiny_blob_penalty;
            if best
                .map(|(_, _, _, best_score, _, _)| score > best_score)
                .unwrap_or(true)
            {
                best = Some((count, sum_x, sum_y, score, bbox_w, bbox_h));
            }
        }
    }

    let Some((count, sum_x, sum_y, _, bbox_w, bbox_h)) = best else {
        return proposed_original(button, false, 0.0);
    };
    if count < 4 {
        return proposed_original(button, false, 0.0);
    }

    let cx = (sum_x / count as u64) as i32;
    let cy = (sum_y / count as u64) as i32;
    let blob_side = bbox_w.max(bbox_h).saturating_add(18);
    let side = blob_side
        .max(button.roi[2].min(button.roi[3]) / 4)
        .clamp(24, 64) as i32;
    let half = side / 2;
    let x = (cx - half).clamp(0, iw.saturating_sub(side as u32) as i32) as u32;
    let y = (cy - half).clamp(0, ih.saturating_sub(side as u32) as i32) as u32;
    let search_area = (side as u32).saturating_mul(side as u32).max(1);
    ProposedLedRoi {
        id: button.id.clone(),
        label: button.label.clone(),
        roi: [x, y, side as u32, side as u32],
        search_roi: Some(search),
        mask: "ellipse".to_string(),
        confidence: (count as f32 / search_area as f32).clamp(0.0, 1.0),
        found: true,
    }
}

fn proposed_original(button: &ButtonRoi, found: bool, confidence: f32) -> ProposedLedRoi {
    ProposedLedRoi {
        id: button.id.clone(),
        label: button.label.clone(),
        roi: button.roi,
        search_roi: button.search_roi,
        mask: button.mask.clone().unwrap_or_else(|| "ellipse".to_string()),
        confidence,
        found,
    }
}

/// Loosen the saturation floor so minor lighting changes still match.
fn sat_floor(v: u8) -> u8 {
    v.saturating_sub(40).max(30)
}

/// Loosen the value floor similarly.
fn val_floor(v: u8) -> u8 {
    v.saturating_sub(40).max(30)
}

/// Return the (5th, 95th) percentile of one HSV channel as (min, max).
fn percentile_channel(samples: &[[u8; 3]], channel: usize) -> (u8, u8) {
    if samples.is_empty() {
        return (0, 255);
    }
    let mut vals: Vec<u8> = samples.iter().map(|h| h[channel]).collect();
    vals.sort_unstable();
    let lo = vals[(vals.len() as f32 * 0.05) as usize];
    let hi = vals[((vals.len() as f32 * 0.95) as usize).min(vals.len() - 1)];
    (lo, hi)
}

/// Draw button ROIs + detected states onto a copy of the warped image, for
/// human-readable evidence in reports. Green = matched a non-UNKNOWN state.
pub fn annotate_warped(
    warped: &RgbImage,
    profile: &CameraProfile,
    state: &DeviceState,
) -> RgbImage {
    use imageproc::drawing::draw_hollow_rect_mut;
    use imageproc::rect::Rect;

    let mut out = warped.clone();
    for (b, reading) in profile.buttons.iter().zip(state.buttons.iter()) {
        let known = reading.status == DetectionStatus::Match;
        let color = if known {
            Rgb([0u8, 255, 0])
        } else {
            Rgb([255u8, 0, 0])
        };
        let rect =
            Rect::at(b.roi[0] as i32, b.roi[1] as i32).of_size(b.roi[2].max(1), b.roi[3].max(1));
        draw_hollow_rect_mut(&mut out, rect, color);
    }
    out
}

/// Helper for a StateRule constructed from learned ranges.
pub fn rule(name: &str, ranges: Vec<HsvRange>) -> StateRule {
    StateRule {
        name: name.to_string(),
        rule_type: None,
        hsv: ranges,
        dark_max_v: None,
        white_max_s: None,
        white_min_v: None,
        source: Some("learned".to_string()),
    }
}

/// Convenience to map a Layout-driven set of grid ROIs back onto buttons.
pub fn ensure_labeled(buttons: &mut [ButtonRoi]) {
    for (i, b) in buttons.iter_mut().enumerate() {
        if b.label.trim().is_empty() {
            b.label = format!("Nút {}", i + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::profile::{Geometry, Layout};

    #[test]
    fn hsv_matches_opencv_primary_colors() {
        assert_eq!(rgb_to_hsv(255, 0, 0)[0], 0); // red hue
        assert_eq!(rgb_to_hsv(0, 255, 0)[0], 60); // green hue (120/2)
        assert_eq!(rgb_to_hsv(0, 0, 255)[0], 120); // blue hue (240/2)
        assert_eq!(rgb_to_hsv(0, 0, 0)[2], 0); // black value
    }

    #[test]
    fn device_state_get_supports_qualified_region_id() {
        let state = DeviceState {
            buttons: vec![
                ButtonReading {
                    id: Some("button_1".to_string()),
                    device_id: Some("switch_4gang".to_string()),
                    label: "Nút 1".to_string(),
                    state: "RED".to_string(),
                    status: DetectionStatus::Match,
                    confidence: 1.0,
                    second_best: None,
                    margin: 1.0,
                    position: None,
                },
                ButtonReading {
                    id: Some("button_1".to_string()),
                    device_id: Some("device_2".to_string()),
                    label: "Nút 1".to_string(),
                    state: "BLUE".to_string(),
                    status: DetectionStatus::Match,
                    confidence: 1.0,
                    second_best: None,
                    margin: 1.0,
                    position: None,
                },
            ],
        };

        assert_eq!(state.get("device_2.button_1").unwrap().state, "BLUE");
        assert_eq!(state.get("switch_4gang.button_1").unwrap().state, "RED");
    }

    fn fill(img: &mut RgbImage, color: [u8; 3]) {
        for p in img.pixels_mut() {
            *p = Rgb(color);
        }
    }

    fn draw_disc(img: &mut RgbImage, cx: i32, cy: i32, radius: i32, color: [u8; 3]) {
        for y in (cy - radius)..=(cy + radius) {
            for x in (cx - radius)..=(cx + radius) {
                if x >= 0
                    && y >= 0
                    && (x as u32) < img.width()
                    && (y as u32) < img.height()
                    && (x - cx).pow(2) + (y - cy).pow(2) <= radius.pow(2)
                {
                    img.put_pixel(x as u32, y as u32, Rgb(color));
                }
            }
        }
    }

    fn test_profile() -> CameraProfile {
        CameraProfile {
            name: None,
            camera: Default::default(),
            lab: None,
            active_camera_id: None,
            active_device_id: None,
            geometry: Geometry {
                corners: [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
                warp: [100, 100],
            },
            layout: Layout::Custom,
            buttons: vec![ButtonRoi {
                device_id: None,
                id: Some("l1".into()),
                label: "L1".into(),
                kind: Some("led".into()),
                roi: [10, 10, 80, 80],
                search_roi: None,
                mask: None,
                expected_center: None,
                max_center_drift: None,
                allowed_states: Vec::new(),
            }],
            states: vec![
                rule("ON", vec![HsvRange::new([0, 70, 70], [10, 255, 255])]),
                rule("OFF", vec![HsvRange::new([90, 70, 70], [140, 255, 255])]),
            ],
            state_models: Default::default(),
            min_ratio: 0.01,
            min_margin: 0.03,
        }
    }

    #[test]
    fn detects_on_and_off_by_color() {
        let profile = test_profile();
        let mut img = RgbImage::new(100, 100);

        fill(&mut img, [10, 10, 10]);
        draw_disc(&mut img, 50, 50, 8, [255, 0, 0]); // red → ON
        let state = read_device(&img, &profile);
        assert_eq!(state.get("L1").unwrap().state, "ON");
        assert_eq!(state.get("l1").unwrap().status, DetectionStatus::Match);

        fill(&mut img, [10, 10, 10]);
        draw_disc(&mut img, 50, 50, 8, [0, 0, 255]); // blue → OFF
        let state = read_device(&img, &profile);
        assert_eq!(state.get("L1").unwrap().state, "OFF");

        fill(&mut img, [10, 10, 10]); // dark → no rule matches → UNKNOWN
        let state = read_device(&img, &profile);
        assert_eq!(state.get("L1").unwrap().state, "UNKNOWN");
        assert_eq!(state.get("L1").unwrap().status, DetectionStatus::Unknown);
    }

    #[test]
    fn proposes_tight_led_roi_around_bright_blob() {
        let mut profile = test_profile();
        profile.buttons[0].roi = [0, 0, 100, 100];
        let mut img = RgbImage::new(100, 100);
        fill(&mut img, [8, 8, 8]);
        for y in 18..25 {
            for x in 70..77 {
                img.put_pixel(x, y, Rgb([0, 0, 255]));
            }
        }

        let proposals = propose_led_rois(&img, &profile);
        assert_eq!(proposals.len(), 1);
        assert!(proposals[0].found);
        assert_eq!(proposals[0].mask, "ellipse");
        let [x, y, w, h] = proposals[0].roi;
        assert!(x <= 73 && x + w >= 73, "roi should contain led x: {:?}", proposals[0].roi);
        assert!(y <= 21 && y + h >= 21, "roi should contain led y: {:?}", proposals[0].roi);
        assert!(w < 100 && h < 100, "roi should be tighter than whole button");
    }

    #[test]
    fn proposal_recovers_led_drift_outside_tight_roi() {
        let mut profile = test_profile();
        profile.buttons[0].roi = [40, 40, 32, 32];
        profile.buttons[0].mask = Some("ellipse".to_string());
        let mut img = RgbImage::new(140, 140);
        fill(&mut img, [35, 35, 35]);
        for y in 78..86 {
            for x in 90..98 {
                if (x as i32 - 94).pow(2) + (y as i32 - 82).pow(2) <= 16 {
                    img.put_pixel(x, y, Rgb([255, 0, 0]));
                }
            }
        }

        let proposals = propose_led_rois(&img, &profile);
        assert!(proposals[0].found, "expanded search should recover a shifted LED");
        let [x, y, w, h] = proposals[0].roi;
        assert!(x <= 94 && x + w >= 94, "roi should contain shifted led x: {:?}", proposals[0].roi);
        assert!(y <= 82 && y + h >= 82, "roi should contain shifted led y: {:?}", proposals[0].roi);
    }

    #[test]
    fn search_roi_is_used_for_proposal_but_not_state_reading() {
        let mut profile = test_profile();
        profile.buttons[0].roi = [12, 12, 24, 24];
        profile.buttons[0].search_roi = Some([0, 0, 100, 100]);
        profile.buttons[0].mask = Some("ellipse".to_string());
        profile.states = vec![rule("RED", vec![HsvRange::new([0, 70, 70], [12, 255, 255])])];
        let mut img = RgbImage::new(100, 100);
        fill(&mut img, [20, 20, 20]);
        draw_disc(&mut img, 74, 72, 7, [255, 0, 0]);

        let state = read_device(&img, &profile);
        assert_eq!(state.get("L1").unwrap().status, DetectionStatus::Unknown);

        let proposal = &propose_led_rois(&img, &profile)[0];
        assert!(proposal.found, "proposal should search wider than the LED read ROI");
        let [x, y, w, h] = proposal.roi;
        assert!(x <= 74 && x + w >= 74, "proposal should contain searched LED x: {:?}", proposal.roi);
        assert!(y <= 72 && y + h >= 72, "proposal should contain searched LED y: {:?}", proposal.roi);
        assert_eq!(proposal.search_roi, Some([0, 0, 100, 100]));
    }

    #[test]
    fn proposal_prefers_classified_led_position_over_nearby_glare() {
        let mut profile = test_profile();
        profile.buttons[0].roi = [20, 20, 80, 80];
        profile.buttons[0].mask = Some("ellipse".to_string());
        profile.states = vec![rule("RED", vec![HsvRange::new([0, 70, 70], [12, 255, 255])])];
        let mut img = RgbImage::new(120, 120);
        fill(&mut img, [45, 45, 45]);
        for y in 60..70 {
            for x in 68..78 {
                if (x as i32 - 73).pow(2) + (y as i32 - 65).pow(2) <= 25 {
                    img.put_pixel(x, y, Rgb([255, 0, 0]));
                }
            }
        }
        for y in 35..45 {
            for x in 35..70 {
                img.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }

        let proposal = &propose_led_rois(&img, &profile)[0];
        let [x, y, w, h] = proposal.roi;
        assert!(proposal.found);
        assert!(x <= 73 && x + w >= 73, "roi should contain classified LED x: {:?}", proposal.roi);
        assert!(y <= 65 && y + h >= 65, "roi should contain classified LED y: {:?}", proposal.roi);
    }

    #[test]
    fn learns_small_round_led_from_bright_pixels_not_dark_background() {
        let mut img = RgbImage::new(80, 80);
        fill(&mut img, [6, 6, 6]);
        draw_disc(&mut img, 40, 40, 4, [0, 0, 255]);

        let ranges = learn_ranges(&img, &[[0, 0, 80, 80]]);
        assert!(
            ranges.iter().any(|range| range.contains(rgb_to_hsv(0, 0, 255))),
            "learned ranges should include blue LED pixels: {:?}",
            ranges
        );
        assert!(
            !ranges.iter().any(|range| range.contains(rgb_to_hsv(6, 6, 6))),
            "learned color state should not be dominated by dark background"
        );
    }

    #[test]
    fn colored_led_confidence_uses_signal_pixels_not_whole_dark_roi() {
        let mut profile = test_profile();
        profile.buttons[0].roi = [0, 0, 80, 80];
        profile.buttons[0].mask = Some("ellipse".to_string());
        profile.states = vec![rule("BLUE", vec![HsvRange::new([110, 70, 70], [130, 255, 255])])];
        let mut img = RgbImage::new(80, 80);
        fill(&mut img, [6, 6, 6]);
        for y in 37..43 {
            for x in 37..43 {
                if (x as i32 - 40).pow(2) + (y as i32 - 40).pow(2) <= 9 {
                    img.put_pixel(x, y, Rgb([0, 0, 255]));
                }
            }
        }

        let state = read_device(&img, &profile);
        let reading = state.get("L1").unwrap();
        assert_eq!(reading.status, DetectionStatus::Match);
        assert_eq!(reading.state, "BLUE");
        assert!(
            reading.confidence >= 0.6,
            "confidence should be high for a clean LED blob, got {}",
            reading.confidence
        );
    }

    #[test]
    fn colored_led_confidence_ignores_bright_gray_background() {
        let mut profile = test_profile();
        profile.buttons[0].roi = [0, 0, 80, 80];
        profile.buttons[0].mask = Some("ellipse".to_string());
        profile.states = vec![rule("RED", vec![HsvRange::new([0, 70, 70], [12, 255, 255])])];
        let mut img = RgbImage::new(80, 80);
        fill(&mut img, [185, 185, 185]);
        draw_disc(&mut img, 40, 40, 6, [255, 0, 0]);

        let state = read_device(&img, &profile);
        let reading = state.get("L1").unwrap();
        assert_eq!(reading.status, DetectionStatus::Match);
        assert_eq!(reading.state, "RED");
        assert!(
            reading.confidence >= 0.6,
            "bright gray background should not dilute LED confidence, got {}",
            reading.confidence
        );
    }

    #[test]
    fn colored_state_requires_compact_led_blob_not_large_color_patch() {
        let mut profile = test_profile();
        profile.buttons[0].roi = [10, 10, 60, 60];
        profile.buttons[0].mask = Some("ellipse".to_string());
        profile.states = vec![rule("RED", vec![HsvRange::new([0, 70, 70], [12, 255, 255])])];
        let mut img = RgbImage::new(90, 90);
        fill(&mut img, [40, 40, 40]);
        for y in 10..70 {
            for x in 10..70 {
                img.put_pixel(x, y, Rgb([255, 0, 0]));
            }
        }

        let state = read_device(&img, &profile);
        let reading = state.get("L1").unwrap();
        assert_eq!(reading.status, DetectionStatus::Unknown);
        assert!(
            reading.confidence < 0.25,
            "large non-LED color patch must not be high confidence, got {}",
            reading.confidence
        );
    }

    #[test]
    fn off_matches_unlit_led_on_bright_gray_background() {
        let mut profile = test_profile();
        profile.buttons[0].roi = [0, 0, 80, 80];
        profile.buttons[0].mask = Some("ellipse".to_string());
        profile.states = vec![StateRule {
            name: "OFF".to_string(),
            rule_type: Some("dark".to_string()),
            hsv: Vec::new(),
            dark_max_v: Some(45),
            white_max_s: None,
            white_min_v: None,
            source: Some("test".to_string()),
        }];
        let mut img = RgbImage::new(80, 80);
        fill(&mut img, [86, 78, 88]);

        let state = read_device(&img, &profile);
        let reading = state.get("L1").unwrap();
        assert_eq!(reading.status, DetectionStatus::Match);
        assert_eq!(reading.state, "OFF");
        assert!(
            reading.confidence >= 0.9,
            "unlit gray LED area should be confidently OFF, got {}",
            reading.confidence
        );
    }

    #[test]
    fn reports_ambiguous_when_second_best_is_too_close() {
        let mut profile = test_profile();
        profile.min_margin = 0.25;
        let mut img = RgbImage::new(100, 100);
        fill(&mut img, [10, 10, 10]);
        for y in 42..59 {
            for x in 42..59 {
                if (x as i32 - 50).pow(2) + (y as i32 - 50).pow(2) <= 64 {
                    let color = if x < 51 { [255, 0, 0] } else { [0, 0, 255] };
                    img.put_pixel(x, y, Rgb(color));
                }
            }
        }

        let state = read_device(&img, &profile);
        let reading = state.get("L1").unwrap();
        assert_eq!(reading.status, DetectionStatus::Ambiguous);
        assert_eq!(reading.state, "AMBIGUOUS");
        assert!(reading.second_best.is_some());
    }

    #[test]
    fn overlapping_color_rules_are_ambiguous_not_first_match() {
        let mut profile = test_profile();
        profile.states = vec![
            rule("RED", vec![HsvRange::new([0, 70, 70], [10, 255, 255])]),
            rule("PINKISH", vec![HsvRange::new([0, 70, 70], [10, 255, 255])]),
        ];
        profile.min_margin = 0.03;
        let mut img = RgbImage::new(100, 100);
        fill(&mut img, [10, 10, 10]);
        draw_disc(&mut img, 50, 50, 8, [255, 0, 0]);

        let state = read_device(&img, &profile);
        let reading = state.get("L1").unwrap();
        assert_eq!(reading.status, DetectionStatus::Ambiguous);
        assert_eq!(reading.state, "AMBIGUOUS");
        assert_eq!(
            reading.second_best.as_ref().map(|s| s.state.as_str()),
            Some("PINKISH")
        );
    }

    #[test]
    fn reports_misaligned_when_blob_center_drifts() {
        let mut profile = test_profile();
        profile.buttons[0].expected_center = Some([50, 50]);
        profile.buttons[0].max_center_drift = Some(10);
        let mut img = RgbImage::new(100, 100);
        fill(&mut img, [10, 10, 10]);
        draw_disc(&mut img, 73, 73, 8, [255, 0, 0]);

        let state = read_device(&img, &profile);
        let reading = state.get("L1").unwrap();
        assert_eq!(reading.status, DetectionStatus::Misaligned);
        assert_eq!(reading.state, "MISALIGNED");
        assert!(reading.position.is_some());
    }

    #[test]
    fn uses_largest_blob_not_average_of_separate_noise() {
        let mut profile = test_profile();
        profile.buttons[0].expected_center = Some([50, 50]);
        profile.buttons[0].max_center_drift = Some(12);
        let mut img = RgbImage::new(100, 100);
        for y in 45..55 {
            for x in 15..25 {
                *img.get_pixel_mut(x, y) = Rgb([255, 0, 0]);
            }
            for x in 75..85 {
                *img.get_pixel_mut(x, y) = Rgb([255, 0, 0]);
            }
        }

        let state = read_device(&img, &profile);
        let reading = state.get("L1").unwrap();
        assert_eq!(reading.status, DetectionStatus::Misaligned);
        assert_eq!(reading.state, "MISALIGNED");
        assert_ne!(reading.position, Some([50, 49]));
    }

    #[test]
    fn allowed_states_prevent_neighbor_color_false_match() {
        let mut profile = test_profile();
        profile.buttons[0].allowed_states = vec!["OFF".to_string()];
        let mut img = RgbImage::new(100, 100);
        fill(&mut img, [255, 0, 0]);

        let state = read_device(&img, &profile);
        let reading = state.get("L1").unwrap();
        assert_eq!(reading.status, DetectionStatus::Unknown);
        assert_eq!(reading.state, "UNKNOWN");
    }

    #[test]
    fn region_scoped_state_models_override_global_states() {
        let mut profile = test_profile();
        profile.states = vec![rule(
            "GLOBAL_RED",
            vec![HsvRange::new([0, 70, 70], [10, 255, 255])],
        )];
        profile.state_models.insert(
            "l1.BLUE".to_string(),
            crate::camera::profile::StateModelRule {
                hsv: vec![HsvRange::new([110, 70, 70], [130, 255, 255])],
                ..Default::default()
            },
        );
        let mut img = RgbImage::new(100, 100);
        fill(&mut img, [10, 10, 10]);
        draw_disc(&mut img, 50, 50, 8, [0, 0, 255]);

        let state = read_device(&img, &profile);
        let reading = state.get("l1").unwrap();
        assert_eq!(reading.status, DetectionStatus::Match);
        assert_eq!(reading.state, "BLUE");
    }

    #[test]
    fn region_scoped_state_models_extend_global_states() {
        let mut profile = test_profile();
        profile.states = vec![rule("RED", vec![HsvRange::new([0, 70, 70], [12, 255, 255])])];
        profile.state_models.insert(
            "l1.TESTBLUE".to_string(),
            crate::camera::profile::StateModelRule {
                hsv: vec![HsvRange::new([110, 70, 70], [130, 255, 255])],
                ..Default::default()
            },
        );

        let rules = profile.effective_state_rules(&profile.buttons[0]);
        assert!(rules.iter().any(|rule| rule.name == "RED"));
        assert!(rules.iter().any(|rule| rule.name == "TESTBLUE"));
    }

    #[test]
    fn dark_and_white_state_models_match_low_saturation_cases() {
        let mut profile = test_profile();
        profile.states = vec![
            StateRule {
                name: "OFF".into(),
                rule_type: Some("dark".into()),
                hsv: Vec::new(),
                dark_max_v: Some(40),
                white_max_s: None,
                white_min_v: None,
                source: Some("test".into()),
            },
            StateRule {
                name: "WHITE".into(),
                rule_type: Some("white".into()),
                hsv: Vec::new(),
                dark_max_v: None,
                white_max_s: Some(30),
                white_min_v: Some(180),
                source: Some("test".into()),
            },
        ];

        let mut img = RgbImage::new(100, 100);
        fill(&mut img, [15, 15, 15]);
        let state = read_device(&img, &profile);
        assert_eq!(state.get("L1").unwrap().state, "OFF");

        fill(&mut img, [20, 20, 20]);
        draw_disc(&mut img, 50, 50, 8, [230, 230, 230]);
        let state = read_device(&img, &profile);
        assert_eq!(state.get("L1").unwrap().state, "WHITE");
    }

    #[test]
    fn learns_red_range_that_matches_red() {
        let mut img = RgbImage::new(100, 100);
        fill(&mut img, [230, 20, 20]);
        let ranges = learn_ranges(&img, &[[10, 10, 80, 80]]);
        let hsv = rgb_to_hsv(230, 20, 20);
        assert!(ranges.iter().any(|r| r.contains(hsv)));
    }

    #[test]
    fn grid_generates_expected_button_count() {
        let buttons = CameraProfile::grid_buttons([500, 500], 2, 2, 0.6);
        assert_eq!(buttons.len(), 4);
        assert_eq!(buttons[0].label, "Nút 1");
        assert_eq!(buttons[3].label, "Nút 4");
    }
}
