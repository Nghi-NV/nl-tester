use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::hardware::protocol::{cmd_color_blink_query, cmd_color_read};
use crate::hardware::transport::SerialTransport;
use crate::hardware::traits::ColorSensorControl;
use crate::hardware::types::{BlinkResult, Color, ColorConfidence, ColorReading, RawColorSample};

pub struct ColorSensorService {
    transport: Arc<Mutex<SerialTransport>>,
    active_channel: Mutex<Option<u8>>,
}

impl ColorSensorService {
    pub fn new(transport: Arc<Mutex<SerialTransport>>) -> Self {
        Self {
            transport,
            active_channel: Mutex::new(Some(1)), // MUX channel defaults to 1 on boot
        }
    }

    pub fn select_channel(&self, channel: u8) -> Result<()> {
        let mut transport = self.transport.lock().unwrap();
        let mut active = self.active_channel.lock().unwrap();
        if *active != Some(channel) {
            let _ = transport.request(
                &crate::hardware::protocol::cmd_color_select(channel),
                |line| line.kind == "color_status" || line.kind == "color" || line.kind == "ok",
                2.0,
            );
            *active = Some(channel);
        }
        Ok(())
    }

    pub fn set_mode(&self, mode: &str) -> Result<()> {
        let mut transport = self.transport.lock().unwrap();
        let _ = transport.request(
            &crate::hardware::protocol::cmd_mode(mode),
            |line| line.kind == "mode" || line.kind == "ok",
            2.0,
        );
        Ok(())
    }

    pub fn light_on(&self, channel: Option<u8>) -> Result<()> {
        let ch = channel.unwrap_or(1);
        let mut transport = self.transport.lock().unwrap();
        transport.request(
            &crate::hardware::protocol::cmd_color_light(ch, true),
            |line| line.kind == "color_light" || line.kind == "ok",
            3.0,
        )?;
        Ok(())
    }

    pub fn light_off(&self, channel: Option<u8>) -> Result<()> {
        let ch = channel.unwrap_or(1);
        let mut transport = self.transport.lock().unwrap();
        transport.request(
            &crate::hardware::protocol::cmd_color_light(ch, false),
            |line| line.kind == "color_light" || line.kind == "ok",
            3.0,
        )?;
        Ok(())
    }

    pub fn set_thresholds(
        &self,
        channel: u8,
        off_pct: u8,
        on_pct: u8,
        min_ms: u16,
        max_ms: u16,
        end_gap_ms: u16,
    ) -> Result<()> {
        let mut transport = self.transport.lock().unwrap();
        transport.request(
            &crate::hardware::protocol::cmd_color_thresholds(channel, off_pct, on_pct, min_ms, max_ms, end_gap_ms),
            |line| line.kind == "bright_cfg" || line.kind == "ok",
            3.0,
        )?;
        Ok(())
    }
}

impl ColorSensorControl for ColorSensorService {
    fn get_light_state(&self, channel: Option<u8>) -> Result<bool> {
        let ch = channel.unwrap_or(1);
        let mut transport = self.transport.lock().unwrap();
        let resp = transport.request(
            &crate::hardware::protocol::cmd_color_light_state(ch),
            |line| line.kind == "color_light" || line.kind == "ok",
            3.0,
        )?;
        let state = resp.get_str("state").unwrap_or("off");
        Ok(state.eq_ignore_ascii_case("on"))
    }

    fn read_color(&self, channel: u8) -> Result<ColorReading> {
        // 1. Select channel only if not already active to avoid clearing firmware integration buffer
        self.select_channel(channel)?;

        let mut transport = self.transport.lock().unwrap();
        let resp = transport.request(
            &cmd_color_read(channel),
            |line| line.kind == "color" || line.kind == "ok",
            5.0,
        )?;

        let red = resp.get_u16("red").unwrap_or(0);
        let green = resp.get_u16("green").unwrap_or(0);
        let blue = resp.get_u16("blue").unwrap_or(0);
        let clear = resp.get_u16("clear").unwrap_or(0);

        let stable_str = resp.get_str("stable").unwrap_or("").trim();
        let instant_str = resp.get_str("color").unwrap_or("").trim();
        let signal_str = resp.get_str("signal").unwrap_or("").trim();
        let is_off_signal = signal_str.eq_ignore_ascii_case("NO_SIGNAL")
            || signal_str.eq_ignore_ascii_case("DARK");

        // 2. Resolve color string: prefer stable color, fall back to instant color, or Off if NO_SIGNAL / UNKNOWN
        let color = if is_off_signal {
            Color::Off
        } else if !stable_str.is_empty() && !stable_str.eq_ignore_ascii_case("UNKNOWN") {
            Color::from_str(stable_str)
        } else if !instant_str.is_empty() && !instant_str.eq_ignore_ascii_case("UNKNOWN") {
            Color::from_str(instant_str)
        } else if signal_str.eq_ignore_ascii_case("LOW_SIGNAL") {
            Color::Off
        } else if !stable_str.is_empty() {
            Color::from_str(stable_str)
        } else {
            Color::Unknown
        };

        let confidence_str = resp.get_str("s_conf").or_else(|| resp.get_str("conf")).unwrap_or("OK");
        let fw_confidence = match confidence_str.trim().to_uppercase().as_str() {
            "OK" | "GOOD" => ColorConfidence::Ok,
            "LOW_CONFIDENCE" | "POOR" => ColorConfidence::LowConfidence,
            "UNCALIBRATED" => ColorConfidence::Uncalibrated,
            _ => ColorConfidence::Invalid,
        };

        // Smart color resolution: directly calculate from raw optical RGBC physics using HSV color space.
        let (final_color, final_confidence) = if is_off_signal {
            (Color::Off, ColorConfidence::Ok)
        } else {
            classify_rgbc_color(red, green, blue, clear)
        };

        Ok(ColorReading {
            channel,
            color: final_color,
            confidence: final_confidence,
            sample: RawColorSample {
                red,
                green,
                blue,
                clear,
            },
        })
    }

