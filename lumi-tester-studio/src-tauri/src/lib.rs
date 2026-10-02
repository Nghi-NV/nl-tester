use lumi_tester::driver::traits::PlatformDriver;
use lumi_tester::runner::{events::TestEvent, executor::TestExecutor, state::TestSummary};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{Emitter, Manager, Window};

mod adb_server;
mod ai;
mod extensions;
mod recent_projects;
mod search;
mod source_control;
mod terminal;
mod workspace;

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
struct PendingRecentProjects(Mutex<Vec<String>>);

#[tauri::command]
fn take_pending_recent_projects(state: tauri::State<'_, PendingRecentProjects>) -> Vec<String> {
    state
        .0
        .lock()
        .map(|mut paths| paths.drain(..).collect())
        .unwrap_or_default()
}

#[tauri::command]
fn exit_application(app: tauri::AppHandle) {
    app.exit(0);
}

impl ActiveRuns {
    fn cancel_all(&self) {
        let senders = self
            .cancel
            .lock()
            .map(|mut runs| runs.drain().map(|(_, sender)| sender).collect::<Vec<_>>())
            .unwrap_or_default();
        for sender in senders {
            let _ = sender.send(());
        }
    }
}

fn stop_background_processes(app: &tauri::AppHandle) {
    app.state::<ai::AiRequestState>().cancel_all();
    app.state::<ActiveRuns>().cancel_all();
    app.state::<terminal::TerminalSessions>().stop_all();
}

#[derive(Default)]
struct ActiveInspector {
    session: tokio::sync::Mutex<Option<lumi_tester::inspector::server::InspectorSession>>,
    output_path: tokio::sync::Mutex<Option<std::path::PathBuf>>,
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
fn set_gps_control(
    speed: Option<f64>,
    paused: Option<bool>,
    speed_mode: Option<String>,
) -> Result<(), String> {
    let mut control = serde_json::Map::new();

    if let Some(speed) = speed {
        if !speed.is_finite() || !(0.0..=200.0).contains(&speed) {
            return Err("GPS speed must be between 0 and 200 km/h".to_string());
        }
        control.insert("speed".to_string(), serde_json::json!(speed));
    }
    if let Some(paused) = paused {
        control.insert("paused".to_string(), serde_json::json!(paused));
    }
    if let Some(speed_mode) = speed_mode {
        if speed_mode != "linear" && speed_mode != "noise" {
            return Err("GPS speed mode must be linear or noise".to_string());
        }
        control.insert("speedMode".to_string(), serde_json::json!(speed_mode));
    }
    if control.is_empty() {
        return Err("No GPS control update was provided".to_string());
    }

    let update = serde_json::to_vec(&serde_json::Value::Object(control))
        .map_err(|error| format!("Could not encode GPS control update: {error}"))?;
    write_gps_control_atomically(std::path::Path::new("/tmp/lumi-gps-control.json"), &update)
}

fn write_gps_control_atomically(path: &std::path::Path, update: &[u8]) -> Result<(), String> {
    use std::io::Write;

    let parent = path
        .parent()
        .ok_or_else(|| "GPS control path has no parent directory".to_string())?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("Could not create a private GPS control update: {error}"))?;
    temporary
        .write_all(update)
        .map_err(|error| format!("Could not write the GPS control update: {error}"))?;
    temporary
        .persist(path)
        .map_err(|error| format!("Could not update the active GPS route: {}", error.error))?;
    Ok(())
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

    let output_dir = tempfile::tempdir()
        .map_err(|error| error.to_string())?
        .keep();
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
    *inspector.output_path.lock().await = None;

    let server = lumi_tester::inspector::InspectorServer::new(
        lumi_tester::inspector::server::InspectorConfig {
            port: 0,
            platform,
            device_serial: device,
            output_file: Some(output.clone()),
            workspace_root: Some(root),
        },
    );
    let session = server
        .start_embedded()
        .await
        .map_err(|error| error.to_string())?;
    let port = session.port;
    *inspector.session.lock().await = Some(session);
    *inspector.output_path.lock().await = Some(output);
    Ok(port)
}

#[tauri::command]
async fn stop_inspector(inspector: tauri::State<'_, ActiveInspector>) -> Result<bool, String> {
    let session = inspector.session.lock().await.take();
    *inspector.output_path.lock().await = None;
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
    Ok(inspector
        .session
        .lock()
        .await
        .as_ref()
        .map(|session| session.port))
}

