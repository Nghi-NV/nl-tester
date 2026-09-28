use ignore::WalkBuilder;
use serde::Serialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::State;

const MAX_RESULTS: usize = 500;
const MAX_FILES_TO_SCAN: usize = 25_000;
const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;
const MAX_PREVIEW_CHARS: usize = 240;

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

#[tauri::command]
pub async fn search_workspace_text(
    workspace_path: String,
    query: String,
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

    tauri::async_runtime::spawn_blocking(move || search_workspace_root(root, &query))
        .await
        .map_err(|error| error.to_string())?
}

fn search_workspace_root(root: PathBuf, query: &str) -> Result<WorkspaceSearchResponse, String> {
    let root = fs::canonicalize(root).map_err(|error| error.to_string())?;
    let query_lower = query.to_lowercase();
    let mut walker = WalkBuilder::new(&root);
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
        if total_bytes.saturating_add(metadata.len()) > MAX_TOTAL_BYTES {
            truncated = true;
            break;
        }

        files_scanned += 1;
        if files_scanned > MAX_FILES_TO_SCAN {
            truncated = true;
            break;
        }

        let Ok(path) = fs::canonicalize(entry.path()) else {
            continue;
        };
        let Ok(relative_path) = path.strip_prefix(&root) else {
            continue;
        };
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
            let mut search_from = 0;
            while let Some((byte_offset, column)) =
                find_case_insensitive_from(line, &query_lower, search_from)
            {
                matches.push(WorkspaceSearchMatch {
                    relative_path: relative_path.to_string_lossy().into_owned(),
                    line_number: index + 1,
                    column,
                    line: preview_line(line, byte_offset),
                });
                if matches.len() > MAX_RESULTS {
                    matches.pop();
                    truncated = true;
                    break;
                }
                search_from = byte_offset
                    + line[byte_offset..]
                        .chars()
                        .next()
                        .map(char::len_utf8)
                        .unwrap_or(1);
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

fn find_case_insensitive_from(
    line: &str,
    query_lower: &str,
    search_from: usize,
) -> Option<(usize, usize)> {
    let lowered = line.to_lowercase();
    let mut lowered_offset = 0;
    for (byte_offset, character) in line.char_indices() {
        if byte_offset >= search_from && lowered[lowered_offset..].starts_with(query_lower) {
            let column = line[..byte_offset].encode_utf16().count() + 1;
            return Some((byte_offset, column));
        }
        lowered_offset += character.to_lowercase().to_string().len();
    }
    None
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
    use super::search_workspace_root;
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
}
