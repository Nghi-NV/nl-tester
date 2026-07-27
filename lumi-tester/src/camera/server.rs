//! Web server for the camera calibration / observation UI.
//!
//! Reuses the same axum + browser-canvas approach as the device inspector.
//! The tester aims the camera, clicks the 4 device corners, picks a layout
//! template, and *learns* the ON/OFF colors by toggling the real device — no
//! HSV numbers or coordinates are ever typed. The same UI doubles as a
//! read-only live view (`observe` mode) during test runs.

use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use crate::camera::detect::{self, DetectionStatus};
use crate::camera::profile::{CameraProfile, Geometry};
use crate::camera::stream::{FfmpegGrabber, FrameSource};

pub struct CalibrateConfig {
    pub rtsp: String,
    pub transport: Option<String>,
    /// Output path for the profile (loaded first if it already exists).
    pub profile_path: PathBuf,
    pub port: u16,
    /// Read-only live view (requires an existing profile).
    pub observe: bool,
}

struct AppState {
    grabber: Arc<dyn FrameSource>,
    rtsp: String,
    output: Mutex<PathBuf>,
    existing: Option<CameraProfile>,
    dims: (u32, u32),
    observe: bool,
}

pub struct CalibrateServer {
    config: CalibrateConfig,
}

impl CalibrateServer {
    pub fn new(config: CalibrateConfig) -> Self {
        Self { config }
    }

    pub async fn start(&self) -> Result<()> {
        println!(
            "📡 Connecting to camera {} …",
            crate::camera::redact_url(&self.config.rtsp)
        );
        let grabber = FfmpegGrabber::start(&self.config.rtsp, self.config.transport.as_deref())
            .context("could not connect to RTSP camera")?;
        let dims = grabber.dimensions();
        println!("✅ Connected ({}x{})", dims.0, dims.1);

        let existing = if self.config.profile_path.exists() {
            match CameraProfile::load(&self.config.profile_path) {
                Ok(p) => {
                    println!(
                        "📂 Loaded existing profile: {}",
                        self.config.profile_path.display()
                    );
                    Some(p)
                }
                Err(e) => {
                    eprintln!("⚠️  Could not load existing profile: {}", e);
                    None
                }
            }
        } else {
            None
        };

        if self.config.observe && existing.is_none() {
            anyhow::bail!(
                "observe mode needs an existing profile at {}",
                self.config.profile_path.display()
            );
        }

        let state = Arc::new(AppState {
            grabber: Arc::new(grabber),
            rtsp: self.config.rtsp.clone(),
            output: Mutex::new(self.config.profile_path.clone()),
            existing,
            dims,
            observe: self.config.observe,
        });

        let app = Router::new()
            .route("/", get(serve_index))
            .route("/view", get(serve_view))
            .route("/api/info", get(api_info))
            .route("/api/frame.jpg", get(api_frame))
            .route("/api/detect", post(api_detect))
            .route("/api/learn", post(api_learn))
            .route("/api/propose-leds", post(api_propose_leds))
            .route("/api/verify", post(api_verify))
            .route("/api/save", post(api_save))
            .with_state(state);

        let addr = SocketAddr::from(([127, 0, 0, 1], self.config.port));
        let mode = if self.config.observe {
            "Observe"
        } else {
            "Calibrate"
        };
        println!("\n🎥 Camera {} UI started!", mode);
        println!("   Open: http://localhost:{}", self.config.port);
        println!("   Output profile: {}", self.config.profile_path.display());
        println!("\n   Press Ctrl+C to stop.\n");

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app.into_make_service()).await?;
        Ok(())
    }
}

async fn serve_index() -> impl IntoResponse {
    let mut html = include_str!("ui/calibrate.html").to_string();
    let css = include_str!("ui/style.css");
    let js = include_str!("ui/script.js");
    html = html.replace("</head>", &format!("<style>{}</style></head>", css));
    html = html.replace("</body>", &format!("<script>{}</script></body>", js));
    Html(html)
}

