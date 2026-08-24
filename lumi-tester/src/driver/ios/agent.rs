//! Client for the `lm-ios-tester` on-device agent (a minimal custom XCUITest target,
//! see `/lm-ios-tester/` at the repo root) - the iOS analogue of `lm-android-tester`.
//!
//! Protocol: persistent TCP connection, one JSON object per line in, one per line out
//! (`{"cmd":"tap","x":100.0,"y":200.0}` -> `{"cmd":"tap","success":true}`), matching the
//! shape already established for the Android agent client
//! (`src/driver/android/driver.rs::send_mirror_request_raw`).

use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

pub const DEFAULT_AGENT_PORT: u16 = 8110;

pub struct AgentClient {
    host: String,
    port: u16,
    stream: Arc<Mutex<Option<TcpStream>>>,
}

impl AgentClient {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
            stream: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn is_ready(&self) -> bool {
        self.send(&serde_json::json!({"cmd": "status"}))
            .await
            .as_ref()
            .map(Self::ok)
            .unwrap_or(false)
    }

    async fn send(&self, req: &serde_json::Value) -> Option<serde_json::Value> {
        let mut line = serde_json::to_vec(req).ok()?;
        line.push(b'\n');

        let mut stream_guard = self.stream.lock().await;
        if stream_guard.is_none() {
            let addr = format!("{}:{}", self.host, self.port);
            let stream = tokio::time::timeout(
                Duration::from_millis(1000),
                TcpStream::connect(&addr),
            )
            .await
            .ok()?
            .ok()?;
            *stream_guard = Some(stream);
        }
        let stream = stream_guard.as_mut()?;

        if tokio::time::timeout(Duration::from_millis(3000), stream.write_all(&line))
            .await
            .ok()?
            .is_err()
        {
            *stream_guard = None;
            return None;
        }

        let mut buf: Vec<u8> = Vec::with_capacity(64 * 1024);
        let mut chunk = [0u8; 64 * 1024];
        loop {
            let read = tokio::time::timeout(Duration::from_millis(8000), stream.read(&mut chunk)).await;
            let n = match read {
                Ok(Ok(n)) => n,
                _ => {
                    *stream_guard = None;
                    return None;
                }
            };
            if n == 0 {
                *stream_guard = None;
                return None;
            }
            buf.extend_from_slice(&chunk[..n]);
            if buf.ends_with(b"\n") {
                break;
            }
            if buf.len() > 64 * 1024 * 1024 {
                *stream_guard = None;
                return None;
            }
        }

        let text = std::str::from_utf8(&buf).ok()?.trim();
        serde_json::from_str(text).ok()
    }

    fn ok(resp: &serde_json::Value) -> bool {
        resp.get("success").and_then(|v| v.as_bool()).unwrap_or(false)
    }

    pub async fn tap(&self, x: f64, y: f64) -> bool {
        self.send(&serde_json::json!({"cmd": "tap", "x": x, "y": y}))
            .await
            .as_ref()
            .map(Self::ok)
            .unwrap_or(false)
    }

    pub async fn long_press(&self, x: f64, y: f64, duration_ms: u64) -> bool {
        self.send(&serde_json::json!({"cmd": "long_press", "x": x, "y": y, "duration_ms": duration_ms}))
            .await
            .as_ref()
            .map(Self::ok)
            .unwrap_or(false)
    }

    pub async fn double_tap(&self, x: f64, y: f64) -> bool {
        self.send(&serde_json::json!({"cmd": "double_tap", "x": x, "y": y}))
            .await
            .as_ref()
            .map(Self::ok)
            .unwrap_or(false)
    }

    pub async fn swipe(&self, x1: f64, y1: f64, x2: f64, y2: f64, duration_ms: u64) -> bool {
        self.send(&serde_json::json!({
            "cmd": "swipe", "x1": x1, "y1": y1, "x2": x2, "y2": y2, "duration_ms": duration_ms
        }))
        .await
        .as_ref()
        .map(Self::ok)
        .unwrap_or(false)
    }

    pub async fn type_text(&self, text: &str) -> bool {
        self.send(&serde_json::json!({"cmd": "type_text", "text": text}))
            .await
            .as_ref()
            .map(Self::ok)
            .unwrap_or(false)
    }

    pub async fn erase_text(&self, count: u32) -> bool {
        self.send(&serde_json::json!({"cmd": "erase_text", "count": count}))
            .await
            .as_ref()
            .map(Self::ok)
            .unwrap_or(false)
    }

