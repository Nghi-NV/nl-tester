use anyhow::Result;

use crate::hardware::types::{ActionResult, BlinkResult, Color, ColorReading, RelayState};

/// Interface cho phần cứng điều khiển Servo (gạt công tắc, bấm nút)
pub trait ServoControl: Send + Sync {
    fn click(&self, channel: u8, hold_ms: Option<u64>) -> Result<ActionResult>;
    fn press(&self, channel: u8) -> Result<ActionResult>;
    fn release(&self, channel: u8) -> Result<ActionResult>;
    fn get_state(&self, channel: u8) -> Result<String>;
    fn repeat(
        &self,
        channel: u8,
        count: u32,
        press_ms: u64,
        release_ms: u64,
    ) -> Result<ActionResult>;
    fn start_repeat(&self, channel: u8, period_ms: u64) -> Result<ActionResult>;
    fn stop_repeat(&self, channel: u8) -> Result<ActionResult>;
    fn release_all(&self) -> Result<ActionResult>;
    fn set_config(
        &self,
        channel: u8,
        press_angle: u8,
        release_angle: u8,
        press_ms: u16,
        release_ms: u16,
        hold_ms: u16,
    ) -> Result<ActionResult>;
}

/// Interface cho phần cứng điều khiển Relay (bật/tắt nguồn)
pub trait RelayControl: Send + Sync {
    fn set_state(&self, channel: u8, state: RelayState) -> Result<ActionResult>;
    fn get_state(&self, channel: u8) -> Result<RelayState>;
    fn all_off(&self) -> Result<ActionResult>;
}

/// Interface cho cảm biến màu sắc & phát hiện chớp tắt LED
pub trait ColorSensorControl: Send + Sync {
    fn read_color(&self, channel: u8) -> Result<ColorReading>;
    fn get_light_state(&self) -> Result<bool>;
    fn wait_for_color(
        &self,
        channel: u8,
        expected: Option<&[Color]>,
        timeout_s: f64,
    ) -> Result<ColorReading>;
    fn wait_for_blinks(
        &self,
        channel: u8,
        after_event_id: Option<u32>,
        timeout_s: f64,
    ) -> Result<BlinkResult>;
}

/// Trait tổng quát cho bất kỳ loại phần cứng/thiết bị tự động hóa nào (STM32, Modbus, SCPI, HTTP Relay, BLE, ...)
pub trait HardwareDriver: Send + Sync {
    /// Tên loại driver phần cứng (ví dụ: "stm32_rs485", "modbus_rtu", "http_relay", "scpi_instrument")
    fn driver_type(&self) -> &str;

    /// Trạng thái kết nối
    fn is_connected(&self) -> bool;

    /// Trả về interface Servo nếu phần cứng hỗ trợ
    fn servo(&self) -> Option<&dyn ServoControl> {
        None
    }

    /// Trả về interface Relay nếu phần cứng hỗ trợ
    fn relay(&self) -> Option<&dyn RelayControl> {
        None
    }

    /// Trả về interface Color Sensor nếu phần cứng hỗ trợ
    fn color_sensor(&self) -> Option<&dyn ColorSensorControl> {
        None
    }

    /// Ngắt kết nối thiết bị
    fn disconnect(&self);
}
