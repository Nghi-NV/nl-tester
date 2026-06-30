//! Warm camera session used by the test runner.
//!
//! Holds a continuously-decoding stream plus the loaded profile so that every
//! device-state command in a flow reuses one RTSP connection (the handshake is
//! slow, ~1-2s, and reconnecting per command would miss LED transitions).

use anyhow::{anyhow, Result};
use image::RgbImage;
use std::time::Duration;

use crate::camera::detect::{self, DeviceState};
use crate::camera::pattern::{self, BlinkPattern, PatternMatch, PatternSample};
use crate::camera::profile::CameraProfile;
use crate::camera::stream::{CameraFrame, FfmpegGrabber, FrameSource};

const POLL_INTERVAL_MS: u64 = 150;
const DEFAULT_TIMEOUT_MS: u64 = 8000;
const DEFAULT_STABLE_FRAMES: u32 = 3;
const MAX_FRAME_AGE_MS: u128 = 3000;

pub struct CameraSession {
    grabber: FfmpegGrabber,
    profile: CameraProfile,
}

pub struct CameraEvidence {
    pub raw: RgbImage,
    pub warped: RgbImage,
    pub annotated: RgbImage,
    pub state: DeviceState,
    pub captured_at: std::time::SystemTime,
}

pub type StateCheckResult = std::result::Result<(), (anyhow::Error, Vec<StateTimelineSample>)>;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateTimelineSample {
    pub elapsed_ms: u64,
    pub captured_at_epoch_ms: u128,
    pub state: String,
    pub status: detect::DetectionStatus,
    pub confidence: f32,
    pub margin: f32,
    pub second_best: Option<detect::StateCandidate>,
}

impl CameraSession {
    /// Connect, probe and start the warm stream. Blocking (RTSP handshake).
    pub fn start(rtsp: &str, transport: Option<&str>, profile: CameraProfile) -> Result<Self> {
        let grabber = FfmpegGrabber::start(rtsp, transport)?;
        Ok(Self { grabber, profile })
    }

    pub fn profile(&self) -> &CameraProfile {
        &self.profile
    }

    fn camera_frame(&self) -> Result<CameraFrame> {
        let frame = self
            .grabber
            .latest_frame()
            .ok_or_else(|| anyhow!("no camera frame available yet"))?;
        let age_ms = frame
            .captured_at
            .elapsed()
            .map(|d| d.as_millis())
            .unwrap_or(MAX_FRAME_AGE_MS + 1);
        if age_ms > MAX_FRAME_AGE_MS {
            anyhow::bail!(
                "camera frame is stale ({}ms old); check RTSP connection or camera stream",
                age_ms
            );
        }
        Ok(frame)
    }

    /// Read all button states from the current frame.
    pub fn read(&self) -> Result<DeviceState> {
        let frame = self.camera_frame()?;
        Ok(detect::read_device(&frame.image, &self.profile))
    }

    /// Current state of a single button.
    pub fn current(&self, button: &str) -> Result<String> {
        let state = self.read()?;
        state
            .get(button)
            .map(|b| b.state.clone())
            .ok_or_else(|| unknown_button(button, &state))
    }

    /// Assert a button equals `expect` right now.
    pub fn assert_state(&self, button: &str, expect: &str) -> Result<()> {
        let actual = self.current(button)?;
        if eq(&actual, expect) {
            Ok(())
        } else {
            Err(anyhow!(
                "device check failed: button '{}' is '{}', expected '{}'",
                button,
                actual,
                expect
            ))
        }
    }

    pub fn assert_state_with_timeline(&self, button: &str, expect: &str) -> StateCheckResult {
        match self.sample_state(button, std::time::SystemTime::now(), 0) {
            Ok((actual, _sample)) if eq(&actual, expect) => Ok(()),
            Ok((actual, sample)) => Err((
                anyhow!(
                    "device check failed: button '{}' is '{}', expected '{}'",
                    button,
                    actual,
                    expect
                ),
                vec![sample],
            )),
            Err(error) => Err((error, Vec::new())),
        }
    }

    /// Poll until `button` reaches `expect` for `stable_frames` consecutive
    /// reads, or the timeout elapses.
    pub async fn wait_state(
        &self,
        button: &str,
        expect: &str,
        timeout_ms: Option<u64>,
        stable_frames: Option<u32>,
    ) -> Result<()> {
        self.wait_state_with_timeline(button, expect, timeout_ms, stable_frames)
            .await
            .map_err(|(error, _)| error)
    }

