//! Frame source: RTSP → decoded RGB frames.
//!
//! Decoding is abstracted behind the [`FrameSource`] trait so a pure-Rust
//! backend (lumi-camera-rtsp + openh264, H.264 only) can be added later without
//! touching the CV / UI / test layers. The default backend shells out to the
//! bundled FFmpeg binary, which decodes every codec the Lumi cameras emit
//! (H.264 / H.265 / H.265+) and works headlessly inside CI test runs.

use anyhow::{anyhow, Context, Result};
use image::RgbImage;
use std::io::Read;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

#[derive(Clone)]
pub struct CameraFrame {
    pub image: RgbImage,
    pub captured_at: SystemTime,
}

/// Anything that can hand out the most recent decoded frame.
pub trait FrameSource: Send + Sync {
    /// The latest decoded frame, if one has arrived yet.
    fn latest(&self) -> Option<RgbImage>;

    /// The latest decoded frame with a local receive timestamp.
    fn latest_frame(&self) -> Option<CameraFrame>;
}

/// Normalize a transport string into an FFmpeg `-rtsp_transport` value.
/// "auto" maps to TCP, which is the most reliable for interleaved decoding.
fn rtsp_transport(transport: Option<&str>) -> &'static str {
    match transport.map(|t| t.trim().to_lowercase()).as_deref() {
        Some("udp") => "udp",
        _ => "tcp",
    }
}

fn ffmpeg() -> Result<PathBuf> {
    // Prefer a full system FFmpeg: the Playwright-bundled build that
    // `find_ffmpeg()` resolves is stripped and lacks the RTSP/mjpeg support
    // required to decode camera streams ("Option not found").
    if let Ok(path) = which::which("ffmpeg") {
        return Ok(path);
    }
    crate::utils::binary_resolver::find_ffmpeg().context(
        "full FFmpeg not found — install it (macOS: `brew install ffmpeg`, \
         Linux: `apt install ffmpeg`) so the camera stream can be decoded",
    )
}

/// Capture a single frame from an RTSP URL as an RGB image (JPEG via FFmpeg).
pub fn snapshot(rtsp: &str, transport: Option<&str>) -> Result<RgbImage> {
    let bin = ffmpeg()?;
    let mut child = std::process::Command::new(&bin)
        .args([
            "-nostdin",
            "-loglevel",
            "error",
            "-rtsp_transport",
            rtsp_transport(transport),
            "-i",
            rtsp,
            "-frames:v",
            "1",
            "-f",
            "image2",
            "-c:v",
            "mjpeg",
            "-q:v",
            "2",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn ffmpeg for snapshot")?;

    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(status) = child
            .try_wait()
            .context("failed to poll ffmpeg snapshot process")?
        {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            if let Some(mut out) = child.stdout.take() {
                let _ = out.read_to_end(&mut stdout);
            }
            if let Some(mut err) = child.stderr.take() {
                let _ = err.read_to_end(&mut stderr);
            }
            if !status.success() || stdout.is_empty() {
                let err = String::from_utf8_lossy(&stderr);
                let redacted =
                    crate::camera::redact_url(err.lines().last().unwrap_or("unknown error"));
                return Err(anyhow!(
                    "ffmpeg could not read a frame from the camera: {}",
                    redacted
                ));
            }

            let img = image::load_from_memory(&stdout)
                .context("failed to decode snapshot JPEG")?
                .to_rgb8();
            return Ok(img);
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(anyhow!(
                "ffmpeg timed out after 20s while reading a camera frame"
            ));
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

/// A warm, continuously-decoding RTSP frame source backed by an FFmpeg
/// `rawvideo` pipe. Keeps the latest frame in shared memory; the child process
/// is killed on drop.
pub struct FfmpegGrabber {
    latest: Arc<Mutex<Option<CameraFrame>>>,
    stop: Arc<AtomicBool>,
    width: u32,
    height: u32,
}

impl FfmpegGrabber {
    /// Connect and start streaming. Probes resolution with a one-shot snapshot,
    /// then spawns a background reader that keeps `latest` up to date.
    pub fn start(rtsp: &str, transport: Option<&str>) -> Result<Self> {
        // Probe dimensions (and validate connectivity) up front.
        let probe = snapshot(rtsp, transport)?;
        let (width, height) = probe.dimensions();

        let latest = Arc::new(Mutex::new(Some(CameraFrame {
            image: probe,
            captured_at: SystemTime::now(),
        })));
        let stop = Arc::new(AtomicBool::new(false));

        let reader_latest = latest.clone();
        let reader_stop = stop.clone();
        let rtsp_owned = rtsp.to_string();
        let transport_owned = transport.map(|s| s.to_string());

        std::thread::Builder::new()
            .name("lumi-camera-grabber".into())
            .spawn(move || {
                reader_loop(
                    &rtsp_owned,
                    transport_owned.as_deref(),
                    width,
                    height,
                    reader_latest,
                    reader_stop,
                );
            })
            .context("failed to spawn camera grabber thread")?;

        Ok(Self {
            latest,
            stop,
            width,
            height,
        })
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl FrameSource for FfmpegGrabber {
    fn latest(&self) -> Option<RgbImage> {
        self.latest
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|f| f.image.clone()))
    }

    fn latest_frame(&self) -> Option<CameraFrame> {
        self.latest.lock().ok().and_then(|g| g.clone())
    }
}

impl Drop for FfmpegGrabber {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// Background loop: (re)spawns FFmpeg and reads fixed-size RGB frames until
/// stopped. Reconnects with a short backoff when the stream drops.
fn reader_loop(
    rtsp: &str,
    transport: Option<&str>,
    width: u32,
    height: u32,
    latest: Arc<Mutex<Option<CameraFrame>>>,
    stop: Arc<AtomicBool>,
) {
    let frame_size = (width as usize) * (height as usize) * 3;
    let bin = match ffmpeg() {
        Ok(b) => b,
        Err(_) => return,
    };

    while !stop.load(Ordering::SeqCst) {
        let mut child = match std::process::Command::new(&bin)
            .args([
                "-nostdin",
                "-loglevel",
                "error",
                "-rtsp_transport",
                rtsp_transport(transport),
                "-i",
                rtsp,
                "-an",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "pipe:1",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => {
                std::thread::sleep(std::time::Duration::from_secs(2));
                continue;
            }
        };

        if let Some(mut out) = child.stdout.take() {
            let mut buf = vec![0u8; frame_size];
            loop {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                match out.read_exact(&mut buf) {
                    Ok(()) => {
                        if let Some(img) = RgbImage::from_raw(width, height, buf.clone()) {
                            if let Ok(mut slot) = latest.lock() {
                                *slot = Some(CameraFrame {
                                    image: img,
                                    captured_at: SystemTime::now(),
                                });
                            }
                        }
                    }
                    Err(_) => break, // EOF / stream dropped → reconnect
                }
            }
        }

        let _ = child.kill();
        let _ = child.wait();

        if stop.load(Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}
