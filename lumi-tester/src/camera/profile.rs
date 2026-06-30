//! Camera profile model for hardware LED verification.
//!
//! A profile captures everything needed to read a physical device's LED states
//! from an RTSP camera, independent of the device kind:
//! - `geometry`: the 4 outer corners of the device on the raw frame, plus the
//!   size of the rectified (perspective-warped) image.
//! - `layout`: how buttons/LEDs are arranged (a grid for 1/2/3/4/.../10-button
//!   switches and sockets, or a free-form list of regions for arbitrary devices).
//! - `buttons`: per-button label + region-of-interest inside the warped image.
//! - `states`: data-driven color rules mapping an HSV color to a state name
//!   (e.g. RED -> "ON", BLUE -> "OFF"). Learned from the device, never hardcoded.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// A single HSV range in OpenCV convention: H in 0..=179, S/V in 0..=255.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HsvRange {
    pub lower: [u8; 3],
    pub upper: [u8; 3],
}

impl HsvRange {
    pub fn new(lower: [u8; 3], upper: [u8; 3]) -> Self {
        Self { lower, upper }
    }

    /// Whether an HSV pixel falls inside this range (hue wrap handled by caller
    /// via multiple ranges).
    pub fn contains(&self, hsv: [u8; 3]) -> bool {
        (self.lower[0]..=self.upper[0]).contains(&hsv[0])
            && (self.lower[1]..=self.upper[1]).contains(&hsv[1])
            && (self.lower[2]..=self.upper[2]).contains(&hsv[2])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_alias_and_camel_case_fields_deserialize() {
        let json = r#"
        {
          "name": "compat",
          "geometry": {
            "corners": [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
            "warp": [100, 100]
          },
          "layout": { "type": "custom" },
          "regions": [
            {
              "id": "wifi_led",
              "label": "WiFi",
              "roi": [10, 10, 20, 20],
              "allowedStates": ["OFF", "WHITE"],
              "expectedCenter": [20, 20],
              "maxCenterDrift": 6
            }
          ],
          "states": [
            { "name": "OFF", "type": "dark", "darkMaxV": 35 },
            { "name": "WHITE", "type": "white", "whiteMaxS": 40, "whiteMinV": 180 }
          ],
          "minMargin": 0.08
        }
        "#;

        let profile: CameraProfile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.buttons.len(), 1);
        assert!(profile.button("wifi_led").is_some());
        assert!(profile.button("WiFi").is_some());
        assert_eq!(
            profile.buttons[0].allowed_states,
            vec!["OFF".to_string(), "WHITE".to_string()]
        );
        assert_eq!(profile.buttons[0].expected_center, Some([20, 20]));
        assert_eq!(profile.buttons[0].max_center_drift, Some(6));
        assert_eq!(profile.states[0].dark_max_v, Some(35));
        assert_eq!(profile.states[1].white_max_s, Some(40));
        assert_eq!(profile.states[1].white_min_v, Some(180));
        assert!((profile.min_margin - 0.08).abs() < f32::EPSILON);
    }

    #[test]
    fn state_models_deserialize_without_repeating_state_name() {
        let json = r#"
        {
          "name": "scoped",
          "geometry": {
            "corners": [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
            "warp": [100, 100]
          },
          "layout": { "type": "custom" },
          "regions": [
            { "id": "button_1", "label": "Nút 1", "roi": [10, 10, 20, 20] }
          ],
          "stateModels": {
            "button_1.OFF": { "type": "dark", "darkMaxV": 30 },
            "button_1.WHITE": { "type": "white", "whiteMaxS": 30, "whiteMinV": 190 }
          }
        }
        "#;

        let profile: CameraProfile = serde_json::from_str(json).unwrap();
        let rules = profile.effective_state_rules(&profile.buttons[0]);
        assert_eq!(rules.len(), 2);
        assert!(rules.iter().any(|rule| rule.name == "OFF"));
        assert!(rules.iter().any(|rule| rule.name == "WHITE"));
        assert_eq!(rules[0].dark_max_v, Some(30));
    }

    #[test]
    fn lab_profile_namespaces_duplicate_button_ids_by_device() {
        let json = r#"
        {
          "name": "bench",
          "activeDeviceId": "switch_4gang_wall",
          "lab": {
            "name": "main bench",
            "activeCameraId": "camera_a",
            "activeDeviceId": "switch_4gang_wall",
            "cameras": [
              { "id": "camera_a", "label": "Camera A" }
            ],
            "devices": [
              {
                "id": "switch_3gang_desk",
                "label": "Công tắc bàn",
                "cameraId": "camera_a",
                "kind": "switch_3",
                "regions": [
                  { "deviceId": "switch_3gang_desk", "id": "button_1", "label": "Nút 1", "roi": [10, 10, 20, 20] }
                ]
              },
              {
                "id": "switch_4gang_wall",
                "label": "Công tắc tường",
                "cameraId": "camera_a",
                "kind": "switch_4",
                "regions": [
                  { "deviceId": "switch_4gang_wall", "id": "button_1", "label": "Nút 1", "roi": [40, 40, 20, 20] }
                ]
              }
            ]
          },
          "geometry": {
            "corners": [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]],
            "warp": [100, 100]
          },
          "layout": { "type": "custom" },
          "regions": [
            { "deviceId": "switch_3gang_desk", "id": "button_1", "label": "Nút 1", "roi": [10, 10, 20, 20] },
            { "deviceId": "switch_4gang_wall", "id": "button_1", "label": "Nút 1", "roi": [40, 40, 20, 20] }
          ],
          "stateModels": {
            "switch_3gang_desk.button_1.ON": { "hsv": [{ "lower": [100, 80, 80], "upper": [130, 255, 255] }] },
            "switch_4gang_wall.button_1.ON": { "hsv": [{ "lower": [0, 80, 80], "upper": [15, 255, 255] }] }
          }
        }
        "#;

        let profile: CameraProfile = serde_json::from_str(json).unwrap();
        let desk_rules = profile.effective_state_rules(&profile.buttons[0]);
        let wall_rules = profile.effective_state_rules(&profile.buttons[1]);

        assert_eq!(profile.lab.as_ref().unwrap().devices.len(), 2);
        assert_eq!(profile.buttons[0].id, profile.buttons[1].id);
        assert_eq!(desk_rules.len(), 1);
        assert_eq!(wall_rules.len(), 1);
        assert_eq!(desk_rules[0].hsv[0].lower[0], 100);
        assert_eq!(wall_rules[0].hsv[0].lower[0], 0);
    }
}

/// A named state defined by one or more HSV ranges (multiple ranges allow hue
/// wrap-around, e.g. red spans both ends of the hue circle).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateRule {
    /// State name reported to tests, e.g. "ON", "OFF", "STANDBY".
    pub name: String,
    /// Optional model type. Defaults to HSV color matching. Supported values:
    /// "color", "dark" (OFF/baseline), and "white" (low saturation, high value).
    #[serde(default, rename = "type")]
    pub rule_type: Option<String>,
    /// HSV ranges that count as this state.
    #[serde(default)]
    pub hsv: Vec<HsvRange>,
    /// Max V for `type: dark`.
    #[serde(default, alias = "darkMaxV")]
    pub dark_max_v: Option<u8>,
    /// Max S for `type: white`.
    #[serde(default, alias = "whiteMaxS")]
    pub white_max_s: Option<u8>,
    /// Min V for `type: white`.
    #[serde(default, alias = "whiteMinV")]
    pub white_min_v: Option<u8>,
    /// How the range was produced: "learned" or "manual" (informational).
    #[serde(default)]
    pub source: Option<String>,
}

