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
        transport.connect(port, baud)
            .map_err(|e| anyhow::anyhow!("Failed to connect to hardware Jig on '{}' (baudrate: {}): {}", port, baud, e))?;

        // Send ping handshake (accept system, ok, version, or info with ready)
        let ping_resp = match transport.request(
            &protocol::cmd_ping(),
            |line| line.kind == "system" || line.kind == "ok" || line.kind == "version" || line.kind == "info",
            1.5,
        ) {
            Ok(r) => r,
            Err(_) => {
                // Fallback to addressed @1 ping for RS485 multi-drop buses
                transport.request(
                    "@1 ping\n",
                    |line| line.kind == "system" || line.kind == "ok" || line.kind == "version" || line.kind == "info",
                    2.0,
                ).map_err(|e| anyhow::anyhow!("Connected to Jig on '{}', but handshake failed (no valid response from MCU): {}", port, e))?
            }
        };

        if ping_resp.kind == "system" && ping_resp.get_str("status") != Some("ready") {
            log::warn!("Handshake returned status: {:?}", ping_resp.raw);
        }

        Ok(())
    }

    pub fn enter_safe_state(&self) -> Result<()> {
        let _ = self.relay.all_off();
        let _ = self.servo.release_all();
        let _ = self.color_sensor.light_off(None);
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JigPortInfo {
    pub port_name: String,
    pub port_type: String,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
    pub vid: Option<u16>,
    pub pid: Option<u16>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JigPingResult {
    pub port: String,
    pub baudrate: u32,
    pub connected: bool,
    pub latency_ms: u64,
    pub raw_response: String,
    pub node_id: Option<u8>,
    pub firmware_version: Option<String>,
    pub system_status: Option<String>,
}

/// Liệt kê toàn bộ các cổng Serial / COM đang cắm vào máy tính kèm metadata
pub fn list_serial_ports() -> Vec<JigPortInfo> {
    let mut results = Vec::new();
    if let Ok(ports) = serialport::available_ports() {
        for p in ports {
            let (port_type, manufacturer, product, serial_number, vid, pid) = match p.port_type {
                serialport::SerialPortType::UsbPort(info) => (
                    "USB".to_string(),
                    info.manufacturer,
                    info.product,
                    info.serial_number,
                    Some(info.vid),
                    Some(info.pid),
                ),
                serialport::SerialPortType::PciPort => ("PCI".to_string(), None, None, None, None, None),
                serialport::SerialPortType::BluetoothPort => ("Bluetooth".to_string(), None, None, None, None, None),
                serialport::SerialPortType::Unknown => ("Unknown".to_string(), None, None, None, None, None),
            };
            results.push(JigPortInfo {
                port_name: p.port_name,
                port_type,
                manufacturer,
                product,
                serial_number,
                vid,
                pid,
            });
        }
    }
    results
}

/// Thử kết nối nhanh và Ping thiết bị Jig phần cứng, trả về thông tin chi tiết
pub fn ping_details(port: &str, baudrate: Option<u32>) -> Result<JigPingResult> {
    let baud = baudrate.unwrap_or(115200);
    let start = std::time::Instant::now();
    let controller = HardwareController::new(None);
    let mut transport = controller.transport.lock().unwrap();
    transport.connect(port, baud)?;
    let resp = match transport.request(
        &protocol::cmd_ping(),
        |line| line.kind == "system" || line.kind == "ok" || line.kind == "version" || line.kind == "info",
        1.5,
    ) {
        Ok(r) => r,
        Err(_) => {
            // Fallback to addressed @1 ping for RS485 multi-drop buses
            transport.request(
                "@1 ping\n",
                |line| line.kind == "system" || line.kind == "ok" || line.kind == "version" || line.kind == "info",
                2.0,
            )?
        }
    };
    let latency_ms = start.elapsed().as_millis() as u64;

    let node_id = resp.get_u32("node_id").map(|n| n as u8).or_else(|| {
        if resp.raw.starts_with("@1") {
            Some(1)
        } else {
            None
        }
    });
    let firmware_version = resp
        .get_str("firmware")
        .or_else(|| resp.get_str("version"))
        .or_else(|| resp.get_str("fw"))
        .map(|s| s.to_string());
    let system_status = resp
        .get_str("status")
        .or_else(|| {
            if resp.kind == "ok" || resp.raw.to_lowercase().contains("ready") {
                Some("ready")
            } else {
                None
            }
        })
        .map(|s| s.to_string());

    Ok(JigPingResult {
        port: port.to_string(),
        baudrate: baud,
        connected: true,
        latency_ms,
        raw_response: resp.raw,
        node_id,
        firmware_version,
        system_status,
    })
}
