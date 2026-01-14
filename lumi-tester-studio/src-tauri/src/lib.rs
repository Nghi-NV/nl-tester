use lumi_tester::driver::traits::PlatformDriver;
use lumi_tester::runner::{events::TestEvent, executor::TestExecutor, state::TestSummary};
use serde::Serialize;
use tauri::{Emitter, Window};

#[derive(Serialize, Clone)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Clone)]
#[serde(tag = "type")]
pub enum StudioTestEvent {
    SessionStarted {
        session_id: String,
    },
    SessionFinished {
        summary: StudioTestSummary,
    },
    FlowStarted {
        flow_name: String,
        flow_path: String,
        command_count: usize,
        depth: usize,
    },
    FlowFinished {
        flow_name: String,
        status: String,
        duration_ms: Option<u64>,
        depth: usize,
    },
    CommandStarted {
        flow_name: String,
        index: usize,
        command: String,
        depth: usize,
    },
    CommandPassed {
        flow_name: String,
        index: usize,
        duration_ms: u64,
        depth: usize,
    },
    CommandFailed {
        flow_name: String,
        index: usize,
        error: String,
        duration_ms: u64,
        depth: usize,
    },
    CommandRetrying {
        flow_name: String,
        index: usize,
        attempt: u32,
        max_attempts: u32,
        depth: usize,
    },
    CommandSkipped {
        flow_name: String,
        index: usize,
        reason: String,
        depth: usize,
    },
    Log {
        message: String,
        depth: usize,
    },
}

#[derive(Serialize, Clone)]
pub struct StudioTestSummary {
    pub total_flows: usize,
    pub total_commands: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub total_duration_ms: Option<u64>,
}

impl From<TestSummary> for StudioTestSummary {
    fn from(s: TestSummary) -> Self {
        Self {
            total_flows: s.total_flows as usize,
            total_commands: s.total_commands as usize,
            passed: s.passed as usize,
            failed: s.failed as usize,
            skipped: s.skipped as usize,
            total_duration_ms: s.total_duration_ms,
        }
    }
}

impl From<TestEvent> for StudioTestEvent {
    fn from(e: TestEvent) -> Self {
        match e {
            TestEvent::SessionStarted { session_id } => {
                StudioTestEvent::SessionStarted { session_id }
            }
            TestEvent::SessionFinished { summary } => StudioTestEvent::SessionFinished {
                summary: summary.into(),
            },
            TestEvent::FlowStarted {
                flow_name,
                flow_path,
                command_count,
                depth,
            } => StudioTestEvent::FlowStarted {
                flow_name,
                flow_path,
                command_count,
                depth,
            },
            TestEvent::FlowFinished {
                flow_name,
                status,
                duration_ms,
                depth,
            } => StudioTestEvent::FlowFinished {
                flow_name,
                status: format!("{:?}", status),
                duration_ms,
                depth,
            },
            TestEvent::CommandStarted {
                flow_name,
                index,
                command,
                depth,
            } => StudioTestEvent::CommandStarted {
                flow_name,
                index,
                command,
                depth,
            },
            TestEvent::CommandPassed {
                flow_name,
                index,
                duration_ms,
                depth,
            } => StudioTestEvent::CommandPassed {
                flow_name,
                index,
                duration_ms,
                depth,
            },
            TestEvent::CommandFailed {
                flow_name,
                index,
                error,
                duration_ms,
                depth,
            } => StudioTestEvent::CommandFailed {
                flow_name,
                index,
                error,
                duration_ms,
                depth,
            },
            TestEvent::CommandRetrying {
                flow_name,
                index,
                attempt,
                max_attempts,
                depth,
            } => StudioTestEvent::CommandRetrying {
                flow_name,
                index,
                attempt,
                max_attempts,
                depth,
            },
            TestEvent::CommandSkipped {
                flow_name,
                index,
                reason,
                depth,
            } => StudioTestEvent::CommandSkipped {
                flow_name,
                index,
                reason,
                depth,
            },
            TestEvent::Log { message, depth } => StudioTestEvent::Log { message, depth },
        }
    }
}

