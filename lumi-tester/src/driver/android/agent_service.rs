//! lm-android-tester agent service management
//!
//! This module manages the lm-android-tester helper (a dedicated, compact automation-speed
//! agent - not a general-purpose screen mirroring tool) on Android devices. It handles
//! deployment, startup, and port forwarding for the fast UI-hierarchy-dump / tap / wait-idle
//! / mock-location paths used by the Android driver.
//!
//! Runs via `adb shell app_process ... dev.lm.tester.core.App` under the `shell` UID
//! (same technique as `uiautomator`/scrcpy) - no `pm install` required. Uses its own port
//! (7899) distinct from the general-purpose `nl-mirror` screen-mirroring app (port 8889), so
//! both can run on the same device without colliding.

use crate::driver::android::adb;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// On-device path for the agent APK
const DEVICE_APK_PATH: &str = "/data/local/tmp/lm-android-tester.apk";

/// Local bundled APK filename (checked in `resources/apk/`)
const LOCAL_APK_NAME: &str = "lm-android-tester.apk";

/// Agent command server port the on-device agent itself listens on (hardcoded in the
/// agent's own source, `App.kt`'s `COMMAND_PORT` - this side can't vary per device).
/// Deliberately different from nl-mirror's 8889 so the two can coexist on the same
/// device (e.g. a human mirroring the screen while a test runs).
const AGENT_PORT: u16 = 7899;

/// Deterministic per-device HOST-side local port for `adb forward`. `adb forward`'s
/// device side is fixed by the agent APK (always `AGENT_PORT`), but the HOST side is
/// just a local TCP listener - and a single shared host port for every device meant
/// two devices' forwards silently fought over the same port (whichever device last
/// ran `adb forward` "owned" it), so an agent-mediated command (screenshot, tap,
/// hierarchy dump, hideKeyboard's visibility check) issued for device A while device
/// B's forward was the most recently established one would silently execute against
/// device B instead - reproduced directly: a `snapshot` targeted at one device came
/// back showing another device's screen entirely. Hashing the serial into a
/// per-device host port removes the collision. Must be a fixed, process-independent
/// hash (not `DefaultHasher`, whose `RandomState` seed varies per process) since two
/// *separate* `lumi-tester` invocations for the same serial must derive the same
/// port to usefully share one forward.
pub fn agent_port_for(serial: Option<&str>) -> u16 {
    match serial {
        Some(s) => {
            const BASE: u32 = 20000;
            const RANGE: u32 = 10000;
            BASE as u16 + (fnv1a_hash(s.as_bytes()) % RANGE) as u16
        }
        // No serial (single implicit device) - keep the original fixed port so
        // existing single-device behavior/logs are unchanged.
        None => AGENT_PORT,
    }
}

