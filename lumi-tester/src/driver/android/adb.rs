use crate::utils::binary_resolver;
use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::Command;

/// Represents an Android device
#[derive(Debug, Clone)]
pub struct Device {
    pub serial: String,
    pub state: String,
}

/// Get list of connected Android devices
pub async fn get_devices() -> Result<Vec<Device>> {
    let adb_path = binary_resolver::find_adb()?;
    let output = Command::new(adb_path)
        .args(["devices"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to execute adb devices")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stderr.is_empty() {
        println!("DEBUG: adb devices stderr:\n{}", stderr);
    }

    let mut devices = Vec::new();

    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            devices.push(Device {
                serial: parts[0].to_string(),
                state: parts[1].to_string(),
            });
        }
    }

    Ok(devices)
}

/// Execute an ADB shell command
pub async fn shell(serial: Option<&str>, cmd: &str) -> Result<String> {
    let mut args = Vec::new();

    if let Some(s) = serial {
        args.push("-s");
        args.push(s);
    }

    args.push("shell");
    args.push(cmd);

    let adb_path = binary_resolver::find_adb()?;
    let output = Command::new(adb_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .with_context(|| format!("Failed to execute: adb shell {}", cmd))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ADB shell command failed: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Execute a raw ADB command
pub async fn exec(serial: Option<&str>, args: &[&str]) -> Result<String> {
    let mut full_args = Vec::new();

    if let Some(s) = serial {
        full_args.push("-s");
        full_args.push(s);
    }

    full_args.extend_from_slice(args);

    let adb_path = binary_resolver::find_adb()?;
    let output = Command::new(adb_path)
        .args(&full_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .with_context(|| format!("Failed to execute: adb {:?}", full_args))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ADB command failed: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Execute ADB exec-out command (faster than shell for binary output)
/// This avoids file I/O on device and transfers data directly to stdout
pub async fn exec_out(serial: Option<&str>, cmd: &str) -> Result<String> {
    let mut args = Vec::new();

    if let Some(s) = serial {
        args.push("-s");
        args.push(s);
    }

    args.push("exec-out");
    args.push(cmd);

    let adb_path = binary_resolver::find_adb()?;
    let output = Command::new(adb_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .with_context(|| format!("Failed to execute: adb exec-out {}", cmd))?;

    // exec-out may not set exit status properly, check if we got output
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if stdout.is_empty() && !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ADB exec-out command failed: {}", stderr);
    }

    Ok(stdout)
}

/// Execute ADB exec-out command and return raw binary data
/// Use this for binary output like screenshots
pub async fn exec_out_binary(serial: Option<&str>, cmd: &str) -> Result<Vec<u8>> {
    let mut args = Vec::new();

    if let Some(s) = serial {
        args.push("-s");
        args.push(s);
    }

    args.push("exec-out");
    args.push(cmd);

    let adb_path = binary_resolver::find_adb()?;
    let output = Command::new(adb_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .with_context(|| format!("Failed to execute: adb exec-out {}", cmd))?;

    if output.stdout.is_empty() && !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ADB exec-out command failed: {}", stderr);
    }

    Ok(output.stdout)
}

/// Pull a file from device
pub async fn pull(serial: Option<&str>, remote: &str, local: &str) -> Result<()> {
    let mut args = Vec::new();

    if let Some(s) = serial {
        args.push("-s".to_string());
        args.push(s.to_string());
    }

    args.push("pull".to_string());
    args.push(remote.to_string());
    args.push(local.to_string());

    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let adb_path = binary_resolver::find_adb()?;
    let output = Command::new(adb_path)
        .args(&args_ref)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .with_context(|| format!("Failed to pull {} to {}", remote, local))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ADB pull failed: {}", stderr);
    }

    Ok(())
}

/// Install an APK on device
pub async fn install(serial: Option<&str>, apk_path: &str) -> Result<()> {
    let mut args = Vec::new();

    if let Some(s) = serial {
        args.push("-s".to_string());
        args.push(s.to_string());
    }

    args.push("install".to_string());
    args.push("-r".to_string()); // Replace existing
    args.push(apk_path.to_string());

    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let adb_path = binary_resolver::find_adb()?;
    let output = Command::new(adb_path)
        .args(&args_ref)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .with_context(|| format!("Failed to install {}", apk_path))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ADB install failed: {}", stderr);
    }

    Ok(())
}

/// Push a file to device
pub async fn push(serial: Option<&str>, local: &str, remote: &str) -> Result<()> {
    let mut args = Vec::new();

    if let Some(s) = serial {
        args.push("-s".to_string());
        args.push(s.to_string());
    }

    args.push("push".to_string());
    args.push(local.to_string());
    args.push(remote.to_string());

    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let adb_path = binary_resolver::find_adb()?;
    let output = Command::new(adb_path)
        .args(&args_ref)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .with_context(|| format!("Failed to push {} to {}", local, remote))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ADB push failed: {}", stderr);
    }

    Ok(())
}

/// Get screen resolution (handles rotation)
pub async fn get_screen_size(serial: Option<&str>) -> Result<(u32, u32)> {
    let output = shell(serial, "wm size").await?;

    // Parse "Physical size: 1080x1920" or similar
    let mut width: u32 = 1080;
    let mut height: u32 = 1920;

    for line in output.lines() {
        // Prefer Override size if set, otherwise Physical size
        if line.contains("Override size:") || line.contains("Physical size:") {
            if let Some(size_str) = line.split(':').nth(1) {
                let size_str = size_str.trim();
                let parts: Vec<&str> = size_str.split('x').collect();
                if parts.len() == 2 {
                    width = parts[0].trim().parse().unwrap_or(1080);
                    height = parts[1].trim().parse().unwrap_or(1920);
                    // If Override size found, use it and break
                    if line.contains("Override size:") {
                        break;
                    }
                }
            }
        }
    }

    // Check rotation to swap dimensions for landscape
    // mRotation=1 (90°) or mRotation=3 (270°) means landscape
    let rotation_output = shell(serial, "dumpsys window displays | grep mRotation")
        .await
        .unwrap_or_default();
    let is_landscape =
        rotation_output.contains("mRotation=1") || rotation_output.contains("mRotation=3");

    if is_landscape && height > width {
        // Swap for landscape
        Ok((height, width))
    } else {
        Ok((width, height))
    }
}
