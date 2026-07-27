use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::hardware::protocol::{cmd_relay_all_off, cmd_relay_off, cmd_relay_on};
use crate::hardware::transport::SerialTransport;
use crate::hardware::traits::RelayControl;
use crate::hardware::types::{ActionResult, RelayState};

pub struct RelayService {
    transport: Arc<Mutex<SerialTransport>>,
}

impl RelayService {
    pub fn new(transport: Arc<Mutex<SerialTransport>>) -> Self {
        Self { transport }
    }
}

impl RelayControl for RelayService {
    fn set_state(&self, channel: u8, state: RelayState) -> Result<ActionResult> {
        let start = Instant::now();
        let cmd = match state {
            RelayState::On => cmd_relay_on(channel),
            RelayState::Off => cmd_relay_off(channel),
        };

        let mut transport = self.transport.lock().unwrap();
        let resp = transport.request(
            &cmd,
            |line| line.kind == "relay" && line.get_u8("channel") == Some(channel),
            3.0,
        )?;

        Ok(ActionResult {
            action: format!("relay.set_{}", state.as_str()),
            channel: Some(channel),
            completed: resp.get_str("status") == Some("completed") || resp.kind == "relay",
            duration_ms: start.elapsed().as_millis() as u64,
            message: None,
        })
    }

    fn all_off(&self) -> Result<ActionResult> {
        let start = Instant::now();
        let mut transport = self.transport.lock().unwrap();
        let resp = transport.request(
            &cmd_relay_all_off(),
            |line| line.kind == "relay" && line.get_str("action") == Some("all_off"),
            3.0,
        )?;

        Ok(ActionResult {
            action: "relay.all_off".to_string(),
            channel: None,
            completed: resp.get_str("status") == Some("completed") || resp.kind == "relay",
            duration_ms: start.elapsed().as_millis() as u64,
            message: None,
        })
    }
}