#[tauri::command]
async fn get_inspector_output_path(
    inspector: tauri::State<'_, ActiveInspector>,
) -> Result<Option<String>, String> {
    Ok(inspector
        .output_path
        .lock()
        .await
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned()))
}

fn map_yaml_segment_location(
    content: &str,
    segment_start: usize,
    line: usize,
    column: usize,
) -> (usize, usize) {
    let prefix = &content[..segment_start.min(content.len())];
    let mapped_line =
        (prefix.matches('\n').count() + line.max(1)).min(content.split('\n').count().max(1));
    let prefix_column = if line <= 1 {
        prefix
            .rsplit('\n')
            .next()
            .unwrap_or_default()
            .trim_end_matches('\r')
            .chars()
            .count()
    } else {
        0
    };
    let source_line = content
        .split('\n')
        .nth(mapped_line.saturating_sub(1))
        .unwrap_or_default()
        .trim_end_matches('\r');
    let source_column = column.max(1).saturating_add(prefix_column);
    let mapped_column = source_line
        .chars()
        .take(source_column.saturating_sub(1))
        .map(char::len_utf16)
        .sum::<usize>()
        + 1;
    let max_column = source_line.encode_utf16().count() + 1;
    (mapped_line, mapped_column.min(max_column))
}

fn yaml_error_location(
    content: &str,
    line: usize,
    column: usize,
    header_error: bool,
) -> (usize, usize) {
    let segment_start = match content.find("---") {
        Some(separator_start) if header_error => {
            let header = &content[..separator_start];
            header.len() - header.trim_start().len()
        }
        Some(separator_start) => {
            let command_start = separator_start + 3;
            let commands = &content[command_start..];
            command_start + commands.len() - commands.trim_start().len()
        }
        None => 0,
    };
    map_yaml_segment_location(content, segment_start, line, column)
}

fn yaml_command_location(content: &str, command_index: usize) -> Option<(usize, usize)> {
    let segment_start = match content.find("---") {
        Some(separator_start) => {
            let command_start = separator_start + 3;
            let commands = &content[command_start..];
            command_start + commands.len() - commands.trim_start().len()
        }
        None => content.len() - content.trim_start().len(),
    };
    let segment = &content[segment_start..];
    let commands = segment
        .lines()
        .enumerate()
        .filter_map(|(line_index, line)| {
            let indentation = line.len() - line.trim_start().len();
            let trimmed = &line[indentation..];
            if trimmed.starts_with("---")
                || !trimmed.starts_with('-')
                || trimmed
                    .chars()
                    .nth(1)
                    .is_some_and(|character| !character.is_whitespace())
            {
                return None;
            }
            Some((line_index, indentation, line))
        })
        .collect::<Vec<_>>();
    let root_indentation = commands.iter().map(|(_, indent, _)| *indent).min();

    if let Some((line_index, indentation, line)) = commands
        .into_iter()
        .filter(|(_, indent, _)| Some(*indent) == root_indentation)
        .nth(command_index)
    {
        let after_dash = &line[indentation + 1..];
        let column_offset = after_dash.len() - after_dash.trim_start().len();
        let byte_column = indentation + 1 + column_offset;
        let column = line[..byte_column].encode_utf16().count() + 1;
        let preceding_lines = content[..segment_start].matches('\n').count();
        return Some((preceding_lines + line_index + 1, column));
    }

    if root_indentation.is_none() {
        let first_line = segment.lines().next()?;
        let column =
            first_line.encode_utf16().count() - first_line.trim_start().encode_utf16().count() + 1;
        return Some((content[..segment_start].matches('\n').count() + 1, column));
    }

    None
}

fn yaml_diagnostic_location(
    content: &str,
    causes: &[String],
    yaml_location: Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    let command_index = causes.iter().find_map(|cause| {
        cause
            .strip_prefix("Failed to parse command #")?
            .split_whitespace()
            .next()?
            .trim_end_matches(':')
            .parse::<usize>()
            .ok()
    });
    command_index
        .and_then(|index| yaml_command_location(content, index.saturating_sub(1)))
        .or_else(|| yaml_header_field_error_location(content, causes))
        .or_else(|| {
            let header_error = causes
                .iter()
                .any(|cause| cause.contains("Failed to parse YAML header"));
            yaml_location
                .map(|(line, column)| yaml_error_location(content, line, column, header_error))
        })
        .or_else(|| {
            causes
                .iter()
                .find_map(|cause| cause.strip_prefix("Unknown command: "))
                .and_then(|name| unknown_command_location(content, name))
        })
}

