//! iOS Driver module
//!
//! This module provides iOS automation support:
//! - Simulators: Uses idb (iOS Development Bridge)
//! - Real devices: Uses WebDriverAgent (WDA) via HTTP API
//!
//! For simulators: requires idb_companion to be running on macOS.
//! For real devices: requires WebDriverAgent running on device (port 8100).

pub mod accessibility;
pub mod driver;
pub mod idb;
pub mod wda;
pub mod wda_setup;

pub use driver::IosDriver;

use anyhow::Result;

/// List connected iOS devices and simulators
pub async fn list_devices() -> Result<()> {
    let devices = idb::list_targets().await?;

    if devices.is_empty() {
        println!("No iOS devices or simulators found.");
        println!("Make sure idb_companion is running and devices are connected.");
    } else {
        println!("Connected iOS devices:");
        for device in devices {
            println!("  {} - {} ({})", device.udid, device.name, device.state);
        }
    }

    Ok(())
}
