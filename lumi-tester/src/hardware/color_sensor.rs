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
}

impl ColorSensorService {
    pub fn new(transport: Arc<Mutex<SerialTransport>>) -> Self {
        Self { transport }
    }

    pub fn light_on(&self) -> Result<()> {
        let mut transport = self.transport.lock().unwrap();
        transport.request(&crate::hardware::protocol::cmd_color_light(true), |line| line.kind == "color_light" || line.kind == "ok", 3.0)?;
        Ok(())
    }

    pub fn light_off(&self) -> Result<()> {
        let mut transport = self.transport.lock().unwrap();
        transport.request(&crate::hardware::protocol::cmd_color_light(false), |line| line.kind == "color_light" || line.kind == "ok", 3.0)?;
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
    fn get_light_state(&self) -> Result<bool> {
        let mut transport = self.transport.lock().unwrap();
        let resp = transport.request(
            "color light?\n",
            |line| line.kind == "color_light" || line.kind == "ok",
            3.0,
        )?;
        let state = resp.get_str("state").unwrap_or("off");
        Ok(state.eq_ignore_ascii_case("on"))
    }

    fn read_color(&self, channel: u8) -> Result<ColorReading> {
        let mut transport = self.transport.lock().unwrap();
        let _ = transport.request(
            &crate::hardware::protocol::cmd_color_select(channel),
            |line| line.kind == "color_status" || line.kind == "color" || line.kind == "ok",
            2.0,
        );
        let resp = transport.request(
            &cmd_color_read(channel),
            |line| line.kind == "color",
            5.0,
        )?;

        let red = resp.get_u16("red").unwrap_or(0);
        let green = resp.get_u16("green").unwrap_or(0);
        let blue = resp.get_u16("blue").unwrap_or(0);
        let clear = resp.get_u16("clear").unwrap_or(0);

        let color_str = resp.get_str("stable").or_else(|| resp.get_str("color")).unwrap_or("UNKNOWN");
        let confidence_str = resp.get_str("s_conf").or_else(|| resp.get_str("conf")).unwrap_or("OK");

        let color = Color::from_str(color_str);
        let confidence = match confidence_str.to_uppercase().as_str() {
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
        after_event_id: Option<u32>,
        timeout_s: f64,
    ) -> Result<BlinkResult> {
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
                    |line| line.kind == "blink" || line.kind == "color",
                    2.0,
                ) {
                    let blink_count = resp.get_u32("count").unwrap_or(0) as usize;
                    if blink_count > 0 {
                        let event_id = resp.get_u32("event_id").unwrap_or(0);
                        let color_str = resp.get_str("color");
                        let color = color_str.map(Color::from_str);

                        return Ok(BlinkResult {
                            event_id,
                            blink_count,
                            color,
                            durations_ms: vec![],
                        });
                    }
                }
            }

            thread::sleep(Duration::from_millis(150));
        }
    }
}