impl StateRule {
    pub fn matches(&self, hsv: [u8; 3]) -> bool {
        match self
            .rule_type
            .as_deref()
            .unwrap_or("color")
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "dark" => hsv[2] <= self.dark_max_v.unwrap_or(45),
            "white" => {
                hsv[1] <= self.white_max_s.unwrap_or(60)
                    && hsv[2] >= self.white_min_v.unwrap_or(120)
            }
            _ => self.hsv.iter().any(|r| r.contains(hsv)),
        }
    }
}

/// Region-scoped state model used by the forward-looking `stateModels` shape:
/// `{ "button_1.ON": { "type": "dark" } }`. The state name can be omitted
/// because the map key already contains it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateModelRule {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "type")]
    pub rule_type: Option<String>,
    #[serde(default)]
    pub hsv: Vec<HsvRange>,
    #[serde(default, alias = "darkMaxV")]
    pub dark_max_v: Option<u8>,
    #[serde(default, alias = "whiteMaxS")]
    pub white_max_s: Option<u8>,
    #[serde(default, alias = "whiteMinV")]
    pub white_min_v: Option<u8>,
    #[serde(default)]
    pub source: Option<String>,
}

impl StateModelRule {
    fn to_state_rule(&self, key: &str) -> StateRule {
        let inferred = key
            .rsplit_once('.')
            .map(|(_, state)| state)
            .unwrap_or(key)
            .to_string();
        StateRule {
            name: self.name.clone().unwrap_or(inferred),
            rule_type: self.rule_type.clone(),
            hsv: self.hsv.clone(),
            dark_max_v: self.dark_max_v,
            white_max_s: self.white_max_s,
            white_min_v: self.white_min_v,
            source: self.source.clone(),
        }
    }
}

