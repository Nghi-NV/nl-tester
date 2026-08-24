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

    // 2. Install the lm-android-tester agent APK (fast UI-automation path for
    // Android). Unlike ADB this isn't downloaded - the release installer (Windows
    // .iss / macOS .pkg) never bundles `resources/apk/`, so without this step the
    // APK was never reachable on a machine that installed via the official
    // installer instead of running from the source checkout. The APK is small and
    // changes rarely, so it's embedded directly in the CLI binary and just written
    // out here - see AgentService::find_apk_path / binary_resolver::find_apk for
    // where it's looked up afterwards.
    install_agent_apk(&install_dir)?;

    println!("\n{}", "All system components are ready!".green().bold());
    println!("Installation directory: {}", install_dir.display());

    Ok(())
}

/// Bytes of the lm-android-tester agent APK, embedded at compile time from the
/// tracked `resources/apk/lm-android-tester.apk`. Rebuild+recommit that file
/// whenever `lm-android-tester`'s source changes; nothing else needs to change.
const AGENT_APK_BYTES: &[u8] = include_bytes!("../../resources/apk/lm-android-tester.apk");

fn install_agent_apk(install_dir: &Path) -> Result<()> {
    let apk_dir = install_dir.join("apk");
    let apk_path = apk_dir.join("lm-android-tester.apk");

    if apk_path.exists() {
        if let Ok(metadata) = fs::metadata(&apk_path) {
            if metadata.len() == AGENT_APK_BYTES.len() as u64 {
                println!("{} lm-android-tester agent APK is already installed.", "✓".green());
                return Ok(());
            }
        }
    }

    println!("{} Installing lm-android-tester agent APK...", "⬇️".yellow());
    fs::create_dir_all(&apk_dir)?;
    fs::write(&apk_path, AGENT_APK_BYTES).context("Failed to write agent APK")?;
    println!("{} Agent APK installed successfully.", "✓".green());
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
