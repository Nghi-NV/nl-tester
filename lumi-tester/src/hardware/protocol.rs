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
        let mut trimmed = line.trim();
        if trimmed.is_empty() {
            anyhow::bail!("Empty line");
        }

        // Strip address prefix e.g. "@1 [SERVO] ..." or "@7 [SYSTEM] ..."
        if trimmed.starts_with('@') {
            if let Some(pos) = trimmed.find('[') {
                trimmed = &trimmed[pos..];
            } else if let Some((_, rest)) = trimmed.split_once(' ') {
                trimmed = rest.trim();
            }
        }

        if !trimmed.starts_with('[') {
            let mut fields = HashMap::new();
            if trimmed.eq_ignore_ascii_case("ok") {
                fields.insert("status".to_string(), "ok".to_string());
                return Ok(Self {
                    kind: "ok".to_string(),
                    fields,
                    raw: trimmed.to_string(),
                });
            }
            if trimmed.eq_ignore_ascii_case("ready") || trimmed.to_lowercase().contains("ready") {
                fields.insert("status".to_string(), "ready".to_string());
                return Ok(Self {
                    kind: "system".to_string(),
                    fields,
                    raw: trimmed.to_string(),
                });
            }
            for token in trimmed.split_whitespace() {
                if let Some((k, v)) = token.split_once('=') {
                    fields.insert(k.to_lowercase(), v.trim_matches(',').to_string());
                }
            }
            fields.insert("message".to_string(), trimmed.to_string());
            return Ok(Self {
                kind: "info".to_string(),
                fields,
                raw: trimmed.to_string(),
            });
        }

        let end_tag = trimmed
            .find(']')
            .ok_or_else(|| anyhow!("Line missing closing tag bracket: {}", line))?;

        let kind = trimmed[1..end_tag].to_lowercase();
        let payload = &trimmed[end_tag + 1..].trim();

        let mut fields = HashMap::new();

        if trimmed.contains('|') && !trimmed.contains("---") && !trimmed.contains("t_ms |") {
            let parts: Vec<&str> = trimmed.split('|').map(|s| s.trim()).collect();
            match kind.as_str() {
                "color" => {
                    let keys = [
                        "prefix", "t_ms", "clear", "red", "green", "blue",
                        "rn", "gn", "bn", "color", "conf", "signal", "stable", "s_conf",
                    ];
                    for (idx, key) in keys.iter().enumerate() {
                        if idx < parts.len() {
                            fields.insert(key.to_string(), parts[idx].to_string());
                        }
                    }
                }
                "bright" | "brightness" => {
                    let keys = [
                        "prefix", "t_ms", "clear", "brightness_pct", "base",
                        "off_th", "on_th", "signal", "red", "green", "blue",
                    ];
                    for (idx, key) in keys.iter().enumerate() {
                        if idx < parts.len() {
                            fields.insert(key.to_string(), parts[idx].to_string());
                        }
                    }
                }
                "blink" | "blink_event" => {
                    let keys = [
                        "prefix", "t_ms", "interval_ms", "burst", "off_ms",
                        "clear", "brightness_pct", "base", "signal",
                    ];
                    for (idx, key) in keys.iter().enumerate() {
                        if idx < parts.len() {
                            fields.insert(key.to_string(), parts[idx].to_string());
                        }
                    }
                    if let Some(burst) = fields.get("burst") {
                        fields.insert("count".to_string(), burst.clone());
                    }
                }
                "cct" => {
                    let keys = [
                        "prefix", "t_ms", "raw_k", "cct_k", "status",
                        "cal", "clear", "red", "green", "blue",
                    ];
                    for (idx, key) in keys.iter().enumerate() {
                        if idx < parts.len() {
                            fields.insert(key.to_string(), parts[idx].to_string());
                        }
                    }
                }
                _ => {}
            }
        }

        for token in payload.split_whitespace() {
            if let Some((k, v)) = token.split_once('=') {
                fields.insert(k.to_lowercase(), v.trim_matches(',').to_string());
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
pub fn cmd_ping(node_id: Option<u8>) -> String {
    if let Some(id) = node_id {
        format!("@{} ping\n", id)
    } else {
        "ping\n".to_string()
    }
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

pub fn cmd_servo_rotate(channel: u8, angle: i32, speed: u32) -> String {
    format!("servo rotate {} {} {}\n", channel, angle, speed)
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
    let eid = after_event_id.unwrap_or(0);
    format!("color blink? {} {}\n", channel, eid)
}

pub fn cmd_color_blink_cursor(channel: u8) -> String {
    format!("color blink_cursor? {}\n", channel)
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

pub fn cmd_mode(mode: &str) -> String {
    format!("mode {}\n", mode.trim().to_lowercase())
}

pub fn cmd_color_select(channel: u8) -> String {
    format!("color select {}\n", channel)
}

pub fn cmd_color_light(channel: u8, enabled: bool) -> String {
    format!("color light {} {}\n", channel, if enabled { "on" } else { "off" })
}

pub fn cmd_color_light_state(channel: u8) -> String {
    format!("color light? {}\n", channel)
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

pub fn cmd_calibration_brightness(channel: u8, mode: &str, color: Option<&str>) -> String {
    if let Some(c) = color {
        format!("calibration brightness {} {} {}\n", channel, mode, c)
    } else {
        format!("calibration brightness {} {}\n", channel, mode)
    }
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

    #[test]
    fn test_parse_addressed_line() {
        let line = "@1 [SERVO] status=completed action=click channel=1 hold_ms=300";
        let parsed = ResponseLine::parse(line).unwrap();
        assert_eq!(parsed.kind, "servo");
        assert_eq!(parsed.get_str("status"), Some("completed"));

        let line7 = "@7 [SYSTEM] status=ready firmware=0.4.0";
        let parsed7 = ResponseLine::parse(line7).unwrap();
        assert_eq!(parsed7.kind, "system");
        assert_eq!(parsed7.get_str("status"), Some("ready"));
    }
}
