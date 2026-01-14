//! REST API endpoints for Inspector
//!
//! Provides endpoints for screenshot, hierarchy, element info, and file management.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::screen_capture::{self, ScreenCapture};
use crate::driver::android::uiautomator;
use crate::recorder::selector_scorer::{SelectorCandidate, SelectorScorer};

/// Shared state for API handlers
pub struct AppState {
    pub screen_capture: ScreenCapture,
    pub yaml_file: std::sync::Mutex<Option<std::path::PathBuf>>,
    pub device_serial: Option<String>,
    /// Cached UI hierarchy (dumped during screenshot capture)
    pub cached_hierarchy: std::sync::Mutex<Option<CachedHierarchy>>,
}

/// Cached parsed hierarchy
pub struct CachedHierarchy {
    pub elements: Vec<uiautomator::UiElement>,
}

/// Response for screenshot endpoint
#[derive(Serialize)]
pub struct ScreenshotResponse {
    pub data: String, // base64
    pub width: u32,
    pub height: u32,
}

/// Response for element at coordinates
#[derive(Serialize)]
pub struct ElementResponse {
    pub found: bool,
    pub selectors: Vec<SelectorInfo>,
    pub element_class: Option<String>,
    pub element_text: Option<String>,
    pub bounds: Option<BoundsInfo>,
}

#[derive(Serialize)]
pub struct SelectorInfo {
    pub selector_type: String,
    pub value: String,
    pub score: u32,
    pub is_stable: bool,
    pub yaml: String,
}

#[derive(Serialize)]
pub struct BoundsInfo {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Query params for element-at endpoint
#[derive(Deserialize)]
pub struct ElementAtQuery {
    pub x: i32,
    pub y: i32,
}

/// Request body for append command
#[derive(Deserialize)]
pub struct AppendCommandRequest {
    pub command_type: String,
    pub selector: Option<SelectorValue>,
    pub text: Option<String>,
}

/// Request body for execute action
#[derive(Deserialize)]
pub struct ExecuteRequest {
    pub action: String,
    pub x: i32,
    pub y: i32,
    pub selector: Option<SelectorValue>,
    pub text: Option<String>,
}

#[derive(Deserialize)]
pub struct SelectorValue {
    pub selector_type: String,
    pub value: String,
}

/// Request for creating/selecting file
#[derive(Deserialize)]
pub struct FileRequest {
    pub path: String,
    pub create_if_missing: bool,
}

#[derive(Serialize)]
pub struct FileResponse {
    pub success: bool,
    pub commands: Vec<String>,
    pub message: Option<String>,
}

/// Build API router
pub fn api_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/screenshot", get(get_screenshot))
        .route("/api/element-at", get(get_element_at))
        .route("/api/hierarchy", get(get_hierarchy))
        .route("/api/packages", get(get_packages))
        .route("/api/command", post(manage_command))
        .route("/api/append-command", post(append_command))
        .route("/api/file", post(select_file))
        .route("/api/file/commands", get(get_commands))
        .route("/api/play-command/:index", post(play_command))
        .route("/api/execute", post(execute_action))
}

#[derive(Deserialize)]
pub struct ScreenshotQuery {
    pub skip_hierarchy: Option<bool>,
}

