use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

use lumi_tester::{driver, recorder, report, runner, utils};

#[derive(Parser)]
#[command(name = "lumi-tester")]
#[command(author = "NL Team")]
#[command(version = "0.1.0")]
#[command(about = "Multi-platform automation testing CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run test file(s) or directory
    Run {
        /// Path to test file or directory
        path: PathBuf,

        /// Target platform (android, ios, web)
        /// Target platform (android, ios, web). parsed from file if not provided.
        #[arg(short, long)]
        platform: Option<String>,

        /// Device serial(s) (Android) or UDID(s) (iOS). Can be specified multiple times.
        #[arg(short, long)]
        device: Vec<String>,

        /// Run tests in parallel across multiple devices
        #[arg(long, default_value = "false")]
        parallel: bool,

        /// Output directory for reports and artifacts
        #[arg(short, long, default_value = "./output")]
        output: PathBuf,

        /// Continue on failure
        #[arg(long, default_value = "false")]
        continue_on_failure: bool,

        /// Enable video recording during test execution
        #[arg(long, short = 'r', default_value = "false")]
        record: bool,

        /// Enable screenshot capture on failures
        #[arg(long, short = 's', default_value = "false")]
        snapshot: bool,

        /// Generate reports (JSON, HTML, JUnit)
        #[arg(long, default_value = "false")]
        report: bool,

        /// Filter tests by tags (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        tags: Option<Vec<String>>,

        /// Run only a specific command by index (0-based)
        #[arg(long)]
        command_index: Option<usize>,

        /// Run only a specific command by name (first match)
        #[arg(long)]
        command_name: Option<String>,
    },

    /// List connected devices
    Devices {
        /// Target platform
        #[arg(short, long, default_value = "android")]
        platform: String,
    },

    /// Generate report from test results
    Report {
        /// Path to test results JSON
        results: PathBuf,

        /// Output format (json, html)
        #[arg(short, long, default_value = "html")]
        format: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    Shell {
        /// Target platform
        #[arg(short, long, default_value = "android")]
        platform: String,

        /// Device serial (Android) or UDID (iOS)
        #[arg(short, long)]
        device: Option<String>,
    },

    /// Manage system components
    System {
        #[command(subcommand)]
        command: SystemCommands,
    },

    /// Record user interactions and generate YAML test file
    Record {
        /// Output file path for the generated YAML
        #[arg(short, long)]
        output: PathBuf,

        /// Device serial (Android)
        #[arg(short, long)]
        device: Option<String>,

        /// App ID to record (auto-detected if not provided)
        #[arg(short, long)]
        app: Option<String>,

        /// Test name for the generated file
        #[arg(short, long)]
        name: Option<String>,

        /// Include wait commands between actions
        #[arg(long, default_value = "true")]
        include_waits: bool,

        /// Include selector alternatives as comments
        #[arg(long, default_value = "true")]
        include_comments: bool,
    },

    /// Start web-based inspector for visual test creation
    Inspect {
        /// Target platform (android, ios, web)
        #[arg(short, long, default_value = "android")]
        platform: String,

        /// Device serial (Android) or UDID (iOS)
        #[arg(short, long)]
        device: Option<String>,

        /// Server port
        #[arg(long, default_value = "9333")]
        port: u16,

        /// Output YAML file (optional, can be selected in UI)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum SystemCommands {
    /// Install required drivers and tools
    Install {
        /// Install all components
        #[arg(long)]
        all: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            path,
            platform,
            device,
            parallel,
            output,
            continue_on_failure,
            record,
            snapshot,
            report,
            tags,
            command_index,
            command_name,
        } => {
            let platform_val = if let Some(p) = platform {
                p
            } else {
                detect_platform(&path).unwrap_or_else(|| "android".to_string())
            };

            println!(
                "{} Running tests from: {}",
                "▶".green().bold(),
                path.display()
            );
            println!("  Platform: {}", platform_val.cyan());
            if !device.is_empty() {
                println!("  Devices: {}", device.join(", ").cyan());
            }
            if parallel {
                println!("  Parallel: {}", "Enabled".yellow());
            }
            if let Some(ref tags_list) = tags {
                println!("  Tags: {}", tags_list.join(", ").yellow());
            }
            println!("  Output: {}", output.display().to_string().cyan());
            if record {
                println!("  Recording: {}", "Enabled".green());
            }
            if snapshot {
                println!("  Snapshots: {}", "Enabled".green());
            }
            if report {
                println!("  Reports: {}", "Enabled".green());
            }
            if let Some(idx) = command_index {
                println!("  Command Index: {}", idx.to_string().yellow());
            }
            if let Some(ref name) = command_name {
                println!("  Command Name: {}", name.cyan());
            }

            runner::run_tests(
                &path,
                &platform_val,
                if device.is_empty() {
                    None
                } else {
                    Some(device)
                },
                &output,
                continue_on_failure,
                parallel,
                record,
                snapshot,
                report,
                tags,
                command_index,
                command_name,
            )
            .await?;
        }

        Commands::Devices { platform } => {
            println!(
                "{} Listing {} devices...",
                "🔍".to_string().blue(),
                platform.cyan()
            );
            driver::list_devices(&platform).await?;
        }

        Commands::Report {
            results,
            format,
            output,
        } => {
            println!(
                "{} Generating {} report from: {}",
                "📊".to_string().blue(),
                format.cyan(),
                results.display()
            );
            report::generate_report(&results, &format, output.as_deref()).await?;
        }

        Commands::Shell { platform, device } => {
            println!(
                "{} Starting interactive shell for {}...",
                "🐚".to_string().blue(),
                platform.cyan()
            );

            let driver: Box<dyn driver::traits::PlatformDriver> = match platform.as_str() {
                "android" => {
                    Box::new(driver::android::AndroidDriver::new(device.as_deref()).await?)
                }
                "ios" => Box::new(driver::ios::IosDriver::new(device.as_deref()).await?),
                _ => anyhow::bail!("Unknown platform: {}", platform),
            };

            runner::shell::run_shell(driver).await?;
        }

        Commands::System { command } => match command {
            SystemCommands::Install { all } => {
                utils::system::handle_system_command(utils::system::SystemCommand::Install { all })
                    .await?;
            }
        },

        Commands::Record {
            output,
            device,
            app,
            name,
            include_waits,
            include_comments,
        } => {
            println!("{} Starting record mode...", "🔴".to_string().red().bold());

            // Create event recorder
            let event_recorder = recorder::EventRecorder::new(device.as_deref()).await?;

            // Start recording
            event_recorder.start_recording().await?;

            // Set up Ctrl+C handler with atomic flag
            let stop_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let stop_flag_handler = stop_flag.clone();

            ctrlc::set_handler(move || {
                println!("\n\n{} Stopping recording...", "⏹️ ".yellow());
                stop_flag_handler.store(true, std::sync::atomic::Ordering::SeqCst);
            })?;

            // Start real-time touch capture using getevent
            println!("\n📲 Monitoring device interactions...");
            println!("   Tap, type, and swipe on your device.");
            println!("   Press Ctrl+C when done.\n");

            // Find touch device
            let getevent_info =
                lumi_tester::driver::android::adb::shell(device.as_deref(), "getevent -pl")
                    .await
                    .unwrap_or_default();

            let touch_device = find_touch_device(&getevent_info);

            if let Some(ref dev) = touch_device {
                println!("📲 Found touch device: {}", dev);

                // Get max touch coordinates for scaling
                let (max_x, max_y) = parse_touch_range(&getevent_info);
                println!("   Touch range: {}x{}", max_x, max_y);

                // Start getevent stream
                let adb_path = lumi_tester::utils::binary_resolver::find_adb()?;
                let mut args = Vec::new();
                if let Some(ref d) = device {
                    args.push("-s".to_string());
                    args.push(d.clone());
                }
                args.push("shell".to_string());
                args.push(format!("getevent -lt {}", dev));

                let mut child = tokio::process::Command::new(&adb_path)
                    .args(&args)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::null())
                    .spawn()?;

                let stdout = child.stdout.take().unwrap();
                let reader = tokio::io::BufReader::new(stdout);

                use tokio::io::AsyncBufReadExt;
                let mut lines = reader.lines();

                let mut current_x: Option<i32> = None;
                let mut current_y: Option<i32> = None;
                let mut touch_down_time: Option<std::time::Instant> = None;

                loop {
                    tokio::select! {
                        line = lines.next_line() => {
                            if stop_flag.load(std::sync::atomic::Ordering::SeqCst) {
                                break;
                            }
                            match line {
                                Ok(Some(line)) => {
                                    // Parse getevent output
                                    if line.contains("ABS_MT_POSITION_X") {
                                        if let Some(val) = parse_hex_value(&line) {
                                            current_x = Some((val as f64 / max_x as f64 * event_recorder.screen_width as f64) as i32);
                                        }
                                    } else if line.contains("ABS_MT_POSITION_Y") {
                                        if let Some(val) = parse_hex_value(&line) {
                                            current_y = Some((val as f64 / max_y as f64 * event_recorder.screen_height as f64) as i32);
                                        }
                                    } else if line.contains("BTN_TOUCH") && line.contains("DOWN") {
                                        touch_down_time = Some(std::time::Instant::now());
                                    } else if line.contains("BTN_TOUCH") && line.contains("UP") {
                                        if let (Some(x), Some(y)) = (current_x, current_y) {
                                            let duration = touch_down_time
                                                .map(|t| t.elapsed().as_millis())
                                                .unwrap_or(0);

                                            if duration > 500 {
                                                // Long press - for now just log
                                                println!("  👆 longPress at ({}, {})", x, y);
                                            } else {
                                                // Record tap
                                                if let Err(e) = event_recorder.record_tap(x, y).await {
                                                    eprintln!("  ⚠️ Failed to record tap: {}", e);
                                                }
                                            }
                                        }
                                        current_x = None;
                                        current_y = None;
                                        touch_down_time = None;
                                    }
                                }
                                Ok(None) => break,
                                Err(_) => break,
                            }
                        }
                        _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                            if stop_flag.load(std::sync::atomic::Ordering::SeqCst) {
                                break;
                            }
                        }
                    }
                }

                let _ = child.kill().await;
            } else {
                println!("⚠️ No touch device found. Running in passive mode.");
                // Fallback: just wait for Ctrl+C
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    if stop_flag.load(std::sync::atomic::Ordering::SeqCst) {
                        break;
                    }
                }
            }

            // Generate YAML - this now runs after Ctrl+C
            let actions = event_recorder.stop_recording().await?;
            let detected_app = event_recorder.get_current_app().await;
            let app_id = app.as_deref().or(detected_app.as_deref());

            let config = recorder::yaml_generator::YamlGeneratorConfig {
                include_comments,
                include_waits,
                min_wait_ms: 1000,
                mask_sensitive: true,
                suggest_assertions: true,
            };

            let generator = recorder::YamlGenerator::with_config(config);
            generator.save_to_file(&actions, app_id, name.as_deref(), &output)?;

            println!("\n{} Recording complete!", "✅".green().bold());
            println!("   Output: {}", output.display().to_string().cyan());
        }

        Commands::Inspect {
            platform,
            device,
            port,
            output,
        } => {
            use lumi_tester::inspector::{server::InspectorConfig, InspectorServer};

            let config = InspectorConfig {
                port,
                platform,
                device_serial: device,
                output_file: output,
            };

            let server = InspectorServer::new(config);
            server.start().await?;
        }
    }

    Ok(())
}