#[tauri::command]
async fn run_test_flow(
    window: Window,
    content: String,
    filename: String,
    file_path: String,
    platform: String,
    device: Option<String>,
) -> Result<String, String> {
    // Write temp file in the SAME directory as original to allow relative paths (e.g. data.csv) to work
    println!(
        "DEBUG: run_test_flow called with filename: {}, file_path: {}",
        filename, file_path
    );

    let original_path = std::path::Path::new(&file_path);
    let parent_dir = original_path.parent().unwrap_or(std::path::Path::new("."));

    // Create hidden temp file
    // Note: If filename already has extension, appending .tmp might be weird but safe.
    let temp_filename = format!(".{}.lumi_tmp_run", filename);
    let temp_file_path = parent_dir.join(&temp_filename);

    println!("DEBUG: Writing temp run file to: {:?}", temp_file_path);

    std::fs::write(&temp_file_path, &content).map_err(|e| e.to_string())?;

    let driver: Box<dyn PlatformDriver> = match platform.as_str() {
        "android" => Box::new(
            lumi_tester::driver::android::AndroidDriver::new(device.as_deref())
                .await
                .map_err(|e| e.to_string())?,
        ),
        "ios" => Box::new(
            lumi_tester::driver::ios::IosDriver::new(device.as_deref())
                .await
                .map_err(|e| e.to_string())?,
        ),
        "web" => {
            // Always show browser when running from Studio (headless = false)
            let mut config = lumi_tester::driver::web::WebDriverConfig::default();
            config.headless = false;
            Box::new(
                lumi_tester::driver::web::WebDriver::new(config)
                    .await
                    .map_err(|e| e.to_string())?,
            )
        }
        _ => return Err(format!("Unknown platform: {}", platform)),
    };

    // Keep output directory in TEMP to avoid watcher loop
    let output_dir = std::env::temp_dir().join("lumi-studio-output");

    // Correctly pass output_dir as 2nd argument
    let mut executor = TestExecutor::new(driver, Some(&output_dir), false, false, None);
    let mut rx = executor.subscribe();
    let window_handle = window.clone();

    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let studio_event: StudioTestEvent = event.into();
            let _ = window_handle.emit("test-event", studio_event);
        }
    });

    let run_res = executor.run_file(&temp_file_path, None, None).await;
    let _ = executor.finish().await;
    let _ = std::fs::remove_file(&temp_file_path);

    run_res.map_err(|e| e.to_string())?;
    Ok("Run complete".to_string())
}

#[tauri::command]
async fn list_devices(platform: String) -> Result<Vec<DeviceInfo>, String> {
    match platform.as_str() {
        "android" => {
            let devices = lumi_tester::driver::android::adb::get_devices()
                .await
                .map_err(|e| e.to_string())?;

            let mut device_infos = Vec::new();
            for device in devices {
                // Get device name using adb shell getprop
                let name = lumi_tester::driver::android::adb::shell(
                    Some(&device.serial),
                    "getprop ro.product.model",
                )
                .await
                .unwrap_or_else(|_| String::new())
                .trim()
                .to_string();

                // Fallback to ro.product.name if model is empty
                let name = if name.is_empty() {
                    lumi_tester::driver::android::adb::shell(
                        Some(&device.serial),
                        "getprop ro.product.name",
                    )
                    .await
                    .unwrap_or_else(|_| device.serial.clone())
                    .trim()
                    .to_string()
                } else {
                    name
                };

                // If still empty, use serial as name
                let name = if name.is_empty() {
                    device.serial.clone()
                } else {
                    format!("{} ({})", name, device.serial)
                };

                device_infos.push(DeviceInfo {
                    id: device.serial,
                    name,
                });
            }
            Ok(device_infos)
        }
        "ios" => {
            let targets = lumi_tester::driver::ios::idb::list_targets()
                .await
                .map_err(|e| e.to_string())?;
            Ok(targets
                .into_iter()
                .map(|t| DeviceInfo {
                    id: t.udid.clone(),
                    name: format!("{} ({})", t.name, t.udid),
                })
                .collect())
        }
        _ => Ok(vec![]),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![run_test_flow, list_devices])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
