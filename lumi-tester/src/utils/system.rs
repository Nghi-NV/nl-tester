use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

pub enum SystemCommand {
    Install { all: bool },
}

pub async fn handle_system_command(command: SystemCommand) -> Result<()> {
    match command {
        SystemCommand::Install { all } => install_components(all).await,
    }
}

async fn install_components(_all: bool) -> Result<()> {
    println!("{}", "Checking system components...".blue().bold());

    let install_dir = get_install_dir()?;
    fs::create_dir_all(&install_dir)?;

    // 1. Check and install ADB
    install_adb(&install_dir).await?;

    // 2. Check and install Playwright
    install_playwright(&install_dir).await?;

    println!("\n{}", "All system components are ready!".green().bold());
    println!("Installation directory: {}", install_dir.display());

    Ok(())
}

fn get_install_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    Ok(home.join(".lumi-tester"))
}

async fn install_adb(install_dir: &Path) -> Result<()> {
    let adb_dir = install_dir.join("platform-tools");
    let adb_bin = if cfg!(windows) {
        adb_dir.join("adb.exe")
    } else {
        adb_dir.join("adb")
    };

    if adb_bin.exists() {
        println!("{} ADB is already installed.", "✓".green());
        return Ok(());
    }

    println!("{} Installing ADB...", "⬇️".yellow());

    let (url, file_name) = if cfg!(target_os = "macos") {
        (
            "https://dl.google.com/android/repository/platform-tools-latest-darwin.zip",
            "platform-tools.zip",
        )
    } else if cfg!(target_os = "windows") {
        (
            "https://dl.google.com/android/repository/platform-tools-latest-windows.zip",
            "platform-tools.zip",
        )
    } else {
        (
            "https://dl.google.com/android/repository/platform-tools-latest-linux.zip",
            "platform-tools.zip",
        )
    };

    let archive_path = install_dir.join(file_name);
    download_file(url, &archive_path).await?;

    println!("Extracting ADB...");
    extract_zip(&archive_path, install_dir)?;

    // Cleanup zip
    fs::remove_file(archive_path)?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&adb_bin)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&adb_bin, perms)?;
    }

    println!("{} ADB installed successfully.", "✓".green());
    Ok(())
}

async fn install_playwright(install_dir: &Path) -> Result<()> {
    let pw_dir = install_dir.join("playwright");
    fs::create_dir_all(&pw_dir)?;

    // Determine driver binary name
    let driver_name = if cfg!(windows) {
        "playwright.exe"
    } else {
        "playwright"
    };

    let mut driver_path = pw_dir.join(driver_name);

    if !driver_path.exists() && !cfg!(windows) {
        let sh_path = pw_dir.join("playwright.sh");
        if sh_path.exists() {
            driver_path = sh_path;
        }
    }

    if driver_path.exists() {
        println!("{} Playwright driver is already installed.", "✓".green());
        // We could verify browsers here, but let's assume if driver exists, we are good or user can run install again
        // For robustness, let's run browser install anyway if requested?
        // For now, minimal check.
    } else {
        println!("{} Installing Playwright driver...", "⬇️".yellow());

        // This is a bit tricky. We need to match the version of playwright-rust crate.
        // crate version 0.0.20 maps to playwright 1.40.0 roughly.
        // Let's use a known working version for now.
        // Ideally we should query the crate or have a constant.
        let version = "1.40.0";

        let (platform, ext) = if cfg!(target_os = "linux") {
            ("linux", ".tar.gz")
        } else if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                ("mac-arm64", ".zip")
            } else {
                ("mac", ".zip")
            }
        } else if cfg!(target_os = "windows") {
            ("win32_x64", ".zip")
        } else {
            anyhow::bail!("Unsupported platform for Playwright");
        };

        let url = format!(
            "https://playwright.azureedge.net/builds/driver/playwright-{}-{}{}",
            version, platform, ext
        );

        let archive_path = pw_dir.join(format!("driver{}", ext));
        download_file(&url, &archive_path).await?;

        if ext == ".zip" {
            extract_zip(&archive_path, &pw_dir)?;
        } else {
            extract_tar_gz(&archive_path, &pw_dir)?;
        }

        fs::remove_file(archive_path)?;

        // Adjust driver_path if it was extracted as .sh
        if !driver_path.exists() && !cfg!(windows) {
            let sh_path = pw_dir.join("playwright.sh");
            if sh_path.exists() {
                driver_path = sh_path;
            }
        }

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&driver_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&driver_path, perms)?;
        }

        println!("{} Playwright driver installed.", "✓".green());
    }

    // Install browsers
    println!("{} Installing Playwright browsers...", "⬇️".yellow());
    let status = std::process::Command::new(&driver_path)
        .arg("install")
        .arg("chromium")
        .arg("ffmpeg")
        .status()
        .context("Failed to run playwright install")?;

    if !status.success() {
        anyhow::bail!("Failed to install browsers");
    }

    println!("{} Playwright browsers installed.", "✓".green());

    // Patch registry.js for macOS ARM64 if needed
    if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        patch_playwright_registry(&pw_dir).await?;
    }

    Ok(())
}

