use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionCheckReport {
    pub cli_current: String,
    pub cli_latest: String,
    pub cli_update_available: bool,
    pub extension_current: Option<String>,
    pub extension_latest: String,
    pub extension_update_available: bool,
    pub target: String,
    pub asset_name: String,
    pub download_url: String,
    pub extension_download_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateOptions {
    pub repo: String,
    pub version: Option<String>,
    pub force: bool,
    pub check_only: bool,
    pub update_extension: bool,
    pub update_all: bool,
    pub json: bool,
}

#[derive(Debug, Deserialize)]
struct GithubReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubReleaseAsset>,
}

pub fn detect_target() -> Result<String> {
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        other => anyhow::bail!("Unsupported architecture: {}", other),
    };

    let os = match std::env::consts::OS {
        "macos" => "apple-darwin",
        "linux" => "unknown-linux-gnu",
        "windows" => "pc-windows-msvc",
        other => anyhow::bail!("Unsupported OS: {}", other),
    };

    Ok(format!("{}-{}", arch, os))
}

pub fn target_binary_name(target: &str) -> String {
    if target.contains("windows") {
        format!("lumi-tester-{}.exe", target)
    } else {
        format!("lumi-tester-{}", target)
    }
}

pub fn parse_version_tuple(v: &str) -> (u32, u32, u32) {
    let clean = v.trim().trim_start_matches("extension-v").trim_start_matches('v');
    let parts: Vec<u32> = clean
        .split('.')
        .filter_map(|p| p.parse::<u32>().ok())
        .collect();
    (
        *parts.first().unwrap_or(&0),
        *parts.get(1).unwrap_or(&0),
        *parts.get(2).unwrap_or(&0),
    )
}

pub fn is_newer_version(latest: &str, current: &str) -> bool {
    parse_version_tuple(latest) > parse_version_tuple(current)
}

pub async fn fetch_version_check(repo: &str) -> Result<VersionCheckReport> {
    let client = reqwest::Client::builder()
        .user_agent(format!("lumi-tester/{}", env!("CARGO_PKG_VERSION")))
        .build()?;

    let url = format!("https://api.github.com/repos/{}/releases", repo);
    let releases: Vec<GithubRelease> = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("Failed to query GitHub releases from {}", url))?
        .error_for_status()
        .with_context(|| "GitHub API returned an error status")?
        .json()
        .await
        .with_context(|| "Failed to parse GitHub releases response")?;

    let cli_release = releases
        .iter()
        .find(|r| r.tag_name.starts_with('v') && !r.tag_name.contains("extension"))
        .ok_or_else(|| anyhow!("No CLI release (v*) found in repository '{}'", repo))?;

    let extension_release = releases
        .iter()
        .find(|r| r.tag_name.starts_with("extension-v"));

    let target = detect_target()?;
    let asset_name = target_binary_name(&target);

    let cli_asset = cli_release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .map(|a| a.browser_download_url.clone())
        .unwrap_or_else(|| {
            format!(
                "https://github.com/{}/releases/download/{}/{}",
                repo, cli_release.tag_name, asset_name
            )
        });

    let (ext_tag, ext_asset_url) = if let Some(ext) = extension_release {
        let vsix_url = ext
            .assets
            .iter()
            .find(|a| a.name.ends_with(".vsix"))
            .map(|a| a.browser_download_url.clone());
        (ext.tag_name.clone(), vsix_url)
    } else {
        ("unknown".to_string(), None)
    };

    let cli_current = format!("v{}", env!("CARGO_PKG_VERSION"));
    let cli_latest = cli_release.tag_name.clone();
    let cli_update_available = is_newer_version(&cli_latest, &cli_current);

    let extension_current = detect_installed_extension_version();
    let extension_latest = ext_tag;
    let extension_update_available = match &extension_current {
        Some(cur) => is_newer_version(&extension_latest, cur),
        None => true,
    };

    Ok(VersionCheckReport {
        cli_current,
        cli_latest,
        cli_update_available,
        extension_current,
        extension_latest,
        extension_update_available,
        target,
        asset_name,
        download_url: cli_asset,
        extension_download_url: ext_asset_url,
    })
}