fn yaml_header_field_error_location(content: &str, causes: &[String]) -> Option<(usize, usize)> {
    let fields = causes.iter().find_map(|cause| {
        cause
            .strip_prefix("Failed to parse header field: ")
            .or_else(|| {
                cause
                    .contains("Failed to parse camera/cameras header")
                    .then_some("camera|cameras")
            })
            .or_else(|| {
                cause
                    .starts_with("Failed to read env file:")
                    .then_some("env")
            })
    })?;
    let header_end = content.find("---").unwrap_or(content.len());
    let lines = content[..header_end].lines().collect::<Vec<_>>();

    for field in fields.split('|') {
        if let Some(location) = lines.iter().enumerate().find_map(|(line_index, line)| {
            let indentation = line.len() - line.trim_start().len();
            let trimmed = &line[indentation..];
            let (key, value) = trimmed.split_once(':')?;
            if key
                .trim()
                .trim_matches(|character| character == '"' || character == '\'')
                != field
            {
                return None;
            }
            let value_column = value.len() - value.trim_start().len();
            let byte_column = indentation + key.len() + 1 + value_column;
            if !value.trim().is_empty() {
                return Some((
                    line_index + 1,
                    line[..byte_column].encode_utf16().count() + 1,
                ));
            }

            for (child_index, child_line) in lines.iter().enumerate().skip(line_index + 1) {
                if child_line.trim().is_empty() {
                    continue;
                }
                let child_indentation = child_line.len() - child_line.trim_start().len();
                if child_indentation <= indentation {
                    break;
                }
                let child_text = &child_line[child_indentation..];
                let child_value = child_text
                    .find(':')
                    .map(|colon| (colon, &child_text[colon + 1..]));
                let child_column = child_value
                    .map(|(colon, value)| {
                        child_indentation + colon + 1 + value.len() - value.trim_start().len()
                    })
                    .unwrap_or(child_indentation);
                return Some((
                    child_index + 1,
                    child_line[..child_column].encode_utf16().count() + 1,
                ));
            }

            Some((
                line_index + 1,
                line[..byte_column].encode_utf16().count() + 1,
            ))
        }) {
            return Some(location);
        }
    }
    None
}

fn unknown_command_location(content: &str, name: &str) -> Option<(usize, usize)> {
    let command_start_line = content
        .find("---")
        .map(|separator_start| content[..separator_start].matches('\n').count() + 1)
        .unwrap_or(0);
    content
        .lines()
        .enumerate()
        .skip(command_start_line)
        .find_map(|(line_index, line)| {
            let indentation = line.len() - line.trim_start().len();
            let command_line = line[indentation..].strip_prefix('-')?;
            let command_text = command_line.trim_start();
            let parsed_name = command_text
                .split_once(':')
                .map(|(key, _)| key)
                .unwrap_or(command_text)
                .trim()
                .trim_matches(|character| character == '"' || character == '\'');
            if parsed_name != name {
                return None;
            }
            let command_column =
                indentation + 1 + command_line.len() - command_line.trim_start().len();
            let column = line[..command_column].encode_utf16().count() + 1;
            Some((line_index + 1, column))
        })
}

#[tauri::command]
async fn validate_yaml_content(
    path: String,
    content: String,
    workspace: tauri::State<'_, workspace::WorkspaceState>,
) -> Result<YamlValidation, String> {
    let source_path = workspace.resolve_existing(std::path::Path::new(&path))?;
    tauri::async_runtime::spawn_blocking(
        move || match lumi_tester::parser::yaml::parse_yaml_content(&content, &source_path) {
            Ok(_) => YamlValidation {
                valid: true,
                diagnostics: Vec::new(),
            },
            Err(error) => {
                let causes = error.chain().map(ToString::to_string).collect::<Vec<_>>();
                let yaml_location = error.chain().find_map(|cause| {
                    cause
                        .downcast_ref::<serde_yaml::Error>()
                        .and_then(serde_yaml::Error::location)
                        .map(|location| (location.line(), location.column()))
                });
                let location = yaml_diagnostic_location(&content, &causes, yaml_location);
                YamlValidation {
                    valid: false,
                    diagnostics: vec![YamlDiagnostic {
                        message: format!("{error:#}"),
                        line: location.map(|value| value.0),
                        column: location.map(|value| value.1),
                    }],
                }
            }
        },
    )
    .await
    .map_err(|error| format!("YAML validation task failed: {error}"))
}

