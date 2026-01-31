//! WebDriverAgent Setup Module
//!
//! Handles automatic installation and startup of WebDriverAgent on real iOS devices.
//! Supports go-ios and tidevice for WDA management.

use anyhow::{Context, Result};
use colored::Colorize;
use std::process::Stdio;
use tokio::process::Command;

/// WDA Bundle IDs (different variants depending on signing)
pub const WDA_BUNDLE_IDS: &[&str] = &[
    "com.facebook.WebDriverAgentRunner.xctrunner",
    "com.facebook.WebDriverAgentRunner",
    "com.facebook.IntegrationApp", // When built with Personal Team
    "com.apple.test.WebDriverAgentRunner-Runner",
];

/// Primary bundle ID to use for launching
pub const WDA_BUNDLE_ID: &str = "com.facebook.IntegrationApp";

/// Represents a WDA launcher tool
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WdaLauncher {
    Xcodebuild, // Best for iOS 17+ (uses native tools)
    GoIos,
    Tidevice,
    None,
}

/// Check which WDA launcher is available
pub async fn detect_launcher() -> WdaLauncher {
    // Check xcodebuild first (best for iOS 17+)
    if Command::new("xcodebuild")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .is_ok()
    {
        return WdaLauncher::Xcodebuild;
    }

    // Check go-ios
    if Command::new("ios")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .is_ok()
    {
        return WdaLauncher::GoIos;
    }

    // Check tidevice
    if Command::new("tidevice")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .is_ok()
    {
        return WdaLauncher::Tidevice;
    }

    WdaLauncher::None
}

/// Check if WDA is installed on the device
pub async fn is_wda_installed(udid: &str, launcher: WdaLauncher) -> bool {
    match launcher {
        WdaLauncher::GoIos => {
            // List apps and check for WDA
            let output = Command::new("ios")
                .args(["apps", "--udid", udid])
                .output()
                .await;

            if let Ok(output) = output {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return WDA_BUNDLE_IDS.iter().any(|id| stdout.contains(id))
                    || stdout.contains("WebDriverAgent");
            }
            false
        }
        WdaLauncher::Tidevice => {
            // List apps using tidevice
            let output = Command::new("tidevice")
                .args(["-u", udid, "applist"])
                .output()
                .await;

            if let Ok(output) = output {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return WDA_BUNDLE_IDS.iter().any(|id| stdout.contains(id))
                    || stdout.contains("WebDriverAgent")
                    || stdout.contains("IntegrationApp");
            }
            false
        }
        WdaLauncher::Xcodebuild | WdaLauncher::None => {
            // For xcodebuild, we check if WDA project exists
            // and if the app was already installed via tidevice check
            false
        }
    }
}

