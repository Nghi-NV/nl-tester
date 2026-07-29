pub mod calibration;
pub mod color_sensor;
pub mod protocol;
pub mod relay;
pub mod servo;
pub mod traits;
pub mod transport;
pub mod types;

use anyhow::Result;
use std::sync::{Arc, Mutex};

pub use calibration::CalibrationService;
pub use color_sensor::ColorSensorService;
pub use relay::RelayService;
pub use servo::ServoService;
pub use traits::{ColorSensorControl, HardwareDriver, RelayControl, ServoControl};
pub use transport::SerialTransport;
pub use types::*;

pub struct HardwareController {
    pub config: HardwareConfig,
    transport: Arc<Mutex<SerialTransport>>,
    pub servo: ServoService,
    pub relay: RelayService,
    pub color_sensor: ColorSensorService,
    pub calibration: CalibrationService,
}

impl HardwareController {
    pub fn new(config: Option<HardwareConfig>) -> Self {
        let cfg = config.unwrap_or_default();
        let transport = Arc::new(Mutex::new(SerialTransport::new()));

        let servo = ServoService::new(Arc::clone(&transport));
        let relay = RelayService::new(Arc::clone(&transport));
        let color_sensor = ColorSensorService::new(Arc::clone(&transport));
        let calibration = CalibrationService::new(Arc::clone(&transport));

        Self {
            config: cfg,
            transport,
            servo,
            relay,
            color_sensor,
            calibration,
        }
    }

    pub fn connect(&self, port: &str, baudrate: Option<u32>) -> Result<()> {
        let baud = baudrate.unwrap_or(self.config.baudrate);
        let mut transport = self.transport.lock().unwrap();
        transport.connect(port, baud)?;

        // Send ping handshake
        let ping_resp =
            transport.request(&protocol::cmd_ping(), |line| line.kind == "system", 2.0)?;
        if ping_resp.get_str("status") != Some("ready") {
            log::warn!("Handshake returned unexpected status: {:?}", ping_resp.raw);
        }

        drop(transport);

        for ch in 1..=8 {
            let _ = self.servo.set_config(ch, 15, 72, 400, 150, 300);
        }

        Ok(())
    }

    pub fn enter_safe_state(&self) -> Result<()> {
        let _ = self.relay.all_off();
        let _ = self.servo.release_all();
        let _ = self.color_sensor.light_off();
        Ok(())
    }

    pub fn system_diagnostics(&self) -> Result<String> {
        let mut transport = self.transport.lock().unwrap();
        let resp = transport.request(
            &protocol::cmd_system_diagnostics(),
            |line| line.kind == "system" || line.kind == "ok",
            3.0,
        )?;
        Ok(resp.raw)
    }
}

impl HardwareDriver for HardwareController {
    fn driver_type(&self) -> &str {
        "stm32_rs485"
    }

    fn is_connected(&self) -> bool {
        let transport = self.transport.lock().unwrap();
        transport.is_connected()
    }

    fn servo(&self) -> Option<&dyn ServoControl> {
        Some(&self.servo)
    }

    fn relay(&self) -> Option<&dyn RelayControl> {
        Some(&self.relay)
    }

    fn color_sensor(&self) -> Option<&dyn ColorSensorControl> {
        Some(&self.color_sensor)
    }

    fn disconnect(&self) {
        let mut transport = self.transport.lock().unwrap();
        transport.disconnect();
    }
}