fn detect_installed_extension_version() -> Option<String> {
    let output = Command::new("code")
        .args(["--list-extensions", "--show-versions"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("lumi-tester") || line.contains("lumijsc.lumi-tester") {
            if let Some((_, ver)) = line.split_once('@') {
                return Some(ver.trim().to_string());
            }
        }
    }
    None
}

pub async fn run_update(options: UpdateOptions) -> Result<()> {
    let report = fetch_version_check(&options.repo).await?;

    if options.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!("\n{}", "🔍 Lumi Tester Version Status".cyan().bold());
    println!(
        "  • CLI Version:       {} (Latest: {}) -> {}",
        report.cli_current.yellow(),
        report.cli_latest.cyan(),
        if report.cli_update_available {
            "Update Available 🚀".green().bold()
        } else {
            "Up to date ✅".green()
        }
    );

    let ext_cur_display = report
        .extension_current
        .as_deref()
        .map(|v| format!("v{}", v))
        .unwrap_or_else(|| "Not detected / VS Code CLI not in PATH".to_string());
    println!(
        "  • VS Code Extension: {} (Latest: {}) -> {}",
        ext_cur_display.yellow(),
        report.extension_latest.cyan(),
        if report.extension_update_available {
            "Update Available 🚀".green().bold()
        } else {
            "Up to date ✅".green()
        }
    );
    println!("  • Platform Target:   {}", report.target.blue());

    if options.check_only {
        return Ok(());
    }

    let should_update_cli = (report.cli_update_available || options.force || options.version.is_some())
        && !options.update_extension;

    let should_update_ext = options.update_extension || options.update_all;

    if should_update_cli {
        let target_ver = options.version.unwrap_or(report.cli_latest.clone());
        let target = detect_target()?;
        let asset_name = target_binary_name(&target);
        let download_url = format!(
            "https://github.com/{}/releases/download/{}/{}",
            options.repo, target_ver, asset_name
        );

        println!("\n{}", format!("⬇️  Downloading Lumi Tester {} ({})...", target_ver, asset_name).cyan());
        println!("  From: {}", download_url.blue());

        let current_exe = std::env::current_exe().context("Could not determine current executable path")?;
        let parent_dir = current_exe.parent().unwrap_or(Path::new("."));
        let temp_file = parent_dir.join(format!(".lumi-tester-update-{}", uuid::Uuid::new_v4()));

        let client = reqwest::Client::builder()
            .user_agent(format!("lumi-tester/{}", env!("CARGO_PKG_VERSION")))
            .build()?;

        let response = client
            .get(&download_url)
            .send()
            .await
            .with_context(|| format!("Failed to download update binary from {}", download_url))?
            .error_for_status()
            .with_context(|| format!("Download returned error status from {}", download_url))?;

        let bytes = response.bytes().await?;
        tokio::fs::write(&temp_file, bytes)
            .await
            .with_context(|| format!("Failed to write temporary binary {}", temp_file.display()))?;

        make_executable(&temp_file)?;

        // Replace current executable
        replace_executable(&temp_file, &current_exe)?;

        println!(
            "{}",
            format!("✅ Successfully updated Lumi Tester CLI to {}!", target_ver)
                .green()
                .bold()
        );
        println!("  Executable: {}", current_exe.display().to_string().cyan());
    }

    if should_update_ext {
        if let Some(vsix_url) = report.extension_download_url {
            println!("\n{}", format!("⬇️  Downloading VS Code Extension ({})...", report.extension_latest).cyan());
            let temp_dir = std::env::temp_dir();
            let vsix_path = temp_dir.join(format!("lumi-tester-{}.vsix", report.extension_latest));

            let client = reqwest::Client::builder()
                .user_agent(format!("lumi-tester/{}", env!("CARGO_PKG_VERSION")))
                .build()?;

            let response = client.get(&vsix_url).send().await?.error_for_status()?;
            let bytes = response.bytes().await?;
            tokio::fs::write(&vsix_path, bytes).await?;

            println!("  Saved VSIX to: {}", vsix_path.display().to_string().blue());

            // Check if code CLI is available to install directly
            let code_check = Command::new("code").arg("--version").output();
            if code_check.is_ok() {
                println!("  Installing extension via 'code --install-extension'...");
                let status = Command::new("code")
                    .args(["--install-extension", &vsix_path.to_string_lossy(), "--force"])
                    .status();

                match status {
                    Ok(s) if s.success() => {
                        println!("{}", "✅ Successfully installed latest Lumi Tester VS Code Extension!".green().bold());
                    }
                    _ => {
                        println!("  {} Could not install extension automatically. Run manually:", "⚠️".yellow());
                        println!("    code --install-extension {}", vsix_path.display());
                    }
                }
            } else {
                println!("  {} VS Code CLI 'code' is not in PATH. To install manually, run:", "ℹ️".blue());
                println!("    code --install-extension {}", vsix_path.display());
            }
        } else {
            println!("  {} No VSIX asset found for extension release.", "⚠️".yellow());
        }
    }

    if !should_update_cli && !should_update_ext && !report.cli_update_available {
        println!("\n{}", "🎉 You are already on the latest version of Lumi Tester!".green().bold());
    }

    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

fn replace_executable(temp_file: &Path, target_exe: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        // On Unix, rename over the existing binary is atomic
        std::fs::rename(temp_file, target_exe)
            .or_else(|_| {
                // If on different mount points, copy and delete
                std::fs::copy(temp_file, target_exe)?;
                let _ = std::fs::remove_file(temp_file);
                Ok::<(), std::io::Error>(())
            })
            .with_context(|| format!("Failed to replace executable at {}", target_exe.display()))?;
    }

    #[cfg(windows)]
    {
        // On Windows, a running executable cannot be directly overwritten, but CAN be renamed!
        let old_exe = target_exe.with_extension(format!("old-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::rename(target_exe, &old_exe);
        std::fs::rename(temp_file, target_exe)
            .or_else(|_| {
                std::fs::copy(temp_file, target_exe)?;
                let _ = std::fs::remove_file(temp_file);
                Ok::<(), std::io::Error>(())
            })
            .with_context(|| format!("Failed to replace executable at {}", target_exe.display()))?;
        let _ = std::fs::remove_file(old_exe);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_target() {
        let target = detect_target().unwrap();
        assert!(!target.is_empty());
        let bin_name = target_binary_name(&target);
        assert!(bin_name.starts_with("lumi-tester-"));
    }

    #[test]
    fn test_target_binary_naming() {
        assert_eq!(
            target_binary_name("x86_64-apple-darwin"),
            "lumi-tester-x86_64-apple-darwin"
        );
        assert_eq!(
            target_binary_name("aarch64-apple-darwin"),
            "lumi-tester-aarch64-apple-darwin"
        );
        assert_eq!(
            target_binary_name("x86_64-pc-windows-msvc"),
            "lumi-tester-x86_64-pc-windows-msvc.exe"
        );
        assert_eq!(
            target_binary_name("x86_64-unknown-linux-gnu"),
            "lumi-tester-x86_64-unknown-linux-gnu"
        );
    }

    #[test]
    fn test_semver_comparison() {
        assert_eq!(parse_version_tuple("v0.1.17"), (0, 1, 17));
        assert_eq!(parse_version_tuple("extension-v0.1.25"), (0, 1, 25));
        assert!(is_newer_version("v0.1.17", "v0.1.16"));
        assert!(is_newer_version("v0.2.0", "v0.1.17"));
        assert!(!is_newer_version("v0.1.16", "v0.1.17"));
        assert!(!is_newer_version("v0.1.17", "v0.1.17"));
    }
}
