//! Discovery/launch for the `lm-ios-tester` on-device agent, mirroring `wda_setup.rs`'s
//! structure (`ensure_wda_running`/`start_iproxy`/`scan_for_wda_host`) but for the new
//! agent's port and Xcode project instead of WDA's.

use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

const SCHEME: &str = "LumiIOSAgentRunner";

/// Locates the `lm-ios-tester/` checkout. Stage-A/dev-only: this project isn't packaged
/// for distribution yet (unlike `lm-android-tester.apk`, which is bundled/resolved via
/// `binary_resolver`), so this only works from within a checkout of this monorepo -
/// `LUMI_IOS_AGENT_PROJECT` overrides for any other layout.
fn find_agent_project() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("LUMI_IOS_AGENT_PROJECT") {
        let path = PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
    }

    let candidates = [
        std::env::current_dir().ok().map(|d| d.join("../lm-ios-tester")),
        std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|p| p.join("../../../../lm-ios-tester"))),
    ];
    for candidate in candidates.into_iter().flatten() {
        let project = candidate.join("LumiIOSAgent.xcodeproj");
        if project.exists() {
            return Some(candidate);
        }
    }
    None
}

/// `host_port` is the per-UDID local port (see `agent::agent_port_for`); `device_port`
/// is always the agent's fixed on-device listen port (`agent::DEFAULT_AGENT_PORT`) -
/// `iproxy`'s two port arguments are host and device respectively, they don't need to
/// match (and for a second concurrently-connected device, must not - see
/// `agent::agent_port_for`'s doc comment for why a shared host port is a real bug, not
/// just a theoretical one).
async fn start_iproxy(udid: &str, host_port: u16, device_port: u16) -> Result<tokio::process::Child> {
    let child = Command::new("iproxy")
        .args([&host_port.to_string(), &device_port.to_string(), "-u", udid])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(child)
}

async fn launch_agent(project_dir: &std::path::Path, udid: &str) -> Result<()> {
    let mut child = Command::new("xcodebuild")
        .current_dir(project_dir)
        .args([
            "test-without-building",
            "-project",
            "LumiIOSAgent.xcodeproj",
            "-scheme",
            SCHEME,
            "-destination",
            &format!("id={}", udid),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    // Deliberately not awaited: this process keeps running for as long as the agent
    // needs to stay alive (same "long-lived test run" shape as WDA's own launch, see
    // `wda_setup.rs::start_wda`'s Xcodebuild branch), so awaiting it here would block
    // forever instead of returning once the launch has been kicked off.
    tokio::spawn(async move {
        let _ = child.wait().await;
    });
    Ok(())
}

/// Ensure the agent is running and reachable at `localhost:port` (the caller-supplied,
/// per-UDID host port from `agent::agent_port_for` - NOT necessarily the agent's own
/// fixed on-device port), auto-starting it (port forward + `xcodebuild
/// test-without-building`) if it isn't. Returns `true` if the agent ends up reachable,
/// `false` if it couldn't be started (caller should fall back to WDA/idb - this must
/// never be the only path).
pub async fn ensure_agent_running(udid: &str, port: u16) -> bool {
    let client = super::agent::AgentClient::new("localhost", port);
    if client.is_ready().await {
        println!("{} lm-ios-tester agent already running on port {}", "✓".green(), port);
        return true;
    }

    let _ = start_iproxy(udid, port, super::agent::DEFAULT_AGENT_PORT).await;
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    if client.is_ready().await {
        println!("{} lm-ios-tester agent reachable via existing session", "✓".green());
        return true;
    }

    let Some(project_dir) = find_agent_project() else {
        return false;
    };

    println!("{} Starting lm-ios-tester agent...", "⏳".yellow());
    if launch_agent(&project_dir, udid).await.is_err() {
        return false;
    }

    for i in 0..60 {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        if client.is_ready().await {
            println!("{} lm-ios-tester agent started ({}s)", "✓".green(), i + 1);
            return true;
        }
    }
    false
}
