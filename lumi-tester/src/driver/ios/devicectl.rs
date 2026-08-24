//! Native `xcrun devicectl` (real devices) / `xcrun simctl` (simulators) wrappers.
//!
//! Replaces idb for everything that isn't UI automation (process/file/lifecycle
//! operations) - UI automation itself (tap/swipe/screenshot/hierarchy/text/launch)
//! goes through the `lm-ios-tester` agent (see `agent.rs`), which doesn't need idb or
//! WDA at all. idb's own device-management daemon (`idb_companion`) was found this
//! session to be flaky (needed a manual `idb connect` after every device reconnect) -
//! `devicectl`/`simctl` are Apple's own first-party tools and don't have that problem.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone, Deserialize)]
pub struct IosTarget {
    pub udid: String,
    pub name: String,
    #[serde(rename = "type")]
    pub target_type: String,
    pub state: String,
}

/// List real devices via `xcrun xctrace list devices` (unchanged from the previous
/// idb.rs implementation - this call was already idb-independent) and simulators via
/// `xcrun simctl list devices --json` (replaces idb's `list-targets --json`, which
/// depended on `idb_companion` being alive - `simctl` talks to CoreSimulator directly).
pub async fn list_targets() -> Result<Vec<IosTarget>> {
    let mut targets = Vec::new();
    let mut seen_udids = std::collections::HashSet::new();

    if let Ok(real_devices) = list_real_devices().await {
        for device in real_devices {
            seen_udids.insert(device.udid.clone());
            targets.push(device);
        }
    }

    if let Ok(sims) = list_simulators().await {
        for sim in sims {
            if !seen_udids.contains(&sim.udid) {
                seen_udids.insert(sim.udid.clone());
                targets.push(sim);
            }
        }
    }

    Ok(targets)
}