/// GET /api/screenshot - Get current screenshot AND refresh hierarchy cache
async fn get_screenshot(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ScreenshotQuery>,
) -> impl IntoResponse {
    let skip_hierarchy = params.skip_hierarchy.unwrap_or(false);

    let screenshot_future = state.screen_capture.capture_base64();

    // If skipping hierarchy, use a dummy future that returns "skipped" immediately
    let hierarchy_future = async {
        if skip_hierarchy {
            Err("Skipped".to_string())
        } else {
            screen_capture::get_hierarchy_android(state.device_serial.as_deref())
                .await
                .map_err(|e| e.to_string())
        }
    };

    // Capture in parallel (if not skipped)
    let (screenshot_result, hierarchy_result) = tokio::join!(screenshot_future, hierarchy_future);

    // Update cache if we got new hierarchy
    if let Ok(hierarchy_xml) = hierarchy_result {
        if let Ok(elements) = uiautomator::parse_hierarchy(&hierarchy_xml) {
            let mut cache = state.cached_hierarchy.lock().unwrap();
            *cache = Some(CachedHierarchy { elements });
        }
    }

    match screenshot_result {
        Ok(data) => {
            let (width, height) = state.screen_capture.dimensions();
            Json(ScreenshotResponse {
                data,
                width,
                height,
            })
            .into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// GET /api/element-at?x=100&y=200 - Get element at coordinates (uses cached hierarchy)
async fn get_element_at(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ElementAtQuery>,
) -> Json<ElementResponse> {
    // Use cached hierarchy (much faster than dumping each time)
    // Clone immediately and drop lock to avoid holding across await
    let cached_elements = {
        let cache = state.cached_hierarchy.lock().unwrap();
        cache.as_ref().map(|c| c.elements.clone())
    };

    let elements = match cached_elements {
        Some(e) => e,
        None => {
            // No cache, need to dump (first time)
            let hierarchy =
                match screen_capture::get_hierarchy_android(state.device_serial.as_deref()).await {
                    Ok(h) => h,
                    Err(_e) => {
                        return Json(ElementResponse {
                            found: false,
                            selectors: vec![],
                            element_class: None,
                            element_text: None,
                            bounds: None,
                        });
                    }
                };
            match uiautomator::parse_hierarchy(&hierarchy) {
                Ok(e) => e,
                Err(_) => {
                    return Json(ElementResponse {
                        found: false,
                        selectors: vec![],
                        element_class: None,
                        element_text: None,
                        bounds: None,
                    });
                }
            }
        }
    };

    // Find element at coordinates
    let (width, height) = state.screen_capture.dimensions();
    let element = find_element_at(&elements, params.x, params.y);

    match element {
        Some(el) => {
            let scorer = SelectorScorer::new(width, height, elements);
            let candidates = scorer.score_element(&el);

            let selectors: Vec<SelectorInfo> = candidates
                .iter()
                .map(|c| SelectorInfo {
                    selector_type: c.selector_type.clone(),
                    value: c.value.clone(),
                    score: c.score,
                    is_stable: c.is_stable,
                    yaml: c.to_yaml("tap"),
                })
                .collect();

            Json(ElementResponse {
                found: true,
                selectors,
                element_class: Some(el.class.clone()),
                element_text: if el.text.is_empty() {
                    None
                } else {
                    Some(el.text.clone())
                },
                bounds: Some(BoundsInfo {
                    left: el.bounds.left,
                    top: el.bounds.top,
                    right: el.bounds.right,
                    bottom: el.bounds.bottom,
                }),
            })
        }
        None => Json(ElementResponse {
            found: false,
            selectors: vec![],
            element_class: None,
            element_text: None,
            bounds: None,
        }),
    }
}

/// GET /api/hierarchy - Get full UI hierarchy as JSON with element bounds
async fn get_hierarchy(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Try to use cached elements first
    let cached = {
        let cache = state.cached_hierarchy.lock().unwrap();
        cache.as_ref().map(|c| c.elements.clone())
    };

    let elements = if let Some(e) = cached {
        e
    } else {
        // Fetch fresh
        match screen_capture::get_hierarchy_android(state.device_serial.as_deref()).await {
            Ok(xml) => match uiautomator::parse_hierarchy(&xml) {
                Ok(e) => e,
                Err(_) => vec![],
            },
            Err(_) => vec![],
        }
    };

    // Convert to JSON-serializable format
    #[derive(Serialize)]
    struct ElementInfo {
        class: String,
        text: String,
        bounds: Option<BoundsInfo>,
    }

    let infos: Vec<ElementInfo> = elements
        .iter()
        .map(|e| ElementInfo {
            class: e.class.clone(),
            text: e.text.clone(),
            bounds: Some(BoundsInfo {
                left: e.bounds.left,
                top: e.bounds.top,
                right: e.bounds.right,
                bottom: e.bounds.bottom,
            }),
        })
        .collect();

    Json(serde_json::json!({ "elements": infos }))
}

/// Request for managing commands (insert/delete)
#[derive(Deserialize)]
struct ManageCommandRequest {
    action: String, // "insert" or "delete"
    index: Option<usize>,
    command: Option<CommandData>,
}

#[derive(Deserialize)]
struct CommandData {
    #[serde(rename = "type")]
    cmd_type: String,
    selector_type: Option<String>,
    value: Option<String>,
    text: Option<String>,
    app: Option<String>,
    #[serde(rename = "clearState")]
    clear_state: Option<bool>,
    ms: Option<u64>,
}

/// POST /api/command - Insert or delete commands
async fn manage_command(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ManageCommandRequest>,
) -> impl IntoResponse {
    let file = state.yaml_file.lock().unwrap();

    if file.is_none() {
        return (StatusCode::BAD_REQUEST, "No file selected").into_response();
    }

    let path = file.as_ref().unwrap().clone();
    drop(file);

    match request.action.as_str() {
        "delete" => {
            let idx = request.index.unwrap_or(0);
            if let Err(e) = delete_command_at(&path, idx) {
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
            }
            (StatusCode::OK, "Deleted").into_response()
        }
        "insert" => {
            let cmd = match request.command {
                Some(c) => c,
                None => return (StatusCode::BAD_REQUEST, "Missing command").into_response(),
            };

            let yaml_line = build_command_yaml(&cmd);

            if let Some(idx) = request.index {
                if let Err(e) = insert_command_at(&path, idx, &yaml_line) {
                    return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
                }
            } else {
                // Append to end
                use std::io::Write;
                let mut file = match std::fs::OpenOptions::new().append(true).open(&path) {
                    Ok(f) => f,
                    Err(e) => {
                        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
                    }
                };
                if let Err(e) = writeln!(file, "{}", yaml_line) {
                    return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
                }
            }
            (StatusCode::OK, "Inserted").into_response()
        }
        _ => (StatusCode::BAD_REQUEST, "Invalid action").into_response(),
    }
}

fn build_command_yaml(cmd: &CommandData) -> String {
    match cmd.cmd_type.as_str() {
        "tap" | "longPress" | "doubleTap" | "see" | "notSee" => {
            if let (Some(sel_type), Some(val)) = (&cmd.selector_type, &cmd.value) {
                format!("- {}:\n    {}: \"{}\"", cmd.cmd_type, sel_type, val)
            } else {
                format!("- {}:", cmd.cmd_type)
            }
        }
        "inputText" => {
            if let Some(text) = &cmd.text {
                format!("- inputText: \"{}\"", text)
            } else {
                "- inputText: \"\"".to_string()
            }
        }
        "open" => {
            if let Some(app) = &cmd.app {
                if cmd.clear_state.unwrap_or(false) {
                    format!("- open:\n    app: \"{}\"\n    clearState: true", app)
                } else {
                    format!("- open: \"{}\"", app)
                }
            } else {
                "- open:".to_string()
            }
        }
        "wait" => {
            if let Some(ms) = cmd.ms {
                format!("- wait: {}", ms)
            } else {
                "- wait: 1000".to_string()
            }
        }
        "back" | "swipeUp" | "swipeDown" | "swipeLeft" | "swipeRight" => {
            format!("- {}:", cmd.cmd_type)
        }
        _ => format!("- {}:", cmd.cmd_type),
    }
}

fn delete_command_at(path: &std::path::Path, idx: usize) -> Result<(), String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let lines: Vec<&str> = content.lines().collect();

    // Find command lines (start with "- ")
    let mut cmd_indices: Vec<usize> = vec![];
    for (i, line) in lines.iter().enumerate() {
        if line.trim().starts_with("- ") {
            cmd_indices.push(i);
        }
    }

    if idx >= cmd_indices.len() {
        return Err("Invalid index".to_string());
    }

    let start_line = cmd_indices[idx];
    let end_line = if idx + 1 < cmd_indices.len() {
        cmd_indices[idx + 1]
    } else {
        lines.len()
    };

    // Remove lines [start_line, end_line)
    let new_lines: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| *i < start_line || *i >= end_line)
        .map(|(_, l)| *l)
        .collect();

    std::fs::write(path, new_lines.join("\n") + "\n").map_err(|e| e.to_string())
}

fn insert_command_at(path: &std::path::Path, idx: usize, yaml: &str) -> Result<(), String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let lines: Vec<&str> = content.lines().collect();

    // Find command lines
    let mut cmd_indices: Vec<usize> = vec![];
    for (i, line) in lines.iter().enumerate() {
        if line.trim().starts_with("- ") {
            cmd_indices.push(i);
        }
    }

    let insert_at = if idx < cmd_indices.len() {
        cmd_indices[idx]
    } else {
        lines.len()
    };

    let mut new_lines: Vec<String> = lines.iter().map(|l| l.to_string()).collect();

    // Insert yaml lines
    let yaml_lines: Vec<&str> = yaml.lines().collect();
    for (i, yl) in yaml_lines.iter().enumerate() {
        new_lines.insert(insert_at + i, yl.to_string());
    }

    std::fs::write(path, new_lines.join("\n") + "\n").map_err(|e| e.to_string())
}

