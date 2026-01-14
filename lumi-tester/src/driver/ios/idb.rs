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
pub async fn list_targets() -> Result<Vec<IosTarget>> {
    let output = run_idb_command(&["list-targets", "--json"]).await?;

    // Parse JSON lines output
    let mut targets = Vec::new();
    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(target) = serde_json::from_str::<IosTarget>(line) {
            targets.push(target);
        }
    }

    Ok(targets)
}

/// Get device info
pub async fn describe(udid: &str) -> Result<String> {
    run_idb_command_with_target(udid, &["describe", "--json"]).await
}

/// Launch an app by bundle ID
pub async fn launch_app(udid: &str, bundle_id: &str) -> Result<()> {
    run_idb_command_with_target(udid, &["launch", bundle_id]).await?;
    Ok(())
}

/// Terminate an app by bundle ID
pub async fn terminate_app(udid: &str, bundle_id: &str) -> Result<()> {
    run_idb_command_with_target(udid, &["terminate", bundle_id]).await?;
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
