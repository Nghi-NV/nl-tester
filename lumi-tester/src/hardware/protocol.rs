use anyhow::{anyhow, Result};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ResponseLine {
    pub kind: String,
    pub fields: HashMap<String, String>,
    pub raw: String,
}

impl ResponseLine {
    pub fn parse(line: &str) -> Result<Self> {
        let trimmed = line.trim();
        if !trimmed.starts_with('[') {
            anyhow::bail!("Line does not start with tag bracket: {}", line);
        }

        let end_tag = trimmed
            .find(']')
            .ok_or_else(|| anyhow!("Line missing closing tag bracket: {}", line))?;

        let kind = trimmed[1..end_tag].to_lowercase();
        let payload = &trimmed[end_tag + 1..].trim();

        let mut fields = HashMap::new();
        for token in payload.split_whitespace() {
            if let Some((k, v)) = token.split_once('=') {
                fields.insert(k.to_lowercase(), v.to_string());
            }
        }

        Ok(Self {
            kind,
            fields,
            raw: trimmed.to_string(),
        })
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(|s| s.as_str())
    }

    pub fn get_u32(&self, key: &str) -> Option<u32> {
        self.fields.get(key)?.parse().ok()
    }

    pub fn get_u16(&self, key: &str) -> Option<u16> {
        self.fields.get(key)?.parse().ok()
    }

    pub fn get_u8(&self, key: &str) -> Option<u8> {
        self.fields.get(key)?.parse().ok()
    }
}

// Protocol Command Builders
pub fn cmd_ping() -> String {
    "ping\n".to_string()
}

pub fn cmd_servo_press(channel: u8) -> String {
    format!("servo press {}\n", channel)
}

pub fn cmd_servo_release(channel: u8) -> String {
    format!("servo release {}\n", channel)
}

pub fn cmd_servo_click(channel: u8, hold_ms: Option<u64>) -> String {
    if let Some(h) = hold_ms {
        format!("servo click {} {}\n", channel, h)
    } else {
        format!("servo click {}\n", channel)
    }
}

pub fn cmd_servo_repeat_start(channel: u8, period_ms: u64) -> String {
    format!("servo repeat_start {} {}\n", channel, period_ms)
}

pub fn cmd_servo_repeat_stop(channel: u8) -> String {
    format!("servo repeat_stop {}\n", channel)
}

pub fn cmd_relay_on(channel: u8) -> String {
    format!("relay on {}\n", channel)
}

pub fn cmd_relay_off(channel: u8) -> String {
    format!("relay off {}\n", channel)
}

pub fn cmd_relay_all_off() -> String {
    "relay all_off\n".to_string()
}

pub fn cmd_color_read(channel: u8) -> String {
    format!("color read {}\n", channel)
}

pub fn cmd_color_blink_query(channel: u8, after_event_id: Option<u32>) -> String {
    if let Some(eid) = after_event_id {
        format!("color blink? {} {}\n", channel, eid)
    } else {
        format!("color blink? {}\n", channel)
    }
}

pub fn cmd_servo_config(
    channel: u8,
    press_angle: u8,
    release_angle: u8,
    press_ms: u16,
    release_ms: u16,
    hold_ms: u16,
) -> String {
    format!(
        "servo config {} {} {} {} {} {}\n",
        channel, press_angle, release_angle, press_ms, release_ms, hold_ms
    )
}

pub fn cmd_servo_release_all() -> String {
    "servo release_all\n".to_string()
}

pub fn cmd_color_select(channel: u8) -> String {
    format!("color select {}\n", channel)
}

pub fn cmd_color_light(enabled: bool) -> String {
    format!("color light {}\n", if enabled { "on" } else { "off" })
}

pub fn cmd_color_thresholds(
    channel: u8,
    off_pct: u8,
    on_pct: u8,
    min_ms: u16,
    max_ms: u16,
    end_gap_ms: u16,
) -> String {
    format!(
        "color thresholds {} {} {} {} {} {}\n",
        channel, off_pct, on_pct, min_ms, max_ms, end_gap_ms
    )
}

pub fn cmd_calibration_color(channel: u8, color: &str) -> String {
    format!("calibration color {} {}\n", channel, color)
}

pub fn cmd_calibration_brightness(channel: u8, mode: &str) -> String {
    format!("calibration brightness {} {}\n", channel, mode)
}

pub fn cmd_calibration_cct(known_kelvin: u16) -> String {
    format!("cct_cal {}\n", known_kelvin)
}

pub fn cmd_calibration_action(action: &str) -> String {
    format!("calibration {}\n", action)
}

pub fn cmd_system_diagnostics() -> String {
    "system diagnostics?\n".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_response_line() {
        let line = "[SERVO] status=completed action=click channel=1 hold_ms=300";
        let parsed = ResponseLine::parse(line).unwrap();
        assert_eq!(parsed.kind, "servo");
        assert_eq!(parsed.get_str("status"), Some("completed"));
        assert_eq!(parsed.get_u8("channel"), Some(1));
        assert_eq!(parsed.get_u32("hold_ms"), Some(300));
    }
}