    pub async fn wait_state_with_timeline(
        &self,
        button: &str,
        expect: &str,
        timeout_ms: Option<u64>,
        stable_frames: Option<u32>,
    ) -> StateCheckResult {
        let deadline = std::time::Instant::now()
            + Duration::from_millis(timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));
        let start = std::time::SystemTime::now();
        let need = stable_frames.unwrap_or(DEFAULT_STABLE_FRAMES).max(1);
        let mut streak = 0u32;
        let mut timeline = Vec::new();

        loop {
            match self.sample_state(button, start, 0) {
                Ok((state_name, sample)) => {
                    timeline.push(sample);
                    keep_recent_timeline(&mut timeline, 3_000);
                    if eq(&state_name, expect) {
                        streak += 1;
                        if streak >= need {
                            return Ok(());
                        }
                    } else {
                        streak = 0;
                    }
                }
                Err(error) => return Err((error, timeline)),
            }

            if std::time::Instant::now() >= deadline {
                let last = timeline
                    .last()
                    .map(|sample| sample.state.as_str())
                    .unwrap_or("UNKNOWN");
                return Err((
                    anyhow!(
                        "timed out waiting for button '{}' to become '{}' (last seen '{}'); \
                         device offline or camera misaligned?",
                        button,
                        expect,
                        last
                    ),
                    timeline,
                ));
            }
            tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
        }
    }

    /// Verify a button starts at `from`, then changes to `to` within the
    /// timeout. Catches false passes where the LED was already in the target
    /// state before the triggering action.
    pub async fn assert_transition(
        &self,
        button: &str,
        from: &str,
        to: &str,
        timeout_ms: Option<u64>,
        stable_frames: Option<u32>,
    ) -> Result<()> {
        self.assert_transition_with_timeline(button, from, to, timeout_ms, stable_frames)
            .await
            .map_err(|(error, _)| error)
    }

    pub async fn assert_transition_with_timeline(
        &self,
        button: &str,
        from: &str,
        to: &str,
        timeout_ms: Option<u64>,
        stable_frames: Option<u32>,
    ) -> StateCheckResult {
        let start_time = std::time::SystemTime::now();
        let (start, sample) = match self.sample_state(button, start_time, 0) {
            Ok(sample) => sample,
            Err(error) => return Err((error, Vec::new())),
        };
        if !eq(&start, from) {
            return Err((
                anyhow!(
                    "transition precondition failed: button '{}' started at '{}', expected '{}'",
                    button,
                    start,
                    from
                ),
                vec![sample],
            ));
        }
        match self
            .wait_state_with_timeline(button, to, timeout_ms, stable_frames)
            .await
        {
            Ok(()) => Ok(()),
            Err((error, mut timeline)) => {
                timeline.insert(0, sample);
                keep_recent_timeline(&mut timeline, 3_000);
                Err((error, timeline))
            }
        }
    }

    /// Observe a region timeline and wait for a blink/reset pattern.
    pub async fn wait_blink_pattern(
        &self,
        button: &str,
        pattern: BlinkPattern,
        timeout_ms: Option<u64>,
        sample_ms: u64,
    ) -> Result<PatternMatch> {
        let result = self
            .observe_blink_pattern(button, pattern.clone(), timeout_ms, sample_ms)
            .await?;
        if result.matched {
            return Ok(result);
        }
        Err(anyhow!(
            "timed out waiting for '{}' to blink '{}' {} time(s) within {}ms; observed {} pulse(s)",
            button,
            pattern.state,
            pattern.count,
            pattern.within_ms,
            result.observed_count
        ))
    }

    /// Observe a region timeline and return the detected blink events whether
    /// the expected pattern matched or not.
    pub async fn observe_blink_pattern(
        &self,
        button: &str,
        pattern: BlinkPattern,
        timeout_ms: Option<u64>,
        sample_ms: u64,
    ) -> Result<PatternMatch> {
        let timeout = timeout_ms.unwrap_or(pattern.within_ms);
        let deadline = std::time::Instant::now() + Duration::from_millis(timeout);
        let start = std::time::SystemTime::now();
        let mut samples: Vec<PatternSample> = Vec::new();
        let mut last_captured_at: Option<std::time::SystemTime> = None;

        loop {
            if let Ok(frame) = self.camera_frame() {
                if last_captured_at != Some(frame.captured_at) {
                    last_captured_at = Some(frame.captured_at);
                    let elapsed_ms = frame
                        .captured_at
                        .duration_since(start)
                        .map(|d| d.as_millis().max(0) as u64)
                        .unwrap_or_else(|_| samples.last().map(|s| s.elapsed_ms).unwrap_or(0));
                    let state = detect::read_device(&frame.image, &self.profile);
                    if let Some(reading) = state.get(button) {
                        samples.push(PatternSample {
                            elapsed_ms,
                            state: reading.state.clone(),
                        });
                        let result = pattern::detect_blinks(&samples, &pattern);
                        if result.matched {
                            return Ok(result);
                        }
                    } else {
                        return Err(unknown_button(button, &state));
                    }
                }
            }

            if std::time::Instant::now() >= deadline {
                let result = pattern::detect_blinks(&samples, &pattern);
                return Ok(result);
            }

            tokio::time::sleep(Duration::from_millis(sample_ms.max(10))).await;
        }
    }

    /// Capture a short state timeline for evidence after a failed assertion.
    pub async fn observe_state_timeline(
        &self,
        button: &str,
        duration_ms: u64,
        sample_ms: u64,
    ) -> Result<Vec<StateTimelineSample>> {
        let deadline = std::time::Instant::now() + Duration::from_millis(duration_ms);
        let start = std::time::SystemTime::now();
        let mut samples: Vec<StateTimelineSample> = Vec::new();
        let mut last_captured_at: Option<std::time::SystemTime> = None;

        loop {
            if let Ok(frame) = self.camera_frame() {
                if last_captured_at != Some(frame.captured_at) {
                    last_captured_at = Some(frame.captured_at);
                    let elapsed_ms = frame
                        .captured_at
                        .duration_since(start)
                        .map(|d| d.as_millis().max(0) as u64)
                        .unwrap_or_else(|_| samples.last().map(|s| s.elapsed_ms).unwrap_or(0));
                    let captured_at_epoch_ms = frame
                        .captured_at
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or(0);
                    let state = detect::read_device(&frame.image, &self.profile);
                    if let Some(reading) = state.get(button) {
                        samples.push(StateTimelineSample {
                            elapsed_ms,
                            captured_at_epoch_ms,
                            state: reading.state.clone(),
                            status: reading.status.clone(),
                            confidence: reading.confidence,
                            margin: reading.margin,
                            second_best: reading.second_best.clone(),
                        });
                    } else {
                        return Err(unknown_button(button, &state));
                    }
                }
            }

            if std::time::Instant::now() >= deadline {
                return Ok(samples);
            }

            tokio::time::sleep(Duration::from_millis(sample_ms.max(10))).await;
        }
    }

    /// Produce an annotated warped image (ROIs + detected states) for evidence.
    pub fn annotated(&self) -> Result<(RgbImage, DeviceState)> {
        let evidence = self.evidence()?;
        Ok((evidence.annotated, evidence.state))
    }

    /// Capture raw + warped + annotated frames and the current detected state.
    pub fn evidence(&self) -> Result<CameraEvidence> {
        let frame = self.camera_frame()?;
        let raw = frame.image;
        let warped = detect::warp_device(&raw, &self.profile);
        let state = detect::read_device_warped(&warped, &self.profile);
        let annotated = detect::annotate_warped(&warped, &self.profile, &state);
        Ok(CameraEvidence {
            raw,
            warped,
            annotated,
            state,
            captured_at: frame.captured_at,
        })
    }

    fn sample_state(
        &self,
        button: &str,
        start: std::time::SystemTime,
        fallback_elapsed_ms: u64,
    ) -> Result<(String, StateTimelineSample)> {
        let frame = self.camera_frame()?;
        let elapsed_ms = frame
            .captured_at
            .duration_since(start)
            .map(|d| d.as_millis().max(0) as u64)
            .unwrap_or(fallback_elapsed_ms);
        let captured_at_epoch_ms = frame
            .captured_at
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let state = detect::read_device(&frame.image, &self.profile);
        let Some(reading) = state.get(button) else {
            return Err(unknown_button(button, &state));
        };
        Ok((
            reading.state.clone(),
            StateTimelineSample {
                elapsed_ms,
                captured_at_epoch_ms,
                state: reading.state.clone(),
                status: reading.status.clone(),
                confidence: reading.confidence,
                margin: reading.margin,
                second_best: reading.second_best.clone(),
            },
        ))
    }
}

fn keep_recent_timeline(samples: &mut Vec<StateTimelineSample>, window_ms: u64) {
    let Some(last) = samples.last().map(|sample| sample.elapsed_ms) else {
        return;
    };
    let cutoff = last.saturating_sub(window_ms);
    let drop_count = samples
        .iter()
        .take_while(|sample| sample.elapsed_ms < cutoff)
        .count();
    if drop_count > 0 {
        samples.drain(0..drop_count);
    }
}

fn eq(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

fn unknown_button(button: &str, state: &DeviceState) -> anyhow::Error {
    let known: Vec<String> = state
        .buttons
        .iter()
        .map(|b| match b.id.as_deref() {
            Some(id) if !id.trim().is_empty() => format!("{} ({})", id, b.label),
            _ => b.label.clone(),
        })
        .collect();
    anyhow!(
        "unknown region '{}' in camera profile; available: {}",
        button,
        known.join(", ")
    )
}