async fn list_simulators() -> Result<Vec<IosTarget>> {
    let output = Command::new("xcrun")
        .args(&["simctl", "list", "devices", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to run xcrun simctl list devices")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let mut targets = Vec::new();
    if let Some(devices_by_runtime) = json.get("devices").and_then(|d| d.as_object()) {
        for devices in devices_by_runtime.values() {
            let Some(list) = devices.as_array() else { continue };
            for d in list {
                let is_available = d
                    .get("isAvailable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if !is_available {
                    continue;
                }
                let udid = d.get("udid").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let name = d.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let state = d.get("state").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if udid.is_empty() {
                    continue;
                }
                targets.push(IosTarget {
                    udid,
                    name,
                    target_type: "simulator".to_string(),
                    state,
                });
            }
        }
    }
    Ok(targets)
}

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
            break;
        }
        if trimmed.is_empty() || trimmed.starts_with("==") {
            continue;
        }

        if (in_devices_section || in_offline_section) && trimmed.contains('(') {
            if let Some(device) = parse_xctrace_device_line(trimmed, in_offline_section) {
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

fn parse_xctrace_device_line(line: &str, is_offline: bool) -> Option<IosTarget> {
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

    if !udid.chars().all(|c| c.is_ascii_hexdigit() || c == '-') || udid.len() < 20 {
        return None;
    }

    let name_part = line[..last_paren_start?].trim();
    let name = if let Some(last_open) = name_part.rfind('(') {
        name_part[..last_open].trim().to_string()
    } else {
        name_part.to_string()
    };

    Some(IosTarget {
        udid,
        name,
        target_type: "device".to_string(),
        state: if is_offline { "Offline".to_string() } else { "Booted".to_string() },
    })
}

pub async fn install_app(udid: &str, app_path: &str, is_simulator: bool) -> Result<()> {
    let output = if is_simulator {
        Command::new("xcrun")
            .args(&["simctl", "install", udid, app_path])
            .output()
            .await
            .context("Failed to run xcrun simctl install")?
    } else {
        Command::new("xcrun")
            .args(&["devicectl", "device", "install", "app", "--device", udid, app_path])
            .output()
            .await
            .context("Failed to run xcrun devicectl device install app")?
    };
    if !output.status.success() {
        anyhow::bail!("Install failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

pub async fn uninstall_app(udid: &str, bundle_id: &str, is_simulator: bool) -> Result<()> {
    let output = if is_simulator {
        Command::new("xcrun")
            .args(&["simctl", "uninstall", udid, bundle_id])
            .output()
            .await
            .context("Failed to run xcrun simctl uninstall")?
    } else {
        Command::new("xcrun")
            .args(&["devicectl", "device", "uninstall", "app", "--device", udid, bundle_id])
            .output()
            .await
            .context("Failed to run xcrun devicectl device uninstall app")?
    };
    if !output.status.success() {
        anyhow::bail!("Uninstall failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

/// Launch by bundle id. Only used when the on-device agent isn't available (simulator,
/// or agent process not running) - on a real device with the agent up, prefer
/// `AgentClient::launch_app`, which uses `[XCUIApplication launch]` and is confirmed
/// reliable (unlike `idb launch`, which failed outright on iOS 26.5.2 with a
/// DeveloperDiskImage version mismatch).
pub async fn launch_app(udid: &str, bundle_id: &str, is_simulator: bool) -> Result<()> {
    let output = if is_simulator {
        Command::new("xcrun")
            .args(&["simctl", "launch", udid, bundle_id])
            .output()
            .await
            .context("Failed to run xcrun simctl launch")?
    } else {
        Command::new("xcrun")
            .args(&[
                "devicectl", "device", "process", "launch", "--terminate-existing",
                "--device", udid, bundle_id,
            ])
            .output()
            .await
            .context("Failed to run xcrun devicectl device process launch")?
    };
    if !output.status.success() {
        anyhow::bail!("Launch failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

pub async fn terminate_app(udid: &str, bundle_id: &str, is_simulator: bool) -> Result<()> {
    if is_simulator {
        let _ = Command::new("xcrun")
            .args(&["simctl", "terminate", udid, bundle_id])
            .output()
            .await;
        return Ok(());
    }
    // devicectl's process-terminate needs a PID, which requires a separate
    // `process list` lookup - the agent's `terminate_app` (`[XCUIApplication terminate]`)
    // is the reliable real-device path and doesn't need this at all; this is only hit
    // as a last-resort fallback when the agent is unavailable, so best-effort is fine.
    if let Ok(list_output) = Command::new("xcrun")
        .args(&["devicectl", "device", "info", "processes", "--device", udid])
        .output()
        .await
    {
        let text = String::from_utf8_lossy(&list_output.stdout);
        if let Some(pid) = text
            .lines()
            .find(|l| l.contains(bundle_id))
            .and_then(|l| l.split_whitespace().next())
        {
            let _ = Command::new("xcrun")
                .args(&["devicectl", "device", "process", "terminate", "--device", udid, "--pid", pid])
                .output()
                .await;
        }
    }
    Ok(())
}

/// Push a file into an app's data container. Real devices use
/// `devicectl device copy to --domain-type appDataContainer --domain-identifier
/// <bundle_id>` (verified syntax via `xcrun devicectl device copy to --help` on this
/// machine); simulators are just local processes with a normal filesystem path, so a
/// plain `cp` into the container `simctl get_app_container` reports is simpler and more
/// reliable than shelling out to another Apple tool for something `cp` already does.
pub async fn push_file(
    udid: &str,
    bundle_id: &str,
    source: &str,
    dest: &str,
    is_simulator: bool,
) -> Result<()> {
    if is_simulator {
        let container = simctl_app_container(udid, bundle_id, "data").await?;
        let dest_path = std::path::Path::new(&container).join(dest.trim_start_matches('/'));
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        tokio::fs::copy(source, &dest_path).await.context("Failed to copy file into simulator app container")?;
        return Ok(());
    }
    let output = Command::new("xcrun")
        .args(&[
            "devicectl", "device", "copy", "to", "--device", udid,
            "--source", source, "--destination", dest,
            "--domain-type", "appDataContainer", "--domain-identifier", bundle_id,
        ])
        .output()
        .await
        .context("Failed to run xcrun devicectl device copy to")?;
    if !output.status.success() {
        anyhow::bail!("push_file failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

pub async fn pull_file(
    udid: &str,
    bundle_id: &str,
    source: &str,
    dest: &str,
    is_simulator: bool,
) -> Result<()> {
    if is_simulator {
        let container = simctl_app_container(udid, bundle_id, "data").await?;
        let source_path = std::path::Path::new(&container).join(source.trim_start_matches('/'));
        tokio::fs::copy(&source_path, dest).await.context("Failed to copy file from simulator app container")?;
        return Ok(());
    }
    let output = Command::new("xcrun")
        .args(&[
            "devicectl", "device", "copy", "from", "--device", udid,
            "--source", source, "--destination", dest,
            "--domain-type", "appDataContainer", "--domain-identifier", bundle_id,
        ])
        .output()
        .await
        .context("Failed to run xcrun devicectl device copy from")?;
    if !output.status.success() {
        anyhow::bail!("pull_file failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

async fn simctl_app_container(udid: &str, bundle_id: &str, kind: &str) -> Result<String> {
    let output = Command::new("xcrun")
        .args(&["simctl", "get_app_container", udid, bundle_id, kind])
        .output()
        .await
        .context("Failed to run xcrun simctl get_app_container")?;
    if !output.status.success() {
        anyhow::bail!("get_app_container failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Open a URL/deep link. `simctl openurl` handles this cleanly on simulators. No
/// devicectl equivalent exists for real devices (`devicectl device process launch`
/// passes plain command-line arguments to the process, not a URL through
/// `application(_:open:options:)` the way idb's `open <url>` used to) - real-device
/// deep-link opening is a genuine, honest gap, not silently faked.
pub async fn open_url(udid: &str, url: &str, is_simulator: bool) -> Result<()> {
    if is_simulator {
        let output = Command::new("xcrun")
            .args(&["simctl", "openurl", udid, url])
            .output()
            .await
            .context("Failed to run xcrun simctl openurl")?;
        if !output.status.success() {
            anyhow::bail!("openurl failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        return Ok(());
    }
    anyhow::bail!(
        "open_link is not supported on physical iOS devices without idb (no devicectl \
         equivalent for launching an app with a URL/deep-link payload exists)"
    )
}

pub async fn screenshot_simulator(udid: &str, output_path: &str) -> Result<()> {
    let output = Command::new("xcrun")
        .args(&["simctl", "io", udid, "screenshot", output_path])
        .output()
        .await
        .context("Failed to run xcrun simctl io screenshot")?;
    if !output.status.success() {
        anyhow::bail!("screenshot failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
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
