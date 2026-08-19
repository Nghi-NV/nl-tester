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

        // 2. Resolve color string: prefer stable color, but fall back to instantaneous color if stable is UNKNOWN
        let color_str = if !stable_str.is_empty() && !stable_str.eq_ignore_ascii_case("UNKNOWN") {
            stable_str
        } else if !instant_str.is_empty() && !instant_str.eq_ignore_ascii_case("UNKNOWN") {
            instant_str
        } else if !stable_str.is_empty() {
            stable_str
        } else {
            "UNKNOWN"
        };

        let confidence_str = resp.get_str("s_conf").or_else(|| resp.get_str("conf")).unwrap_or("OK");

        let color = Color::from_str(color_str);
        let confidence = match confidence_str.trim().to_uppercase().as_str() {
            "OK" | "GOOD" => ColorConfidence::Ok,
            "LOW_CONFIDENCE" | "POOR" => ColorConfidence::LowConfidence,
            "UNCALIBRATED" => ColorConfidence::Uncalibrated,
            _ => ColorConfidence::Invalid,
        };

        Ok(ColorReading {
            channel,
            color,
            confidence,
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

        loop {
            if start.elapsed() > timeout {
                anyhow::bail!(
                    "Timeout ({:.1}s) waiting for expected color on channel {}",
                    timeout_s,
                    channel
                );
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
