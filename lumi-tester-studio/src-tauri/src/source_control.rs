use serde::Serialize;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};
use tauri::State;

const MAX_DIFF_BYTES: usize = 1024 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlFile {
    path: String,
    index_status: String,
    worktree_status: String,
    staged: bool,
    modified: bool,
    deleted: bool,
    untracked: bool,
    conflict: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlSnapshot {
    branch: String,
    branches: Vec<String>,
    files: Vec<SourceControlFile>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlDiff {
    text: String,
    truncated: bool,
}

#[tauri::command]
pub async fn source_control_status(
    workspace_path: String,
    workspace: State<'_, crate::workspace::WorkspaceState>,
) -> Result<SourceControlSnapshot, String> {
    let root = resolve_workspace(&workspace_path, &workspace)?;
    tauri::async_runtime::spawn_blocking(move || status_at(&root))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn source_control_init(
    workspace_path: String,
    workspace: State<'_, crate::workspace::WorkspaceState>,
) -> Result<(), String> {
    let root = resolve_workspace(&workspace_path, &workspace)?;
    tauri::async_runtime::spawn_blocking(move || {
        run_git(&root, ["init"])?;
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn source_control_diff(
    workspace_path: String,
    path: String,
    staged: bool,
    workspace: State<'_, crate::workspace::WorkspaceState>,
) -> Result<SourceControlDiff, String> {
    let root = resolve_workspace(&workspace_path, &workspace)?;
    let path = validate_relative_path(&path)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut args = vec!["diff", "--no-ext-diff", "--no-color", "--unified=3"];
        if staged {
            args.push("--cached");
        }
        args.push("--");
        let pathspec = literal_pathspec(&path);
        let output = run_git(&root, args.into_iter().chain([pathspec.as_str()]))?;
        let text = String::from_utf8_lossy(&output.stdout).into_owned();
        let truncated = text.len() > MAX_DIFF_BYTES;
        Ok(SourceControlDiff {
            text: if truncated {
                text.chars().take(MAX_DIFF_BYTES).collect()
            } else {
                text
            },
            truncated,
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn source_control_stage(
    workspace_path: String,
    path: String,
    stage: bool,
    workspace: State<'_, crate::workspace::WorkspaceState>,
) -> Result<(), String> {
    let root = resolve_workspace(&workspace_path, &workspace)?;
    let path = validate_relative_path(&path)?;
    tauri::async_runtime::spawn_blocking(move || {
        let pathspec = literal_pathspec(&path);
        if stage {
            run_git(&root, ["add", "--", pathspec.as_str()])?;
        } else {
            run_git(&root, ["reset", "-q", "--", pathspec.as_str()])?;
        }
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn source_control_switch_branch(
    workspace_path: String,
    branch: String,
    workspace: State<'_, crate::workspace::WorkspaceState>,
) -> Result<(), String> {
    let root = resolve_workspace(&workspace_path, &workspace)?;
    if branch.trim().is_empty() || branch.len() > 255 {
        return Err("Choose a valid local Git branch".to_string());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let snapshot = status_at(&root)?;
        if !snapshot
            .branches
            .iter()
            .any(|candidate| candidate == &branch)
        {
            return Err("The selected local branch no longer exists".to_string());
        }
        run_git(&root, ["switch", "--no-guess", branch.as_str()])?;
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn source_control_commit(
    workspace_path: String,
    message: String,
    workspace: State<'_, crate::workspace::WorkspaceState>,
) -> Result<String, String> {
    let message = message.trim().to_string();
    if message.is_empty() || message.len() > 2000 {
        return Err("Commit message must contain 1 to 2000 characters".to_string());
    }
    let root = resolve_workspace(&workspace_path, &workspace)?;
    tauri::async_runtime::spawn_blocking(move || {
        let output = run_git(&root, ["commit", "--message", message.as_str()])?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

fn resolve_workspace(
    workspace_path: &str,
    workspace: &State<'_, crate::workspace::WorkspaceState>,
) -> Result<PathBuf, String> {
    let root = workspace.resolve_existing(Path::new(workspace_path))?;
    if !root.is_dir() {
        return Err("Source Control requires an open workspace folder".to_string());
    }
    Ok(root)
}

fn validate_relative_path(path: &str) -> Result<String, String> {
    let normalized = path.replace('\\', "/");
    let requested = Path::new(&normalized);
    if normalized.is_empty()
        || requested.is_absolute()
        || requested
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("The Git path must be relative to the open workspace".to_string());
    }
    Ok(normalized)
}

fn literal_pathspec(path: &str) -> String {
    format!(":(literal){path}")
}

fn status_at(root: &Path) -> Result<SourceControlSnapshot, String> {
    let branch = run_git(root, ["branch", "--show-current"])?;
    let branch = String::from_utf8_lossy(&branch.stdout).trim().to_string();
    let branch_output = run_git(root, ["branch", "--format=%(refname:short)"])?;
    let branches = String::from_utf8_lossy(&branch_output.stdout)
        .lines()
        .map(str::trim)
        .filter(|branch| !branch.is_empty())
        .map(str::to_string)
        .collect();
    let status = run_git(
        root,
        [
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--no-renames",
        ],
    )?;
    let mut files = Vec::new();
    for entry in status
        .stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
    {
        if entry.len() < 4 {
            continue;
        }
        let index = entry[0] as char;
        let worktree = entry[1] as char;
        let path = String::from_utf8_lossy(&entry[3..]).replace('\\', "/");
        let untracked = index == '?' && worktree == '?';
        let conflict = matches!(
            (index, worktree),
            ('U', _) | (_, 'U') | ('A', 'A') | ('D', 'D')
        );
        files.push(SourceControlFile {
            path,
            index_status: index.to_string(),
            worktree_status: worktree.to_string(),
            staged: index != ' ' && !untracked,
            modified: worktree != ' ' && !untracked,
            deleted: index == 'D' || worktree == 'D',
            untracked,
            conflict,
        });
    }
    files.sort_by(|left, right| left.path.to_lowercase().cmp(&right.path.to_lowercase()));
    Ok(SourceControlSnapshot {
        branch,
        branches,
        files,
    })
}

fn run_git<I, S>(root: &Path, args: I) -> Result<Output, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|error| format!("Could not start Git: {error}"))?;
    if output.status.success() {
        Ok(output)
    } else {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if message.is_empty() {
            "Git command failed".to_string()
        } else {
            message
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{literal_pathspec, validate_relative_path};

    #[test]
    fn git_paths_are_passed_as_literal_pathspecs() {
        assert_eq!(literal_pathspec(":(glob)**"), ":(literal):(glob)**");
        assert_eq!(
            literal_pathspec("normal file.yaml"),
            ":(literal)normal file.yaml"
        );
    }

    #[test]
    fn workspace_root_and_parent_traversal_are_not_git_file_paths() {
        assert!(validate_relative_path(".").is_err());
        assert!(validate_relative_path("../outside.yaml").is_err());
        assert!(validate_relative_path("flows/test.yaml").is_ok());
    }
}
