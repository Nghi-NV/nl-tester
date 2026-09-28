use lumi_tester::driver::traits::PlatformDriver;
use lumi_tester::runner::{events::TestEvent, executor::TestExecutor, state::TestSummary};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{Emitter, Window};

mod workspace;
mod terminal;
mod ai;
mod extensions;
mod search;

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
    CommandAutoHealed {
        flow_name: String,
        index: usize,
        original_selector: String,
        healed_target: String,
        confidence: f32,
        suggestion: String,
        depth: usize,
    },
    AppCrashed {
        app_id: String,
        flow_name: String,
        command_index: usize,
        depth: usize,
    },
    Log {
        message: String,
        depth: usize,
    },
}

#[derive(Serialize, Clone)]
struct StudioRunEvent {
    #[serde(flatten)]
    event: StudioTestEvent,
    run_id: String,
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

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StudioRunResult {
    pub summary: Option<StudioTestSummary>,
    pub output_path: String,
    pub cancelled: bool,
}

#[derive(Default)]
struct ActiveRuns {
    cancel: Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>,
}

#[derive(Default)]
struct ActiveInspector {
    session: tokio::sync::Mutex<Option<lumi_tester::inspector::server::InspectorSession>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YamlDiagnostic {
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YamlValidation {
    pub valid: bool,
    pub diagnostics: Vec<YamlDiagnostic>,
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
            TestEvent::CommandAutoHealed {
                flow_name,
                index,
                original_selector,
                healed_target,
                confidence,
                suggestion,
                depth,
            } => StudioTestEvent::CommandAutoHealed {
                flow_name,
                index,
                original_selector,
                healed_target,
                confidence,
                suggestion,
                depth,
            },
            TestEvent::AppCrashed {
                app_id,
                flow_name,
                command_index,
                depth,
            } => StudioTestEvent::AppCrashed {
                app_id,
                flow_name,
                command_index,
                depth,
            },
            TestEvent::Log { message, depth } => StudioTestEvent::Log { message, depth },
        }
    }
}

#[tauri::command]
async fn run_test_flow(
    window: Window,
    workspace: tauri::State<'_, workspace::WorkspaceState>,
    active_runs: tauri::State<'_, ActiveRuns>,
    content: String,
    file_path: String,
    run_id: String,
    platform: String,
    device: Option<String>,
    command_index: Option<usize>,
    from_command_index: Option<usize>,
) -> Result<StudioRunResult, String> {
    // Keep the temporary flow beside the source so relative data/env paths resolve as usual.
    let original_path = workspace.resolve_existing(std::path::Path::new(&file_path))?;
    let parent_dir = original_path.parent().unwrap_or(std::path::Path::new("."));
    let mut temp_file = tempfile::Builder::new()
        .prefix(".lumi-run-")
        .suffix(".lumi_tmp_run")
        .tempfile_in(parent_dir)
        .map_err(|error| error.to_string())?;
    std::io::Write::write_all(&mut temp_file, content.as_bytes())
        .map_err(|error| error.to_string())?;
    let temp_file_path = temp_file.path().to_path_buf();

    let output_dir = tempfile::tempdir().map_err(|error| error.to_string())?.keep();
    let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();
    active_runs
        .cancel
        .lock()
        .map_err(|_| "Run state is unavailable".to_string())?
        .insert(run_id.clone(), cancel_tx);

    let driver_future = async {
        match platform.as_str() {
            "android" => Ok(Box::new(
                lumi_tester::driver::android::AndroidDriver::new(device.as_deref())
                    .await
                    .map_err(|error| error.to_string())?,
            ) as Box<dyn PlatformDriver>),
            "ios" => Ok(Box::new(
                lumi_tester::driver::ios::IosDriver::new(device.as_deref())
                    .await
                    .map_err(|error| error.to_string())?,
            ) as Box<dyn PlatformDriver>),
            "web" => {
                let mut config = lumi_tester::driver::web::WebDriverConfig::default();
                config.headless = false;
                Ok(Box::new(
                    lumi_tester::driver::web::WebDriver::new(config)
                        .await
                        .map_err(|error| error.to_string())?,
                ) as Box<dyn PlatformDriver>)
            }
            _ => Err(format!("Unknown platform: {}", platform)),
        }
    };

    let driver_result = tokio::select! {
        result = driver_future => Some(result),
        _ = &mut cancel_rx => None,
    };
    let driver = match driver_result {
        None => {
            if let Ok(mut runs) = active_runs.cancel.lock() {
                runs.remove(&run_id);
            }
            return Ok(StudioRunResult {
                summary: None,
                output_path: output_dir.to_string_lossy().into_owned(),
                cancelled: true,
            });
        }
        Some(Ok(driver)) => driver,
        Some(Err(error)) => {
            if let Ok(mut runs) = active_runs.cancel.lock() {
                runs.remove(&run_id);
            }
            return Err(error);
        }
    };

    // Correctly pass output_dir as 2nd argument
    let mut executor =
        TestExecutor::new(driver, Some(&output_dir), false, false, false, true, None);
    let mut rx = executor.subscribe();
    let window_handle = window.clone();
    let temp_path_for_events = temp_file_path.clone();
    let source_path_for_events = original_path.clone();
    let run_id_for_events = run_id.clone();
    let (summary_tx, summary_rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        let mut summary_tx = Some(summary_tx);
        while let Ok(event) = rx.recv().await {
            let is_finished = matches!(&event, TestEvent::SessionFinished { .. });
            let mut studio_event: StudioTestEvent = event.into();
            if let StudioTestEvent::FlowStarted { flow_path, .. } = &mut studio_event {
                if std::path::Path::new(flow_path) == temp_path_for_events {
                    *flow_path = source_path_for_events.to_string_lossy().into_owned();
                }
            }
            let finished_summary = match &studio_event {
                StudioTestEvent::SessionFinished { summary } => Some(summary.clone()),
                _ => None,
            };
            let _ = window_handle.emit(
                "test-event",
                StudioRunEvent {
                    event: studio_event,
                    run_id: run_id_for_events.clone(),
                },
            );
            if let Some(summary) = finished_summary {
                if let Some(sender) = summary_tx.take() {
                    let _ = sender.send(summary);
                }
            }
            if is_finished {
                break;
            }
        }
    });

