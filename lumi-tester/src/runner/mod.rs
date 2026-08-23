pub mod context;
pub mod events;
pub mod executor;
pub mod js_engine;
pub mod shell;
pub mod state;

use anyhow::Result;
use colored::Colorize;
use std::path::{Path, PathBuf};

pub use events::*;
pub use state::*;

/// Run tests from a file or directory
pub async fn run_tests(
    path: &Path,
    platform: &str,
    devices: Option<Vec<String>>,
    output: &Path,
    continue_on_failure: bool,
    parallel: bool,
    record: bool,
    snapshot: bool,
    report: bool,
    events_jsonl: bool,
    tags: Option<Vec<String>>,
    command_index: Option<usize>,
    from_command_index: Option<usize>,
    command_name: Option<String>,
    repeat: u32,
) -> Result<()> {
    let platform = platform
        .trim_matches('"')
        .trim_matches('\'')
        .to_ascii_lowercase();

    // 1. Resolve devices
    let device_serials = match devices {
        Some(d) => d,
        None => {
            if platform == "android" || platform == "android_auto" {
                let connected = crate::driver::android::adb::get_devices().await?;
                if connected.is_empty() {
                    anyhow::bail!("No Android devices connected");
                }
                connected.into_iter().map(|d| d.serial).collect()
            } else if platform == "web" {
                vec!["chromium".to_string()]
            } else if platform == "macos" || platform == "windows" {
                vec!["local".to_string()]
            } else {
                vec!["".to_string()] // Default for others
            }
        }
    };

    if device_serials.is_empty() {
        anyhow::bail!("No devices available for execution");
    }

    // 2. Collect all test files
    let mut all_files = Vec::new();
    if path.is_dir() {
        for entry in walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let path = e.path();
                let is_yaml = path
                    .extension()
                    .map_or(false, |ext| ext == "yaml" || ext == "yml");
                let name = e.file_name().to_string_lossy();

                // Skip files in utility directories that are meant to be called by scenario flows.
                let path_str = path.to_string_lossy();
                let in_subflows =
                    path_str.contains("/subflows/") || path_str.contains("\\subflows\\");
                let in_screens =
                    path_str.contains("/screens/") || path_str.contains("\\screens\\");

                is_yaml
                    && !in_subflows
                    && !in_screens
                    && name != "setup.yaml"
                    && name != "setup.yml"
                    && name != "teardown.yaml"
                    && name != "teardown.yml"
            })
        {
            all_files.push(entry.path().to_path_buf());
        }
    } else {
        all_files.push(path.to_path_buf());
    }

    if all_files.is_empty() {
        println!("{} No test files found.", "ℹ".blue());
        return Ok(());
    }

    // 3. Execution logic
    if parallel && device_serials.len() > 1 {
        println!(
            "{} Parallel execution enabled across {} devices",
            "🚀".yellow(),
            device_serials.len()
        );

        let chunk_size = (all_files.len() as f64 / device_serials.len() as f64).ceil() as usize;
        let chunks = all_files.chunks(chunk_size);

        let mut handles = Vec::new();
        let path_owned = path.to_path_buf();
        let platform_owned = platform.clone();
        let output_owned = Some(output.to_path_buf());

        for (i, chunk) in chunks.enumerate() {
            let device = device_serials[i].clone();
            let files = chunk.to_vec();

            // Auto-detect platform from device ID if possible
            let mut device_platform = platform_owned.clone();
            if device.contains('-') && device.len() == 36 {
                // Heuristic: UUID format usually implies iOS simulator/device
                device_platform = "ios".to_string();
            } else if device.contains('.') || device.chars().all(|c| c.is_alphanumeric()) {
                // IP address or alphanumeric serial usually implies Android
                // But check if it conflicts with iOS heuristic?
                // iOS UUID is alphanumeric + dashes. Android serial is alphanum.
                // We'll stick to: if it LOOKS like a UUID, it's iOS. Else default to provided platform or Android.
                if platform_owned == "auto" {
                    device_platform = "android".to_string();
                }
            }

            let output = output_owned.clone();
            let base_path = path_owned.clone();
            let tags_chunk = tags.clone();
            let cmd_idx = command_index;
            let from_cmd_idx = from_command_index;
            let cmd_name = command_name.clone();

            let handle = tokio::spawn(async move {
                run_on_device(
                    &base_path,
                    &files,
                    &device_platform,
                    Some(&device),
                    output.as_deref(),
                    continue_on_failure,
                    record,
                    snapshot,
                    report,
                    events_jsonl,
                    tags_chunk,
                    cmd_idx,
                    from_cmd_idx,
                    cmd_name,
                    repeat,
                )
                .await
            });
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await?;
        }

        println!("{} All parallel test tasks finished.", "✅".green());
        Ok(())
    } else {
        // Sequential run on primary device (or all files on one device)
        let primary_device = device_serials.first().map(|s| s.as_str());
        run_on_device(
            path,
            &all_files,
            &platform,
            primary_device,
            Some(output),
            continue_on_failure,
            record,
            snapshot,
            report,
            events_jsonl,
            tags,
            command_index,
            from_command_index,
            command_name,
            repeat,
        )
        .await
    }
}