/// POST /api/append-command - Append command to YAML file
async fn append_command(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AppendCommandRequest>,
) -> impl IntoResponse {
    let file = state.yaml_file.lock().unwrap();

    if file.is_none() {
        return (StatusCode::BAD_REQUEST, "No file selected").into_response();
    }

    let path = file.as_ref().unwrap();

    // Build YAML command
    let yaml_line = build_yaml_command(&request);

    // Append to file
    use std::io::Write;
    let mut file = match std::fs::OpenOptions::new().append(true).open(path) {
        Ok(f) => f,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    if let Err(e) = writeln!(file, "{}", yaml_line) {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    (StatusCode::OK, "Command appended").into_response()
}

/// POST /api/file - Select or create YAML file
async fn select_file(
    State(state): State<Arc<AppState>>,
    Json(request): Json<FileRequest>,
) -> impl IntoResponse {
    let path = std::path::PathBuf::from(&request.path);

    if !path.exists() && request.create_if_missing {
        // Create new file with header
        let header = format!(
            r#"name: "{}"
platform: android
# Auto-generated by lumi-tester inspect
---
"#,
            path.file_stem().unwrap_or_default().to_string_lossy()
        );

        if let Err(e) = std::fs::write(&path, header) {
            return Json(FileResponse {
                success: false,
                commands: vec![],
                message: Some(e.to_string()),
            });
        }
    }

    if !path.exists() {
        return Json(FileResponse {
            success: false,
            commands: vec![],
            message: Some("File does not exist".to_string()),
        });
    }

    // Update state
    {
        let mut file = state.yaml_file.lock().unwrap();
        *file = Some(path.clone());
    }

    // Parse existing commands
    let commands = parse_yaml_commands(&path);

    Json(FileResponse {
        success: true,
        commands,
        message: None,
    })
}

/// GET /api/file/commands - Get commands from current file
async fn get_commands(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let file = state.yaml_file.lock().unwrap();

    match file.as_ref() {
        Some(path) => {
            let commands = parse_yaml_commands(path);
            Json(FileResponse {
                success: true,
                commands,
                message: None,
            })
        }
        None => Json(FileResponse {
            success: false,
            commands: vec![],
            message: Some("No file selected".to_string()),
        }),
    }
}

/// POST /api/play-command/:index - Play a specific command
async fn play_command(
    State(state): State<Arc<AppState>>,
    Path(index): Path<usize>,
) -> (StatusCode, String) {
    use crate::driver::android::adb;

    // Get path - clone immediately and drop lock
    let path = {
        let file = state.yaml_file.lock().unwrap();
        match file.as_ref() {
            Some(p) => p.clone(),
            None => return (StatusCode::BAD_REQUEST, "No file selected".to_string()),
        }
    };

    let commands = parse_yaml_commands(&path);
    if index >= commands.len() {
        return (StatusCode::BAD_REQUEST, "Invalid command index".to_string());
    }

    let cmd = commands[index].clone();
    let serial = state.device_serial.as_deref();

    // Simple execution based on command type
    let result = if cmd.contains("tap:") {
        // For tap, we should find element but for now just acknowledge
        if let Some(value) = extract_selector_value(&cmd) {
            // Would need to find element coordinates - simplified for now
            adb::shell(serial, "input tap 500 500").await
        } else {
            Ok("No selector".to_string())
        }
    } else if cmd.contains("see:") {
        Ok("Assertion (see) - visual check only".to_string())
    } else if cmd.contains("open:") {
        if let Some(app_id) = extract_selector_value(&cmd) {
            adb::shell(serial, &format!("monkey -p {} 1", app_id)).await
        } else {
            Ok("No app ID".to_string())
        }
    } else if cmd.contains("longPress:") {
        adb::shell(serial, "input swipe 500 500 500 500 1000").await
    } else if cmd.contains("inputText:") {
        if let Some(text) = extract_text_value(&cmd) {
            let escaped = text.replace(" ", "%s");
            adb::shell(serial, &format!("input text '{}'", escaped)).await
        } else {
            Ok("No text".to_string())
        }
    } else {
        Ok(format!("Unknown: {}", cmd))
    };

    match result {
        Ok(output) => (StatusCode::OK, format!("✓ {}", cmd)),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// Extract selector value from command string like '- tap: "Hello"'
fn extract_selector_value(cmd: &str) -> Option<String> {
    // Match quoted value after ": "
    if let Some(start) = cmd.find('"') {
        if let Some(end) = cmd[start + 1..].find('"') {
            return Some(cmd[start + 1..start + 1 + end].to_string());
        }
    }
    None
}

/// Extract text value from inputText command
fn extract_text_value(cmd: &str) -> Option<String> {
    // Look for "text: " pattern
    if let Some(idx) = cmd.find("text:") {
        let rest = &cmd[idx + 5..].trim();
        if let Some(start) = rest.find('"') {
            if let Some(end) = rest[start + 1..].find('"') {
                return Some(rest[start + 1..start + 1 + end].to_string());
            }
        }
    }
    extract_selector_value(cmd)
}

/// POST /api/execute - Execute action on device
async fn execute_action(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ExecuteRequest>,
) -> impl IntoResponse {
    use crate::driver::android::adb;

    let serial = state.device_serial.as_deref();

    let result = match request.action.as_str() {
        "tap" => adb::shell(serial, &format!("input tap {} {}", request.x, request.y)).await,
        "longPress" => {
            adb::shell(
                serial,
                &format!(
                    "input swipe {} {} {} {} 1000",
                    request.x, request.y, request.x, request.y
                ),
            )
            .await
        }
        "doubleTap" => {
            // Double tap is just two taps
            let _ = adb::shell(serial, &format!("input tap {} {}", request.x, request.y)).await;
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            adb::shell(serial, &format!("input tap {} {}", request.x, request.y)).await
        }
        "inputText" => {
            if let Some(text) = &request.text {
                // First tap to focus
                let _ = adb::shell(serial, &format!("input tap {} {}", request.x, request.y)).await;
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                // Then type
                let escaped = text.replace(" ", "%s").replace("'", "\\'");
                adb::shell(serial, &format!("input text '{}'", escaped)).await
            } else {
                Ok("No text provided".to_string())
            }
        }
        "swipeUp" => adb::shell(serial, "input swipe 500 1500 500 500 300").await,
        "swipeDown" => adb::shell(serial, "input swipe 500 500 500 1500 300").await,
        "swipeLeft" => adb::shell(serial, "input swipe 900 1000 200 1000 300").await,
        "swipeRight" => adb::shell(serial, "input swipe 200 1000 900 1000 300").await,
        "back" => adb::shell(serial, "input keyevent 4").await,
        "hideKeyboard" => adb::shell(serial, "input keyevent 111").await,
        "see" | "notSee" | "wait" => {
            // No action needed
            Ok("Meta action".to_string())
        }
        _ => Ok(format!("Unknown action: {}", request.action)),
    };

    match result {
        Ok(_) => (StatusCode::OK, "Action executed").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// GET /api/packages - List installed packages
async fn get_packages(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    use crate::driver::android::adb;
    let serial = state.device_serial.as_deref();

    // List 3rd party packages
    match adb::shell(serial, "pm list packages -3").await {
        Ok(output) => {
            let packages: Vec<String> = output
                .lines()
                .filter(|l| l.starts_with("package:"))
                .map(|l| l.replace("package:", "").trim().to_string())
                .collect();
            Json(serde_json::json!({ "packages": packages })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// Helper functions

fn find_element_at(
    elements: &[uiautomator::UiElement],
    x: i32,
    y: i32,
) -> Option<uiautomator::UiElement> {
    let mut best: Option<&uiautomator::UiElement> = None;
    let mut best_area = i64::MAX;

    for el in elements {
        let bounds = &el.bounds;
        if x >= bounds.left && x <= bounds.right && y >= bounds.top && y <= bounds.bottom {
            let area = (bounds.right - bounds.left) as i64 * (bounds.bottom - bounds.top) as i64;
            if area < best_area {
                best_area = area;
                best = Some(el);
            }
        }
    }

    best.cloned()
}

fn build_yaml_command(request: &AppendCommandRequest) -> String {
    match request.command_type.as_str() {
        "tap" => {
            if let Some(ref sel) = request.selector {
                match sel.selector_type.as_str() {
                    "id" => format!("- tap:\n    id: \"{}\"", sel.value),
                    "text" => format!("- tap: \"{}\"", sel.value),
                    "contentDesc" => format!("- tap:\n    contentDesc: \"{}\"", sel.value),
                    "point" => format!("- tap:\n    point: \"{}\"", sel.value),
                    _ => format!("- tap: \"{}\"", sel.value),
                }
            } else {
                "- tap: \"unknown\"".to_string()
            }
        }
        "longPress" => {
            if let Some(ref sel) = request.selector {
                match sel.selector_type.as_str() {
                    "id" => format!("- longPress:\n    id: \"{}\"", sel.value),
                    "text" => format!("- longPress: \"{}\"", sel.value),
                    "contentDesc" => format!("- longPress:\n    contentDesc: \"{}\"", sel.value),
                    _ => format!("- longPress: \"{}\"", sel.value),
                }
            } else {
                "- longPress: \"unknown\"".to_string()
            }
        }
        "see" => {
            if let Some(ref sel) = request.selector {
                format!("- see: \"{}\"", sel.value)
            } else {
                "- see: \"unknown\"".to_string()
            }
        }
        "inputText" => {
            let text = request.text.as_deref().unwrap_or("");
            if let Some(ref sel) = request.selector {
                format!(
                    "- inputText:\n    {}: \"{}\"\n    text: \"{}\"",
                    sel.selector_type, sel.value, text
                )
            } else {
                format!("- inputText: \"{}\"", text)
            }
        }
        "open" => {
            let app = request.text.as_deref().unwrap_or("com.example.app");
            format!("- open: \"{}\"", app)
        }
        "open_clear" => {
            let app = request.text.as_deref().unwrap_or("com.example.app");
            format!("- open:\n    app: \"{}\"\n    clearState: true", app)
        }
        _ => format!("- {}: \"unknown\"", request.command_type),
    }
}

fn parse_yaml_commands(path: &std::path::Path) -> Vec<String> {
    println!("Parsing YAML from: {}", path.display());
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            println!("Error reading file: {}", e);
            return vec![];
        }
    };

    let mut commands = Vec::new();
    let mut current_cmd = String::new();
    let mut in_command = false;

    for line in content.lines() {
        // println!("Line: {}", line); // Too verbose
        if line.trim().starts_with("- ") {
            // New command starts
            if in_command && !current_cmd.is_empty() {
                // Save previous command
                commands.push(current_cmd.clone());
            }
            current_cmd = line.to_string();
            in_command = true;
        } else if in_command
            && (line.starts_with("    ") || line.starts_with("\t") || line.starts_with("  "))
        {
            // Continuation of current command (indented)
            // Keep multiline format
            current_cmd.push('\n');
            current_cmd.push_str(line);
        } else if line.trim().starts_with("---")
            || line.trim().starts_with("name:")
            || line.trim().starts_with("platform:")
            || line.trim().starts_with("#")
        {
            // Skip metadata
            continue;
        } else if line.trim().is_empty() {
            // Empty line, save current command if any
            if in_command && !current_cmd.is_empty() {
                commands.push(current_cmd.clone());
                current_cmd.clear();
                in_command = false;
            }
        }
    }

    // Don't forget last command
    if in_command && !current_cmd.is_empty() {
        commands.push(current_cmd.clone());
    }

    println!("Found {} commands", commands.len());
    commands
}