    let (run_res, cancelled) = tokio::select! {
        result = executor.run_file(&temp_file_path, command_index, from_command_index, None) => (result, false),
        _ = &mut cancel_rx => (Ok(()), true),
    };
    if let Ok(mut runs) = active_runs.cancel.lock() {
        runs.remove(&run_id);
    }
    let finish_res = executor.finish().await;

    run_res.map_err(|e| e.to_string())?;
    finish_res.map_err(|error| error.to_string())?;
    let summary = summary_rx.await.ok();
    Ok(StudioRunResult {
        summary,
        output_path: output_dir.to_string_lossy().into_owned(),
        cancelled,
    })
}

#[tauri::command]
fn stop_test_flow(run_id: String, active_runs: tauri::State<'_, ActiveRuns>) -> bool {
    let sender = active_runs
        .cancel
        .lock()
        .ok()
        .and_then(|mut runs| runs.remove(&run_id));
    sender.is_some_and(|sender| sender.send(()).is_ok())
}

#[tauri::command]
async fn start_inspector(
    workspace_path: String,
    output_path: String,
    platform: String,
    device: Option<String>,
    workspace: tauri::State<'_, workspace::WorkspaceState>,
    inspector: tauri::State<'_, ActiveInspector>,
) -> Result<u16, String> {
    let root = workspace.resolve_existing(std::path::Path::new(&workspace_path))?;
    if !root.is_dir() {
        return Err("Open a workspace folder before starting Inspector".to_string());
    }
    let output = workspace.resolve_existing(std::path::Path::new(&output_path))?;
    if !output.is_file() {
        return Err("Open a YAML file as the Inspector output first".to_string());
    }

    let previous = inspector.session.lock().await.take();
    if let Some(previous) = previous {
        previous.stop().await.map_err(|error| error.to_string())?;
    }

    let server = lumi_tester::inspector::InspectorServer::new(
        lumi_tester::inspector::server::InspectorConfig {
            port: 0,
            platform,
            device_serial: device,
            output_file: Some(output),
            workspace_root: Some(root),
        },
    );
    let session = server
        .start_embedded()
        .await
        .map_err(|error| error.to_string())?;
    let port = session.port;
    *inspector.session.lock().await = Some(session);
    Ok(port)
}