/// Perspective geometry: 4 source corners on the raw frame + warped size.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Geometry {
    /// 4 corners in the raw frame, order: top-left, top-right, bottom-right,
    /// bottom-left (clockwise from TL). Stored as [x, y] in source pixels.
    pub corners: [[f32; 2]; 4],
    /// Rectified image size [width, height].
    #[serde(default = "default_warp")]
    pub warp: [u32; 2],
}

fn default_warp() -> [u32; 2] {
    [500, 500]
}

/// Device layout, drives automatic ROI generation in the warped image.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Layout {
    /// Regular grid: covers 1/2/3/4/6/10-button switches and socket outlets.
    Grid {
        rows: u32,
        cols: u32,
        /// Fraction (0.0..1.0) of each cell used as the LED search ROI, centered.
        #[serde(default = "default_cell_fill")]
        cell_fill: f32,
    },
    /// Free-form regions for arbitrary devices.
    Custom,
}

fn default_cell_fill() -> f32 {
    0.6
}

/// A single button/LED region inside the warped image: [x, y, w, h].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonRoi {
    /// Optional parent device namespace. Required when a lab profile has
    /// multiple devices that each contain `button_1`, `button_2`, ...
    #[serde(default, alias = "deviceId")]
    pub device_id: Option<String>,
    /// Stable id used by tests, e.g. `button_1` or `status`.
    #[serde(default)]
    pub id: Option<String>,
    pub label: String,
    #[serde(default)]
    pub kind: Option<String>,
    /// Tight LED read ROI. Detection/classification happens here.
    pub roi: [u32; 4],
    /// Wider search area used by auto-detect to re-center the LED ROI when the
    /// camera or device shifts. Backward compatible: when omitted, callers derive
    /// a search area from `roi`.
    #[serde(default, alias = "searchRoi")]
    pub search_roi: Option<[u32; 4]>,
    /// `rect` (default) or `ellipse`.
    #[serde(default)]
    pub mask: Option<String>,
    #[serde(default, alias = "expectedCenter")]
    pub expected_center: Option<[u32; 2]>,
    #[serde(default, alias = "maxCenterDrift")]
    pub max_center_drift: Option<u32>,
    /// Restrict this region to a subset of state names.
    #[serde(default, alias = "allowedStates")]
    pub allowed_states: Vec<String>,
}

/// The connection part of a profile. Kept separate so device geometry can be
/// reused across cameras / credentials.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CameraConn {
    #[serde(default)]
    pub rtsp: Option<String>,
    /// "tcp" | "udp" | "auto" (default auto).
    #[serde(default)]
    pub transport: Option<String>,
}

/// Lab-level camera metadata. A lab can use one camera to observe multiple
/// devices, or split devices across several cameras.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LabCamera {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub rtsp: Option<String>,
    #[serde(default)]
    pub transport: Option<String>,
}

/// Lab-level device metadata. Region IDs are unique only inside this device;
/// the globally stable test target is `<device_id>.<region_id>`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LabDevice {
    pub id: String,
    pub label: String,
    #[serde(default, alias = "cameraId")]
    pub camera_id: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub regions: Vec<ButtonRoi>,
}

/// Forward-compatible lab profile metadata embedded in `CameraProfile`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LabProfile {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub cameras: Vec<LabCamera>,
    #[serde(default)]
    pub devices: Vec<LabDevice>,
    #[serde(default, alias = "activeCameraId")]
    pub active_camera_id: Option<String>,
    #[serde(default, alias = "activeDeviceId")]
    pub active_device_id: Option<String>,
}

