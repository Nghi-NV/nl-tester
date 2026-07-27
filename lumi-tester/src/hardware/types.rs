use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServoChannelConfig {
    pub press_angle: u8,
    pub release_angle: u8,
    pub press_duration_ms: u16,
    pub release_duration_ms: u16,
    pub hold_duration_ms: u16,
    pub min_angle: u8,
    pub max_angle: u8,
}

impl Default for ServoChannelConfig {
    fn default() -> Self {
        Self {
            press_angle: 72,
            release_angle: 15,
            press_duration_ms: 400,
            release_duration_ms: 150,
            hold_duration_ms: 300,
            min_angle: 0,
            max_angle: 180,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareConfig {
    pub baudrate: u32,
    pub command_timeout_s: f64,
    pub sensor_timeout_s: f64,
    pub handshake_timeout_s: f64,
    pub servo_channels: HashMap<u8, ServoChannelConfig>,
    pub relay_channels: Vec<u8>,
    pub color_sensor_channels: Vec<u8>,
}

impl Default for HardwareConfig {
    fn default() -> Self {
        Self {
            baudrate: 115200,
            command_timeout_s: 2.0,
            sensor_timeout_s: 5.0,
            handshake_timeout_s: 2.0,
            servo_channels: HashMap::new(),
            relay_channels: vec![1, 2, 3, 4],
            color_sensor_channels: vec![1],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Color {
    Red,
    Green,
    Blue,
    Yellow,
    Cyan,
    Magenta,
    Pink,
    White,
    Unknown,
}

impl Color {
    pub fn as_str(&self) -> &'static str {
        match self {
            Color::Red => "RED",
            Color::Green => "GREEN",
            Color::Blue => "BLUE",
            Color::Yellow => "YELLOW",
            Color::Cyan => "CYAN",
            Color::Magenta => "MAGENTA",
            Color::Pink => "PINK",
            Color::White => "WHITE",
            Color::Unknown => "UNKNOWN",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "RED" => Color::Red,
            "GREEN" => Color::Green,
            "BLUE" => Color::Blue,
            "YELLOW" => Color::Yellow,
            "CYAN" => Color::Cyan,
            "MAGENTA" => Color::Magenta,
            "PINK" => Color::Pink,
            "WHITE" => Color::White,
            _ => Color::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorConfidence {
    Ok,
    LowConfidence,
    Uncalibrated,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayState {
    On,
    Off,
}

impl RelayState {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelayState::On => "on",
            RelayState::Off => "off",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "on" | "1" | "true" => RelayState::On,
            _ => RelayState::Off,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action: String,
    pub channel: Option<u8>,
    pub completed: bool,
    pub duration_ms: u64,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawColorSample {
    pub red: u16,
    pub green: u16,
    pub blue: u16,
    pub clear: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorReading {
    pub channel: u8,
    pub color: Color,
    pub confidence: ColorConfidence,
    pub sample: RawColorSample,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlinkResult {
    pub event_id: u32,
    pub blink_count: usize,
    pub color: Option<Color>,
    pub durations_ms: Vec<u64>,
}
