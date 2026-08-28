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

/// List real devices via `xcrun devicectl list devices` (see `list_real_devices`'s doc
/// comment - replaces a much slower `xctrace`-based implementation) and simulators via
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

/// `xcrun devicectl list devices` (JSON) - replaces a previous `xcrun xctrace list
/// devices` implementation that measured at ~2.6s per call (confirmed live,
/// repeatedly) vs. this one's ~100-200ms. `xctrace` is Instruments' own device
/// discovery, which does noticeably more probing than `devicectl`'s (CoreDevice,
/// Apple's current first-party device-management stack, already used elsewhere in
/// this file for install/uninstall/push/pull/terminate). Since `IosDriver::new`
/// calls this on *every* `lumi-tester` invocation just to resolve a device by UDID,
/// that 2.5s difference was pure per-command overhead - the exact "every command
/// feels slow, worse than idb" symptom this was found chasing down.
async fn list_real_devices() -> Result<Vec<IosTarget>> {
    let json_path = std::env::temp_dir().join(format!("lumi_devicectl_{}.json", std::process::id()));
    let output = Command::new("xcrun")
        .args([
            "devicectl",
            "list",
            "devices",
            "--json-output",
            json_path.to_string_lossy().as_ref(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await
        .context("Failed to run xcrun devicectl list devices")?;

    if !output.status.success() {
        let _ = std::fs::remove_file(&json_path);
        return Ok(Vec::new());
    }

    let raw = std::fs::read_to_string(&json_path);
    let _ = std::fs::remove_file(&json_path);
    let Ok(raw) = raw else {
        return Ok(Vec::new());
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Ok(Vec::new());
    };

    let mut devices = Vec::new();
    if let Some(list) = parsed["result"]["devices"].as_array() {
        for dev in list {
            let name = dev["deviceProperties"]["name"].as_str().unwrap_or("").to_string();
            // `identifier` (top-level) is devicectl's own internal UUID, NOT the
            // classic ECID-derived UDID (e.g. "00008101-...") every other tool in
            // this codebase (idb historically, `xcrun devicectl device install`'s
            // own `--device` flag, users' saved device IDs) actually uses - that
            // one's under `hardwareProperties.udid` instead. Using the wrong one
            // here silently broke every existing `--device <udid>` invocation
            // ("Device with UDID ... not found") since nothing else in the
            // codebase would ever produce or accept devicectl's internal UUID.
            let udid = dev["hardwareProperties"]["udid"].as_str().unwrap_or("").to_string();
            if name.is_empty() || udid.is_empty() {
                continue;
            }
            let lower = name.to_lowercase();
            if lower.contains("mac") || lower.contains("apple watch") {
                continue;
            }
            // Same "Booted"/"Offline" convention `IosDriver::new`'s no-UDID-given
            // fallback picker already expects (borrowed from simctl's simulator
            // terminology, kept for real devices too rather than introducing a
            // second state vocabulary the picker would also need to understand).
            let connected = dev["connectionProperties"]["tunnelState"].as_str() == Some("connected");
            devices.push(IosTarget {
                udid,
                name,
                target_type: "device".to_string(),
                state: if connected { "Booted".to_string() } else { "Offline".to_string() },
            });
        }
    }

    Ok(devices)
}

/// Best-effort detection of whichever third-party app is currently on screen on a
/// real device, so the Inspector can auto-attach instead of requiring a manual pick
/// (unlike macOS/Windows, a mobile device only ever shows one app at a time). There
/// is no public API for "the frontmost app" on an unjailbroken device, so this
/// combines two `devicectl` calls:
/// - `device info apps --include-all-apps` (the plain, no-flags version only lists
///   apps *built by the connected Mac* - excludes TestFlight/App Store installs like
///   the app actually under test) gives every installed app's bundle identifier and
///   container `url`, e.g. `.../Application/<uuid>/GOFA.app/`.
/// - `device info processes` gives every *running* process's executable path,
///   `.../Application/<uuid>/GOFA.app/GOFA` for an app - the same `<uuid>` ties the
///   two together, which is why this matches on that instead of the product/binary
///   name: Flutter apps (e.g. Lumi Life+) all share the literal binary name
///   `Runner`, so name-matching would confuse every Flutter app under test with
///   every other one installed on the device.
/// A backgrounded-but-not-yet-evicted app still shows up as "running" here (e.g.
/// switching apps via a deep link leaves the previous one resident), so when
/// several installed-app processes are alive at once this picks the one with the
/// highest PID - empirically the most-recently-launched-or-resumed process, i.e.
/// the one actually on screen, confirmed live: switching from Lumi Life+ into GOFA
/// via deep link left both processes running, with GOFA's PID higher.
pub async fn get_foreground_app(udid: &str) -> Result<Option<String>> {
    let apps = fetch_devicectl_json(udid, &["device", "info", "apps", "--include-all-apps"]).await?;
    let Some(apps) = apps else { return Ok(None) };

    let uuid_re = regex::Regex::new(r"/Application/([0-9A-Fa-f-]{36})/").unwrap();
    let mut uuid_to_bundle: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Some(list) = apps["result"]["apps"].as_array() {
        for app in list {
            let (Some(url), Some(bundle_id)) = (app["url"].as_str(), app["bundleIdentifier"].as_str()) else {
                continue;
            };
            if bundle_id == "com.lumi.LumiIOSAgentRunner.xctrunner" {
                continue;
            }
            if let Some(caps) = uuid_re.captures(url) {
                uuid_to_bundle.insert(caps[1].to_string(), bundle_id.to_string());
            }
        }
    }
    if uuid_to_bundle.is_empty() {
        return Ok(None);
    }

    let procs = fetch_devicectl_json(udid, &["device", "info", "processes"]).await?;
    let Some(procs) = procs else { return Ok(None) };

    let mut best: Option<(i64, String)> = None;
    if let Some(list) = procs["result"]["runningProcesses"].as_array() {
        for p in list {
            let (Some(exe), Some(pid)) = (p["executable"].as_str(), p["processIdentifier"].as_i64()) else {
                continue;
            };
            let Some(caps) = uuid_re.captures(exe) else { continue };
            let Some(bundle_id) = uuid_to_bundle.get(&caps[1]) else { continue };
            if best.as_ref().is_none_or(|(best_pid, _)| pid > *best_pid) {
                best = Some((pid, bundle_id.clone()));
            }
        }
    }

    Ok(best.map(|(_, bundle_id)| bundle_id))
}

async fn fetch_devicectl_json(udid: &str, subcommand: &[&str]) -> Result<Option<serde_json::Value>> {
    let json_path = std::env::temp_dir().join(format!(
        "lumi_devicectl_{}_{}.json",
        subcommand.join("_"),
        std::process::id()
    ));
    let output = Command::new("xcrun")
        .arg("devicectl")
        .args(subcommand)
        .args(["--device", udid, "--json-output"])
        .arg(&json_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await
        .context("Failed to run xcrun devicectl")?;

    if !output.status.success() {
        let _ = std::fs::remove_file(&json_path);
        return Ok(None);
    }

    let raw = std::fs::read_to_string(&json_path);
    let _ = std::fs::remove_file(&json_path);
    let Ok(raw) = raw else { return Ok(None) };
    Ok(serde_json::from_str(&raw).ok())
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
