use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
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

#[derive(Debug, Clone)]
pub struct SupportedIde {
    pub name: String,
    pub executable: PathBuf,
}

pub fn discover_supported_ides() -> Vec<SupportedIde> {
    let mut ides = Vec::new();
    let mut seen_paths = HashSet::new();

    #[allow(dead_code)]
    struct Candidate {
        name: &'static str,
        bins: &'static [&'static str],
        mac_paths: &'static [&'static str],
        win_env_paths: &'static [(&'static str, &'static str)],
        linux_paths: &'static [&'static str],
    }

    let candidates = [
        Candidate {
            name: "VS Code",
            bins: &["code", "code.cmd", "code.exe"],
            mac_paths: &[
                "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code",
                "~/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code",
            ],
            win_env_paths: &[
                ("LOCALAPPDATA", "Programs\\Microsoft VS Code\\bin\\code.cmd"),
                ("PROGRAMFILES", "Microsoft VS Code\\bin\\code.cmd"),
                ("ProgramFiles(x86)", "Microsoft VS Code\\bin\\code.cmd"),
            ],
            linux_paths: &["/usr/bin/code", "/usr/share/code/bin/code", "/snap/bin/code"],
        },
        Candidate {
            name: "Antigravity IDE",
            bins: &["antigravity", "antigravity.cmd", "antigravity.exe"],
            mac_paths: &[
                "/Applications/Antigravity.app/Contents/Resources/app/bin/antigravity",
                "~/Applications/Antigravity.app/Contents/Resources/app/bin/antigravity",
                "~/.antigravity/antigravity/bin/antigravity",
            ],
            win_env_paths: &[
                ("LOCALAPPDATA", "Programs\\Antigravity\\bin\\antigravity.cmd"),
                ("LOCALAPPDATA", "Programs\\Antigravity\\Antigravity.exe"),
                ("PROGRAMFILES", "Antigravity\\bin\\antigravity.cmd"),
                ("USERPROFILE", ".antigravity\\antigravity\\bin\\antigravity.cmd"),
            ],
            linux_paths: &[
                "/usr/bin/antigravity",
                "~/.local/share/antigravity/bin/antigravity",
                "~/.antigravity/antigravity/bin/antigravity",
            ],
        },
        Candidate {
            name: "Cursor",
            bins: &["cursor", "cursor.cmd", "cursor.exe"],
            mac_paths: &[
                "/Applications/Cursor.app/Contents/Resources/app/bin/cursor",
                "~/Applications/Cursor.app/Contents/Resources/app/bin/cursor",
            ],
            win_env_paths: &[
                ("LOCALAPPDATA", "Programs\\cursor\\bin\\cursor.cmd"),
                ("LOCALAPPDATA", "cursor\\bin\\cursor.cmd"),
            ],
            linux_paths: &["/usr/bin/cursor"],
        },
        Candidate {
            name: "Windsurf",
            bins: &["windsurf", "windsurf.cmd", "windsurf.exe"],
            mac_paths: &[
                "/Applications/Windsurf.app/Contents/Resources/app/bin/windsurf",
                "~/Applications/Windsurf.app/Contents/Resources/app/bin/windsurf",
            ],
            win_env_paths: &[
                ("LOCALAPPDATA", "Programs\\windsurf\\bin\\windsurf.cmd"),
            ],
            linux_paths: &["/usr/bin/windsurf"],
        },
        Candidate {
            name: "VSCodium",
            bins: &["codium", "codium.cmd", "codium.exe"],
            mac_paths: &[
                "/Applications/VSCodium.app/Contents/Resources/app/bin/codium",
                "~/Applications/VSCodium.app/Contents/Resources/app/bin/codium",
            ],
            win_env_paths: &[
                ("LOCALAPPDATA", "Programs\\VSCodium\\bin\\codium.cmd"),
            ],
            linux_paths: &["/usr/bin/codium", "/snap/bin/codium"],
        },
        Candidate {
            name: "VS Code Insiders",
            bins: &["code-insiders", "code-insiders.cmd"],
            mac_paths: &[
                "/Applications/Visual Studio Code - Insiders.app/Contents/Resources/app/bin/code-insiders",
                "~/Applications/Visual Studio Code - Insiders.app/Contents/Resources/app/bin/code-insiders",
            ],
            win_env_paths: &[
                ("LOCALAPPDATA", "Programs\\Microsoft VS Code Insiders\\bin\\code-insiders.cmd"),
            ],
            linux_paths: &["/usr/bin/code-insiders", "/snap/bin/code-insiders"],
        },
    ];

    for c in &candidates {
        let mut found = false;

        // 1. Check in PATH first
        for bin in c.bins {
            if let Ok(p) = which::which(bin) {
                let canonical = p.canonicalize().unwrap_or_else(|_| p.clone());
                if seen_paths.insert(canonical) {
                    ides.push(SupportedIde {
                        name: c.name.to_string(),
                        executable: p,
                    });
                    found = true;
                    break;
                }
            }
        }

        if found {
            continue;
        }

        // 2. Check well-known macOS paths
        #[cfg(target_os = "macos")]
        for raw in c.mac_paths {
            let expanded = if raw.starts_with("~/") {
                if let Some(home) = dirs::home_dir() {
                    home.join(&raw[2..])
                } else {
                    PathBuf::from(raw)
                }
            } else {
                PathBuf::from(raw)
            };

            if expanded.exists() {
                let canonical = expanded.canonicalize().unwrap_or_else(|_| expanded.clone());
                if seen_paths.insert(canonical) {
                    ides.push(SupportedIde {
                        name: c.name.to_string(),
                        executable: expanded,
                    });
                    break;
                }
            }
        }

        // 3. Check well-known Windows paths
        #[cfg(target_os = "windows")]
        for &(env_var, subpath) in c.win_env_paths {
            if let Ok(val) = std::env::var(env_var) {
                let full = PathBuf::from(val).join(subpath);
                if full.exists() {
                    let canonical = full.canonicalize().unwrap_or_else(|_| full.clone());
                    if seen_paths.insert(canonical) {
                        ides.push(SupportedIde {
                            name: c.name.to_string(),
                            executable: full,
                        });
                        break;
                    }
                }
            }
        }

        // 4. Check well-known Linux paths
        #[cfg(target_os = "linux")]
        for raw in c.linux_paths {
            let expanded = if raw.starts_with("~/") {
                if let Some(home) = dirs::home_dir() {
                    home.join(&raw[2..])
                } else {
                    PathBuf::from(raw)
                }
            } else {
                PathBuf::from(raw)
            };

            if expanded.exists() {
                let canonical = expanded.canonicalize().unwrap_or_else(|_| expanded.clone());
                if seen_paths.insert(canonical) {
                    ides.push(SupportedIde {
                        name: c.name.to_string(),
                        executable: expanded,
                    });
                    break;
                }
            }
        }
    }

    ides
}

