use super::{WorkspaceEntry, WorkspaceState};
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfo {
    pub path: String,
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFileReference {
    pub path: String,
    pub relative_path: String,
    pub name: String,
}

#[tauri::command]
pub fn open_workspace(
    path: String,
    app: AppHandle,
    workspace: State<'_, WorkspaceState>,
) -> Result<WorkspaceInfo, String> {
    let canonical = std::fs::canonicalize(path).map_err(|error| error.to_string())?;
    if !canonical.is_dir() {
        return Err("Choose a folder to open as a workspace".to_string());
    }
    let name = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Workspace")
        .to_string();
    workspace.set_root(canonical.clone())?;
    crate::recent_projects::record(&app, canonical.clone());
    Ok(WorkspaceInfo {
        path: canonical.to_string_lossy().into_owned(),
        name,
    })
}

#[tauri::command]
pub fn resolve_workspace_file_reference(
    source_path: String,
    reference: String,
    workspace: State<'_, WorkspaceState>,
) -> Result<WorkspaceFileReference, String> {
    let root = workspace.root()?;
    let path = workspace.resolve_file_reference(Path::new(&source_path), &reference)?;
    let relative_path = path
        .strip_prefix(&root)
        .map_err(|_| "The referenced file is outside the open workspace".to_string())?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("File")
        .to_string();
    Ok(WorkspaceFileReference {
        path: path.to_string_lossy().into_owned(),
        relative_path: relative_path.to_string_lossy().into_owned(),
        name,
    })
}

#[tauri::command]
pub fn read_workspace_dir(
    path: String,
    show_hidden: bool,
    workspace: State<'_, WorkspaceState>,
) -> Result<Vec<WorkspaceEntry>, String> {
    workspace.list_directory(Path::new(&path), show_hidden)
}

#[tauri::command]
pub async fn list_workspace_file_paths(
    show_hidden: bool,
    workspace: State<'_, WorkspaceState>,
) -> Result<Vec<String>, String> {
    let root = workspace.root()?;
    tauri::async_runtime::spawn_blocking(move || super::collect_file_paths(&root, show_hidden))
        .await
        .map_err(|error| format!("Workspace path indexing failed: {error}"))?
}

#[tauri::command]
pub async fn read_workspace_file(
    path: String,
    workspace: State<'_, WorkspaceState>,
) -> Result<String, String> {
    let path = workspace.resolve_existing(Path::new(&path))?;
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::read_to_string(path).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("File read task failed: {error}"))?
}

#[tauri::command]
pub fn write_workspace_file(
    path: String,
    content: String,
    workspace: State<'_, WorkspaceState>,
) -> Result<(), String> {
    workspace.write_file(Path::new(&path), &content)
}

#[tauri::command]
pub fn create_workspace_file(
    parent: String,
    name: String,
    content: String,
    workspace: State<'_, WorkspaceState>,
) -> Result<String, String> {
    workspace
        .create_file(Path::new(&parent), &name, &content)
        .map(|path| path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn create_workspace_dir(
    parent: String,
    name: String,
    workspace: State<'_, WorkspaceState>,
) -> Result<String, String> {
    workspace
        .create_directory(Path::new(&parent), &name)
        .map(|path| path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn remove_workspace_entry(
    path: String,
    workspace: State<'_, WorkspaceState>,
) -> Result<(), String> {
    workspace.remove_entry(Path::new(&path))
}

#[tauri::command]
pub fn move_workspace_entry(
    path: String,
    destination: String,
    name: String,
    workspace: State<'_, WorkspaceState>,
) -> Result<String, String> {
    workspace
        .move_entry(Path::new(&path), Path::new(&destination), &name)
        .map(|path| path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn copy_workspace_entry(
    path: String,
    destination: String,
    name: String,
    workspace: State<'_, WorkspaceState>,
) -> Result<String, String> {
    workspace
        .copy_entry(Path::new(&path), Path::new(&destination), &name)
        .map(|path| path.to_string_lossy().into_owned())
}
