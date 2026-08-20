use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::hardware::protocol::{
    cmd_servo_click, cmd_servo_press, cmd_servo_release, cmd_servo_repeat_start,
    cmd_servo_repeat_stop,
};
use crate::hardware::transport::SerialTransport;
use crate::hardware::traits::ServoControl;
use crate::hardware::types::ActionResult;

pub struct ServoService {
    transport: Arc<Mutex<SerialTransport>>,
}

impl ServoService {
    pub fn new(transport: Arc<Mutex<SerialTransport>>) -> Self {
        Self { transport }
    }
}

impl ServoControl for ServoService {
    fn press(&self, channel: u8) -> Result<ActionResult> {
        let start = Instant::now();
        let mut transport = self.transport.lock().unwrap();
        let resp = transport.request(
            &cmd_servo_press(channel),
            |line| line.kind == "servo" || line.kind == "ok",
            3.0,
        )?;

        Ok(ActionResult {
            action: "servo.press".to_string(),
            channel: Some(channel),
            completed: resp.get_str("status") != Some("error"),
            duration_ms: start.elapsed().as_millis() as u64,
            message: resp.get_str("message").map(|s| s.to_string()),
        })
    }

    fn release(&self, channel: u8) -> Result<ActionResult> {
        let start = Instant::now();
        let mut transport = self.transport.lock().unwrap();
        let resp = transport.request(
            &cmd_servo_release(channel),
            |line| line.kind == "servo" || line.kind == "ok",
            3.0,
        )?;

        Ok(ActionResult {
            action: "servo.release".to_string(),
            channel: Some(channel),
            completed: resp.get_str("status") != Some("error"),
            duration_ms: start.elapsed().as_millis() as u64,
            message: resp.get_str("message").map(|s| s.to_string()),
        })
    }

    fn rotate(&self, channel: u8, angle: i32, speed: u32) -> Result<ActionResult> {
        let start = Instant::now();
        let mut transport = self.transport.lock().unwrap();
        let resp = transport.request(
            &crate::hardware::protocol::cmd_servo_rotate(channel, angle, speed),
            |line| line.kind == "servo" || line.kind == "ok",
            3.0,
        )?;

        Ok(ActionResult {
            action: "servo.rotate".to_string(),
            channel: Some(channel),
            completed: resp.get_str("status") != Some("error"),
            duration_ms: start.elapsed().as_millis() as u64,
            message: resp.get_str("message").map(|s| s.to_string()),
        })
    }

    fn click(&self, channel: u8, hold_duration_ms: Option<u64>) -> Result<ActionResult> {
        let start = Instant::now();
        let mut transport = self.transport.lock().unwrap();
        let timeout_s = 3.0 + (hold_duration_ms.unwrap_or(300) as f64 / 1000.0);

        let resp = transport.request(
            &cmd_servo_click(channel, hold_duration_ms),
            |line| line.kind == "servo" || line.kind == "ok",
            timeout_s,
        )?;

        Ok(ActionResult {
            action: "servo.click".to_string(),
            channel: Some(channel),
            completed: resp.get_str("status") != Some("error"),
            duration_ms: start.elapsed().as_millis() as u64,
            message: resp.get_str("message").map(|s| s.to_string()),
        })
    }

    fn repeat(
        &self,
        channel: u8,
        count: u32,
        press_ms: u64,
        release_ms: u64,
    ) -> Result<ActionResult> {
        let start = Instant::now();
        for _ in 0..count {
            self.press(channel)?;
            std::thread::sleep(std::time::Duration::from_millis(press_ms));
            self.release(channel)?;
            std::thread::sleep(std::time::Duration::from_millis(release_ms));
        }
        Ok(ActionResult {
            action: "servo.repeat".to_string(),
            channel: Some(channel),
            completed: true,
            duration_ms: start.elapsed().as_millis() as u64,
            message: None,
        })
    }

    fn start_repeat(&self, channel: u8, period_ms: u64) -> Result<ActionResult> {
        let start = Instant::now();
        let mut transport = self.transport.lock().unwrap();
        let resp = transport.request(
            &cmd_servo_repeat_start(channel, period_ms),
            |line| line.kind == "servo" || line.kind == "ok",
            3.0,
        )?;

        Ok(ActionResult {
            action: "servo.repeat_start".to_string(),
            channel: Some(channel),
            completed: resp.get_str("status") != Some("error"),
            duration_ms: start.elapsed().as_millis() as u64,
            message: resp.get_str("message").map(|s| s.to_string()),
        })
    }

