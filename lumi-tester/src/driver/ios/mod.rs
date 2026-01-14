//! iOS Driver module using idb (iOS Development Bridge)
//!
//! This module provides iOS automation support for both simulators and real devices.
//! It requires idb_companion to be running on macOS.

pub mod driver;
pub mod idb;
pub mod accessibility;

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
