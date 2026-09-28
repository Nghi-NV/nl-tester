//! REST API endpoints for Inspector
//!
//! Provides endpoints for screenshot, hierarchy, element info, and file management.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::screen_capture::{self, ScreenCapture};
use crate::driver::android::uiautomator;
use crate::recorder::selector_scorer::SelectorScorer;
use crate::recorder::yaml_generator::YamlGenerator;
use crate::recorder::SelectorCandidate;

/// Shared state for API handlers
pub struct AppState {
    pub screen_capture: ScreenCapture,
    pub yaml_file: std::sync::Mutex<Option<std::path::PathBuf>>,
    pub device_serial: Option<String>,
    pub current_target_app: std::sync::Mutex<Option<String>>,
    /// Cached UI hierarchy (dumped during screenshot capture)
    pub cached_hierarchy: std::sync::Mutex<Option<CachedHierarchy>>,
    pub workspace_root: Option<std::path::PathBuf>,
}

/// Resolves which app to dump the hierarchy of. A manual pick (via `/api/target-app`)
/// always wins when present - needed for macOS/Windows, where multiple windows can
/// be open at once and "the app on screen" is inherently ambiguous, so those
/// platforms must always be told explicitly. Android/iOS only ever show one app at a
/// time, so instead of falling back to a manual-selection-required search box
/// there (which silently dumped the home screen/status bar until the user picked
/// something), live-detect whatever's actually in the foreground on every call.
/// Deliberately NOT cached into `current_target_app` - re-detecting each call is
/// what keeps this in sync as the user navigates between screens/apps on the device
/// without ever touching the Inspector's search box.
async fn resolve_bundle_id(platform: &str, state: &AppState) -> Option<String> {
    // `current_target_app` is seeded with `device_serial` at server startup (see
    // `server.rs::start`), not `None` - so "has a manual value" can't just mean
    // "is Some": on a fresh Inspector session it's always Some(the device
    // serial/UDID), which is not a real app selection and must be treated the
    // same as unset (same reasoning `get_hierarchy_ios` already applies one layer
    // down for the same seeded value).
    let manual = state.current_target_app.lock().unwrap().clone();
    if let Some(id) = manual.filter(|s| !s.trim().is_empty() && Some(s.as_str()) != state.device_serial.as_deref()) {
        return Some(id);
    }
    match platform {
        "android" => crate::driver::android::adb::get_foreground_package(state.device_serial.as_deref())
            .await
            .ok()
            .flatten(),
        "ios" => {
            let udid = state.device_serial.as_deref()?;
            crate::driver::ios::devicectl::get_foreground_app(udid).await.ok().flatten()
        }
        _ => None,
    }
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

#[derive(Serialize, Clone, Debug)]
pub struct BreadcrumbNode {
    pub class_name: String,
    pub short_class: String,
    pub resource_id: Option<String>,
    pub text: Option<String>,
    pub bounds: BoundsInfo,
    pub is_target: bool,
}

/// Response for element at coordinates
#[derive(Serialize)]
pub struct ElementResponse {
    pub found: bool,
    pub selectors: Vec<SelectorInfo>,
    pub element_class: Option<String>,
    pub element_text: Option<String>,
    pub bounds: Option<BoundsInfo>,
    pub app_id: Option<String>,
    pub supported_commands: Vec<String>,
    #[serde(default)]
    pub hierarchy: Vec<BreadcrumbNode>,
    #[serde(default)]
    pub attributes: std::collections::HashMap<String, String>,
}

#[derive(Serialize)]
pub struct SelectorInfo {
    pub selector_type: String,
    pub value: String,
    pub score: u32,
    pub is_stable: bool,
    pub yaml: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
}

#[derive(Serialize, Clone, Debug)]
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

#[derive(Deserialize)]
pub struct TargetAppRequest {
    pub app_id: String,
}

/// Build API router
pub fn api_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/screenshot", get(get_screenshot))
        .route("/api/element-at", get(get_element_at))
        .route("/api/suggest-selectors", get(get_suggest_selectors))
        .route("/api/hierarchy", get(get_hierarchy))
        .route("/api/packages", get(get_packages))
        .route("/api/app-icon", get(get_app_icon))
        .route("/api/target-app", post(set_target_app))
        .route("/api/command", post(manage_command))
        .route("/api/append-command", post(append_command))
        .route("/api/file", post(select_file))
        .route("/api/file/commands", get(get_commands))
        .route("/api/play-command/:index", post(play_command))
        .route("/api/execute", post(execute_action))
}

#[derive(Deserialize)]
pub struct AppIconQuery {
    pub path: String,
}

async fn get_app_icon(Query(query): Query<AppIconQuery>) -> impl IntoResponse {
    #[cfg(target_os = "macos")]
    {
        if let Some(png_bytes) = crate::driver::macos::MacosBridge::get_app_icon_png(&query.path) {
            return (
                StatusCode::OK,
                [
                    ("Content-Type", "image/png"),
                    ("Cache-Control", "public, max-age=86400"),
                ],
                png_bytes,
            )
                .into_response();
        }
    }
    (StatusCode::NOT_FOUND, "Icon not found").into_response()
}

