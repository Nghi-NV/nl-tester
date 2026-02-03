pub mod adb;
pub mod audio_service;
pub mod driver;
pub mod mirror_service;
pub mod uiautomator;

pub use driver::AndroidDriver;

use anyhow::Result;
use colored::Colorize;

/// List connected Android devices
pub async fn list_devices() -> Result<()> {
    let devices = adb::get_devices().await?;

    if devices.is_empty() {
        println!("  No Android devices connected");
    } else {
        println!("  Found {} device(s):", devices.len());
        for device in devices {
            println!(
                "    {} {} ({})",
                "•".green(),
                device.serial.white().bold(),
                device.state.dimmed()
            );
        }
    }

    Ok(())
}
