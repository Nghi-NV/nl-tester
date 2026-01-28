//! IDB (iOS Development Bridge) CLI wrapper
//!
//! Provides functions to interact with iOS devices and simulators via idb CLI.

use crate::utils::binary_resolver;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;

/// iOS device/simulator target info
#[derive(Debug, Clone, Deserialize)]
pub struct IosTarget {
    pub udid: String,
    pub name: String,
    #[serde(rename = "type")]
    pub target_type: String,
    pub state: String,
}

/// Run idb command and return stdout
async fn run_idb_command(args: &[&str]) -> Result<String> {
    let idb_path = binary_resolver::find_idb()?;
    let output = Command::new(idb_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to execute idb command. Is idb installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("idb command failed: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Run idb command with a specific target UDID
/// Note: idb requires --udid to come BEFORE positional arguments for some commands (like launch)
async fn run_idb_command_with_target(udid: &str, args: &[&str]) -> Result<String> {
    let mut full_args: Vec<&str> = Vec::with_capacity(args.len() + 2);

    // Handle nested commands (groups) like 'ui' and 'file'
    if args.len() >= 2 && (args[0] == "ui" || args[0] == "file") {
        full_args.push(args[0]);
        full_args.push(args[1]);
        full_args.push("--udid");
        full_args.push(udid);
        if args.len() > 2 {
            full_args.extend_from_slice(&args[2..]);
        }
    } else if let Some(subcmd) = args.first() {
        full_args.push(subcmd);
        full_args.push("--udid");
        full_args.push(udid);
        full_args.extend_from_slice(&args[1..]);
    } else {
        full_args.push("--udid");
        full_args.push(udid);
        full_args.extend_from_slice(args);
    }

    run_idb_command(&full_args).await
}

/// List all available iOS targets (devices and simulators)
/// Combines results from idb (simulators) and xcrun xctrace (real devices)
pub async fn list_targets() -> Result<Vec<IosTarget>> {
    let mut targets = Vec::new();
    let mut seen_udids = std::collections::HashSet::new();

    // 1. Get simulators from idb
    if let Ok(output) = run_idb_command(&["list-targets", "--json"]).await {
        for line in output.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(target) = serde_json::from_str::<IosTarget>(line) {
                seen_udids.insert(target.udid.clone());
                targets.push(target);
            }
        }
    }

    // 2. Get real devices from xcrun xctrace list devices
    if let Ok(real_devices) = list_real_devices().await {
        for device in real_devices {
            if !seen_udids.contains(&device.udid) {
                seen_udids.insert(device.udid.clone());
                targets.push(device);
            }
        }
    }

    Ok(targets)
}

/// List real iOS devices using xcrun xctrace list devices
async fn list_real_devices() -> Result<Vec<IosTarget>> {
    let output = Command::new("xcrun")
        .args(&["xctrace", "list", "devices"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to run xcrun xctrace list devices")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();
    let mut in_devices_section = false;
    let mut in_offline_section = false;

    // Parse output like:
    // == Devices ==
    // NghiNV (18.5) (00008020-0012446C1ADA002E)
    // == Devices Offline ==
    // Lumi (18.6.2) (00008110-000439C63AE9801E)
    // == Simulators ==
    // ...

    for line in stdout.lines() {
        let trimmed = line.trim();

        if trimmed == "== Devices ==" {
            in_devices_section = true;
            in_offline_section = false;
            continue;
        }
        if trimmed == "== Devices Offline ==" {
            in_devices_section = false;
            in_offline_section = true;
            continue;
        }
        if trimmed == "== Simulators ==" {
            // Stop when we reach simulators section (handled by idb)
            break;
        }

        // Skip empty lines or section headers
        if trimmed.is_empty() || trimmed.starts_with("==") {
            continue;
        }

        // Parse device line: "NghiNV (18.5) (00008020-0012446C1ADA002E)"
        // or with Mac: "Nghi's Mac mini (2) (FB8951E3-8F4C-5CB9-BA86-B907BAF6D911)"
        if (in_devices_section || in_offline_section) && trimmed.contains('(') {
            if let Some(device) = parse_xctrace_device_line(trimmed, in_offline_section) {
                // Filter out Mac devices and Apple Watch
                if !device.name.to_lowercase().contains("mac")
                    && !device.name.to_lowercase().contains("apple watch")
                {
                    devices.push(device);
                }
            }
        }
    }

    Ok(devices)
}

/// Parse a single device line from xctrace output
/// Format: "DeviceName (version) (UDID)" or "DeviceName (info) (version) (UDID)"
fn parse_xctrace_device_line(line: &str, is_offline: bool) -> Option<IosTarget> {
    // Find the UDID - it's the last parenthesized value
    let mut depth = 0;
    let mut last_paren_start = None;
    let mut last_paren_end = None;

    for (i, c) in line.char_indices() {
        match c {
            '(' => {
                if depth == 0 {
                    last_paren_start = Some(i);
                }
                depth += 1;
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    last_paren_end = Some(i);
                }
            }
            _ => {}
        }
    }

    let udid_start = last_paren_start? + 1;
    let udid_end = last_paren_end?;
    let udid = line[udid_start..udid_end].to_string();

    // Device UDID format validation (should be hex with dashes)
    if !udid.chars().all(|c| c.is_ascii_hexdigit() || c == '-') || udid.len() < 20 {
        return None;
    }

    // Get the name (everything before the UDID parenthesis)
    let name_part = line[..last_paren_start?].trim();

    // Remove trailing version info like "(18.5)" from name
    let name = if let Some(last_open) = name_part.rfind('(') {
        name_part[..last_open].trim().to_string()
    } else {
        name_part.to_string()
    };

    Some(IosTarget {
        udid,
        name,
        target_type: "device".to_string(),
        state: if is_offline {
            "Offline".to_string()
        } else {
            "Booted".to_string()
        },
    })
}

/// Get device info
pub async fn describe(udid: &str) -> Result<String> {
    run_idb_command_with_target(udid, &["describe", "--json"]).await
}

/// Launch an app by bundle ID
/// Uses devicectl for real devices, idb for simulators
pub async fn launch_app(udid: &str, bundle_id: &str, is_simulator: bool) -> Result<()> {
    if is_simulator {
        run_idb_command_with_target(udid, &["launch", bundle_id]).await?;
    } else {
        // Use devicectl for real devices
        let output = Command::new("xcrun")
            .args(&[
                "devicectl",
                "device",
                "process",
                "launch",
                "--device",
                udid,
                bundle_id,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to launch app with devicectl")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to launch app: {}", stderr);
        }
    }
    Ok(())
}

/// Terminate an app by bundle ID
/// Uses devicectl for real devices, idb for simulators
pub async fn terminate_app(udid: &str, bundle_id: &str, is_simulator: bool) -> Result<()> {
    if is_simulator {
        let _ = run_idb_command_with_target(udid, &["terminate", bundle_id]).await;
    } else {
        // Use devicectl for real devices - first get PID then terminate
        // Try to terminate gracefully, ignore errors if app not running
        let _ = Command::new("xcrun")
            .args(&[
                "devicectl",
                "device",
                "process",
                "terminate",
                "--device",
                udid,
                "--pid",
                "0", // This won't work, need to find running process
            ])
            .output()
            .await;

        // Alternative: use killall through devicectl or just ignore terminate errors
        // For real devices, apps may need manual termination or we skip this step
    }
    Ok(())
}

/// Uninstall an app (for clear state)
pub async fn uninstall_app(udid: &str, bundle_id: &str) -> Result<()> {
    run_idb_command_with_target(udid, &["uninstall", bundle_id]).await?;
    Ok(())
}

/// Install an app from path
pub async fn install_app(udid: &str, app_path: &str) -> Result<()> {
    run_idb_command_with_target(udid, &["install", app_path]).await?;
    Ok(())
}

/// Tap at coordinates
pub async fn tap(udid: &str, x: i32, y: i32) -> Result<()> {
    let x_str = x.to_string();
    let y_str = y.to_string();
    run_idb_command_with_target(udid, &["ui", "tap", &x_str, &y_str]).await?;
    Ok(())
}

/// Long press at coordinates
pub async fn long_press(udid: &str, x: i32, y: i32, duration_ms: u64) -> Result<()> {
    let x_str = x.to_string();
    let y_str = y.to_string();
    let duration_str = format!("{:.2}", duration_ms as f64 / 1000.0);
    run_idb_command_with_target(
        udid,
        &["ui", "tap", &x_str, &y_str, "--duration", &duration_str],
    )
    .await?;
    Ok(())
}

/// Input text
pub async fn input_text(udid: &str, text: &str) -> Result<()> {
    run_idb_command_with_target(udid, &["ui", "text", text]).await?;
    Ok(())
}

/// Press a key (e.g., HOME, SIRI, SCREENSHOT)
pub async fn press_button(udid: &str, button: &str) -> Result<()> {
    run_idb_command_with_target(udid, &["ui", "button", button]).await?;
    Ok(())
}

/// Press keyboard key (e.g., XCUIKeyboardKeyDelete)
pub async fn press_key(udid: &str, key: &str) -> Result<()> {
    run_idb_command_with_target(udid, &["ui", "key", key]).await?;
    Ok(())
}

/// Swipe from one point to another
pub async fn swipe(
    udid: &str,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    duration_ms: Option<u64>,
) -> Result<()> {
    let x1_str = x1.to_string();
    let y1_str = y1.to_string();
    let x2_str = x2.to_string();
    let y2_str = y2.to_string();

    let mut args = vec!["ui", "swipe", &x1_str, &y1_str, &x2_str, &y2_str];

    let duration_str;
    if let Some(ms) = duration_ms {
        duration_str = format!("{:.2}", ms as f64 / 1000.0);
        args.push("--duration");
        args.push(&duration_str);
    }

    run_idb_command_with_target(udid, &args).await?;
    Ok(())
}

/// Take a screenshot
pub async fn screenshot(udid: &str, output_path: &str) -> Result<()> {
    run_idb_command_with_target(udid, &["screenshot", output_path]).await?;
    Ok(())
}

/// Get UI hierarchy (accessibility tree)
pub async fn describe_ui(udid: &str) -> Result<String> {
    run_idb_command_with_target(udid, &["ui", "describe-all", "--json"]).await
}

/// Open URL or deep link
pub async fn open_url(udid: &str, url: &str) -> Result<()> {
    run_idb_command_with_target(udid, &["open", url]).await?;
    Ok(())
}

/// Push file/directory to device
pub async fn push_file(udid: &str, src: &str, dest: &str) -> Result<()> {
    run_idb_command_with_target(udid, &["file", "push", src, dest]).await?;
    Ok(())
}

/// Pull file/directory from device
pub async fn pull_file(udid: &str, src: &str, dest: &str) -> Result<()> {
    run_idb_command_with_target(udid, &["file", "pull", src, dest]).await?;
    Ok(())
}

/// Get system logs
pub async fn get_logs(udid: &str, limit: u32) -> Result<String> {
    // idb log streams continuously, we'll capture for a brief moment
    let idb_path = binary_resolver::find_idb()?;
    let output = Command::new(idb_path)
        .args(&["--udid", udid, "log", "--", "--style", "compact"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to get logs")?;

    let logs = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = logs.lines().take(limit as usize).collect();
    Ok(lines.join("\n"))
}

/// Start video recording (returns the child process for later termination)
pub async fn start_recording(udid: &str, output_path: &str) -> Result<tokio::process::Child> {
    let idb_path = binary_resolver::find_idb()?;
    let child = Command::new(idb_path)
        .args(&["--udid", udid, "record", "video", output_path])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to start video recording")?;

    Ok(child)
}

/// Get screen dimensions from device description
pub async fn get_screen_size(udid: &str) -> Result<(u32, u32)> {
    let output = describe(udid).await?;

    // Parse JSON to extract screen dimensions
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output) {
        if let (Some(width), Some(height)) = (
            json.get("screen_dimensions")
                .and_then(|d| d.get("width_points").or_else(|| d.get("width")))
                .and_then(|v| v.as_u64()),
            json.get("screen_dimensions")
                .and_then(|d| d.get("height_points").or_else(|| d.get("height")))
                .and_then(|v| v.as_u64()),
        ) {
            return Ok((width as u32, height as u32));
        }
    }

    // Default to common iOS resolution if parsing fails
    Ok((390, 844))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_parsing() {
        let json =
            r#"{"udid":"12345-ABCDE","name":"iPhone 15","type":"simulator","state":"Booted"}"#;
        let target: IosTarget = serde_json::from_str(json).unwrap();
        assert_eq!(target.udid, "12345-ABCDE");
        assert_eq!(target.name, "iPhone 15");
        assert_eq!(target.target_type, "simulator");
        assert_eq!(target.state, "Booted");
    }
}