async fn patch_playwright_registry(pw_dir: &Path) -> Result<()> {
    println!(
        "{} Checking Playwright registry for macOS ARM64...",
        "🔍".cyan()
    );

    // Path structure: playwright/package/lib/utils/registry.js
    // OR playwright-driver/package/lib/utils/registry.js depending on extraction
    // Let's search for package/lib/utils/registry.js
    let mut registry_path = None;
    for entry in walkdir::WalkDir::new(pw_dir)
        .min_depth(1)
        .max_depth(4)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_name() == "registry.js" {
            let path = entry.path();
            if path.to_string_lossy().contains("lib/utils") {
                registry_path = Some(path.to_path_buf());
                break;
            }
        }
    }

    if let Some(path) = registry_path {
        let content = fs::read_to_string(&path)?;
        if !content.contains("mac15-arm64") {
            println!(
                "{} Patching registry.js for macOS 15+ ARM64...",
                "🔧".yellow()
            );

            // Logic to inject the missing entry
            // We want to add 'mac15-arm64' to the same mapping as 'mac14-arm64'
            // Simple replace approach
            let patched_content = content.replace(
                "'mac14-arm64':",
                "'mac15-arm64': 'mac14-arm64',\n    'mac14-arm64':",
            );

            fs::write(&path, patched_content)?;
            println!("{} Registry patched successfully.", "✓".green());
        } else {
            println!("{} Registry already supports macOS 15+ ARM64.", "✓".green());
        }
    } else {
        println!("{} Could not find registry.js to patch.", "⚠️".yellow());
    }

    Ok(())
}

async fn download_file(url: &str, path: &Path) -> Result<()> {
    let response = reqwest::get(url).await.context("Failed to send request")?;
    let content = response.bytes().await.context("Failed to get bytes")?;
    let mut file = fs::File::create(path).context("Failed to create file")?;
    std::io::copy(&mut Cursor::new(content), &mut file).context("Failed to write to file")?;
    Ok(())
}

fn extract_zip(archive_path: &Path, target_dir: &Path) -> Result<()> {
    let file = fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // For ADB, we want to strip the top-level folder if it exists, or just extract.
    // Platform tools zip usually has 'platform-tools' as root.
    // Playwright zip has 'playwright-driver' usually.
    // We'll just extract all.

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => target_dir.join(path),
            None => continue,
        };

        if (*file.name()).ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }

        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
            }
        }
    }
    Ok(())
}

fn extract_tar_gz(archive_path: &Path, target_dir: &Path) -> Result<()> {
    let tar_gz = fs::File::open(archive_path)?;
    let tar = flate2::read::GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(tar);
    archive.unpack(target_dir)?;
    Ok(())
}