    /// Only RETURN/ENTER, DELETE/BACKSPACE, TAB, ESCAPE are implemented on-device -
    /// see `LumiCommandHandler.m`'s `press_key` dispatch. Anything else returns `false`.
    pub async fn press_key(&self, key: &str) -> bool {
        self.send(&serde_json::json!({"cmd": "press_key", "key": key}))
            .await
            .as_ref()
            .map(Self::ok)
            .unwrap_or(false)
    }

    /// Only "home"/"volumeup"/"volumedown" are implemented on-device.
    pub async fn press_button(&self, name: &str) -> bool {
        self.send(&serde_json::json!({"cmd": "press_button", "name": name}))
            .await
            .as_ref()
            .map(Self::ok)
            .unwrap_or(false)
    }

    /// `[XCUIApplication terminate]`+`launch` (native XCTest, not idb). `idb
    /// terminate`/`idb launch` were confirmed broken on this iOS 26.5.2 device (idb
    /// silently no-ops on terminate, and launch fails on a DeveloperDiskImage version
    /// mismatch) - the app process was never actually being restarted despite every
    /// `launchApp` reporting success, which left the agent's `XCUIApplication` window
    /// resolution permanently stuck on a stale window. Returns whether the app reached
    /// the running-foreground state within the agent's own wait budget.
    pub async fn launch_app(&self, bundle_id: &str) -> bool {
        let resp = self
            .send(&serde_json::json!({"cmd": "launch_app", "bundleId": bundle_id}))
            .await;
        match resp {
            Some(r) if Self::ok(&r) => r
                .get("data")
                .and_then(|d| d.get("cameUp"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            _ => false,
        }
    }

    pub async fn terminate_app(&self, bundle_id: &str) -> bool {
        self.send(&serde_json::json!({"cmd": "terminate_app", "bundleId": bundle_id}))
            .await
            .as_ref()
            .map(Self::ok)
            .unwrap_or(false)
    }

    /// Real-device location simulation via `XCTRunnerDaemonSession` (the same private
    /// XCTest daemon RPC WebDriverAgent's `fb_setSimulatedLocation:` uses - see
    /// `LumiPrivateXCTest.h`). Requires iOS 16.4+; fails cleanly (not silently) on
    /// unsupported devices/OS versions.
    pub async fn set_location(&self, lat: f64, lon: f64, alt: f64) -> bool {
        self.send(&serde_json::json!({"cmd": "set_location", "lat": lat, "lon": lon, "alt": alt}))
            .await
            .as_ref()
            .map(Self::ok)
            .unwrap_or(false)
    }

    pub async fn clear_location(&self) -> bool {
        self.send(&serde_json::json!({"cmd": "clear_location"}))
            .await
            .as_ref()
            .map(Self::ok)
            .unwrap_or(false)
    }

    /// Returns the raw hierarchy JSON value (schema: type/label/identifier/value/
    /// placeholder/frame{x,y,width,height}/enabled/visible/children) - a strict subset of
    /// `accessibility::IosElement`'s primary (non-alias) field names, so it can be handed
    /// straight to `accessibility::parse_ui_hierarchy` after `serde_json::to_string`.
    pub async fn hierarchy(&self, bundle_id: &str) -> Option<serde_json::Value> {
        let resp = self
            .send(&serde_json::json!({"cmd": "hierarchy", "bundleId": bundle_id}))
            .await?;
        if Self::ok(&resp) {
            resp.get("data").cloned()
        } else {
            None
        }
    }

    pub async fn screenshot_base64(&self) -> Option<String> {
        let resp = self.send(&serde_json::json!({"cmd": "screenshot"})).await?;
        if Self::ok(&resp) {
            resp.get("data").and_then(|d| d.as_str()).map(|s| s.to_string())
        } else {
            None
        }
    }

    pub async fn get_screen_size(&self) -> Option<(u32, u32)> {
        let resp = self.send(&serde_json::json!({"cmd": "get_screen_size"})).await?;
        if !Self::ok(&resp) {
            return None;
        }
        let data = resp.get("data")?;
        let w = data.get("width")?.as_f64()? as u32;
        let h = data.get("height")?.as_f64()? as u32;
        Some((w, h))
    }

    pub async fn set_orientation(&self, mode: &str) -> bool {
        self.send(&serde_json::json!({"cmd": "set_orientation", "mode": mode}))
            .await
            .as_ref()
            .map(Self::ok)
            .unwrap_or(false)
    }

    pub async fn get_orientation(&self) -> Option<String> {
        let resp = self.send(&serde_json::json!({"cmd": "get_orientation"})).await?;
        if Self::ok(&resp) {
            resp.get("data").and_then(|d| d.as_str()).map(|s| s.to_string())
        } else {
            None
        }
    }
}