/// Start WDA on the device
pub async fn start_wda(
    udid: &str,
    launcher: WdaLauncher,
    port: u16,
) -> Result<tokio::process::Child> {
    match launcher {
        WdaLauncher::Xcodebuild => {
            println!("  {} Starting WDA using xcodebuild...", "⏳".yellow());
            println!("  {} Please run WDA from Xcode or use:", "ℹ".blue());
            println!("      xcodebuild test-without-building -project WebDriverAgent.xcodeproj \\");
            println!(
                "        -scheme WebDriverAgentRunner -destination 'id={}'",
                udid
            );

            // Start iproxy for port forwarding
            let child = start_iproxy(udid, port).await?;
            Ok(child)
        }
        WdaLauncher::GoIos => {
            println!("  {} Starting WDA using go-ios...", "⏳".yellow());

            // Start WDA using go-ios runwda
            let child = Command::new("ios")
                .args(["runwda", "--bundleid", WDA_BUNDLE_ID, "--udid", udid])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .context("Failed to start WDA with go-ios")?;

            // Also start iproxy for port forwarding
            let _ = start_iproxy(udid, port).await;

            Ok(child)
        }
        WdaLauncher::Tidevice => {
            println!("  {} Starting WDA using tidevice...", "⏳".yellow());

            // Start WDA proxy using tidevice (includes port forwarding)
            let child = Command::new("tidevice")
                .args([
                    "-u",
                    udid,
                    "wdaproxy",
                    "-B",
                    WDA_BUNDLE_ID,
                    "--port",
                    &port.to_string(),
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .context("Failed to start WDA with tidevice")?;

            Ok(child)
        }
        WdaLauncher::None => {
            anyhow::bail!("No WDA launcher available. Please install go-ios or tidevice.")
        }
    }
}

/// Start iproxy for port forwarding
pub async fn start_iproxy(udid: &str, port: u16) -> Result<tokio::process::Child> {
    let child = Command::new("iproxy")
        .args([&port.to_string(), &port.to_string(), "-u", udid])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to start iproxy")?;

    Ok(child)
}

/// Show instructions for installing WDA
pub fn show_install_instructions() {
    println!();
    println!(
        "{}",
        "════════════════════════════════════════════════════════════".yellow()
    );
    println!("{}", "  WebDriverAgent Setup Required".yellow().bold());
    println!(
        "{}",
        "════════════════════════════════════════════════════════════".yellow()
    );
    println!();
    println!("  To run UI tests on real iOS devices, you need WebDriverAgent.");
    println!();
    println!("  {} Install go-ios (recommended):", "1.".cyan().bold());
    println!(
        "     {}",
        "go install github.com/danielpaulus/go-ios/cmd/ios@latest".dimmed()
    );
    println!();
    println!("  {} Build WebDriverAgent:", "2.".cyan().bold());
    println!(
        "     {}",
        "git clone https://github.com/appium/WebDriverAgent.git".dimmed()
    );
    println!("     {}", "cd WebDriverAgent".dimmed());
    println!("     {}", "Open WebDriverAgent.xcodeproj in Xcode".dimmed());
    println!(
        "     {}",
        "Select your device and run WebDriverAgentRunner".dimmed()
    );
    println!();
    println!("  {} Run test again", "3.".cyan().bold());
    println!();
    println!(
        "{}",
        "════════════════════════════════════════════════════════════".yellow()
    );
    println!();
}

/// Ensure WDA is running, with automatic setup if needed
pub async fn ensure_wda_running(udid: &str, port: u16) -> Result<Option<tokio::process::Child>> {
    // First check if WDA is already running
    let client = super::wda::WdaClient::new(port);
    if client.is_ready().await.unwrap_or(false) {
        println!(
            "{} WebDriverAgent already running on port {}",
            "✓".green(),
            port
        );
        return Ok(None);
    }

    // Detect available launcher
    let launcher = detect_launcher().await;

    // For Xcodebuild (iOS 17+), we try to find WDA via network
    if launcher == WdaLauncher::Xcodebuild {
        println!("{} iOS 17+ detected. Scanning for WDA...", "ℹ".blue());

        // Start iproxy for port forwarding (in case USB connection works)
        let _ = start_iproxy(udid, port).await;

        // Wait a moment for iproxy
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Scan for WDA host (localhost first, then network)
        if let Some(host) = scan_for_wda_host(port).await {
            println!("{} WebDriverAgent found at {}:{}", "✓".green(), host, port);
            // Store the host for later use
            std::env::set_var("WDA_HOST", &host);
            return Ok(None);
        }

        // WDA not running - show iOS 17+ specific instructions
        println!();
        println!(
            "{}",
            "════════════════════════════════════════════════════════════".yellow()
        );
        println!(
            "{}",
            "  WebDriverAgent Not Running (iOS 17+)".yellow().bold()
        );
        println!(
            "{}",
            "════════════════════════════════════════════════════════════".yellow()
        );
        println!();
        println!("  Please start WDA from Xcode and KEEP it running:");
        println!();
        println!(
            "  {} Open Xcode with WebDriverAgent.xcodeproj",
            "1.".cyan().bold()
        );
        println!(
            "  {} Select scheme: {}",
            "2.".cyan().bold(),
            "WebDriverAgentRunner".green()
        );
        println!("  {} Select your device as destination", "3.".cyan().bold());
        println!(
            "  {} Run: {} or click Test button",
            "4.".cyan().bold(),
            "Product > Test (Cmd+U)".green()
        );
        println!(
            "  {} Keep Xcode running, then re-run this command",
            "5.".cyan().bold()
        );
        println!();
        println!(
            "{}",
            "════════════════════════════════════════════════════════════".yellow()
        );
        println!();
        return Ok(None);
    }

    if launcher == WdaLauncher::None {
        println!(
            "{} No WDA launcher found (go-ios or tidevice)",
            "⚠️".yellow()
        );
        show_install_instructions();
        return Ok(None);
    }

    // Check if WDA is installed on device (for go-ios/tidevice)
    if !is_wda_installed(udid, launcher).await {
        println!("{} WebDriverAgent not installed on device", "⚠️".yellow());
        show_install_instructions();
        return Ok(None);
    }

    // Start WDA
    let child = start_wda(udid, launcher, port).await?;

    // Wait for WDA to be ready (max 30 seconds)
    println!("  {} Waiting for WDA to start...", "⏳".yellow());
    for i in 0..30 {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        if client.is_ready().await.unwrap_or(false) {
            println!(
                "{} WebDriverAgent started successfully ({}s)",
                "✓".green(),
                i + 1
            );
            return Ok(Some(child));
        }
    }

    println!("{} WDA failed to start within 30 seconds", "✗".red());
    Ok(Some(child))
}

/// Scan for WDA on network - returns the host where WDA is reachable
pub async fn scan_for_wda_host(port: u16) -> Option<String> {
    use std::time::Duration;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .ok()?;

    // Try localhost first (iproxy)
    if let Ok(resp) = client
        .get(format!("http://localhost:{}/status", port))
        .send()
        .await
    {
        if resp.status().is_success() {
            return Some("localhost".to_string());
        }
    }

    // Try to get local IP and scan same subnet
    if let Ok(output) = std::process::Command::new("sh")
        .args([
            "-c",
            "ifconfig | grep 'inet ' | grep -v 127.0.0.1 | awk '{print $2}' | head -1",
        ])
        .output()
    {
        let local_ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if let Some(prefix) = local_ip.rsplit_once('.') {
            // Scan IPs 1-254 in parallel (limit to common ranges for speed)
            let prefix = prefix.0.to_string();
            let port = port;

            // Common device IP ranges (try most common first)
            let common_ips: Vec<u8> = (100..=130).chain(1..=30).chain(131..=254).collect();

            for chunk in common_ips.chunks(30) {
                let futures: Vec<_> = chunk
                    .iter()
                    .map(|i| {
                        let host = format!("{}.{}", prefix, i);
                        let client = client.clone();
                        async move {
                            if let Ok(resp) = client
                                .get(format!("http://{}:{}/status", host, port))
                                .send()
                                .await
                            {
                                if resp.status().is_success() {
                                    return Some(host);
                                }
                            }
                            None
                        }
                    })
                    .collect();

                let results = futures::future::join_all(futures).await;
                if let Some(Some(host)) = results.into_iter().find(|r| r.is_some()) {
                    return Some(host);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_launcher() {
        let launcher = detect_launcher().await;
        println!("Detected launcher: {:?}", launcher);
    }
}
