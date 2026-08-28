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

    // 3. Install the lm-ios-tester agent's Xcode project source (macOS only - iOS
    // automation is only ever run from a Mac). Same rationale as the Android APK:
    // `agent_setup::find_agent_project` previously only looked for this project
    // relative to a monorepo checkout, so it was unreachable for anyone who
    // installed lumi-tester via curl/Homebrew instead of cloning the source repo.
    // Unlike the APK, this doesn't need code signing to run on a *simulator* (no
    // Apple Developer account required) - `xcodebuild build-for-testing` compiles
    // it fresh from this extracted source using whatever Xcode toolchain is
    // already on the machine. Real *device* runs still need the user's own Team ID
    // configured in Xcode - that's an Apple platform requirement, not something
    // this extraction step can route around.
    if cfg!(target_os = "macos") {
        install_ios_agent_project(&install_dir)?;
    }

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

/// Bytes of the lm-ios-tester agent's Xcode project source, zipped from the
/// tracked `resources/ios/lm_ios_tester.zip` (contents of `lm-ios-tester/` at the
/// repo root - `project.yml`, `LumiIOSAgent.xcodeproj/`, `LumiIOSAgentRunner/`, no
/// build artifacts). Rebuild+recommit that zip whenever `lm-ios-tester`'s source
/// changes; nothing else needs to change. Regenerate with (from the repo root):
///   cd lm-ios-tester && zip -r -X -q ../lumi-tester/resources/ios/lm_ios_tester.zip . -x "*.DS_Store"
const IOS_AGENT_PROJECT_ZIP: &[u8] = include_bytes!("../../resources/ios/lm_ios_tester.zip");

/// Extracts the embedded lm-ios-tester Xcode project to `~/.lumi-tester/lm-ios-tester/`
/// if not already present there. Public (not just called from `system install`) so
/// `agent_setup::find_agent_project` can call it lazily on first use too - matching
/// the Android agent's on-demand-extraction parity (see
/// `binary_resolver::find_apk`), so this works whether or not the user ever ran
/// `lumi-tester system install --all` explicitly first.
pub fn ensure_ios_agent_project_extracted() -> Result<PathBuf> {
    let install_dir = get_install_dir()?;
    let project_dir = install_dir.join("lm-ios-tester");
    install_ios_agent_project(&install_dir)?;
    Ok(project_dir)
}

fn install_ios_agent_project(install_dir: &Path) -> Result<()> {
    let project_dir = install_dir.join("lm-ios-tester");
    // Marker records the embedded zip's own byte length (not just presence) so an
    // upgrade to a newer lumi-tester version - with different lm-ios-tester source
    // baked in - re-extracts instead of silently keeping stale files forever, same
    // staleness check the Android APK install uses (`install_agent_apk` above).
    let marker_path = project_dir.join(".lumi_ios_agent_zip_len");
    let expected_marker = IOS_AGENT_PROJECT_ZIP.len().to_string();

    if marker_path.exists() {
        if let Ok(existing) = fs::read_to_string(&marker_path) {
            if existing.trim() == expected_marker {
                println!("{} lm-ios-tester agent project is already installed.", "✓".green());
                return Ok(());
            }
        }
    }

    println!("{} Installing lm-ios-tester agent project...", "⬇️".yellow());
    // Clear out any previous version's files first - zip extraction only overwrites
    // files present in the new archive, it wouldn't remove ones that existed in an
    // older version but not this one.
    let _ = fs::remove_dir_all(&project_dir);
    fs::create_dir_all(&project_dir)?;

    let tmp_zip = std::env::temp_dir().join(format!(
        "lm_ios_tester_{}.zip",
        std::process::id()
    ));
    fs::write(&tmp_zip, IOS_AGENT_PROJECT_ZIP).context("Failed to write embedded iOS agent zip")?;
    let extract_result = extract_zip(&tmp_zip, &project_dir);
    let _ = fs::remove_file(&tmp_zip);
    extract_result.context("Failed to extract lm-ios-tester agent project")?;
    fs::write(&marker_path, &expected_marker).context("Failed to write iOS agent version marker")?;

    println!("{} Agent project installed successfully.", "✓".green());
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