async fn serve_view() -> impl IntoResponse {
    Html(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Lumi Camera View</title>
  <style>
    body { margin: 0; background: #111; color: #eee; font-family: system-ui, sans-serif; }
    header { height: 44px; display: flex; align-items: center; justify-content: space-between; padding: 0 14px; background: #181818; border-bottom: 1px solid #333; }
    main { height: calc(100vh - 45px); display: grid; place-items: center; overflow: hidden; }
    img { max-width: 100%; max-height: 100%; object-fit: contain; }
    a { color: #8ab4ff; text-decoration: none; }
    .muted { color: #aaa; font-size: 13px; }
  </style>
</head>
<body>
  <header>
    <strong>Camera View</strong>
    <span class="muted">Live frame · <a href="/">Profile</a></span>
  </header>
  <main><img id="frame" alt="Camera live frame" /></main>
  <script>
    const frame = document.getElementById("frame");
    function refresh() {
      const next = new Image();
      next.onload = () => { frame.src = next.src; setTimeout(refresh, 250); };
      next.onerror = () => setTimeout(refresh, 1000);
      next.src = "/api/frame.jpg?ts=" + Date.now();
    }
    refresh();
  </script>
</body>
</html>"#,
    )
}

async fn api_info(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(serde_json::json!({
        "hasRtsp": !state.rtsp.trim().is_empty(),
        "rtspRedacted": crate::camera::redact_url(&state.rtsp),
        "width": state.dims.0,
        "height": state.dims.1,
        "observe": state.observe,
        "output": state.output.lock().unwrap().display().to_string(),
        "profile": state.existing,
    }))
}

fn encode_jpeg(img: &image::RgbImage) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 80);
    enc.encode_image(img)?;
    Ok(buf)
}

async fn api_frame(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.grabber.latest() {
        Some(img) => match encode_jpeg(&img) {
            Ok(bytes) => ([(header::CONTENT_TYPE, "image/jpeg")], bytes).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
        None => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

/// Run detection for a draft profile against the current frame.
async fn api_detect(
    State(state): State<Arc<AppState>>,
    Json(profile): Json<CameraProfile>,
) -> impl IntoResponse {
    let Some(frame) = state.grabber.latest() else {
        return Json(serde_json::json!({ "error": "no frame yet" }));
    };
    let warped = detect::warp_device(&frame, &profile);
    let device = detect::read_device_warped(&warped, &profile);
    let warped_jpg = encode_jpeg(&warped).ok().map(|b| {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(b)
    });
    Json(serde_json::json!({
        "states": device,
        "warped": warped_jpg,
    }))
}

#[derive(serde::Deserialize)]
struct LearnRequest {
    geometry: Geometry,
    rois: Vec<[u32; 4]>,
}

/// Sample LED colors inside the given warped ROIs and return HSV ranges.
async fn api_learn(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LearnRequest>,
) -> impl IntoResponse {
    let Some(frame) = state.grabber.latest() else {
        return Json(serde_json::json!({ "error": "no frame yet" }));
    };
    // Build a minimal profile just to reuse the warp.
    let probe = CameraProfile {
        name: None,
        camera: Default::default(),
        lab: None,
        active_camera_id: None,
        active_device_id: None,
        geometry: req.geometry,
        layout: crate::camera::profile::Layout::Custom,
        buttons: vec![],
        states: vec![],
        state_models: Default::default(),
        min_ratio: 0.01,
        min_margin: 0.03,
    };
    let warped = detect::warp_device(&frame, &probe);
    let ranges = detect::learn_ranges(&warped, &req.rois);
    Json(serde_json::json!({ "ranges": ranges }))
}

async fn api_propose_leds(
    State(state): State<Arc<AppState>>,
    Json(profile): Json<CameraProfile>,
) -> impl IntoResponse {
    let Some(frame) = state.grabber.latest() else {
        return Json(serde_json::json!({ "error": "no frame yet" }));
    };
    let warped = detect::warp_device(&frame, &profile);
    let proposals = detect::propose_led_rois(&warped, &profile);
    Json(serde_json::json!({
        "proposals": proposals,
    }))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerifyRequest {
    profile: CameraProfile,
    #[serde(default)]
    duration_ms: Option<u64>,
    #[serde(default)]
    sample_ms: Option<u64>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct VerifyRegion {
    id: Option<String>,
    label: String,
    samples: u32,
    match_count: u32,
    unknown_count: u32,
    ambiguous_count: u32,
    misaligned_count: u32,
    unstable: bool,
    min_confidence: f32,
    avg_confidence: f32,
    states: BTreeMap<String, u32>,
    missing_states: Vec<String>,
    #[serde(skip)]
    confidence_sum: f32,
}

impl VerifyRegion {
    fn from_button(button: &crate::camera::profile::ButtonRoi, profile: &CameraProfile) -> Self {
        let missing_states = missing_learned_states(button, profile);
        Self {
            id: button.id.clone(),
            label: button.label.clone(),
            samples: 0,
            match_count: 0,
            unknown_count: 0,
            ambiguous_count: 0,
            misaligned_count: 0,
            unstable: false,
            min_confidence: 1.0,
            avg_confidence: 0.0,
            states: BTreeMap::new(),
            missing_states,
            confidence_sum: 0.0,
        }
    }

    fn record(&mut self, reading: &detect::ButtonReading) {
        self.samples += 1;
        match reading.status {
            DetectionStatus::Match => self.match_count += 1,
            DetectionStatus::Unknown => self.unknown_count += 1,
            DetectionStatus::Ambiguous => self.ambiguous_count += 1,
            DetectionStatus::Misaligned => self.misaligned_count += 1,
        }
        *self.states.entry(reading.state.clone()).or_insert(0) += 1;
        self.min_confidence = self.min_confidence.min(reading.confidence);
        self.confidence_sum += reading.confidence;
    }

    fn finish(&mut self) {
        if self.samples == 0 {
            self.min_confidence = 0.0;
            self.avg_confidence = 0.0;
        } else {
            self.avg_confidence = self.confidence_sum / self.samples as f32;
        }
        self.unstable = self.states.len() > 1;
    }

    fn ok(&self) -> bool {
        self.samples > 0
            && self.unknown_count == 0
            && self.ambiguous_count == 0
            && self.misaligned_count == 0
            && !self.unstable
            && self.min_confidence >= 0.05
    }
}

fn missing_learned_states(
    button: &crate::camera::profile::ButtonRoi,
    profile: &CameraProfile,
) -> Vec<String> {
    if button.allowed_states.is_empty() {
        return Vec::new();
    }
    let region_id = button.id.as_deref().unwrap_or(&button.label);
    button
        .allowed_states
        .iter()
        .filter(|state| {
            let state = state.trim();
            if state.is_empty() {
                return false;
            }
            let scoped_key = format!("{}.{}", region_id, state);
            !profile
                .state_models
                .keys()
                .any(|key| key.eq_ignore_ascii_case(&scoped_key))
                && !profile
                    .states
                    .iter()
                    .any(|rule| rule.name.eq_ignore_ascii_case(state))
        })
        .cloned()
        .collect()
}

/// Sample the current profile for a few seconds and report whether each ROI is
/// stable, known, and confidently separated from adjacent colors.
async fn api_verify(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyRequest>,
) -> impl IntoResponse {
    let duration_ms = req.duration_ms.unwrap_or(5_000).clamp(500, 30_000);
    let sample_ms = req.sample_ms.unwrap_or(200).clamp(50, 2_000);
    let mut regions: Vec<VerifyRegion> = req
        .profile
        .buttons
        .iter()
        .map(|button| VerifyRegion::from_button(button, &req.profile))
        .collect();
    let start = Instant::now();
    let deadline = start + Duration::from_millis(duration_ms);
    let mut sample_count = 0u32;
    let mut last_captured_at: Option<SystemTime> = None;
    let mut latest_frame_age_ms: Option<u64> = None;

    while Instant::now() < deadline {
        if let Some(frame) = state.grabber.latest_frame() {
            if last_captured_at == Some(frame.captured_at) {
                tokio::time::sleep(Duration::from_millis(sample_ms)).await;
                continue;
            }
            latest_frame_age_ms = frame
                .captured_at
                .elapsed()
                .ok()
                .map(|age| age.as_millis() as u64);
            last_captured_at = Some(frame.captured_at);

            let warped = detect::warp_device(&frame.image, &req.profile);
            let device = detect::read_device_warped(&warped, &req.profile);
            for (region, reading) in regions.iter_mut().zip(device.buttons.iter()) {
                region.record(reading);
            }
            sample_count += 1;
        }
        tokio::time::sleep(Duration::from_millis(sample_ms)).await;
    }

    for region in regions.iter_mut() {
        region.finish();
    }
    let max_frame_age_ms = sample_ms.saturating_mul(3).max(1_000);
    let fresh = latest_frame_age_ms
        .map(|age| age <= max_frame_age_ms)
        .unwrap_or(false);
    let ok =
        sample_count >= 3 && fresh && !regions.is_empty() && regions.iter().all(VerifyRegion::ok);

    Json(serde_json::json!({
        "ok": ok,
        "sampleCount": sample_count,
        "fresh": fresh,
        "latestFrameAgeMs": latest_frame_age_ms,
        "minConfidenceFloor": 0.05,
        "durationMs": start.elapsed().as_millis() as u64,
        "regions": regions,
    }))
}

async fn api_save(
    State(state): State<Arc<AppState>>,
    Json(profile): Json<CameraProfile>,
) -> impl IntoResponse {
    if state.observe {
        return Json(serde_json::json!({ "error": "observe mode is read-only" }));
    }
    let path = state.output.lock().unwrap().clone();
    match profile.save(&path) {
        Ok(()) => Json(serde_json::json!({ "ok": true, "path": path.display().to_string() })),
        Err(e) => Json(serde_json::json!({ "error": e.to_string() })),
    }
}
