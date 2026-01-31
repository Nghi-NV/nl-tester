//! WebDriverAgent (WDA) HTTP Client
//!
//! Provides functions to interact with iOS real devices via WebDriverAgent HTTP API.
//! WDA runs on port 8100 by default and uses XCTest framework under the hood.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Default WDA port
pub const DEFAULT_WDA_PORT: u16 = 8100;

/// WDA HTTP Client for real iOS device automation
pub struct WdaClient {
    /// Base URL for WDA (e.g., "http://localhost:8100")
    base_url: String,
    /// HTTP client
    client: reqwest::Client,
    /// Current session ID (created on first use)
    session_id: Option<String>,
}

/// WDA Session response
#[derive(Debug, Deserialize)]
struct SessionResponse {
    value: SessionValue,
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SessionValue {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
}

/// WDA Status response
#[derive(Debug, Deserialize)]
pub struct WdaStatus {
    pub value: WdaStatusValue,
}

#[derive(Debug, Deserialize)]
pub struct WdaStatusValue {
    pub ready: bool,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
}

/// Generic WDA response
#[derive(Debug, Deserialize)]
struct WdaResponse<T> {
    value: T,
}

/// Touch action for tap
#[derive(Debug, Serialize)]
struct TapAction {
    x: f64,
    y: f64,
}

/// Touch and hold action
#[derive(Debug, Serialize)]
struct TouchAndHoldAction {
    x: f64,
    y: f64,
    duration: f64,
}

/// Swipe/drag action
#[derive(Debug, Serialize)]
struct DragAction {
    #[serde(rename = "fromX")]
    from_x: f64,
    #[serde(rename = "fromY")]
    from_y: f64,
    #[serde(rename = "toX")]
    to_x: f64,
    #[serde(rename = "toY")]
    to_y: f64,
    duration: f64,
}

/// Keys input action
#[derive(Debug, Serialize)]
struct KeysAction {
    value: Vec<String>,
}

/// Button press action
#[derive(Debug, Serialize)]
struct ButtonAction {
    name: String,
}

impl WdaClient {
    /// Create a new WDA client
    pub fn new(port: u16) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url: format!("http://localhost:{}", port),
            client,
            session_id: None,
        }
    }

    /// Create WDA client with custom host (for remote devices)
    pub fn with_host(host: &str, port: u16) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url: format!("http://{}:{}", host, port),
            client,
            session_id: None,
        }
    }

    /// Check if WDA is ready
    pub async fn is_ready(&self) -> Result<bool> {
        let url = format!("{}/status", self.base_url);
        match self.client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    if let Ok(status) = resp.json::<WdaStatus>().await {
                        return Ok(status.value.ready);
                    }
                }
                Ok(false)
            }
            Err(_) => Ok(false),
        }
    }

    /// Get or create a session
    pub async fn ensure_session(&mut self) -> Result<String> {
        if let Some(ref session_id) = self.session_id {
            return Ok(session_id.clone());
        }

        // Try to get existing session from status
        let url = format!("{}/status", self.base_url);
        if let Ok(resp) = self.client.get(&url).send().await {
            if let Ok(status) = resp.json::<WdaStatus>().await {
                if let Some(session_id) = status.value.session_id {
                    self.session_id = Some(session_id.clone());
                    return Ok(session_id);
                }
            }
        }

        // Create new session
        let url = format!("{}/session", self.base_url);
        let body = serde_json::json!({
            "capabilities": {}
        });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to create WDA session")?;

        let session_resp: SessionResponse = resp
            .json()
            .await
            .context("Failed to parse session response")?;

        let session_id = session_resp
            .session_id
            .or(session_resp.value.session_id)
            .ok_or_else(|| anyhow::anyhow!("No session ID in response"))?;

        self.session_id = Some(session_id.clone());
        Ok(session_id)
    }

    /// Tap at coordinates
    pub async fn tap(&mut self, x: i32, y: i32) -> Result<()> {
        let session_id = self.ensure_session().await?;
        let url = format!("{}/session/{}/wda/tap/0", self.base_url, session_id);

        let action = TapAction {
            x: x as f64,
            y: y as f64,
        };

        self.client
            .post(&url)
            .json(&action)
            .send()
            .await
            .context("Failed to tap")?;

        Ok(())
    }

    /// Long press (touch and hold)
    pub async fn long_press(&mut self, x: i32, y: i32, duration_ms: u64) -> Result<()> {
        let session_id = self.ensure_session().await?;
        let url = format!("{}/session/{}/wda/touchAndHold", self.base_url, session_id);

        let action = TouchAndHoldAction {
            x: x as f64,
            y: y as f64,
            duration: duration_ms as f64 / 1000.0,
        };

        self.client
            .post(&url)
            .json(&action)
            .send()
            .await
            .context("Failed to long press")?;

        Ok(())
    }

    /// Swipe from one point to another
    pub async fn swipe(
        &mut self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        duration_ms: Option<u64>,
    ) -> Result<()> {
        let session_id = self.ensure_session().await?;
        let url = format!(
            "{}/session/{}/wda/dragFromToForDuration",
            self.base_url, session_id
        );

        let action = DragAction {
            from_x: x1 as f64,
            from_y: y1 as f64,
            to_x: x2 as f64,
            to_y: y2 as f64,
            duration: duration_ms.unwrap_or(300) as f64 / 1000.0,
        };

        self.client
            .post(&url)
            .json(&action)
            .send()
            .await
            .context("Failed to swipe")?;

        Ok(())
    }

    /// Input text
    pub async fn input_text(&mut self, text: &str) -> Result<()> {
        let session_id = self.ensure_session().await?;
        let url = format!("{}/session/{}/wda/keys", self.base_url, session_id);

        // WDA expects each character as a separate string in the array
        let chars: Vec<String> = text.chars().map(|c| c.to_string()).collect();
        let action = KeysAction { value: chars };

        self.client
            .post(&url)
            .json(&action)
            .send()
            .await
            .context("Failed to input text")?;

        Ok(())
    }

    /// Press hardware button (home, volumeUp, volumeDown)
    pub async fn press_button(&mut self, button: &str) -> Result<()> {
        let session_id = self.ensure_session().await?;
        let url = format!("{}/session/{}/wda/pressButton", self.base_url, session_id);

        let action = ButtonAction {
            name: button.to_lowercase(),
        };

        self.client
            .post(&url)
            .json(&action)
            .send()
            .await
            .context("Failed to press button")?;

        Ok(())
    }

    /// Get UI hierarchy (source)
    pub async fn get_source(&mut self) -> Result<String> {
        let session_id = self.ensure_session().await?;
        let url = format!("{}/session/{}/source", self.base_url, session_id);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get source")?;

        let body = resp.text().await?;
        Ok(body)
    }

    /// Take screenshot (returns base64 encoded PNG)
    pub async fn screenshot(&self) -> Result<String> {
        let url = format!("{}/screenshot", self.base_url);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to take screenshot")?;

        let response: WdaResponse<String> = resp.json().await?;
        Ok(response.value)
    }

    /// Save screenshot to file
    pub async fn screenshot_to_file(&self, path: &str) -> Result<()> {
        use base64::Engine;
        let base64_data = self.screenshot().await?;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&base64_data)
            .context("Failed to decode screenshot")?;
        std::fs::write(path, decoded).context("Failed to write screenshot")?;
        Ok(())
    }

    /// Get screen size
    pub async fn get_window_size(&mut self) -> Result<(u32, u32)> {
        let session_id = self.ensure_session().await?;
        let url = format!("{}/session/{}/window/size", self.base_url, session_id);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get window size")?;

        #[derive(Deserialize)]
        struct SizeValue {
            width: u32,
            height: u32,
        }

        let response: WdaResponse<SizeValue> = resp.json().await?;
        Ok((response.value.width, response.value.height))
    }

    /// Double tap
    pub async fn double_tap(&mut self, x: i32, y: i32) -> Result<()> {
        let session_id = self.ensure_session().await?;
        let url = format!("{}/session/{}/wda/doubleTap", self.base_url, session_id);

        let action = TapAction {
            x: x as f64,
            y: y as f64,
        };

        self.client
            .post(&url)
            .json(&action)
            .send()
            .await
            .context("Failed to double tap")?;

        Ok(())
    }

    /// Press keyboard key (for special keys like Return, Delete)
    pub async fn press_key(&mut self, key: &str) -> Result<()> {
        // Map common key names to XCUIKeyboard constants
        let key_value = match key.to_uppercase().as_str() {
            "RETURN" | "ENTER" => "\n",
            "DELETE" | "BACKSPACE" => "\u{8}", // Backspace character
            "TAB" => "\t",
            "ESCAPE" | "ESC" => "\u{1b}",
            _ => key,
        };

        self.input_text(key_value).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = WdaClient::new(8100);
        assert_eq!(client.base_url, "http://localhost:8100");
    }

    #[test]
    fn test_client_with_host() {
        let client = WdaClient::with_host("192.168.1.100", 8100);
        assert_eq!(client.base_url, "http://192.168.1.100:8100");
    }
}