/// Run a set of files on a specific device
async fn run_on_device(
    base_path: &Path,
    files: &[PathBuf],
    platform: &str,
    device: Option<&str>,
    output: Option<&Path>,
    continue_on_failure: bool,
    record: bool,
    snapshot: bool,
    report: bool,
    events_jsonl: bool,
    tags: Option<Vec<String>>,
    command_index: Option<usize>,
    from_command_index: Option<usize>,
    command_name: Option<String>,
    repeat: u32,
) -> Result<()> {
    // Pre-parse first file's `speed:` header field (e.g. "fast", "turbo") so the driver
    // can honor a flow-level speed profile when LUMI_SPEED isn't set.
    let flow_speed: Option<String> = if !files.is_empty() {
        crate::parser::yaml::parse_test_file(&files[0])
            .ok()
            .and_then(|flow| flow.speed)
    } else {
        None
    };

    // Pre-parse first file to extract web driver config (for close_when_finish support)
    let web_config = if platform == "web" && !files.is_empty() {
        use crate::parser::yaml::parse_test_file;

        // Parse first file to get header config
        if let Ok(flow) = parse_test_file(&files[0]) {
            use crate::driver::web::{BrowserType, WebDriverConfig};
            let mut config = WebDriverConfig::default();

            // Apply close_when_finish from YAML header
            if let Some(close) = flow.close_when_finish {
                config.close_when_finish = close;
            }

            // Apply browser type if specified
            if let Some(ref b) = flow.browser {
                config.browser_type = match b.to_lowercase().as_str() {
                    "firefox" => BrowserType::Firefox,
                    "webkit" => BrowserType::Webkit,
                    _ => BrowserType::Chromium,
                };
            }
            Some(config)
        } else {
            None
        }
    } else {
        None
    };

    // Strip quotes from platform if present (YAML parsing quirk)
    let platform_clean = platform
        .trim_matches('"')
        .trim_matches('\'')
        .to_ascii_lowercase();

    let driver: Box<dyn crate::driver::traits::PlatformDriver> = match platform_clean.as_str() {
        "android" => Box::new(
            crate::driver::android::AndroidDriver::new_with_speed(
                device,
                flow_speed.as_deref(),
            )
            .await?,
        ),
        "android_auto" => {
            Box::new(crate::driver::android_auto::AndroidAutoDriver::new(device, true).await?)
        }
        "web" => {
            use crate::driver::web::{WebDriver, WebDriverConfig};
            let config = web_config.unwrap_or_else(WebDriverConfig::default);
            Box::new(WebDriver::new(config).await?)
        }
        "ios" => Box::new(crate::driver::ios::IosDriver::new(device).await?),
        "macos" => Box::new(crate::driver::macos::MacosDriver::new()),
        "windows" => Box::new(crate::driver::windows::WindowsDriver::new()),
        _ => anyhow::bail!("Unknown platform: {}", platform_clean),
    };

    let target_name = if base_path.is_file() {
        base_path.file_stem().map(|s| s.to_string_lossy().to_string())
    } else {
        base_path.file_name().map(|s| s.to_string_lossy().to_string())
    };

    let mut executor = executor::TestExecutor::new_with_events(
        driver,
        output,
        continue_on_failure,
        record,
        snapshot,
        report,
        tags,
        events_jsonl,
    )
    .with_target_name(target_name.as_deref());
    let base_dir = if base_path.is_dir() {
        base_path
    } else {
        base_path.parent().unwrap_or(Path::new("."))
    };

    // 1. Run Setup hook
    for f in ["setup.yaml", "setup.yml"] {
        let p = base_dir.join(f);
        if p.exists() {
            if let Err(e) = executor.run_file(&p, None, None, None).await {
                let _ = executor.finish().await;
                return Err(e);
            }
            break;
        }
    }

    // 2. Run Main files (repeated N times)
    for round in 0..repeat {
        if repeat > 1 {
            println!(
                "\n{} Repeat round {}/{}",
                "🔁".cyan(),
                round + 1,
                repeat
            );
        }
        for file in files {
            if let Err(e) = executor
                .run_file(file, command_index, from_command_index, command_name.as_deref())
                .await
            {
                let _ = executor.finish().await;
                return Err(e);
            }
        }
    }

    // 3. Run Teardown hook
    for f in ["teardown.yaml", "teardown.yml"] {
        let p = base_dir.join(f);
        if p.exists() {
            if let Err(e) = executor.run_file(&p, None, None, None).await {
                let _ = executor.finish().await;
                return Err(e);
            }
            break;
        }
    }

    executor.finish().await
}