async fn set_target_app(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TargetAppRequest>,
) -> impl IntoResponse {
    {
        let mut target = state.current_target_app.lock().unwrap();
        *target = if req.app_id.is_empty() {
            None
        } else {
            Some(req.app_id.clone())
        };
    }
    // Invalidate hierarchy cache
    {
        let mut cache = state.cached_hierarchy.lock().unwrap();
        *cache = None;
    }
    (StatusCode::OK, "Target app updated").into_response()
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

    let platform = state.screen_capture.platform().to_string();
    let selected_bundle_id = resolve_bundle_id(&platform, &state).await;
    let target_app = selected_bundle_id.clone().or_else(|| state.device_serial.clone());
    let target_app_clone = target_app.clone();
    let target = match platform.as_str() {
        "macos" => target_app.clone(),
        _ => state.device_serial.clone(),
    };

    // If skipping hierarchy, use a dummy future that returns "skipped" immediately
    let hierarchy_future = async move {
        if skip_hierarchy {
            Err("Skipped".to_string())
        } else {
            screen_capture::get_hierarchy_for_platform(&platform, target.as_deref(), selected_bundle_id.as_deref())
                .await
                .map_err(|e| e.to_string())
        }
    };

    let screenshot_future =
        state.screen_capture.capture_base64_with_target(target_app_clone.as_deref());

    // Capture in parallel (if not skipped)
    let (screenshot_result, hierarchy_result) = tokio::join!(screenshot_future, hierarchy_future);

    // Update cache if we got new hierarchy
    if let Ok(hierarchy_data) = hierarchy_result {
        let (dim_w, dim_h) = state.screen_capture.dimensions();
        let elements = screen_capture::parse_hierarchy_for_platform(
            state.screen_capture.platform(),
            &hierarchy_data,
            target_app.as_deref().unwrap_or(""),
            dim_w,
            dim_h,
        );

        if !elements.is_empty() {
            let mut cache = state.cached_hierarchy.lock().unwrap();
            *cache = Some(CachedHierarchy { elements });
        }
    }

    match screenshot_result {
        Ok((data, width, height)) => {
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

/// Formats a `SelectorCandidate`'s display value the same way regardless of
/// which endpoint produced it (element-at vs suggest-selectors), so both
/// stay visually consistent.
fn format_selector_value(c: &SelectorCandidate) -> String {
    match c.selector_type.as_str() {
        "relative" => {
            if let (Some(anchor), Some(dir)) = (&c.relative_anchor, &c.relative_direction) {
                let type_prefix = if !c.value.is_empty() && c.value != "unknown" {
                    format!("type: {}, ", c.value)
                } else {
                    String::new()
                };
                let index_suffix = if let Some(idx) = c.index {
                    if idx > 0 { format!(" (index {})", idx) } else { String::new() }
                } else {
                    String::new()
                };
                format!("{}{}: \"{}\"{}", type_prefix, dir, anchor.value, index_suffix)
            } else {
                c.value.clone()
            }
        }
        "type" => {
            if let Some(idx) = c.index {
                if idx > 0 {
                    format!("{} (index {})", c.value, idx)
                } else {
                    c.value.clone()
                }
            } else {
                c.value.clone()
            }
        }
        _ => {
            if let Some(idx) = c.index {
                if idx > 0 {
                    format!("{} (index {})", c.value, idx)
                } else {
                    c.value.clone()
                }
            } else {
                c.value.clone()
            }
        }
    }
}

/// Converts scored candidates into API-facing `SelectorInfo` rows, dropping
/// the scorer's element-center-based point candidates (callers add their own
/// click-position-based point selectors instead).
fn build_selector_infos(candidates: &[SelectorCandidate], generator: &YamlGenerator) -> Vec<SelectorInfo> {
    candidates
        .iter()
        .filter(|c| c.selector_type != "point")
        .map(|c| SelectorInfo {
            selector_type: c.selector_type.clone(),
            value: format_selector_value(c),
            score: c.score,
            is_stable: c.is_stable,
            yaml: generator.generate_candidate_yaml(c, "tap"),
            description: c.reason.clone(),
            index: c.index,
        })
        .collect()
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

    let platform = state.screen_capture.platform();
    let selected_bundle_id = resolve_bundle_id(platform, &state).await;
    let target_app = selected_bundle_id.clone().or_else(|| state.device_serial.clone());

    let elements = match cached_elements {
        Some(e) => e,
        None => {
            // No cache, need to dump (first time)
            let target = match platform {
                "macos" => target_app.clone(),
                _ => state.device_serial.clone(),
            };
            let raw_hierarchy = match screen_capture::get_hierarchy_for_platform(
                platform,
                target.as_deref(),
                selected_bundle_id.as_deref(),
            )
            .await
            {
                    Ok(h) => h,
                    Err(_) => {
                        return Json(ElementResponse {
                            found: false,
                            selectors: vec![],
                            element_class: None,
                            element_text: None,
                            bounds: None,
                            app_id: None,
                            supported_commands: vec![],
                            hierarchy: vec![],
                            attributes: std::collections::HashMap::new(),
                        });
                    }
                };

            let (dim_w, dim_h) = state.screen_capture.dimensions();
            let parsed = screen_capture::parse_hierarchy_for_platform(
                platform,
                &raw_hierarchy,
                target_app.as_deref().unwrap_or(""),
                dim_w,
                dim_h,
            );

            if parsed.is_empty() {
                return Json(ElementResponse {
                    found: false,
                    selectors: vec![],
                    element_class: None,
                    element_text: None,
                    bounds: None,
                    app_id: None,
                    supported_commands: vec![],
                    hierarchy: vec![],
                    attributes: std::collections::HashMap::new(),
                });
            }
            parsed
        }
    };

    // Find element at coordinates
    let (width, height) = state.screen_capture.dimensions();
    let element = find_element_at(&elements, params.x, params.y);

    match element {
        Some(el) => {
            let scorer = SelectorScorer::new(width, height, elements.clone());
            let candidates = scorer.score_element(&el);
            let generator = YamlGenerator::new();
            let mut selectors: Vec<SelectorInfo> = build_selector_infos(&candidates, &generator);

            // Always generate align and offset variants for semantic candidates
            let el_w = (el.bounds.right - el.bounds.left).max(1) as f64;
            let el_h = (el.bounds.bottom - el.bounds.top).max(1) as f64;
            let rel_x = (params.x - el.bounds.left) as f64 / el_w;
            let rel_y = (params.y - el.bounds.top) as f64 / el_h;

            if rel_x >= 0.0 && rel_x <= 1.0 && rel_y >= 0.0 && rel_y <= 1.0 {
                let align_preset = if rel_x >= 0.70 {
                    Some("right")
                } else if rel_x <= 0.30 {
                    Some("left")
                } else if rel_y >= 0.70 {
                    Some("bottom")
                } else if rel_y <= 0.30 {
                    Some("top")
                } else {
                    Some("center")
                };

                let offset_x_pct = (rel_x * 100.0).round() as i32;
                let offset_y_pct = (rel_y * 100.0).round() as i32;

                // For the top semantic candidates (e.g. text, id, type, relative), generate align & offset variants
                let semantic_candidates: Vec<_> = candidates
                    .iter()
                    .filter(|c| c.selector_type != "point")
                    .take(3)
                    .cloned()
                    .collect();

                for c in &semantic_candidates {
                    let base_yaml = generator.generate_candidate_yaml(c, "tap");
                    let base_val = format_selector_value(c);
                    if let Some(align_name) = align_preset {
                        let align_yaml = format!("{}\n    align: {}", base_yaml, align_name);
                        selectors.push(SelectorInfo {
                            selector_type: format!("{}+align", c.selector_type),
                            value: format!("{} (align: {})", base_val, align_name),
                            score: c.score.saturating_add(5),
                            is_stable: c.is_stable,
                            yaml: align_yaml,
                            description: format!("Target {} of {}", align_name, c.reason),
                            index: c.index,
                        });
                    }

                    let offset_val = format!("{}%,{}%", offset_x_pct, offset_y_pct);
                    let offset_yaml = format!("{}\n    offset: \"{}\"", base_yaml, offset_val);
                    selectors.push(SelectorInfo {
                        selector_type: format!("{}+offset", c.selector_type),
                        value: format!("{} (offset: {})", base_val, offset_val),
                        score: c.score,
                        is_stable: c.is_stable,
                        yaml: offset_yaml,
                        description: format!("Relative offset within element ({})", offset_val),
                        index: c.index,
                    });
                }
            }

            // Add click-position POINT selectors (actual click coords, not element center)
            let click_x_pct = (params.x as f64 / width as f64 * 100.0).round() as u32;
            let click_y_pct = (params.y as f64 / height as f64 * 100.0).round() as u32;
            selectors.push(SelectorInfo {
                selector_type: "point".to_string(),
                value: format!("{}%,{}%", click_x_pct, click_y_pct),
                score: 20,
                is_stable: false,
                yaml: format!("- tap:\n    point: \"{}%,{}%\"", click_x_pct, click_y_pct),
                description: "Click position (percentage)".to_string(),
                index: None,
            });
            selectors.push(SelectorInfo {
                selector_type: "point".to_string(),
                value: format!("{},{}", params.x, params.y),
                score: 15,
                is_stable: false,
                yaml: format!("- tap:\n    point: \"{},{}\"", params.x, params.y),
                description: "Click position (absolute pixels)".to_string(),
                index: None,
            });

            let enclosing = find_enclosing_elements_at(&elements, params.x, params.y);
            let hierarchy: Vec<BreadcrumbNode> = enclosing
                .into_iter()
                .map(|node| {
                    let short_class = node.class.split('.').last().unwrap_or(&node.class).to_string();
                    let is_target = node.bounds.left == el.bounds.left
                        && node.bounds.top == el.bounds.top
                        && node.bounds.right == el.bounds.right
                        && node.bounds.bottom == el.bounds.bottom
                        && node.class == el.class;

                    BreadcrumbNode {
                        class_name: node.class.clone(),
                        short_class,
                        resource_id: if node.resource_id.is_empty() { None } else { Some(node.resource_id.clone()) },
                        text: if node.text.is_empty() { None } else { Some(node.text.clone()) },
                        bounds: BoundsInfo {
                            left: node.bounds.left,
                            top: node.bounds.top,
                            right: node.bounds.right,
                            bottom: node.bounds.bottom,
                        },
                        is_target,
                    }
                })
                .collect();

            let mut attributes = std::collections::HashMap::new();
            attributes.insert("class".to_string(), el.class.clone());
            if !el.resource_id.is_empty() {
                attributes.insert("resource-id".to_string(), el.resource_id.clone());
            }
            if !el.text.is_empty() {
                attributes.insert("text".to_string(), el.text.clone());
            }
            if !el.content_desc.is_empty() {
                attributes.insert("content-desc".to_string(), el.content_desc.clone());
            }
            if !el.package.is_empty() {
                attributes.insert("package".to_string(), el.package.clone());
            }
            attributes.insert("bounds".to_string(), format!("[{},{}][{},{}]", el.bounds.left, el.bounds.top, el.bounds.right, el.bounds.bottom));
            attributes.insert("dimensions".to_string(), format!("{} × {} px", (el.bounds.right - el.bounds.left).max(0), (el.bounds.bottom - el.bounds.top).max(0)));
            attributes.insert("clickable".to_string(), el.clickable.to_string());
            attributes.insert("enabled".to_string(), el.enabled.to_string());
            attributes.insert("focusable".to_string(), el.focusable.to_string());
            attributes.insert("focused".to_string(), el.focused.to_string());
            attributes.insert("scrollable".to_string(), el.scrollable.to_string());
            attributes.insert("password".to_string(), el.password.to_string());
            if !el.hint.is_empty() {
                attributes.insert("hint".to_string(), el.hint.clone());
            }
            if !el.index.is_empty() {
                attributes.insert("index".to_string(), el.index.clone());
            }

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
                app_id: if el.package.is_empty() {
                    None
                } else {
                    Some(el.package.clone())
                },
                supported_commands: {
                    let mut cmds = vec!["see".to_string(), "wait".to_string(), "waitUntilVisible".to_string()];
                    let is_input = el.class == "Input" || el.class == "TextField" || el.class.contains("Edit");
                    if el.clickable || el.focusable || is_input || el.class == "Button" {
                        cmds.push("tap".to_string());
                        cmds.push("doubleTap".to_string());
                        cmds.push("longPress".to_string());
                        cmds.push("rightClick".to_string());
                    }
                    if is_input || el.focusable || el.class.contains("Field") {
                        cmds.push("inputText".to_string());
                        cmds.push("clearText".to_string());
                    }
                    if el.scrollable {
                        cmds.push("scrollTo".to_string());
                        cmds.push("swipe".to_string());
                    }
                    cmds
                },
                hierarchy,
                attributes,
            })
        }
        None => {
            // No element found, but still provide click-position point selectors
            let click_x_pct = (params.x as f64 / width as f64 * 100.0).round() as u32;
            let click_y_pct = (params.y as f64 / height as f64 * 100.0).round() as u32;
            let selectors = vec![
                SelectorInfo {
                    selector_type: "point".to_string(),
                    value: format!("{}%,{}%", click_x_pct, click_y_pct),
                    score: 20,
                    is_stable: false,
                    yaml: format!("- tap:\n    point: \"{}%,{}%\"", click_x_pct, click_y_pct),
                    description: "Click position (percentage)".to_string(),
                    index: None,
                },
                SelectorInfo {
                    selector_type: "point".to_string(),
                    value: format!("{},{}", params.x, params.y),
                    score: 15,
                    is_stable: false,
                    yaml: format!("- tap:\n    point: \"{},{}\"", params.x, params.y),
                    description: "Click position (absolute pixels)".to_string(),
                    index: None,
                },
            ];
            Json(ElementResponse {
                found: false,
                selectors,
                element_class: None,
                element_text: None,
                bounds: None,
                app_id: None,
                supported_commands: vec![],
                hierarchy: vec![],
                attributes: std::collections::HashMap::new(),
            })
        }
    }
}

/// Query params for suggest-selectors endpoint
#[derive(Deserialize)]
pub struct SuggestSelectorsQuery {
    /// Case-insensitive substring match against text/resource-id/content-desc/class
    pub query: Option<String>,
    /// "x,y" - when set, results are ranked by proximity to this point and
    /// elements containing it are preferred over textual/clickable ranking
    pub point: Option<String>,
    pub limit: Option<usize>,
    #[serde(default)]
    pub include_non_clickable: bool,
}

#[derive(Serialize)]
pub struct SuggestionInfo {
    pub text: Option<String>,
    pub resource_id: Option<String>,
    pub content_desc: Option<String>,
    pub class: String,
    pub clickable: bool,
    pub bounds: BoundsInfo,
    pub distance_to_point: Option<f64>,
    pub best_selector: Option<SelectorInfo>,
    pub selectors: Vec<SelectorInfo>,
}

#[derive(Serialize)]
pub struct SuggestSelectorsResponse {
    pub count: usize,
    pub suggestions: Vec<SuggestionInfo>,
}

/// Filters elements by a case-insensitive substring match against
/// text/resource-id/content-desc/class - the query-based counterpart to
/// `find_element_at`'s point-based lookup. Mirrors the old JS tool's
/// `elementMatchesQuery`, but runs on the already cross-platform-normalized
/// `UiElement` list instead of hand-parsed Android XML.
fn elements_matching_query<'a>(
    elements: &'a [uiautomator::UiElement],
    query: Option<&str>,
) -> Vec<&'a uiautomator::UiElement> {
    let needle = query.map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty());
    let Some(needle) = needle else {
        return elements.iter().collect();
    };
    elements
        .iter()
        .filter(|el| {
            el.text.to_lowercase().contains(&needle)
                || el.resource_id.to_lowercase().contains(&needle)
                || el.content_desc.to_lowercase().contains(&needle)
                || el.class.to_lowercase().contains(&needle)
        })
        .collect()
}

