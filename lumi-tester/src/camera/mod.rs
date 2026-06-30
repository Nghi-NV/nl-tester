//! Camera-based hardware verification.
//!
//! Uses an RTSP camera to read the LED state of physical devices (smart
//! switches with 1/2/3/4/…/10 buttons, sockets, and other devices) so testers
//! can close the hardware-in-the-loop: an app flow, tester, lab script or robot
//! changes the real device, then the camera verifies the LED state/pattern.
//!
//! Sub-commands (`lumi-tester camera …`):
//! - `calibrate` — web UI to pick the device corners, choose a layout, and
//!   learn the ON/OFF colors by toggling the real device.
//! - `snapshot`  — grab one frame (to aim the camera).
//! - `detect`    — read button states from a saved profile (one-shot or watch).

pub mod detect;
pub mod pattern;
pub mod profile;
pub mod server;
pub mod session;
pub mod stream;

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};

pub use profile::CameraProfile;
pub use session::CameraSession;

/// Redact credentials from an RTSP/URL string before writing logs or reports.
pub fn redact_url(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut cursor = 0;

    while let Some(relative_scheme_end) = raw[cursor..].find("://") {
        let scheme_end = cursor + relative_scheme_end;
        let scheme_start = raw[..scheme_end]
            .rfind(|c: char| c.is_whitespace() || c == '\'' || c == '"' || c == '[' || c == '(')
            .map(|idx| idx + 1)
            .unwrap_or(0);
        if scheme_start < cursor {
            out.push_str(&raw[cursor..scheme_end + 3]);
            cursor = scheme_end + 3;
            continue;
        }

        let authority_start = scheme_end + 3;
        let authority_end = raw[authority_start..]
            .find(['/', '?', '#', ' ', '\t', '\n', '\r', '\'', '"', ']', ')'])
            .map(|idx| authority_start + idx)
            .unwrap_or(raw.len());
        let authority = &raw[authority_start..authority_end];
        let Some(at_idx) = authority.rfind('@') else {
            out.push_str(&raw[cursor..authority_end]);
            cursor = authority_end;
            continue;
        };
        let host = &authority[at_idx + 1..];
        out.push_str(&raw[cursor..scheme_start]);
        out.push_str(&raw[scheme_start..scheme_end]);
        out.push_str("://***@");
        out.push_str(host);
        cursor = authority_end;
    }

    out.push_str(&raw[cursor..]);
    out
}

/// Launch the calibration (or observation) web UI.
pub async fn run_calibrate(
    rtsp: String,
    profile_path: PathBuf,
    port: u16,
    transport: Option<String>,
    observe: bool,
) -> Result<()> {
    let url = format!("http://localhost:{}", port);
    // Open the browser shortly after the server starts.
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        open_browser(&url);
    });

    let server = server::CalibrateServer::new(server::CalibrateConfig {
        rtsp,
        transport,
        profile_path,
        port,
        observe,
    });
    server.start().await
}

/// Capture a single frame to a file (JPEG/PNG by extension).
pub fn run_snapshot(rtsp: &str, out: &Path, transport: Option<&str>) -> Result<()> {
    println!(
        "{} Capturing frame from {}…",
        "📸".blue(),
        redact_url(rtsp).cyan()
    );
    let img = stream::snapshot(rtsp, transport)?;
    img.save(out)
        .with_context(|| format!("failed to write snapshot: {}", out.display()))?;
    println!(
        "{} Saved {}x{} frame to {}",
        "✓".green(),
        img.width(),
        img.height(),
        out.display().to_string().cyan()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::redact_url;

    #[test]
    fn redacts_rtsp_credentials() {
        let raw = format!(
            "{}{}:{}{}10.0.0.5:554/live/ch0",
            "rtsp://", "user", "pass", "@"
        );
        assert_eq!(redact_url(&raw), "rtsp://***@10.0.0.5:554/live/ch0");
    }

    #[test]
    fn leaves_urls_without_credentials_unchanged() {
        assert_eq!(
            redact_url("rtsp://10.0.0.5:554/live/ch0"),
            "rtsp://10.0.0.5:554/live/ch0"
        );
        assert_eq!(redact_url("${CAMERA_RTSP}"), "${CAMERA_RTSP}");
    }

    #[test]
    fn redacts_credentials_inside_error_messages() {
        let raw = format!(
            "ffmpeg failed for [{}{}:{}{}10.0.0.5/live] and {}{}:{}{}10.0.0.6/live",
            "rtsp://", "user", "pass", "@", "rtsp://", "u2", "p2", "@"
        );
        assert_eq!(
            redact_url(&raw),
            "ffmpeg failed for [rtsp://***@10.0.0.5/live] and rtsp://***@10.0.0.6/live"
        );
    }
}

/// Read button states from a saved profile. One-shot prints JSON; `watch`
/// streams transitions until interrupted.
pub async fn run_detect(
    rtsp: Option<String>,
    profile_path: &Path,
    transport: Option<String>,
    watch: bool,
) -> Result<()> {
    let prof = CameraProfile::load(profile_path)?;
    let url = rtsp
        .or_else(|| prof.camera.rtsp.clone())
        .context("no RTSP url provided and none stored in the profile")?;
    let transport = transport.or_else(|| prof.camera.transport.clone());

    let session = CameraSession::start(&url, transport.as_deref(), prof)?;

    if !watch {
        // Allow the warm stream a brief moment, then read once.
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        let state = session.read()?;
        println!("{}", serde_json::to_string_pretty(&state)?);
        return Ok(());
    }

    println!("{} Watching device states (Ctrl+C to stop)…", "🚀".green());
    let mut last: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    loop {
        if let Ok(state) = session.read() {
            for b in &state.buttons {
                let prev = last.get(&b.label).cloned();
                if prev.as_deref() != Some(b.state.as_str()) {
                    let ts = chrono::Local::now().format("%H:%M:%S");
                    println!(
                        "[{}] {} : {} → {} ({}%)",
                        ts.to_string().dimmed(),
                        b.label.cyan(),
                        prev.as_deref().unwrap_or("—"),
                        b.state.bold(),
                        (b.confidence * 100.0) as u32
                    );
                    last.insert(b.label.clone(), b.state.clone());
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

/// Best-effort open the default browser.
fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}