    fn stop_repeat(&self, channel: u8) -> Result<ActionResult> {
        let start = Instant::now();
        let mut transport = self.transport.lock().unwrap();
        let resp = transport.request(
            &cmd_servo_repeat_stop(channel),
            |line| line.kind == "servo" || line.kind == "ok",
            3.0,
        )?;

        Ok(ActionResult {
            action: "servo.repeat_stop".to_string(),
            channel: Some(channel),
            completed: resp.get_str("status") != Some("error"),
            duration_ms: start.elapsed().as_millis() as u64,
            message: resp.get_str("message").map(|s| s.to_string()),
        })
    }

    fn release_all(&self) -> Result<ActionResult> {
        let start = Instant::now();
        let mut transport = self.transport.lock().unwrap();
        let resp = transport.request(
            &crate::hardware::protocol::cmd_servo_release_all(),
            |line| line.kind == "servo" || line.kind == "ok",
            3.0,
        )?;

        Ok(ActionResult {
            action: "servo.release_all".to_string(),
            channel: None,
            completed: true,
            duration_ms: start.elapsed().as_millis() as u64,
            message: None,
        })
    }

    fn set_config(
        &self,
        channel: u8,
        press_angle: u8,
        release_angle: u8,
        press_ms: u16,
        release_ms: u16,
        hold_ms: u16,
    ) -> Result<ActionResult> {
        let start = Instant::now();
        let mut transport = self.transport.lock().unwrap();
        let port_name = transport.port_name.clone().unwrap_or_default();
        let _resp = transport.request(
            &crate::hardware::protocol::cmd_servo_config(
                channel,
                press_angle,
                release_angle,
                press_ms,
                release_ms,
                hold_ms,
            ),
            |line| line.kind == "servo" || line.kind == "ok",
            3.0,
        )?;

        record_servo_configured(
            &port_name,
            channel,
            press_angle,
            release_angle,
            press_ms,
            release_ms,
            hold_ms,
        );

        Ok(ActionResult {
            action: "servo.set_config".to_string(),
            channel: Some(channel),
            completed: true,
            duration_ms: start.elapsed().as_millis() as u64,
            message: None,
        })
    }

    fn get_state(&self, channel: u8) -> Result<String> {
        let mut transport = self.transport.lock().unwrap();
        let resp = transport.request(
            &format!("servo state? {}\n", channel),
            |line| line.kind == "servo" || line.kind == "ok",
            3.0,
        )?;
        let state = resp.get_str("state").unwrap_or("UNKNOWN");
        let angle = resp.get_str("angle").unwrap_or("--");
        Ok(format!("{} @ {}°", state, angle))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CachedServoConfig {
    pub press_angle: u8,
    pub release_angle: u8,
    pub press_ms: u16,
    pub release_ms: u16,
    pub hold_ms: u16,
}

fn get_servo_cache_path() -> std::path::PathBuf {
    std::env::temp_dir().join(".lumi_servo_config_cache.json")
}

pub fn is_servo_already_configured(
    port: &str,
    channel: u8,
    press_angle: u8,
    release_angle: u8,
    press_ms: u16,
    release_ms: u16,
    hold_ms: u16,
) -> bool {
    let cache_path = get_servo_cache_path();
    if let Ok(content) = std::fs::read_to_string(&cache_path) {
        if let Ok(map) = serde_json::from_str::<std::collections::HashMap<String, CachedServoConfig>>(&content) {
            let key = format!("{}:{}", port, channel);
            if let Some(cached) = map.get(&key) {
                return cached.press_angle == press_angle
                    && cached.release_angle == release_angle
                    && cached.press_ms == press_ms
                    && cached.release_ms == release_ms
                    && cached.hold_ms == hold_ms;
            }
        }
    }
    false
}

pub fn record_servo_configured(
    port: &str,
    channel: u8,
    press_angle: u8,
    release_angle: u8,
    press_ms: u16,
    release_ms: u16,
    hold_ms: u16,
) {
    let cache_path = get_servo_cache_path();
    let mut map: std::collections::HashMap<String, CachedServoConfig> = std::fs::read_to_string(&cache_path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default();

    let key = format!("{}:{}", port, channel);
    map.insert(
        key,
        CachedServoConfig {
            press_angle,
            release_angle,
            press_ms,
            release_ms,
            hold_ms,
        },
    );

    if let Ok(serialized) = serde_json::to_string(&map) {
        let _ = std::fs::write(&cache_path, serialized);
    }
}