/// GET /api/suggest-selectors?query=<text>&point=<x,y>&limit=<n> - Scored selector
/// suggestions for elements matching a query and/or near a point, using the same
/// `SelectorScorer` (with real cross-element duplicate/index detection) that
/// `/api/element-at` already uses - so results are correct on every platform this
/// Inspector supports, not just Android.
async fn get_suggest_selectors(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SuggestSelectorsQuery>,
) -> Json<SuggestSelectorsResponse> {
    let cached_elements = {
        let cache = state.cached_hierarchy.lock().unwrap();
        cache.as_ref().map(|c| c.elements.clone())
    };

    let platform = state.screen_capture.platform();
    let selected_bundle_id = resolve_bundle_id(platform, &state).await;
    let target_app = selected_bundle_id.clone().or_else(|| state.device_serial.clone());

    let elements = match cached_elements {
        Some(e) => e,
        None => {
            let target = match platform {
                "macos" => target_app.clone(),
                _ => state.device_serial.clone(),
            };
            let raw_hierarchy = match screen_capture::get_hierarchy_for_platform(
                platform,
                target.as_deref(),
                selected_bundle_id.as_deref(),
            )
            .await
            {
                Ok(h) => h,
                Err(_) => {
                    return Json(SuggestSelectorsResponse { count: 0, suggestions: vec![] });
                }
            };

            let (dim_w, dim_h) = state.screen_capture.dimensions();
            screen_capture::parse_hierarchy_for_platform(
                platform,
                &raw_hierarchy,
                target_app.as_deref().unwrap_or(""),
                dim_w,
                dim_h,
            )
        }
    };

    let (width, height) = state.screen_capture.dimensions();
    let parsed_point: Option<(f64, f64)> = params.point.as_deref().and_then(parse_point_arg);

    Json(suggest_selectors_from_elements(
        &elements,
        width,
        height,
        params.query.as_deref(),
        parsed_point,
        params.limit.unwrap_or(10),
        params.include_non_clickable,
    ))
}