fn detect_installed_extension_version() -> Option<String> {
    let ides = discover_supported_ides();
    for ide in &ides {
        if let Ok(output) = Command::new(&ide.executable)
            .args(["--list-extensions", "--show-versions"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.contains("lumi-tester") || line.contains("lumijsc.lumi-tester") {
                        if let Some((_, ver)) = line.split_once('@') {
                            return Some(format!("{} ({})", ver.trim(), ide.name));
                        }
                    }
                }
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
        .unwrap_or_else(|| "Not detected in any IDE".to_string());
    println!(
        "  • IDE Extension:     {} (Latest: {}) -> {}",
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

        download_file_with_progress(&client, &download_url, &temp_file, &asset_name).await?;

        let proc_pb = create_process_progress_bar(100, "Setting executable permissions...");
        proc_pb.set_position(25);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        make_executable(&temp_file)?;
        proc_pb.set_position(60);
        proc_pb.set_message("Replacing current executable binary...");
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Replace current executable
        replace_executable(&temp_file, &current_exe)?;
        proc_pb.set_position(100);
        proc_pb.finish_and_clear();
        println!("  {} CLI binary installation completed (100%)", "✓".green());

        println!(
            "\n{}",
            format!("✅ Successfully updated Lumi Tester CLI to {}!", target_ver)
                .green()
                .bold()
        );
        println!("  Executable: {}", current_exe.display().to_string().cyan());
    }

    if should_update_ext {
        if let Some(vsix_url) = report.extension_download_url {
            println!("\n{}", format!("⬇️  Downloading IDE Extension ({})...", report.extension_latest).cyan());
            let temp_dir = std::env::temp_dir();
            let vsix_name = format!("lumi-tester-{}.vsix", report.extension_latest);
            let vsix_path = temp_dir.join(&vsix_name);

            let client = reqwest::Client::builder()
                .user_agent(format!("lumi-tester/{}", env!("CARGO_PKG_VERSION")))
                .build()?;

            download_file_with_progress(&client, &vsix_url, &vsix_path, &vsix_name).await?;
            println!("  Saved VSIX to: {}", vsix_path.display().to_string().blue());

            let ides = discover_supported_ides();
            if !ides.is_empty() {
                println!("\n  {} Installing extension across detected IDEs...", "🔌".cyan());
                for ide in &ides {
                    let proc_pb = create_process_progress_bar(
                        100,
                        &format!("Installing into {} ({}) ...", ide.name, ide.executable.display()),
                    );
                    proc_pb.set_position(30);

                    let status = Command::new(&ide.executable)
                        .args(["--install-extension", &vsix_path.to_string_lossy(), "--force"])
                        .status();

                    match status {
                        Ok(s) if s.success() => {
                            proc_pb.set_position(100);
                            proc_pb.finish_and_clear();
                            println!(
                                "  {}",
                                format!("✅ Successfully installed latest Lumi Tester extension into {}!", ide.name)
                                    .green()
                                    .bold()
                            );
                        }
                        _ => {
                            proc_pb.abandon_with_message(format!("Install into {} failed ⚠️", ide.name));
                            println!(
                                "  {} Could not install extension into {} automatically. Run manually:",
                                "⚠️".yellow(),
                                ide.name
                            );
                            println!("    {} --install-extension {}", ide.executable.display(), vsix_path.display());
                        }
                    }
                }
            } else {
                println!(
                    "  {} No supported IDE CLI (VS Code, Antigravity, Cursor, Windsurf, VSCodium) found in PATH or standard locations. To install manually, run:",
                    "ℹ️".blue()
                );
                println!("    <ide-cli> --install-extension {}", vsix_path.display());
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

async fn download_file_with_progress(
    client: &reqwest::Client,
    download_url: &str,
    target_path: &Path,
    item_name: &str,
) -> Result<()> {
    use tokio::io::AsyncWriteExt;

    let mut response = client
        .get(download_url)
        .send()
        .await
        .with_context(|| format!("Failed to download from {}", download_url))?
        .error_for_status()
        .with_context(|| format!("Download returned error status from {}", download_url))?;

    let total_size = response.content_length();
    let is_tty = std::io::stdout().is_terminal();

    let pb = if is_tty {
        if let Some(len) = total_size {
            let pb = ProgressBar::with_draw_target(
                Some(len),
                ProgressDrawTarget::stdout_with_hz(15),
            );
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("  {spinner:.cyan} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({percent}%)")
                    .unwrap_or_else(|_| ProgressStyle::default_bar())
                    .progress_chars("━╸─"),
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(80));
            pb
        } else {
            let pb = ProgressBar::with_draw_target(
                None,
                ProgressDrawTarget::stdout_with_hz(15),
            );
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("  {spinner:.cyan} [{elapsed_precise}] {bytes}")
                    .unwrap_or_else(|_| ProgressStyle::default_spinner()),
            );
            pb.enable_steady_tick(std::time::Duration::from_millis(80));
            pb
        }
    } else {
        ProgressBar::hidden()
    };

    let mut file = tokio::fs::File::create(target_path)
        .await
        .with_context(|| format!("Failed to create temporary file {}", target_path.display()))?;

    let mut downloaded: u64 = 0;
    let mut last_pct: u64 = 0;

    while let Some(chunk) = response
        .chunk()
        .await
        .with_context(|| "Error while streaming download chunks")?
    {
        file.write_all(&chunk)
            .await
            .with_context(|| "Failed to write downloaded bytes")?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);

        if !is_tty {
            if let Some(total) = total_size {
                let pct = (downloaded * 100) / total;
                if pct >= last_pct + 25 || pct == 100 {
                    last_pct = pct;
                    println!(
                        "  [{:>3}%] {:.2} MB / {:.2} MB",
                        pct,
                        downloaded as f64 / 1_048_576.0,
                        total as f64 / 1_048_576.0
                    );
                }
            }
        }
    }

    file.flush().await.with_context(|| "Failed to flush downloaded file")?;
    pb.finish_and_clear();
    println!("  {} Downloaded {} (100%)", "✓".green(), item_name);

    Ok(())
}

fn create_process_progress_bar(total_steps: u64, initial_msg: &str) -> ProgressBar {
    if std::io::stdout().is_terminal() {
        let pb = ProgressBar::with_draw_target(
            Some(total_steps),
            ProgressDrawTarget::stdout_with_hz(15),
        );
        pb.set_style(
            ProgressStyle::default_bar()
                .template("  {spinner:.green} [{elapsed_precise}] [{wide_bar:.green/white}] {percent}% {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("━╸─"),
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        pb.set_message(initial_msg.to_string());
        pb
    } else {
        ProgressBar::hidden()
    }
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

        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("codesign")
                .args(["-s", "-", "-f", &target_exe.to_string_lossy()])
                .output();
        }
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