#[tauri::command]
fn get_lumi_yaml_schema() -> &'static str {
    include_str!("../../../lumi-tester/schema/lumi-test.schema.json")
}

#[tauri::command]
async fn list_devices(
    platform: String,
    adb_server: tauri::State<'_, adb_server::AdbServerLifecycle>,
) -> Result<Vec<DeviceInfo>, String> {
    match platform.as_str() {
        "android" => {
            adb_server.mark_started_if_missing();
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
    let app = tauri::Builder::default()
        .manage(workspace::WorkspaceState::default())
        .manage(adb_server::AdbServerLifecycle::default())
        .manage(ai::AiRequestState::default())
        .manage(ActiveRuns::default())
        .manage(PendingRecentProjects::default())
        .manage(ActiveInspector::default())
        .manage(terminal::TerminalSessions::default())
        .setup(|app| {
            adb_server::cleanup_orphaned_app_server(app.handle());
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            run_test_flow,
            stop_test_flow,
            set_gps_control,
            start_inspector,
            stop_inspector,
            get_inspector_port,
            get_inspector_output_path,
            validate_yaml_content,
            get_lumi_yaml_schema,
            list_devices,
            ai::detect_ai_providers,
            ai::generate_ai_response,
            ai::cancel_ai_response,
            extensions::list_lumi_extensions,
            extensions::install_lumi_extension,
            extensions::set_lumi_extension_enabled,
            extensions::remove_lumi_extension,
            terminal::start_terminal,
            terminal::write_terminal,
            terminal::resize_terminal,
            terminal::stop_terminal,
            search::search_workspace_text,
            search::replace_workspace_text,
            source_control::source_control_status,
            source_control::source_control_init,
            source_control::source_control_diff,
            source_control::source_control_stage,
            source_control::source_control_switch_branch,
            source_control::source_control_commit,
            workspace::open_workspace,
            workspace::resolve_workspace_file_reference,
            workspace::read_workspace_dir,
            workspace::list_workspace_file_paths,
            workspace::read_workspace_file,
            workspace::write_workspace_file,
            workspace::create_workspace_file,
            workspace::create_workspace_dir,
            workspace::remove_workspace_entry,
            workspace::move_workspace_entry,
            workspace::copy_workspace_entry,
            take_pending_recent_projects,
            exit_application
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| match event {
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Opened { urls } => {
            let paths = urls
                .into_iter()
                .filter_map(|url| url.to_file_path().ok())
                .filter(|path| path.is_dir())
                .filter_map(|path| std::fs::canonicalize(path).ok())
                .map(|path| path.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            if !paths.is_empty() {
                let pending = app_handle.state::<PendingRecentProjects>();
                if let Ok(mut queued) = pending.0.lock() {
                    for path in paths {
                        if !queued.contains(&path) {
                            queued.push(path);
                        }
                    }
                }
                let _ = app_handle.emit("lumi-open-recent-project", ());
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        }
        tauri::RunEvent::WindowEvent {
            event: tauri::WindowEvent::Destroyed,
            ..
        } if app_handle.webview_windows().is_empty() => app_handle.exit(0),
        tauri::RunEvent::ExitRequested { .. } => {
            stop_background_processes(app_handle);
            adb_server::cleanup_on_exit(app_handle);
        }
        _ => {}
    });
}

#[cfg(test)]
mod validation_location_tests {
    use super::{map_yaml_segment_location, unknown_command_location, yaml_diagnostic_location};
    use std::path::Path;

    fn location_for_invalid_yaml(content: &str) -> Option<(usize, usize)> {
        location_for_invalid_yaml_at(content, Path::new("flow.yaml"))
    }

    fn location_for_invalid_yaml_at(content: &str, path: &Path) -> Option<(usize, usize)> {
        let error = lumi_tester::parser::yaml::parse_yaml_content(content, path)
            .expect_err("fixture should fail parsing");
        let causes = error.chain().map(ToString::to_string).collect::<Vec<_>>();
        let yaml_location = error.chain().find_map(|cause| {
            cause
                .downcast_ref::<serde_yaml::Error>()
                .and_then(serde_yaml::Error::location)
                .map(|location| (location.line(), location.column()))
        });
        yaml_diagnostic_location(content, &causes, yaml_location)
    }

    #[test]
    fn locates_unknown_shorthand_command_at_command_token() {
        let content = "appId: com.example\n---\n- launchApp\n- dfaf\n";
        assert_eq!(location_for_invalid_yaml(content), Some((4, 3)));
        assert_eq!(unknown_command_location(content, "dfaf"), Some((4, 3)));
    }

    #[test]
    fn locates_unknown_mapping_command_at_command_token() {
        let content = "platform: android\n---\n- launchApp\n- noSuchCommand:\n    text: x\n";
        assert_eq!(location_for_invalid_yaml(content), Some((4, 3)));
        assert_eq!(
            unknown_command_location(content, "noSuchCommand"),
            Some((4, 3))
        );
    }

    #[test]
    fn locates_bad_command_parameters_on_the_failing_command() {
        let content = "platform: android\n---\n- launchApp\n- tap:\n    index: not-a-number\n";
        assert_eq!(location_for_invalid_yaml(content), Some((4, 3)));
    }

    #[test]
    fn locates_invalid_nested_commands_on_their_outer_command() {
        let content = "platform: android\n---\n- launchApp\n- forEach:\n    in: [one]\n    commands: [launchApp]\n";
        assert_eq!(location_for_invalid_yaml(content), Some((4, 3)));
    }

    #[test]
    fn locates_invalid_command_value_shape() {
        let content = "platform: android\n---\n- launchApp\n- 42\n";
        assert_eq!(location_for_invalid_yaml(content), Some((4, 3)));
    }

    #[test]
    fn locates_command_yaml_syntax_error_after_front_matter() {
        let content = "platform: android\n---\n- launchApp\n- tap:\n    text: [unterminated\n";
        assert_eq!(
            location_for_invalid_yaml(content).map(|location| location.0),
            Some(6)
        );
    }

    #[test]
    fn locates_invalid_custom_section_indentation_at_the_yaml_error() {
        let content = "platform: android\n---\n  - launchApp\n  - noSuchCommand:\n      text: x\n";
        assert_eq!(location_for_invalid_yaml(content), Some((4, 18)));
    }

    #[test]
    fn locates_header_yaml_syntax_error_before_front_matter() {
        let content = "platform: android\nappId: [unterminated\n---\n- launchApp\n";
        assert_eq!(
            location_for_invalid_yaml(content).map(|location| location.0),
            Some(2)
        );
    }

    #[test]
    fn locates_invalid_commands_in_legacy_sequence_yaml() {
        let content = "- launchApp\r\n- unknownStep\r\n";
        assert_eq!(location_for_invalid_yaml(content), Some((2, 3)));
    }

    #[test]
    fn locates_invalid_commands_in_test_flow_commands_field() {
        let content = "platform: android\ncommands:\n  - launchApp\n  - tap:\n      index: nope\n";
        assert_eq!(location_for_invalid_yaml(content), Some((4, 5)));
    }

    #[test]
    fn locates_invalid_platform_in_legacy_test_flow_header() {
        let content = "platform: not-a-platform\nappId: com.example\n";
        assert_eq!(location_for_invalid_yaml(content), Some((1, 11)));
    }

    #[test]
    fn locates_missing_env_file_in_yaml_header() {
        let content = "env:\n  file: '.lumi-test-env-file-that-does-not-exist'\n---\n- launchApp\n";
        assert_eq!(
            location_for_invalid_yaml_at(content, Path::new("/tmp/flow.yaml")),
            Some((2, 9))
        );
    }

    #[test]
    fn maps_unicode_columns_to_monaco_utf16_columns() {
        assert_eq!(map_yaml_segment_location("😀x\n", 0, 1, 2), (1, 3));
    }
}

#[cfg(all(test, unix))]
mod gps_control_security_tests {
    use super::write_gps_control_atomically;
    use std::os::unix::fs::symlink;

    #[test]
    fn gps_control_update_replaces_symlink_without_writing_through_it() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("outside.json");
        let destination = directory.path().join("gps-control.json");
        std::fs::write(&target, b"leave this file unchanged").unwrap();
        symlink(&target, &destination).unwrap();

        write_gps_control_atomically(&destination, b"{\"paused\":true}").unwrap();

        assert_eq!(
            std::fs::read(&target).unwrap(),
            b"leave this file unchanged"
        );
        assert_eq!(std::fs::read(&destination).unwrap(), b"{\"paused\":true}");
        assert!(!std::fs::symlink_metadata(&destination)
            .unwrap()
            .file_type()
            .is_symlink());
    }
}
