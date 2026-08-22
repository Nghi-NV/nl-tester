use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::hardware::protocol::cmd_color_read;
use crate::hardware::transport::SerialTransport;
use crate::hardware::traits::ColorSensorControl;
use crate::hardware::types::{BlinkResult, Color, ColorConfidence, ColorReading, PulseDetail, RawColorSample};

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
            0.5,
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

        let fw_color = if is_off_signal {
            Color::Off
        } else if !stable_str.is_empty() && !stable_str.eq_ignore_ascii_case("UNKNOWN") {
            Color::from_str(stable_str)
        } else if !instant_str.is_empty() && !instant_str.eq_ignore_ascii_case("UNKNOWN") {
            Color::from_str(instant_str)
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

        // Smart color resolution: prefer firmware calibrated cluster, fallback to RGBC HSV classifier
        let (final_color, final_confidence) = if is_off_signal {
            (Color::Off, ColorConfidence::Ok)
        } else if fw_color != Color::Unknown {
            (fw_color, fw_confidence)
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
        _after_event_id: Option<u32>,
        min_pulse_ms: Option<u64>,
        max_pulse_ms: Option<u64>,
        timeout_s: f64,
    ) -> Result<BlinkResult> {
        let _ = self.select_channel(channel);
        let start = Instant::now();
        let timeout = Duration::from_secs_f64(timeout_s);
        let exp_color_enum = expected_color.map(Color::from_str);

        let mut sw_blink_count = 0usize;
        let mut in_pulse = false;
        let mut pulse_start = Instant::now();
        let mut pulse_durations = Vec::new();
        let mut pulses = Vec::new();
        let mut peak_sample: Option<RawColorSample> = None;
        let mut peak_delta = (0u16, 0u16, 0u16, 0u16);
        let mut last_detected_color = None;
        let mut last_pulse_end: Option<Instant> = None;
        let mut last_reading: Option<ColorReading> = None;
        let settle_gap = Duration::from_millis(450);

        // Dynamic Ambient Baseline Tracker (auto-adapts to ambient light, dark boxes, and external fixtures)
        let mut r_base: Option<u16> = None;
        let mut g_base: Option<u16> = None;
        let mut b_base: Option<u16> = None;
        let mut c_base: Option<u16> = None;

        loop {
            if start.elapsed() > timeout {
                if let Some(exp_c) = expected_count {
                    if sw_blink_count == exp_c {
                        return Ok(BlinkResult {
                            event_id: 0,
                            blink_count: sw_blink_count,
                            color: last_detected_color,
                            durations_ms: pulse_durations,
                            pulses,
                        });
                    }
                }
                let last_info = match last_reading {
                    Some(ref r) => format!(
                        "last sensor reading: Color={} (Conf={:?}, RGBC=[R:{} G:{} B:{} C:{}], Baseline=[R:{} G:{} B:{} C:{}])",
                        r.color.as_str(),
                        r.confidence,
                        r.sample.red,
                        r.sample.green,
                        r.sample.blue,
                        r.sample.clear,
                        r_base.unwrap_or(0),
                        g_base.unwrap_or(0),
                        b_base.unwrap_or(0),
                        c_base.unwrap_or(0)
                    ),
                    None => "no sensor readings received".to_string(),
                };
                anyhow::bail!(
                    "Timeout ({:.1}s) waiting for blink events on channel {} (detected {} blinks, expected {:?}, durations: {:?}, {})",
                    timeout_s,
                    channel,
                    sw_blink_count,
                    expected_count,
                    pulse_durations,
                    last_info
                );
            }

            // 1. Settle gap check (runs every iteration once blinks have finished)
            if !in_pulse {
                if let Some(lpe) = last_pulse_end {
                    if lpe.elapsed() >= settle_gap {
                        if let Some(exp_c) = expected_count {
                            if sw_blink_count == exp_c {
                                return Ok(BlinkResult {
                                    event_id: 0,
                                    blink_count: sw_blink_count,
                                    color: last_detected_color,
                                    durations_ms: pulse_durations,
                                    pulses,
                                });
                            } else if sw_blink_count > exp_c {
                                anyhow::bail!(
                                    "Blink count mismatch on channel {}: detected {} blinks ({:?}), but expected exactly {}",
                                    channel,
                                    sw_blink_count,
                                    pulse_durations,
                                    exp_c
                                );
                            }
                        } else if sw_blink_count > 0 {
                            return Ok(BlinkResult {
                                event_id: 0,
                                blink_count: sw_blink_count,
                                color: last_detected_color,
                                durations_ms: pulse_durations,
                                pulses,
                            });
                        }
                    }
                }
            }

            // 2. Real-time optical pulse & blink tracker with dynamic ambient cancellation
            if let Ok(reading) = self.read_color(channel) {
                last_reading = Some(reading.clone());

                // Continuously track minimum observed optical noise floor as ambient baseline
                let rb = match r_base {
                    Some(b) => {
                        let new_b = b.min(reading.sample.red);
                        r_base = Some(new_b);
                        new_b
                    }
                    None => {
                        r_base = Some(reading.sample.red);
                        reading.sample.red
                    }
                };
                let gb = match g_base {
                    Some(b) => {
                        let new_b = b.min(reading.sample.green);
                        g_base = Some(new_b);
                        new_b
                    }
                    None => {
                        g_base = Some(reading.sample.green);
                        reading.sample.green
                    }
                };
                let bb = match b_base {
                    Some(b) => {
                        let new_b = b.min(reading.sample.blue);
                        b_base = Some(new_b);
                        new_b
                    }
                    None => {
                        b_base = Some(reading.sample.blue);
                        reading.sample.blue
                    }
                };
                let cb = match c_base {
                    Some(b) => {
                        let new_b = b.min(reading.sample.clear);
                        c_base = Some(new_b);
                        new_b
                    }
                    None => {
                        c_base = Some(reading.sample.clear);
                        reading.sample.clear
                    }
                };

                // Delta from ambient baseline (pure optical emission of the LED)
                let delta_r = reading.sample.red.saturating_sub(rb);
                let delta_g = reading.sample.green.saturating_sub(gb);
                let delta_b = reading.sample.blue.saturating_sub(bb);
                let delta_c = reading.sample.clear.saturating_sub(cb);

                // Pulse threshold: Intensity jump >= 8 count or delta RGB sum >= 12
                let pulse_active = delta_c >= 8.max(cb / 4) || (delta_r + delta_g + delta_b) >= 12;

                // Color matching using pure delta optical emission:
                let is_pink_match = expected_color.map(|s| s.eq_ignore_ascii_case("PINK") || s.to_uppercase().contains("PINK")).unwrap_or(false)
                    && pulse_active
                    && (reading.color == Color::Pink
                        || reading.color == Color::Magenta
                        || reading.color == Color::Red
                        || (delta_r >= 6 && delta_b >= 8 && (delta_r as f32 / delta_b.max(1) as f32) >= 0.25));

                let is_matching_color = match (&exp_color_enum, reading.color) {
                    (Some(exp_c), c) => {
                        let matches_any = if let Some(exp_str) = expected_color {
                            exp_str.split('|').any(|part| {
                                let part = part.trim();
                                c.as_str().eq_ignore_ascii_case(part) || Color::from_str(part) == c
                            })
                        } else {
                            false
                        };
                        (*exp_c == c && pulse_active) || is_pink_match || matches_any
                    }
                    (None, Color::Off) | (None, Color::Unknown) => false,
                    (None, _) => pulse_active,
                };

                let matched_color = if is_matching_color {
                    if is_pink_match {
                        Some(Color::Pink)
                    } else {
                        Some(reading.color)
                    }
                } else {
                    None
                };

                if is_matching_color {
                    if !in_pulse {
                        in_pulse = true;
                        pulse_start = Instant::now();
                        last_detected_color = matched_color;
                        peak_sample = Some(reading.sample.clone());
                        peak_delta = (delta_r, delta_g, delta_b, delta_c);
                    } else if reading.sample.clear >= peak_sample.as_ref().map_or(0, |s| s.clear) {
                        peak_sample = Some(reading.sample.clone());
                        peak_delta = (delta_r, delta_g, delta_b, delta_c);
                    }
                } else if in_pulse {
                    in_pulse = false;
                    let duration = pulse_start.elapsed().as_millis() as u64;
                    let min_ok = min_pulse_ms.map_or(true, |min_v| duration >= min_v);
                    let max_ok = max_pulse_ms.map_or(true, |max_v| duration <= max_v);
                    if min_ok && max_ok {
                        sw_blink_count += 1;
                        pulse_durations.push(duration);
                        let p_sample = peak_sample.take().unwrap_or(reading.sample.clone());
                        let p_color = matched_color.unwrap_or(last_detected_color.unwrap_or(reading.color));
                        let p_detail = PulseDetail {
                            index: sw_blink_count,
                            duration_ms: duration,
                            color: p_color,
                            sample: p_sample,
                            delta: peak_delta,
                        };
                        pulses.push(p_detail);
                        last_pulse_end = Some(Instant::now());
                    }
                }
            }

            thread::sleep(Duration::from_millis(30));
        }
    }
}

/// Smart RGBC Classifier using HSV color space and optical chromaticity
pub fn classify_rgbc_color(r: u16, g: u16, b: u16, c: u16) -> (Color, ColorConfidence) {
    // 1. If clear intensity is low or RGB is ambient background baseline (< 35) -> Off
    if c < 85 || (r < 35 && g < 35 && b < 35) {
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
        // Red / Pink / Warm Red: 340..360 or 0..28
        h if (340.0..=360.0).contains(&h) || (0.0..28.0).contains(&h) => {
            if sat < 0.20 {
                Color::White
            } else if sat < 0.55 && (g > 15 || b > 15) {
                Color::Pink
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
