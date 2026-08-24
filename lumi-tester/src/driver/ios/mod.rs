//! iOS Driver module
//!
//! This module provides iOS automation support via the `lm-ios-tester` on-device agent
//! (a minimal custom XCUITest target, see `/lm-ios-tester/` at the repo root) for UI
//! automation, and native `xcrun devicectl`/`xcrun simctl` for process/file/lifecycle
//! operations. idb and WebDriverAgent are no longer used anywhere in this module.
//!
//! For simulators and real devices alike: requires the on-device agent running
//! (port 8110), auto-started/managed by `agent_setup`.

pub mod accessibility;
pub mod agent;
pub mod agent_setup;
pub mod devicectl;
pub mod driver;

pub use driver::IosDriver;

use anyhow::Result;

/// List connected iOS devices and simulators
pub async fn list_devices() -> Result<()> {
    let devices = devicectl::list_targets().await?;

    if devices.is_empty() {
        println!("No iOS devices or simulators found.");
    } else {
        println!("Connected iOS devices:");
        for device in devices {
            println!("  {} - {} ({})", device.udid, device.name, device.state);
        }
    }

    Ok(())
}
