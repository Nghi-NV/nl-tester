use anyhow::Result;
use dirs;
use std::path::PathBuf;
use std::process::Command;

/// Tìm binary từ bundled resources hoặc install directory (chỉ dùng static file, không dùng system PATH)
pub fn find_binary(name: &str) -> Result<PathBuf> {
    let mut checked_paths = Vec::new();

    // 1. Tìm từ bundled resources (trong app bundle)
    let (bundled_path, mut bundled_paths) = find_bundled_binary_with_logs(name);
    checked_paths.append(&mut bundled_paths);
    if let Some(path) = bundled_path {
        if path.exists() {
            return Ok(path);
        }
    }

    // 2. Tìm từ install directory (~/.lumi-tester) - cho development mode và fallback
    if let Some(home) = dirs::home_dir() {
        let install_dir = home.join(".lumi-tester");

        // Cho ADB: ~/.lumi-tester/platform-tools/adb
        if name == "adb" || name == "adb.exe" {
            let adb_path = if cfg!(windows) {
                install_dir.join("platform-tools").join("adb.exe")
            } else {
                install_dir.join("platform-tools").join("adb")
            };
            checked_paths.push(format!("Install Dir (ADB): {:?}", adb_path));
            if adb_path.exists() {
                return Ok(adb_path);
            }
        }

        // Cho FFmpeg: ~/.lumi-tester/playwright/ffmpeg
        if name == "ffmpeg" || name == "ffmpeg.exe" {
            let ffmpeg_path = if cfg!(windows) {
                install_dir.join("playwright").join("ffmpeg.exe")
            } else {
                install_dir.join("playwright").join("ffmpeg")
            };
            checked_paths.push(format!("Install Dir (FFmpeg): {:?}", ffmpeg_path));
            if ffmpeg_path.exists() {
                return Ok(ffmpeg_path);
            }
        }
    }

    // 3. Fallback to system PATH
    if let Ok(path) = which::which(name) {
        return Ok(path);
    }

    Err(anyhow::anyhow!(
        "Could not find static or system binary '{}'. Checked paths:\n{}",
        name,
        checked_paths.join("\n")
    ))
}

/// Tìm binary từ bundled resources
/// Tauri bundle resources thường ở trong app bundle
fn find_bundled_binary_with_logs(name: &str) -> (Option<PathBuf>, Vec<String>) {
    let mut checked_paths = Vec::new();

    // Lấy đường dẫn của executable hiện tại
    if let Ok(exe_path) = std::env::current_exe() {
        checked_paths.push(format!("Current EXE: {:?}", exe_path));

        // Trên macOS: app bundle structure
        #[cfg(target_os = "macos")]
        {
            // Tauri app bundle: App.app/Contents/MacOS/app
            // Resources ở: App.app/Contents/Resources/
            if let Some(app_bundle) = exe_path
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
            {
                let resources_dir = app_bundle.join("Contents/Resources");

                // Case 1: Nested resources (common in Tauri v2 bundles with broad glob patterns)
                // Path: Contents/Resources/resources/binaries/[name]
                let nested_path = resources_dir.join("resources").join("binaries").join(name);
                checked_paths.push(format!("MacOS Nested: {:?}", nested_path));
                if nested_path.exists() {
                    return (Some(nested_path), checked_paths);
                }

                // Case 2: Flat resources
                // Path: Contents/Resources/binaries/[name]
                let flat_path = resources_dir.join("binaries").join(name);
                checked_paths.push(format!("MacOS Flat: {:?}", flat_path));
                if flat_path.exists() {
                    return (Some(flat_path), checked_paths);
                }
            }

            // Thử tìm từ thư mục hiện tại (cho development)
            if let Ok(cwd) = std::env::current_dir() {
                // Thử từ src-tauri/resources/binaries
                let dev_path = cwd
                    .join("src-tauri")
                    .join("resources")
                    .join("binaries")
                    .join(name);
                checked_paths.push(format!("Dev Path (src-tauri): {:?}", dev_path));
                if dev_path.exists() {
                    return (Some(dev_path), checked_paths);
                }

                // Thử từ resources/binaries (CWD direct)
                let cwd_path = cwd.join("resources").join("binaries").join(name);
                checked_paths.push(format!("Dev Path (CWD): {:?}", cwd_path));
                if cwd_path.exists() {
                    return (Some(cwd_path), checked_paths);
                }
            }
        }

        // Trên Windows: resources ở cùng thư mục với exe hoặc trong resources/
        #[cfg(target_os = "windows")]
        {
            if let Some(exe_dir) = exe_path.parent() {
                let exe_name = format!("{}.exe", name);

                // Case 1: Nested resources/resources/binaries/[name].exe
                let nested_path = exe_dir
                    .join("resources")
                    .join("resources")
                    .join("binaries")
                    .join(&exe_name);
                checked_paths.push(format!("Win Nested: {:?}", nested_path));
                if nested_path.exists() {
                    return (Some(nested_path), checked_paths);
                }

                // Case 2: Standard resources/binaries/[name].exe
                let standard_path = exe_dir.join("resources").join("binaries").join(&exe_name);
                checked_paths.push(format!("Win Standard: {:?}", standard_path));
                if standard_path.exists() {
                    return (Some(standard_path), checked_paths);
                }

                // Case 3: Direct sibling (unlikely for bundled resources but good fallback)
                let sibling_path = exe_dir.join(&exe_name);
                checked_paths.push(format!("Win Sibling: {:?}", sibling_path));
                if sibling_path.exists() {
                    return (Some(sibling_path), checked_paths);
                }
            }
        }

        // Trên Linux: resources ở cùng thư mục với binary hoặc trong resources/
        #[cfg(target_os = "linux")]
        {
            if let Some(exe_dir) = exe_path.parent() {
                // Case 1: Nested resources/resources/binaries/[name]
                let nested_path = exe_dir
                    .join("resources")
                    .join("resources")
                    .join("binaries")
                    .join(name);
                checked_paths.push(format!("Linux Nested: {:?}", nested_path));
                if nested_path.exists() {
                    return (Some(nested_path), checked_paths);
                }

                // Case 2: Standard resources/binaries/[name]
                let standard_path = exe_dir.join("resources").join("binaries").join(name);
                checked_paths.push(format!("Linux Standard: {:?}", standard_path));
                if standard_path.exists() {
                    return (Some(standard_path), checked_paths);
                }

                // Case 3: Direct sibling
                let sibling_path = exe_dir.join(name);
                checked_paths.push(format!("Linux Sibling: {:?}", sibling_path));
                if sibling_path.exists() {
                    return (Some(sibling_path), checked_paths);
                }
            }
        }
    } else {
        checked_paths.push("Failed to get current_exe".to_string());
    }

    (None, checked_paths)
}