#[tauri::command]
async fn stop_inspector(inspector: tauri::State<'_, ActiveInspector>) -> Result<bool, String> {
    let session = inspector.session.lock().await.take();
    if let Some(session) = session {
        session.stop().await.map_err(|error| error.to_string())?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
async fn get_inspector_port(
    inspector: tauri::State<'_, ActiveInspector>,
) -> Result<Option<u16>, String> {
    Ok(inspector.session.lock().await.as_ref().map(|session| session.port))
}

#[tauri::command]
async fn validate_yaml_content(
    path: String,
    content: String,
    workspace: tauri::State<'_, workspace::WorkspaceState>,
) -> Result<YamlValidation, String> {
    let source_path = workspace.resolve_existing(std::path::Path::new(&path))?;
    tauri::async_runtime::spawn_blocking(move || {
        match lumi_tester::parser::yaml::parse_yaml_content(&content, &source_path) {
            Ok(_) => YamlValidation {
                valid: true,
                diagnostics: Vec::new(),
            },
            Err(error) => {
                let location = error.chain().find_map(|cause| {
                    cause
                        .downcast_ref::<serde_yaml::Error>()
                        .and_then(serde_yaml::Error::location)
                        .map(|location| (location.line(), location.column()))
                });
                YamlValidation {
                    valid: false,
                    diagnostics: vec![YamlDiagnostic {
                        message: error.to_string(),
                        line: location.map(|value| value.0),
                        column: location.map(|value| value.1),
                    }],
                }
            }
        }
    })
    .await
    .map_err(|error| format!("YAML validation task failed: {error}"))
}

#[tauri::command]
fn get_lumi_yaml_schema() -> &'static str {
    include_str!("../../../lumi-tester/schema/lumi-test.schema.json")
}

#[tauri::command]
async fn list_devices(platform: String) -> Result<Vec<DeviceInfo>, String> {
    match platform.as_str() {
        "android" => {
            let devices = lumi_tester::driver::android::adb::get_devices()
                .await
                .map_err(|e| e.to_string())?;

            // Resolve each device's display name concurrently instead of one device at
            // a time - each is 1-2 `adb shell getprop` round-trips (~50-300ms each),
            // and this call runs on a 5s poll timer, so N devices used to cost N x that
            // sequentially.
            let tasks: Vec<_> = devices
                .into_iter()
                .map(|device| {
                    tokio::spawn(async move {
                        let serial = device.serial;

                        // Get device name using adb shell getprop
                        let name = lumi_tester::driver::android::adb::shell(
                            Some(&serial),
                            "getprop ro.product.model",
                        )
                        .await
                        .unwrap_or_else(|_| String::new())
                        .trim()
                        .to_string();

                        // Fallback to ro.product.name if model is empty
                        let name = if name.is_empty() {
                            lumi_tester::driver::android::adb::shell(
                                Some(&serial),
                                "getprop ro.product.name",
                            )
                            .await
                            .unwrap_or_else(|_| serial.clone())
                            .trim()
                            .to_string()
                        } else {
                            name
                        };

                        // If still empty, use serial as name
                        let name = if name.is_empty() {
                            serial.clone()
                        } else {
                            format!("{} ({})", name, serial)
                        };

                        DeviceInfo { id: serial, name }
                    })
                })
                .collect();

            let mut device_infos = Vec::new();
            for task in tasks {
                if let Ok(info) = task.await {
                    device_infos.push(info);
                }
            }
            Ok(device_infos)
        }
        "ios" => {
            let targets = lumi_tester::driver::ios::devicectl::list_targets()
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
        .manage(workspace::WorkspaceState::default())
        .manage(ActiveRuns::default())
        .manage(ActiveInspector::default())
        .manage(terminal::TerminalSessions::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            run_test_flow,
            stop_test_flow,
            start_inspector,
            stop_inspector,
            get_inspector_port,
            validate_yaml_content,
            get_lumi_yaml_schema,
            list_devices,
            ai::detect_ai_providers,
            ai::generate_ai_response,
            extensions::list_lumi_extensions,
            extensions::install_lumi_extension,
            extensions::set_lumi_extension_enabled,
            extensions::remove_lumi_extension,
            terminal::start_terminal,
            terminal::write_terminal,
            terminal::resize_terminal,
            terminal::stop_terminal,
            search::search_workspace_text,
            workspace::open_workspace,
            workspace::resolve_workspace_file_reference,
            workspace::read_workspace_dir,
            workspace::read_workspace_file,
            workspace::write_workspace_file,
            workspace::create_workspace_file,
            workspace::create_workspace_dir,
            workspace::remove_workspace_entry,
            workspace::move_workspace_entry
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