    fn wait_for_color(
        &self,
        channel: u8,
        expected: Option<&[Color]>,
        timeout_s: f64,
    ) -> Result<ColorReading> {
        let start = Instant::now();
        let timeout = Duration::from_secs_f64(timeout_s);

        let mut last_reading: Option<ColorReading> = None;
        loop {
            if start.elapsed() > timeout {
                let exp_str = match expected {
                    Some(colors) => colors.iter().map(|c| c.as_str()).collect::<Vec<_>>().join(", "),
                    None => "any valid color".to_string(),
                };
                if let Some(ref r) = last_reading {
                    anyhow::bail!(
                        "Timeout ({:.1}s) waiting for expected color [{}] on channel {} (current actual: {}, RGBC=[R:{} G:{} B:{} C:{}])",
                        timeout_s,
                        exp_str,
                        channel,
                        r.color.as_str(),
                        r.sample.red,
                        r.sample.green,
                        r.sample.blue,
                        r.sample.clear
                    );
                } else {
                    anyhow::bail!(
                        "Timeout ({:.1}s) waiting for expected color [{}] on channel {}",
                        timeout_s,
                        exp_str,
                        channel
                    );
                }
            }

            match self.read_color(channel) {
                Ok(reading) => {
                    if let Some(exp_colors) = expected {
                        if exp_colors.contains(&reading.color) {
                            return Ok(reading);
                        }
                    } else if reading.color != Color::Unknown {
                        return Ok(reading);
                    }
                    last_reading = Some(reading);
                }
                Err(_) => {}
            }

            thread::sleep(Duration::from_millis(100));
        }
    }

    fn wait_for_blinks(
        &self,
        channel: u8,
        expected_color: Option<&str>,
        expected_count: Option<usize>,
        after_event_id: Option<u32>,
        min_pulse_ms: Option<u64>,
        max_pulse_ms: Option<u64>,
        timeout_s: f64,
    ) -> Result<BlinkResult> {
        let _ = self.select_channel(channel);
        let start = Instant::now();
        let timeout = Duration::from_secs_f64(timeout_s);

        loop {
            if start.elapsed() > timeout {
                anyhow::bail!(
                    "Timeout ({:.1}s) waiting for blink events on channel {}",
                    timeout_s,
                    channel
                );
            }

            {
                let mut transport = self.transport.lock().unwrap();
                if let Ok(resp) = transport.request(
                    &cmd_color_blink_query(channel, after_event_id),
                    |line| line.kind == "blink" || line.kind == "blink_event" || line.kind == "color",
                    2.0,
                ) {
                    let blink_count = resp.get_u32("count").unwrap_or(0) as usize;
                    if blink_count > 0 {
                        let event_id = resp.get_u32("event_id").unwrap_or(0);
                        let color_str = resp.get_str("color");
                        let color = color_str.map(Color::from_str);

                        // Parse durations if present
                        let durations_ms: Vec<u64> = resp.get_str("durations_ms")
                            .unwrap_or("")
                            .split(',')
                            .filter_map(|s| s.trim().parse::<u64>().ok())
                            .collect();

                        // Check filters if specified
                        let color_matched = match (expected_color, color) {
                            (Some(exp), Some(c)) => c.as_str().eq_ignore_ascii_case(exp) || Color::from_str(exp) == c,
                            (Some(_), None) => false,
                            (None, _) => true,
                        };

                        let count_matched = match expected_count {
                            Some(exp_c) => exp_c == blink_count,
                            None => true,
                        };

                        let pulse_matched = if durations_ms.is_empty() {
                            true
                        } else {
                            durations_ms.iter().all(|&d| {
                                let min_ok = min_pulse_ms.map_or(true, |min_v| d >= min_v);
                                let max_ok = max_pulse_ms.map_or(true, |max_v| d <= max_v);
                                min_ok && max_ok
                            })
                        };

                        if color_matched && count_matched && pulse_matched {
                            return Ok(BlinkResult {
                                event_id,
                                blink_count,
                                color,
                                durations_ms,
                            });
                        }
                    }
                }
            }

            thread::sleep(Duration::from_millis(150));
        }
    }
}