/// Full camera profile persisted as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraProfile {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub camera: CameraConn,
    #[serde(default)]
    pub lab: Option<LabProfile>,
    #[serde(default, alias = "activeCameraId")]
    pub active_camera_id: Option<String>,
    #[serde(default, alias = "activeDeviceId")]
    pub active_device_id: Option<String>,
    pub geometry: Geometry,
    pub layout: Layout,
    /// Named regions to read. For switches these are buttons; for a home
    /// controller they are individual status LEDs. `regions` is accepted as an
    /// alias so non-button devices read naturally.
    #[serde(alias = "regions")]
    pub buttons: Vec<ButtonRoi>,
    #[serde(default)]
    pub states: Vec<StateRule>,
    /// Forward-compatible region-scoped models keyed by `<region_id>.<STATE>`.
    /// When present for a region, these rules take precedence over global
    /// `states`, so adjacent LEDs can use different thresholds for the same
    /// logical state without duplicating profiles.
    #[serde(
        default,
        rename = "stateModels",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub state_models: BTreeMap<String, StateModelRule>,
    /// Minimum fraction of ROI pixels matching a state to accept it (anti-noise).
    #[serde(default = "default_min_ratio")]
    pub min_ratio: f32,
    /// Minimum gap between best and second-best states to accept a match.
    #[serde(default = "default_min_margin", alias = "minMargin")]
    pub min_margin: f32,
}

fn default_min_ratio() -> f32 {
    0.01
}

fn default_min_margin() -> f32 {
    0.03
}

impl CameraProfile {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read camera profile: {}", path.display()))?;
        let profile: CameraProfile = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse camera profile: {}", path.display()))?;
        Ok(profile)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).ok();
            }
        }
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(path, text)
            .with_context(|| format!("failed to write camera profile: {}", path.display()))?;
        Ok(())
    }

    /// Look up a button ROI by label (case-insensitive, trimmed).
    pub fn button(&self, label: &str) -> Option<&ButtonRoi> {
        let want = label.trim().to_lowercase();
        self.buttons.iter().find(|b| {
            b.label.trim().to_lowercase() == want
                || b.id
                    .as_deref()
                    .map(|id| id.trim().to_lowercase() == want)
                    .unwrap_or(false)
        })
    }

    pub fn effective_state_rules(&self, roi: &ButtonRoi) -> Vec<StateRule> {
        let mut rules = self.states.clone();
        let scoped = self
            .state_models
            .iter()
            .filter_map(|(key, rule)| {
                let (region, _) = key.rsplit_once('.')?;
                if region_matches(roi, region) {
                    Some(rule.to_state_rule(key))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for scoped_rule in scoped {
            if let Some(existing) = rules
                .iter_mut()
                .find(|rule| rule.name.eq_ignore_ascii_case(&scoped_rule.name))
            {
                *existing = scoped_rule;
            } else {
                rules.push(scoped_rule);
            }
        }
        rules
    }

    /// Generate button ROIs from a grid layout, filling the warped rectangle.
    /// Labels default to "Nút 1".."Nút N" in row-major order.
    pub fn grid_buttons(warp: [u32; 2], rows: u32, cols: u32, cell_fill: f32) -> Vec<ButtonRoi> {
        let (w, h) = (warp[0] as f32, warp[1] as f32);
        let cw = w / cols as f32;
        let ch = h / rows as f32;
        let fill = cell_fill.clamp(0.1, 1.0);
        let mut out = Vec::new();
        let mut idx = 1u32;
        for r in 0..rows {
            for c in 0..cols {
                let cx = (c as f32 + 0.5) * cw;
                let cy = (r as f32 + 0.5) * ch;
                let rw = cw * fill;
                let rh = ch * fill;
                let x = (cx - rw / 2.0).max(0.0) as u32;
                let y = (cy - rh / 2.0).max(0.0) as u32;
                out.push(ButtonRoi {
                    device_id: None,
                    id: Some(format!("button_{}", idx)),
                    label: format!("Nút {}", idx),
                    kind: Some("button_led".to_string()),
                    roi: [x, y, rw as u32, rh as u32],
                    search_roi: None,
                    mask: Some("ellipse".to_string()),
                    expected_center: None,
                    max_center_drift: None,
                    allowed_states: Vec::new(),
                });
                idx += 1;
            }
        }
        out
    }
}

fn region_matches(roi: &ButtonRoi, region: &str) -> bool {
    let want = region.trim();
    if let (Some(device_id), Some(region_id)) = (roi.device_id.as_deref(), roi.id.as_deref()) {
        let qualified = format!("{}.{}", device_id.trim(), region_id.trim());
        if qualified.eq_ignore_ascii_case(want) {
            return true;
        }
        if want.contains('.') {
            return false;
        }
    }
    roi.id
        .as_deref()
        .map(|id| {
            id.trim().eq_ignore_ascii_case(want)
                || want
                    .rsplit_once('.')
                    .map(|(_, suffix)| suffix.trim().eq_ignore_ascii_case(id.trim()))
                    .unwrap_or(false)
        })
        .unwrap_or(false)
        || roi.label.trim().eq_ignore_ascii_case(want)
}