fn detect_platform(path: &std::path::Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines().take(20) {
        if let Some(rest) = line.trim().strip_prefix("platform:") {
            let p = rest.trim().to_lowercase();
            let p = p.trim_matches('"').trim_matches('\'').to_string();
            if !p.is_empty() {
                return Some(p);
            }
        }
    }
    None
}

/// Find the primary touch input device from getevent -pl output
fn find_touch_device(getevent_output: &str) -> Option<String> {
    let mut current_device: Option<String> = None;

    for line in getevent_output.lines() {
        if line.starts_with("add device") || line.contains("/dev/input/event") {
            // Extract device path
            if let Some(path_start) = line.find("/dev/input/") {
                let rest = &line[path_start..];
                let device_path = rest.split_whitespace().next().unwrap_or(rest);
                // Clean up device path - remove trailing colon or other chars
                let clean_path = device_path.trim_end_matches(':').trim();
                current_device = Some(clean_path.to_string());
            }
        } else if line.contains("ABS_MT_POSITION_X") || line.contains("ABS_MT_TOUCH") {
            // This device supports multi-touch, it's likely the main touchscreen
            if let Some(ref device) = current_device {
                return Some(device.clone());
            }
        }
    }

    None
}

/// Parse touch coordinate range from getevent -pl output
fn parse_touch_range(getevent_output: &str) -> (f64, f64) {
    let mut max_x: f64 = 32767.0; // Default
    let mut max_y: f64 = 32767.0;

    for line in getevent_output.lines() {
        // Look for lines like: ABS_MT_POSITION_X : value 0, min 0, max 1080
        if line.contains("ABS_MT_POSITION_X") && line.contains("max") {
            if let Some(max_val) = extract_max_value(line) {
                max_x = max_val as f64;
            }
        } else if line.contains("ABS_MT_POSITION_Y") && line.contains("max") {
            if let Some(max_val) = extract_max_value(line) {
                max_y = max_val as f64;
            }
        }
    }

    (max_x, max_y)
}

/// Extract max value from getevent line
fn extract_max_value(line: &str) -> Option<i32> {
    // Format: "ABS_MT_POSITION_X : value 0, min 0, max 1080, fuzz 0, flat 0, resolution 0"
    if let Some(max_pos) = line.find("max ") {
        let rest = &line[max_pos + 4..];
        let val_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        return val_str.parse().ok();
    }
    None
}

/// Parse hex or decimal value from getevent line
fn parse_hex_value(line: &str) -> Option<i32> {
    // Format: [timestamp] /dev/input/eventX: EV_ABS ABS_MT_POSITION_X 0000abcd
    let parts: Vec<&str> = line.split_whitespace().collect();
    if let Some(last) = parts.last() {
        // Value is usually in hex without 0x prefix
        if last.chars().all(|c| c.is_ascii_hexdigit()) {
            return i32::from_str_radix(last, 16).ok();
        }
        // Try decimal
        return last.parse().ok();
    }
    None
}