/// Smart RGBC Classifier using HSV color space and optical chromaticity
pub fn classify_rgbc_color(r: u16, g: u16, b: u16, c: u16) -> (Color, ColorConfidence) {
    // 1. If clear intensity is extremely low or total RGB is negligible -> Off
    if c < 30 || (r < 15 && g < 15 && b < 15) {
        return (Color::Off, ColorConfidence::Ok);
    }

    let r_f = r as f32;
    let g_f = g as f32;
    let b_f = b as f32;
    let max_val = r_f.max(g_f).max(b_f);
    let min_val = r_f.min(g_f).min(b_f);
    let delta = max_val - min_val;

    if max_val == 0.0 {
        return (Color::Off, ColorConfidence::Ok);
    }

    let sat = delta / max_val;

    // 2. White / Neutral check:
    // If saturation is very low and clear/RGB is reasonably bright -> White
    if sat < 0.12 {
        return (Color::White, ColorConfidence::Ok);
    }

    // 3. Compute Hue in degrees [0, 360)
    let hue = if delta == 0.0 {
        0.0
    } else if (max_val - r_f).abs() < f32::EPSILON {
        let mut h = 60.0 * (((g_f - b_f) / delta) % 6.0);
        if h < 0.0 {
            h += 360.0;
        }
        h
    } else if (max_val - g_f).abs() < f32::EPSILON {
        60.0 * (((b_f - r_f) / delta) + 2.0)
    } else {
        60.0 * (((r_f - g_f) / delta) + 4.0)
    };

    // 4. Map Hue + Saturation to Color
    let color = match hue {
        // Red / Warm Red: 340..360 or 0..28
        h if (340.0..=360.0).contains(&h) || (0.0..28.0).contains(&h) => {
            if sat < 0.20 {
                Color::White
            } else {
                Color::Red
            }
        }
        // Yellow / Amber: 28..65
        h if (28.0..65.0).contains(&h) => {
            if sat < 0.15 {
                Color::White
            } else {
                Color::Yellow
            }
        }
        // Green: 65..170
        h if (65.0..170.0).contains(&h) => {
            if sat < 0.15 {
                Color::White
            } else {
                Color::Green
            }
        }
        // Cyan: 170..205
        h if (170.0..205.0).contains(&h) => {
            if sat < 0.15 {
                Color::White
            } else {
                Color::Cyan
            }
        }
        // Blue: 205..265
        h if (205.0..265.0).contains(&h) => {
            if sat < 0.15 {
                Color::White
            } else {
                Color::Blue
            }
        }
        // Magenta / Purple: 265..315
        h if (265.0..315.0).contains(&h) => {
            if sat < 0.15 {
                Color::White
            } else {
                Color::Magenta
            }
        }
        // Pink / Rose: 315..345
        h if (315.0..345.0).contains(&h) => {
            if sat < 0.15 {
                Color::White
            } else if sat < 0.55 && r_f > b_f {
                Color::Pink
            } else {
                Color::Magenta
            }
        }
        _ => Color::Unknown,
    };

    (color, ColorConfidence::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_blue() {
        // Sample from actual NC2 device reading: [R:74 G:83 B:111 C:219]
        let (color, conf) = classify_rgbc_color(74, 83, 111, 219);
        assert_eq!(color, Color::Blue);
        assert_eq!(conf, ColorConfidence::Ok);
    }

    #[test]
    fn test_classify_off_dark() {
        let (color, conf) = classify_rgbc_color(5, 4, 3, 12);
        assert_eq!(color, Color::Off);
        assert_eq!(conf, ColorConfidence::Ok);
    }

    #[test]
    fn test_classify_white() {
        let (color, _) = classify_rgbc_color(200, 205, 202, 500);
        assert_eq!(color, Color::White);
    }

    #[test]
    fn test_classify_green() {
        let (color, _) = classify_rgbc_color(40, 180, 50, 300);
        assert_eq!(color, Color::Green);
    }

    #[test]
    fn test_classify_red() {
        let (color, _) = classify_rgbc_color(190, 40, 30, 280);
        assert_eq!(color, Color::Red);
    }
}
