use anyhow::Result;
use std::sync::{Arc, Mutex};

use crate::hardware::protocol::{
    cmd_calibration_action, cmd_calibration_brightness, cmd_calibration_cct, cmd_calibration_color,
};
use crate::hardware::transport::SerialTransport;
use crate::hardware::types::ActionResult;

pub struct CalibrationService {
    transport: Arc<Mutex<SerialTransport>>,
}

impl CalibrationService {
    pub fn new(transport: Arc<Mutex<SerialTransport>>) -> Self {
        Self { transport }
    }

    pub fn calibrate_color(&self, channel: u8, color: &str) -> Result<ActionResult> {
        let mut transport = self.transport.lock().unwrap();
        let resp = transport.request(
            &cmd_calibration_color(channel, color),
            |line| line.kind == "cal" || line.kind == "ok",
            10.0,
        )?;
        Ok(ActionResult {
            action: "calibration.calibrate_color".to_string(),
            channel: Some(channel),
            completed: true,
            duration_ms: 0,
            message: resp.get_str("message").map(|s| s.to_string()),
        })
    }

    pub fn calibrate_brightness(&self, channel: u8, mode: &str, color: Option<&str>) -> Result<ActionResult> {
        let mut transport = self.transport.lock().unwrap();
        let resp = transport.request(
            &cmd_calibration_brightness(channel, mode, color),
            |line| line.kind == "cal" || line.kind == "ok",
            10.0,
        )?;
        Ok(ActionResult {
            action: format!("calibration.calibrate_brightness_{}", mode),
            channel: Some(channel),
            completed: true,
            duration_ms: 0,
            message: resp.get_str("message").map(|s| s.to_string()),
        })
    }

    pub fn add_cct_point(&self, channel: u8, known_kelvin: u16) -> Result<ActionResult> {
        let mut transport = self.transport.lock().unwrap();
        let resp = transport.request(
            &cmd_calibration_cct(known_kelvin),
            |line| line.kind == "cct_cal" || line.kind == "ok",
            10.0,
        )?;
        Ok(ActionResult {
            action: "calibration.add_cct_point".to_string(),
            channel: Some(channel),
            completed: true,
            duration_ms: 0,
            message: resp.get_str("message").map(|s| s.to_string()),
        })
    }

    pub fn action(&self, action_name: &str) -> Result<ActionResult> {
        let mut transport = self.transport.lock().unwrap();
        let resp = transport.request(
            &cmd_calibration_action(action_name),
            |line| line.kind == "cal" || line.kind == "cct_cal" || line.kind == "ok",
            5.0,
        )?;
        Ok(ActionResult {
            action: format!("calibration.{}", action_name),
            channel: None,
            completed: true,
            duration_ms: 0,
            message: resp.get_str("message").map(|s| s.to_string()),
        })
    }
}