/// Parses a "x,y" point string into `(x, y)` - shared by the HTTP query param
/// and the CLI `--point` flag.
pub fn parse_point_arg(p: &str) -> Option<(f64, f64)> {
    let (x, y) = p.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

/// Core suggest-selectors ranking, independent of how the elements were
/// obtained (live Inspector hierarchy vs. a saved dump loaded by the CLI).
/// Reuses `SelectorScorer` - the same engine already proven correct and
/// cross-platform by `/api/element-at` - so results are consistent whether
/// called from the Inspector, the MCP tool, or the CLI directly.
pub fn suggest_selectors_from_elements(
    elements: &[uiautomator::UiElement],
    width: u32,
    height: u32,
    query: Option<&str>,
    point: Option<(f64, f64)>,
    limit: usize,
    include_non_clickable: bool,
) -> SuggestSelectorsResponse {
    let generator = YamlGenerator::new();
    let scorer = SelectorScorer::new(width, height, elements.to_vec());

    let mut matched: Vec<(&uiautomator::UiElement, Option<f64>)> = elements_matching_query(elements, query)
        .into_iter()
        .filter(|el| include_non_clickable || el.clickable || !el.text.is_empty() || !el.content_desc.is_empty())
        .filter(|el| el.enabled)
        .map(|el| {
            let distance = point.map(|(px, py)| {
                let cx = (el.bounds.left + el.bounds.right) as f64 / 2.0;
                let cy = (el.bounds.top + el.bounds.bottom) as f64 / 2.0;
                ((cx - px).powi(2) + (cy - py).powi(2)).sqrt()
            });
            (el, distance)
        })
        .collect();

    matched.sort_by(|(el_a, dist_a), (el_b, dist_b)| {
        if let Some((px, py)) = point {
            let contains = |el: &uiautomator::UiElement| {
                px >= el.bounds.left as f64
                    && px <= el.bounds.right as f64
                    && py >= el.bounds.top as f64
                    && py <= el.bounds.bottom as f64
            };
            let a_contains = contains(el_a);
            let b_contains = contains(el_b);
            if a_contains != b_contains {
                return b_contains.cmp(&a_contains);
            }
            return dist_a
                .unwrap_or(f64::INFINITY)
                .partial_cmp(&dist_b.unwrap_or(f64::INFINITY))
                .unwrap_or(std::cmp::Ordering::Equal);
        }
        std::cmp::Ordering::Equal
    });

    let all_suggestions: Vec<SuggestionInfo> = matched
        .into_iter()
        .map(|(el, distance)| {
            let candidates = scorer.score_element(el);
            let selectors = build_selector_infos(&candidates, &generator);
            let best_selector = selectors.first().map(|s| SelectorInfo {
                selector_type: s.selector_type.clone(),
                value: s.value.clone(),
                score: s.score,
                is_stable: s.is_stable,
                yaml: s.yaml.clone(),
                description: s.description.clone(),
                index: s.index,
            });
            SuggestionInfo {
                text: if el.text.is_empty() { None } else { Some(el.text.clone()) },
                resource_id: if el.resource_id.is_empty() { None } else { Some(el.resource_id.clone()) },
                content_desc: if el.content_desc.is_empty() { None } else { Some(el.content_desc.clone()) },
                class: el.class.clone(),
                clickable: el.clickable,
                bounds: BoundsInfo {
                    left: el.bounds.left,
                    top: el.bounds.top,
                    right: el.bounds.right,
                    bottom: el.bounds.bottom,
                },
                distance_to_point: distance.map(|d| (d * 100.0).round() / 100.0),
                best_selector,
                selectors,
            }
        })
        .filter(|s| s.best_selector.is_some())
        .collect();

    let count = all_suggestions.len();
    let suggestions: Vec<SuggestionInfo> = all_suggestions.into_iter().take(limit).collect();

    SuggestSelectorsResponse { count, suggestions }
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
        let platform = state.screen_capture.platform();
        let selected_bundle_id = resolve_bundle_id(platform, &state).await;
        let target_app = selected_bundle_id.clone().or_else(|| state.device_serial.clone());
        let target = match platform {
            "macos" => target_app.clone(),
            _ => state.device_serial.clone(),
        };
        let (dim_w, dim_h) = state.screen_capture.dimensions();
        match screen_capture::get_hierarchy_for_platform(platform, target.as_deref(), selected_bundle_id.as_deref()).await {
            Ok(raw) => screen_capture::parse_hierarchy_for_platform(
                platform,
                &raw,
                target_app.as_deref().unwrap_or(""),
                dim_w,
                dim_h,
            ),
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

    if let Some(root) = state.workspace_root.as_deref() {
        if !path_is_within_workspace(&path, root) {
            return Json(FileResponse {
                success: false,
                commands: vec![],
                message: Some("Inspector files must stay inside the open Lumi workspace".to_string()),
            });
        }
    }

    if !path.exists() && request.create_if_missing {
        let plat = state.screen_capture.platform();
        let app_line = if let Some(ref dev) = state.device_serial {
            format!("appId: {}\n", dev)
        } else {
            String::new()
        };
        let header = format!(
            r#"platform: {}
{}# Auto-generated by lumi-tester inspect
---
"#,
            plat, app_line
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

fn path_is_within_workspace(path: &std::path::Path, root: &std::path::Path) -> bool {
    let Ok(root) = std::fs::canonicalize(root) else {
        return false;
    };
    let candidate = if path.exists() {
        std::fs::canonicalize(path).ok()
    } else {
        path.parent()
            .and_then(|parent| std::fs::canonicalize(parent).ok())
            .and_then(|parent| path.file_name().map(|name| parent.join(name)))
    };
    candidate.is_some_and(|candidate| candidate.starts_with(root))
}

#[cfg(test)]
mod workspace_path_tests {
    use super::path_is_within_workspace;
    use std::{fs, time::SystemTime};

    #[test]
    fn only_accepts_existing_or_new_files_inside_workspace() {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp = std::env::temp_dir().join(format!("lumi-inspector-path-{nonce}"));
        let workspace = temp.join("workspace");
        let outside = temp.join("outside");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let existing = workspace.join("flow.yaml");
        let missing_inside = workspace.join("new-flow.yaml");
        let outside_file = outside.join("flow.yaml");
        fs::write(&existing, "---\n").unwrap();
        fs::write(&outside_file, "---\n").unwrap();

        assert!(path_is_within_workspace(&existing, &workspace));
        assert!(path_is_within_workspace(&missing_inside, &workspace));
        assert!(!path_is_within_workspace(&outside_file, &workspace));
        assert!(!path_is_within_workspace(&outside.join("new-flow.yaml"), &workspace));

        #[cfg(unix)]
        {
            let link = workspace.join("outside.yaml");
            std::os::unix::fs::symlink(&outside_file, &link).unwrap();
            assert!(!path_is_within_workspace(&link, &workspace));
        }

        fs::remove_dir_all(temp).unwrap();
    }
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
        if let Some(_value) = extract_selector_value(&cmd) {
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
        Ok(_output) => (StatusCode::OK, format!("✓ {}", cmd)),
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
    let platform = state.screen_capture.platform();

    if platform == "macos" {
        #[cfg(target_os = "macos")]
        {
            let target_app = {
                let cur = state.current_target_app.lock().unwrap();
                cur.clone().or_else(|| state.device_serial.clone())
            };
            let (offset_x, offset_y) = if let Some(app) = target_app.as_deref().filter(|s| !s.trim().is_empty()) {
                if let Some((_win_id, x, y, _w, _h)) = crate::driver::macos::MacosBridge::get_app_window_info(app) {
                    (x as i32, y as i32)
                } else {
                    let bridge = crate::driver::macos::MacosBridge::new();
                    if let Some(bounds) = bridge.get_window_bounds(app) {
                        if bounds.width > 0.0 && bounds.height > 0.0 {
                            (bounds.x as i32, bounds.y as i32)
                        } else {
                            (0, 0)
                        }
                    } else {
                        (0, 0)
                    }
                }
            } else {
                (0, 0)
            };

            let actual_x = request.x + offset_x;
            let actual_y = request.y + offset_y;

            let result: anyhow::Result<()> = (|| {
                match request.action.as_str() {
                    "tap" => {
                        crate::driver::macos::MacosBridge::click_at(actual_x, actual_y, true)?
                    }
                    "doubleTap" => {
                        crate::driver::macos::MacosBridge::double_click_at(actual_x, actual_y)?
                    }
                    "rightClick" => {
                        crate::driver::macos::MacosBridge::right_click_at(actual_x, actual_y)?
                    }
                    "inputText" => {
                        if let Some(text) = &request.text {
                            crate::driver::macos::MacosBridge::click_at(
                                actual_x, actual_y, true,
                            )?;
                            std::thread::sleep(std::time::Duration::from_millis(150));
                            let target = target_app.as_deref().unwrap_or("");
                            let bridge = crate::driver::macos::MacosBridge::new();
                            if let Some(pid) = bridge.find_app_pid(target).ok().flatten() {
                                bridge.post_key_events(pid, text)?;
                            } else {
                                bridge.post_key(text)?;
                            }
                        }
                    }
                    "hideKeyboard" => {
                        crate::driver::macos::MacosBridge::new().post_key("escape")?
                    }
                    "see" | "notSee" | "wait" => {}
                    _ => {}
                }
                Ok(())
            })();

            return match result {
                Ok(_) => (StatusCode::OK, "Action executed").into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
            };
        }
        #[cfg(not(target_os = "macos"))]
        {
            return (
                StatusCode::BAD_REQUEST,
                "macOS is only supported on macOS hosts",
            )
                .into_response();
        }
    }

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
    let platform = state.screen_capture.platform();

    if platform == "macos" {
        #[cfg(target_os = "macos")]
        {
            match crate::driver::macos::MacosBridge::list_running_apps() {
                Ok(packages) => {
                    return Json(serde_json::json!({ "packages": packages })).into_response();
                }
                Err(e) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            return Json(serde_json::json!({ "packages": Vec::<String>::new() })).into_response();
        }
    }

    if platform == "ios" {
        // The Android fallback below (`adb shell pm list packages`) doesn't apply to
        // iOS at all - `state.device_serial` there is a UDID, not an Android serial,
        // and there previously was no "ios" branch here, so this always fell through
        // to that Android path and failed with "adb: device '<udid>' not found".
        // That meant the app-search dropdown could never list anything for iOS, so
        // there was no way to select a target app - and without a selected app,
        // `/api/hierarchy` falls back to a bundle id that resolves to nothing useful
        // (see `get_hierarchy_ios`'s doc comment). `idb list-apps` works for both a
        // real device and a simulator given `--udid`; filtered to `user` (third-party)
        // apps only, matching the Android path's `-3` filter, so this doesn't get
        // cluttered with hundreds of Apple system apps.
        let udid = state.device_serial.clone().unwrap_or_default();
        return match tokio::process::Command::new("idb")
            .args(["list-apps", "--udid", &udid])
            .output()
            .await
        {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout);
                let packages: Vec<String> = text
                    .lines()
                    .filter_map(|line| {
                        let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
                        let (bundle_id, name, app_type) = (parts.first()?, parts.get(1)?, parts.get(2)?);
                        if *app_type != "user" {
                            return None;
                        }
                        // Reordered to "name | bundleId | path" - the shape
                        // `loadPackages()` in script.js already parses (same
                        // convention the macOS branch above uses).
                        Some(format!("{} | {} | ", name, bundle_id))
                    })
                    .collect();
                Json(serde_json::json!({ "packages": packages })).into_response()
            }
            Ok(out) => (StatusCode::INTERNAL_SERVER_ERROR, String::from_utf8_lossy(&out.stderr).to_string()).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to run idb list-apps: {}", e)).into_response(),
        };
    }

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
    let mut matching: Vec<(&uiautomator::UiElement, i64, i32, f64)> = Vec::new();

    for el in elements {
        let bounds = &el.bounds;
        if x >= bounds.left && x <= bounds.right && y >= bounds.top && y <= bounds.bottom {
            let width = bounds.right - bounds.left;
            let height = bounds.bottom - bounds.top;
            let area = width as i64 * height as i64;
            if area <= 0 || width <= 0 || height <= 0 {
                continue;
            }

            let is_generic_system_id = el.resource_id == "android:id/content"
                || el.resource_id == "android:id/decor_content_parent"
                || el.resource_id == "android:id/navigationBarBackground"
                || el.resource_id == "android:id/statusBarBackground"
                || el.resource_id == "android:id/custom"
                || el.resource_id == "android:id/touch_outside";

            let desc_lower = el.content_desc.trim().to_lowercase();
            let text_lower = el.text.trim().to_lowercase();

            // Detect modal scrims / backdrop dismiss barriers (e.g. Flutter "Dismiss", "Modal barrier", "Scrim")
            let is_backdrop = desc_lower == "dismiss"
                || desc_lower == "scrim"
                || desc_lower == "backdrop"
                || desc_lower == "modal barrier"
                || desc_lower == "modal-barrier"
                || desc_lower == "dialog-overlay"
                || text_lower == "dismiss"
                || text_lower == "scrim"
                || text_lower == "backdrop"
                || is_generic_system_id;

            let is_input = el.class == "Input"
                || el.class == "TextField"
                || el.class.contains("Edit")
                || el.class.to_lowercase().contains("edittext");

            let is_control = is_input
                || el.class == "Button"
                || el.class == "CheckBox"
                || el.class == "RadioButton"
                || el.class == "Switch"
                || el.class == "ComboBox"
                || el.class == "Link"
                || el.class == "Slider"
                || el.class.contains("SeekBar")
                || el.clickable
                || el.scrollable
                || el.focusable;

            let has_semantic_id = !el.resource_id.trim().is_empty() && !is_generic_system_id;
            let has_text = !el.text.trim().is_empty()
                || (!el.content_desc.trim().is_empty() && !is_backdrop)
                || !el.hint.trim().is_empty()
                || has_semantic_id;

            let is_container = is_backdrop
                || el.class == "Group"
                || el.class == "Window"
                || el.class == "WebArea"
                || el.class == "ScrollArea"
                || el.class == "ScrollView"
                || el.class == "android.widget.FrameLayout"
                || el.class.ends_with("Layout")
                || el.class.ends_with("ViewGroup");

            // Calculate distance from touch point (x, y) to element center
            let center_x = (bounds.left + bounds.right) as f64 / 2.0;
            let center_y = (bounds.top + bounds.bottom) as f64 / 2.0;
            let dx = (x as f64) - center_x;
            let dy = (y as f64) - center_y;
            let dist_to_center = (dx * dx + dy * dy).sqrt();

            // Direct interactive control priority
            let priority = if is_backdrop {
                0 // Backdrops have absolute lowest priority
            } else if is_input {
                7 // Text inputs being clicked
            } else if is_control && has_text && !is_container {
                6 // Interactive controls with label / stable ID (e.g. Buttons, Switches, Tabs)
            } else if is_control && !is_container {
                5 // Interactive controls without label (e.g. Sliders, SeekBars, custom clickable Views)
            } else if has_text && !is_container {
                4 // Text labels, icons, titles
            } else if !is_container {
                3 // Leaf view / component
            } else if has_text {
                2 // Container with label
            } else {
                1 // Raw layout container
            };

            matching.push((el, area, priority, dist_to_center));
        }
    }

    if matching.is_empty() {
        return None;
    }

    // Sort strategy:
    // 1. Non-backdrops strictly before backdrops.
    // 2. Interactive / control priority: An interactive control (is_control / is_input)
    //    at the clicked coordinate is prioritized over enclosing layout wrappers.
    // 3. Significant area difference (> 1.4x): The smaller enclosing element is chosen.
    // 4. Comparable area (within 1.4x): Higher priority wins, then closer distance to center.
    matching.sort_by(|a, b| {
        let (_el_a, area_a, prio_a, dist_a) = a;
        let (_el_b, area_b, prio_b, dist_b) = b;

        // If one is backdrop and other is not, non-backdrop strictly wins
        let is_backdrop_a = *prio_a == 0;
        let is_backdrop_b = *prio_b == 0;
        if is_backdrop_a != is_backdrop_b {
            return if is_backdrop_a {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Less
            };
        }

        // Check if one is a direct interactive control (priority >= 5) and the other is a container (priority <= 2)
        let is_active_control_a = *prio_a >= 5;
        let is_active_control_b = *prio_b >= 5;
        if is_active_control_a && !is_active_control_b {
            return std::cmp::Ordering::Less;
        }
        if !is_active_control_a && is_active_control_b {
            return std::cmp::Ordering::Greater;
        }

        // Compare areas: If area difference is significant (> 1.4x), smaller area wins!
        let ratio = (*area_a as f64) / (*area_b as f64);
        if ratio < 0.7 {
            std::cmp::Ordering::Less
        } else if ratio > 1.4 {
            std::cmp::Ordering::Greater
        } else {
            // Comparable area: higher priority wins, tie-breaker closest to center, then smaller area
            prio_b
                .cmp(prio_a)
                .then_with(|| dist_a.partial_cmp(dist_b).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| area_a.cmp(area_b))
        }
    });

    matching.first().map(|(el, _, _, _)| (*el).clone())
}

fn find_enclosing_elements_at<'a>(
    elements: &'a [uiautomator::UiElement],
    x: i32,
    y: i32,
) -> Vec<&'a uiautomator::UiElement> {
    let mut matching: Vec<&'a uiautomator::UiElement> = elements
        .iter()
        .filter(|el| {
            let b = &el.bounds;
            x >= b.left && x <= b.right && y >= b.top && y <= b.bottom && (b.right > b.left) && (b.bottom > b.top)
        })
        .collect();

    // Sort by area descending (outermost root first -> innermost leaf last)
    matching.sort_by_key(|el| {
        let w = (el.bounds.right - el.bounds.left) as i64;
        let h = (el.bounds.bottom - el.bounds.top) as i64;
        -(w * h)
    });

    matching
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

#[cfg(test)]
mod suggest_selectors_tests {
    use super::*;
    use crate::driver::android::uiautomator::{Bounds, UiElement};

    fn el(class: &str, text: &str, resource_id: &str, bounds: (i32, i32, i32, i32)) -> UiElement {
        UiElement {
            class: class.to_string(),
            text: text.to_string(),
            resource_id: resource_id.to_string(),
            content_desc: String::new(),
            bounds: Bounds { left: bounds.0, top: bounds.1, right: bounds.2, bottom: bounds.3 },
            clickable: true,
            enabled: true,
            focusable: true,
            focused: false,
            hint: String::new(),
            scrollable: false,
            password: false,
            index: String::new(),
            package: "com.example".to_string(),
        }
    }

    /// Bug class 1 (old JS `suggestSelectorsFromAndroidXml`): duplicate `text` across
    /// elements was never detected, so both would get the exact same "best" selector
    /// with no index - tapping it would always hit the first match, silently wrong
    /// for the second. `SelectorScorer` must assign a real, distinct index per
    /// occurrence based on actual position among duplicates.
    #[test]
    fn duplicate_text_gets_correct_index() {
        let elements = vec![
            el("android.widget.Button", "Login", "com.example:id/login", (10, 20, 210, 100)),
            el("android.widget.TextView", "Login", "", (10, 120, 210, 180)),
        ];

        let response = suggest_selectors_from_elements(&elements, 1080, 1920, Some("login"), None, 10, false);
        assert_eq!(response.count, 2);

        let first = response.suggestions.iter().find(|s| s.resource_id.is_some()).unwrap();
        let first_best = first.best_selector.as_ref().unwrap();
        assert!(first_best.index.is_none() || first_best.index == Some(0));

        let second = response.suggestions.iter().find(|s| s.resource_id.is_none()).unwrap();
        let second_best = second.best_selector.as_ref().unwrap();
        assert_eq!(second_best.selector_type, "text");
        assert_eq!(
            second_best.index,
            Some(1),
            "second element sharing the same text must be indexed 1, not silently dropped or defaulted to 0"
        );
        assert!(second_best.yaml.contains("index: 1"));
    }

    /// Bug class 2 (old JS): `type` candidates always hardcoded `index: 0` regardless
    /// of the element's real position among same-class siblings - a selector for the
    /// 5th same-type element would still claim index 0 and tap the wrong one.
    #[test]
    fn same_type_siblings_get_real_position_index() {
        let elements = vec![
            el("android.widget.ImageView", "", "", (0, 0, 50, 50)),
            el("android.widget.ImageView", "", "", (60, 0, 110, 50)),
            el("android.widget.ImageView", "", "", (120, 0, 170, 50)),
        ];

        let response = suggest_selectors_from_elements(&elements, 1080, 1920, None, None, 10, true);
        assert_eq!(response.count, 3);

        let mut type_indices: Vec<usize> = response
            .suggestions
            .iter()
            .map(|s| {
                s.selectors
                    .iter()
                    .find(|sel| sel.selector_type == "type")
                    .and_then(|sel| sel.index)
                    .unwrap_or(0)
            })
            .collect();
        type_indices.sort();

        assert_eq!(
            type_indices,
            vec![0, 1, 2],
            "type-selector index must reflect each element's real position among same-type siblings, not always 0"
        );
    }

    /// Bug class 3 (old JS): resource-id candidates were unconditionally scored 100
    /// with no stability filtering - an auto-generated-looking id (UUID/timestamp/
    /// random-number suffix) was ranked as the single best, most trustworthy selector.
    #[test]
    fn auto_generated_looking_resource_id_is_deprioritized() {
        let elements = vec![el(
            "android.widget.FrameLayout",
            "",
            "com.example:id/view12345",
            (0, 0, 100, 100),
        )];

        let response = suggest_selectors_from_elements(&elements, 1080, 1920, None, None, 10, true);
        let suggestion = &response.suggestions[0];
        let id_selector = suggestion
            .selectors
            .iter()
            .find(|s| s.selector_type == "id")
            .expect("id candidate should still be offered");

        assert!(
            !id_selector.is_stable,
            "an id matching an auto-generated pattern must not be marked stable"
        );
        assert!(
            id_selector.score < 100,
            "an auto-generated-looking id must not receive the old unconditional 100 score, got {}",
            id_selector.score
        );
    }
}
