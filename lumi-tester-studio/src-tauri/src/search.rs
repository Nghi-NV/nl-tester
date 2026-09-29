use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::State;

const MAX_RESULTS: usize = 500;
const MAX_FILES_TO_SCAN: usize = 25_000;
const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;
const MAX_PREVIEW_CHARS: usize = 240;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSearchOptions {
    #[serde(default)]
    case_sensitive: bool,
    #[serde(default)]
    whole_word: bool,
    #[serde(default)]
    regex: bool,
    #[serde(default)]
    include_pattern: String,
    #[serde(default)]
    exclude_pattern: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSearchMatch {
    relative_path: String,
    line_number: usize,
    column: usize,
    line: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSearchResponse {
    matches: Vec<WorkspaceSearchMatch>,
    files_scanned: usize,
    truncated: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceReplaceResponse {
    files_changed: usize,
    replacements: usize,
    truncated: bool,
}

#[tauri::command]
pub async fn search_workspace_text(
    workspace_path: String,
    query: String,
    scope_path: Option<String>,
    options: WorkspaceSearchOptions,
    workspace: State<'_, crate::workspace::WorkspaceState>,
) -> Result<WorkspaceSearchResponse, String> {
    let query = query.trim().to_string();
    if query.is_empty() || query.chars().count() > 256 {
        return Err("Search text must contain between 1 and 256 characters".to_string());
    }

    let root = workspace.resolve_existing(Path::new(&workspace_path))?;
    if !root.is_dir() {
        return Err("The open workspace is not a folder".to_string());
    }
    let search_root = match scope_path {
        Some(scope_path) => workspace.resolve_existing(Path::new(&scope_path))?,
        None => root.clone(),
    };
    if !search_root.is_dir() {
        return Err("The search scope is not a folder".to_string());
    }
    if !search_root.starts_with(&root) {
        return Err("The search scope is outside the open workspace".to_string());
    }

    tauri::async_runtime::spawn_blocking(move || {
        search_workspace_scope(root, search_root, &query, &options)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn replace_workspace_text(
    workspace_path: String,
    query: String,
    replacement: String,
    scope_path: Option<String>,
    options: WorkspaceSearchOptions,
    workspace: State<'_, crate::workspace::WorkspaceState>,
) -> Result<WorkspaceReplaceResponse, String> {
    let query = query.trim().to_string();
    if query.is_empty() || query.chars().count() > 256 {
        return Err("Search text must contain between 1 and 256 characters".to_string());
    }
    if replacement.len() > MAX_FILE_SIZE as usize {
        return Err("Replacement text exceeds the per-file size limit".to_string());
    }

    let root = workspace.resolve_existing(Path::new(&workspace_path))?;
    if !root.is_dir() {
        return Err("The open workspace is not a folder".to_string());
    }
    let search_root = match scope_path {
        Some(scope_path) => workspace.resolve_existing(Path::new(&scope_path))?,
        None => root.clone(),
    };
    if !search_root.is_dir() || !search_root.starts_with(&root) {
        return Err("The search scope must be a folder inside the open workspace".to_string());
    }

    tauri::async_runtime::spawn_blocking(move || {
        replace_workspace_scope(root, search_root, &query, &replacement, &options)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg(test)]
fn search_workspace_root(root: PathBuf, query: &str) -> Result<WorkspaceSearchResponse, String> {
    search_workspace_scope(
        root.clone(),
        root,
        query,
        &WorkspaceSearchOptions::default(),
    )
}

fn search_workspace_scope(
    workspace_root: PathBuf,
    search_root: PathBuf,
    query: &str,
    options: &WorkspaceSearchOptions,
) -> Result<WorkspaceSearchResponse, String> {
    let workspace_root = fs::canonicalize(workspace_root).map_err(|error| error.to_string())?;
    let search_root = fs::canonicalize(search_root).map_err(|error| error.to_string())?;
    if !search_root.starts_with(&workspace_root) {
        return Err("The search scope is outside the open workspace".to_string());
    }
    let matcher = compile_matcher(query, options)?;
    let (include, exclude) = compile_path_patterns(options)?;
    let mut walker = WalkBuilder::new(&search_root);
    walker
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .parents(true)
        .require_git(false)
        .follow_links(false)
        .filter_entry(|entry| entry.depth() == 0 || entry.file_name() != ".git");

    let mut matches = Vec::new();
    let mut files_scanned = 0;
    let mut total_bytes = 0_u64;
    let mut truncated = false;

    for item in walker.build() {
        let entry = match item {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        if entry.depth() == 0 {
            continue;
        }

        let metadata = match fs::symlink_metadata(entry.path()) {
            Ok(metadata) if metadata.file_type().is_file() => metadata,
            _ => continue,
        };
        if metadata.len() > MAX_FILE_SIZE {
            continue;
        }
        let Ok(path) = fs::canonicalize(entry.path()) else {
            continue;
        };
        let Ok(relative_path) = path.strip_prefix(&workspace_root) else {
            continue;
        };
        let relative_path_text = relative_path.to_string_lossy().replace('\\', "/");
        if !path_matches(&relative_path_text, include.as_ref(), exclude.as_ref()) {
            continue;
        }
        if total_bytes.saturating_add(metadata.len()) > MAX_TOTAL_BYTES {
            truncated = true;
            break;
        }
        files_scanned += 1;
        if files_scanned > MAX_FILES_TO_SCAN {
            truncated = true;
            break;
        }
        let Ok(file) = fs::File::open(&path) else {
            continue;
        };
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        if file
            .take(MAX_FILE_SIZE + 1)
            .read_to_end(&mut bytes)
            .is_err()
        {
            continue;
        }
        if bytes.len() as u64 > MAX_FILE_SIZE {
            continue;
        }
        total_bytes = total_bytes.saturating_add(bytes.len() as u64);
        if bytes.contains(&0) {
            continue;
        }
        let Ok(content) = String::from_utf8(bytes) else {
            continue;
        };

        for (index, line) in content.lines().enumerate() {
            for found in matcher.find_iter(line) {
                matches.push(WorkspaceSearchMatch {
                    relative_path: relative_path_text.clone(),
                    line_number: index + 1,
                    column: line[..found.start()].encode_utf16().count() + 1,
                    line: preview_line(line, found.start()),
                });
                if matches.len() > MAX_RESULTS {
                    matches.pop();
                    truncated = true;
                    break;
                }
            }
            if truncated && matches.len() == MAX_RESULTS {
                break;
            }
        }
        if truncated && matches.len() == MAX_RESULTS {
            break;
        }
    }

    Ok(WorkspaceSearchResponse {
        matches,
        files_scanned: files_scanned.min(MAX_FILES_TO_SCAN),
        truncated,
    })
}

fn replace_workspace_scope(
    workspace_root: PathBuf,
    search_root: PathBuf,
    query: &str,
    replacement: &str,
    options: &WorkspaceSearchOptions,
) -> Result<WorkspaceReplaceResponse, String> {
    let workspace_root = fs::canonicalize(workspace_root).map_err(|error| error.to_string())?;
    let search_root = fs::canonicalize(search_root).map_err(|error| error.to_string())?;
    if !search_root.starts_with(&workspace_root) {
        return Err("The search scope is outside the open workspace".to_string());
    }
    let matcher = compile_matcher(query, options)?;
    let (include, exclude) = compile_path_patterns(options)?;
    let mut walker = WalkBuilder::new(&search_root);
    walker
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .parents(true)
        .require_git(false)
        .follow_links(false)
        .filter_entry(|entry| entry.depth() == 0 || entry.file_name() != ".git");

    let mut files_scanned = 0;
    let mut total_bytes = 0_u64;
    let mut files_changed = 0;
    let mut replacements = 0;
    let mut truncated = false;
    for item in walker.build() {
        let Ok(entry) = item else { continue };
        if entry.depth() == 0 {
            continue;
        }
        let metadata = match fs::symlink_metadata(entry.path()) {
            Ok(metadata) if metadata.file_type().is_file() => metadata,
            _ => continue,
        };
        if metadata.len() > MAX_FILE_SIZE {
            continue;
        }
        let Ok(path) = fs::canonicalize(entry.path()) else {
            continue;
        };
        let Ok(relative_path) = path.strip_prefix(&workspace_root) else {
            continue;
        };
        let relative_path_text = relative_path.to_string_lossy().replace('\\', "/");
        if !path_matches(&relative_path_text, include.as_ref(), exclude.as_ref()) {
            continue;
        }
        if total_bytes.saturating_add(metadata.len()) > MAX_TOTAL_BYTES {
            truncated = true;
            break;
        }
        files_scanned += 1;
        if files_scanned > MAX_FILES_TO_SCAN {
            truncated = true;
            break;
        }
        let Ok(bytes) = fs::read(&path) else { continue };
        total_bytes = total_bytes.saturating_add(bytes.len() as u64);
        if bytes.contains(&0) {
            continue;
        }
        let Ok(content) = String::from_utf8(bytes) else {
            continue;
        };
        let count = matcher.find_iter(&content).count();
        if count == 0 {
            continue;
        }
        let replaced = matcher.replace_all(&content, replacement).into_owned();
        if replaced != content {
            if replaced.len() as u64 > MAX_FILE_SIZE {
                return Err(format!(
                    "Replacing matches would make {} exceed the 2 MiB file limit",
                    relative_path_text
                ));
            }
            fs::write(&path, replaced).map_err(|error| {
                format!(
                    "Could not replace matches in {}: {error}",
                    relative_path_text
                )
            })?;
            files_changed += 1;
            replacements += count;
        }
    }
    Ok(WorkspaceReplaceResponse {
        files_changed,
        replacements,
        truncated,
    })
}

fn compile_matcher(query: &str, options: &WorkspaceSearchOptions) -> Result<Regex, String> {
    let pattern = if options.regex {
        query.to_string()
    } else {
        regex::escape(query)
    };
    let pattern = if options.whole_word {
        format!(r"\b(?:{pattern})\b")
    } else {
        pattern
    };
    RegexBuilder::new(&pattern)
        .case_insensitive(!options.case_sensitive)
        .build()
        .map_err(|error| format!("Invalid search pattern: {error}"))
}

fn compile_path_patterns(
    options: &WorkspaceSearchOptions,
) -> Result<(Option<GlobSet>, Option<GlobSet>), String> {
    let compile = |pattern: &str| -> Result<Option<GlobSet>, String> {
        if pattern.trim().is_empty() {
            return Ok(None);
        }
        if pattern.len() > 2048 {
            return Err("File filter patterns must be 2048 characters or shorter".to_string());
        }
        let mut builder = GlobSetBuilder::new();
        let items = pattern
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>();
        if items.len() > 32 {
            return Err("Use at most 32 comma-separated file filter patterns".to_string());
        }
        for item in items {
            builder.add(
                Glob::new(item).map_err(|error| format!("Invalid file glob '{item}': {error}"))?,
            );
        }
        builder.build().map(Some).map_err(|error| error.to_string())
    };
    Ok((
        compile(&options.include_pattern)?,
        compile(&options.exclude_pattern)?,
    ))
}

fn path_matches(path: &str, include: Option<&GlobSet>, exclude: Option<&GlobSet>) -> bool {
    include.is_none_or(|patterns| patterns.is_match(path))
        && !exclude.is_some_and(|patterns| patterns.is_match(path))
}

fn preview_line(line: &str, match_byte_offset: usize) -> String {
    let chars: Vec<char> = line.chars().collect();
    let match_char_offset = line[..match_byte_offset].chars().count();
    let start = match_char_offset.saturating_sub(MAX_PREVIEW_CHARS / 2);
    let end = (start + MAX_PREVIEW_CHARS).min(chars.len());
    let mut preview: String = chars[start..end].iter().collect();
    if start > 0 {
        preview.insert(0, '…');
    }
    if end < chars.len() {
        preview.push('…');
    }
    preview
}

#[cfg(test)]
mod tests {
    use super::{search_workspace_root, search_workspace_scope, WorkspaceSearchOptions};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn search_returns_line_and_unicode_editor_column_and_honors_gitignore() {
        let root = tempdir().unwrap();
        fs::write(root.path().join(".gitignore"), "ignored.txt\n").unwrap();
        fs::write(
            root.path().join("flow.yaml"),
            "# 🧪 x Welcome Welcome\n- tap: submit\n",
        )
        .unwrap();
        fs::write(root.path().join("ignored.txt"), "welcome").unwrap();

        let result = search_workspace_root(root.path().to_path_buf(), "welcome").unwrap();
        assert_eq!(result.matches.len(), 2);
        assert_eq!(result.matches[0].line_number, 1);
        assert_eq!(result.matches[0].column, 8);
        assert_eq!(result.matches[1].column, 16);
        assert_eq!(result.matches[0].relative_path, "flow.yaml");
    }

    #[test]
    fn search_includes_hidden_environment_files_and_skips_binary_files() {
        let root = tempdir().unwrap();
        fs::write(root.path().join(".env"), "API_TOKEN=local-secret\n").unwrap();
        fs::write(root.path().join("sample.bin"), [0, 1, 2, 3, b'a']).unwrap();

        let result = search_workspace_root(root.path().to_path_buf(), "local-secret").unwrap();
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].relative_path, ".env");
    }

    #[test]
    fn scoped_search_only_returns_matches_from_the_selected_folder() {
        let root = tempdir().unwrap();
        let selected_folder = root.path().join("flows").join("checkout");
        fs::create_dir_all(&selected_folder).unwrap();
        fs::write(selected_folder.join("purchase.yaml"), "tap: Buy now\n").unwrap();
        fs::write(root.path().join("other.yaml"), "tap: Buy now\n").unwrap();

        let result = search_workspace_scope(
            root.path().to_path_buf(),
            selected_folder,
            "buy now",
            &WorkspaceSearchOptions::default(),
        )
        .unwrap();

        assert_eq!(result.matches.len(), 1);
        assert_eq!(
            result.matches[0].relative_path.replace('\\', "/"),
            "flows/checkout/purchase.yaml"
        );
    }
}