/// Tìm ADB binary (có thể là adb hoặc adb.exe)
pub fn find_adb() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        find_binary("adb.exe").or_else(|_| find_binary("adb"))
    }
    #[cfg(not(windows))]
    {
        find_binary("adb")
    }
}

/// Tìm IDB binary
pub fn find_idb() -> Result<PathBuf> {
    find_binary("idb")
}

/// Tìm FFmpeg binary
pub fn find_ffmpeg() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        find_binary("ffmpeg.exe").or_else(|_| find_binary("ffmpeg"))
    }
    #[cfg(not(windows))]
    {
        find_binary("ffmpeg")
    }
}

/// Tạo Command với binary đã resolve
pub fn command_with_resolved_binary(name: &str) -> Result<Command> {
    let binary_path = find_binary(name)?;
    let cmd = Command::new(binary_path);
    Ok(cmd)
}

/// Tạo Command với ADB đã resolve
pub fn adb_command() -> Result<Command> {
    let adb_path = find_adb()?;
    let cmd = Command::new(adb_path);
    Ok(cmd)
}

/// Tạo Command với IDB đã resolve
pub fn idb_command() -> Result<Command> {
    let idb_path = find_idb()?;
    let cmd = Command::new(idb_path);
    Ok(cmd)
}

/// Find bundled APK file (for ADBKeyBoard, etc.)
pub fn find_apk(name: &str) -> Option<PathBuf> {
    // Try bundled resources first
    if let Ok(exe_path) = std::env::current_exe() {
        #[cfg(target_os = "macos")]
        {
            // macOS app bundle: App.app/Contents/Resources/apk/[name].apk
            if let Some(resources) = exe_path
                .parent()
                .and_then(|p| p.parent())
                .map(|p| p.join("Resources"))
            {
                let apk_path = resources.join("apk").join(name);
                if apk_path.exists() {
                    return Some(apk_path);
                }
                // Also check resources/resources/apk (Tauri nested)
                let nested_path = resources.join("resources").join("apk").join(name);
                if nested_path.exists() {
                    return Some(nested_path);
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(exe_dir) = exe_path.parent() {
                let apk_path = exe_dir.join("resources").join("apk").join(name);
                if apk_path.exists() {
                    return Some(apk_path);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(exe_dir) = exe_path.parent() {
                let apk_path = exe_dir.join("resources").join("apk").join(name);
                if apk_path.exists() {
                    return Some(apk_path);
                }
            }
        }
    }

    // Check ~/.lumi-tester/apk/ for development/installed
    if let Some(home) = dirs::home_dir() {
        let apk_path = home.join(".lumi-tester").join("apk").join(name);
        if apk_path.exists() {
            return Some(apk_path);
        }
    }

    None
}
