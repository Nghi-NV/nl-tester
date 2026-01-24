pub mod android;
pub mod android_auto;
pub mod common;
pub mod image_matcher;
pub mod ios;
pub mod traits;
pub mod web;

use anyhow::Result;

/// List connected devices for the specified platform
pub async fn list_devices(platform: &str) -> Result<()> {
    match platform {
        "android" => android::list_devices().await,
        "ios" => ios::list_devices().await,
        "web" => {
            println!("Web browsers listing not applicable");
            Ok(())
        }
        _ => {
            anyhow::bail!("Unknown platform: {}", platform);
        }
    }
}