/// Minimal stable FNV-1a hash (32-bit). Deterministic across processes/runs, unlike
/// `std::collections::hash_map::DefaultHasher`.
fn fnv1a_hash(bytes: &[u8]) -> u32 {
    const FNV_OFFSET_BASIS: u32 = 0x811c9dc5;
    const FNV_PRIME: u32 = 0x01000193;
    let mut hash = FNV_OFFSET_BASIS;
    for &b in bytes {
        hash ^= b as u32;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Set once the first time the agent turns out to be unavailable, so the "falling back to
/// the slower adb-based path" diagnostic below is only ever printed once per process
/// instead of spamming stderr on every driver instance / retry.
static WARNED_UNAVAILABLE: AtomicBool = AtomicBool::new(false);

/// AgentService manages the lm-android-tester helper on Android devices
pub struct AgentService;

impl AgentService {
    /// Get the path to the agent APK.
    /// Checks CWD-relative `resources/apk/` first (matches local dev/build layout), then
    /// falls back to the bundled-app/install-dir resolution used for other bundled APKs
    /// (see `binary_resolver::find_apk`) - otherwise this silently fails whenever the CLI
    /// isn't invoked from the crate root, which used to mean the agent (and with it, the
    /// fast UI-hierarchy-dump path) would silently never activate.
    pub fn find_apk_path() -> Option<PathBuf> {
        if let Ok(cwd) = std::env::current_dir() {
            let apk_path = cwd.join("resources/apk").join(LOCAL_APK_NAME);
            if apk_path.exists() {
                return Some(apk_path);
            }
        }

        crate::utils::binary_resolver::find_apk(LOCAL_APK_NAME)
    }

    /// Report (once per process) that the fast agent path is unavailable and why, so users
    /// aren't left guessing why a run is slower than expected. Safe to call repeatedly -
    /// only the first call actually prints.
    fn warn_unavailable_once(reason: &str) {
        if WARNED_UNAVAILABLE
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            eprintln!(
                "  ⚠️ lm-android-tester agent unavailable ({}). Falling back to the slower \
                 adb-based path (uiautomator dump / adb shell input) for this run. \
                 Build it with `cd lm-android-tester && ./gradlew assembleDebug` and copy the \
                 APK to `resources/apk/{}` to enable the fast path.",
                reason, LOCAL_APK_NAME
            );
        }
    }

    /// Check if the agent is running on the device
    pub async fn is_running(serial: Option<&str>) -> bool {
        let cmd = "pgrep -f 'dev.lm.tester.core.App' 2>/dev/null || true";
        match adb::shell(serial, cmd).await {
            Ok(output) => !output.trim().is_empty(),
            Err(_) => false,
        }
    }

    /// Verify connectivity to the agent's command port by sending a ping command
    pub async fn verify_connection(serial: Option<&str>) -> bool {
        use std::io::{Read, Write};

        let addr = format!("127.0.0.1:{}", agent_port_for(serial));
        match std::net::TcpStream::connect_timeout(
            &addr.parse().unwrap(),
            std::time::Duration::from_millis(500),
        ) {
            Ok(mut stream) => {
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(500)));
                let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(500)));

                // Send ping command
                if stream.write_all(b"{\"cmd\":\"ping\"}\n").is_err() {
                    return false;
                }

                // Must receive a response to confirm server is actually processing commands
                let mut buf = [0u8; 256];
                match stream.read(&mut buf) {
                    Ok(n) if n > 0 => true,
                    _ => {
                        // No response = server is stuck/zombie, force restart
                        false
                    }
                }
            }
            Err(_) => false,
        }
    }

    /// Get the APK file size on device (0 if not exists)
    /// SHA-256 of the on-device APK, or `None` if it doesn't exist / can't be read.
    async fn get_device_apk_hash(serial: Option<&str>) -> Option<String> {
        let cmd = format!("sha256sum {} 2>/dev/null", DEVICE_APK_PATH);
        let output = adb::shell(serial, &cmd).await.ok()?;
        // `sha256sum` output is "<hash>  <path>" - take the first whitespace-delimited field.
        output.split_whitespace().next().map(|s| s.to_string())
    }

    /// SHA-256 of the local APK file.
    fn get_local_apk_hash(local_apk: &Path) -> Result<String> {
        use sha2::{Digest, Sha256};
        let bytes = std::fs::read(local_apk)
            .map_err(|e| anyhow!("Failed to read local APK: {}", e))?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Deploy the agent APK to the device if needed.
    /// Returns true if APK was pushed, false if already up to date
    pub async fn deploy_if_needed(serial: Option<&str>, local_apk: &Path) -> Result<bool> {
        let local_size = std::fs::metadata(local_apk)
            .map_err(|e| anyhow!("Failed to read APK metadata: {}", e))?
            .len();

        // Compare content hashes, not just file size - two different builds can easily
        // land on the exact same byte count (observed directly: two builds of this APK,
        // 846638 bytes each, with different SHA-256 hashes because the actual bytecode
        // differed). A size-only check would silently skip redeploying a real code change
        // whenever that coincidence occurs, leaving a stale agent running on-device with
        // no indication anything was wrong - exactly the kind of thing that must not
        // happen when "deployed" is supposed to mean the fast path is trustworthy.
        let local_hash = Self::get_local_apk_hash(local_apk)?;
        let device_hash = Self::get_device_apk_hash(serial).await;

        if device_hash.as_deref() == Some(local_hash.as_str()) {
            eprintln!("  ✓ lm-android-tester agent already up to date on device");
            return Ok(false);
        }

        if device_hash.is_none() {
            eprintln!("  📦 lm-android-tester agent not found on device, deploying ({} bytes)...", local_size);
        } else {
            eprintln!("  📦 Deploying updated lm-android-tester agent ({} bytes)...", local_size);
        }

        // Push APK to device
        let apk_path_str = local_apk.to_string_lossy();
        let serial_args: Vec<String> = match serial {
            Some(s) => vec!["-s".to_string(), s.to_string()],
            None => vec![],
        };

        let mut args = serial_args;
        args.extend([
            "push".to_string(),
            apk_path_str.to_string(),
            DEVICE_APK_PATH.to_string(),
        ]);

        let output = tokio::process::Command::new("adb")
            .args(&args)
            .output()
            .await
            .map_err(|e| anyhow!("Failed to push APK: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to push APK: {}", stderr));
        }

        eprintln!("  ✓ lm-android-tester agent deployed");
        Ok(true)
    }

    /// Stop the agent if running
    pub async fn stop(serial: Option<&str>) -> Result<()> {
        let _ = adb::shell(
            serial,
            "pkill -9 -f 'app_process.*lm-android-tester' 2>/dev/null || true",
        )
        .await;
        let _ = adb::shell(
            serial,
            "pkill -9 -f 'dev.lm.tester' 2>/dev/null || true",
        )
        .await;
        Ok(())
    }

    /// Start the agent
    pub async fn start(serial: Option<&str>) -> Result<()> {
        // Stop any existing instance first
        Self::stop(serial).await?;

        eprintln!("  🚀 Starting lm-android-tester agent...");

        // Start server in background
        // Use sh -c with & to detach from adb
        let cmd = format!(
            "sh -c 'CLASSPATH={} app_process / dev.lm.tester.core.App >/dev/null 2>&1 &'",
            DEVICE_APK_PATH
        );

        let _ = adb::shell(serial, &cmd).await;

        // Wait for startup (up to 2 seconds)
        for i in 0..10 {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            if Self::is_running(serial).await {
                eprintln!("  ✓ lm-android-tester agent started ({}ms)", (i + 1) * 200);
                return Ok(());
            }
        }

        Err(anyhow!("lm-android-tester agent failed to start"))
    }

    /// Setup ADB port forwarding for the agent's command port. Host side is a
    /// per-serial port (see `agent_port_for`); device side is always `AGENT_PORT`,
    /// the fixed port the agent APK itself listens on.
    pub async fn setup_port_forward(serial: Option<&str>) -> Result<()> {
        let host_port = agent_port_for(serial);
        let serial_args: Vec<String> = match serial {
            Some(s) => vec!["-s".to_string(), s.to_string()],
            None => vec![],
        };

        let mut args = serial_args;
        args.extend([
            "forward".to_string(),
            format!("tcp:{}", host_port),
            format!("tcp:{}", AGENT_PORT),
        ]);

        let output = tokio::process::Command::new("adb")
            .args(&args)
            .output()
            .await
            .map_err(|e| anyhow!("Failed to forward port {}: {}", host_port, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Failed to forward port {}: {}",
                host_port,
                stderr
            ));
        }

        Ok(())
    }

    /// Initialize an agent session: deploy if needed, start service, setup port forward.
    /// On any failure, reports (once per process) why the fast path is unavailable.
    pub async fn init_session(serial: Option<&str>) -> Result<()> {
        let result = Self::init_session_inner(serial).await;
        if let Err(ref e) = result {
            Self::warn_unavailable_once(&e.to_string());
        }
        result
    }

    async fn init_session_inner(serial: Option<&str>) -> Result<()> {
        // 1. Find APK - this is the "not installed / not bundled" check: if the local APK
        // can't be found at all, there's nothing to deploy and the fast path is skipped.
        let apk_path = Self::find_apk_path().ok_or_else(|| {
            anyhow!(
                "agent APK not found locally (checked resources/apk/{} and bundled resources)",
                LOCAL_APK_NAME
            )
        })?;

        // 2. Deploy APK if needed (this itself detects "device doesn't have it yet" via a
        // 0-byte device-side size and pushes it automatically)
        let apk_updated = Self::deploy_if_needed(serial, &apk_path).await?;

        // 3. If APK was updated, force restart to load new code
        if apk_updated {
            eprintln!("  🔄 Agent updated, restarting...");
            Self::start(serial).await?;
            Self::setup_port_forward(serial).await?;
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;

            if Self::verify_connection(serial).await {
                eprintln!("  ✓ Agent restarted with new APK");
                return Ok(());
            } else {
                return Err(anyhow!("agent failed to start after APK update"));
            }
        }

        // 4. Check if the agent is already running and reachable
        let is_process_running = Self::is_running(serial).await;
        let mut is_reachable = if is_process_running {
            Self::verify_connection(serial).await
        } else {
            false
        };

        // 5. If not reachable, try to re-establish port forward first. This is far
        // cheaper than a full restart below (~200-400ms vs ~1-2s: stop+start+poll) and is
        // the actual fix for the common case - `adb forward` mappings can go stale
        // between separate `lumi-tester run` invocations (confirmed on-device: the same
        // still-running, perfectly healthy agent process answered instantly once the
        // forward was simply re-established) without the agent process itself being
        // affected at all. Retries a couple of times before falling back to a full
        // restart, in case the forward needs a moment to actually take effect.
        if !is_reachable {
            eprintln!("  🔄 Setting up port forward...");
            for attempt in 0..3 {
                if let Err(e) = Self::setup_port_forward(serial).await {
                    eprintln!("  ⚠️ Port forward failed: {}", e);
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                is_reachable = Self::verify_connection(serial).await;
                if is_reachable || attempt == 2 {
                    break;
                }
            }
        }

        // 6. If still not reachable, start/restart the service
        if !is_reachable {
            if is_process_running {
                eprintln!("  ⚠️ Agent process exists but unreachable. Restarting...");
            }
            Self::start(serial).await?;

            // Verify again after start
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if !Self::verify_connection(serial).await {
                // One more attempt: re-establish port forward after service start
                eprintln!("  🔄 Re-establishing port forward after service start...");
                Self::setup_port_forward(serial).await?;
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;

                if !Self::verify_connection(serial).await {
                    return Err(anyhow!(
                        "agent started but port {} is not reachable",
                        agent_port_for(serial)
                    ));
                }
            }
        } else {
            eprintln!("  ✓ Agent already running");
        }

        Ok(())
    }
}
