use serde::Serialize;
use std::io::Read;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::State;

const MAX_CONTEXT_BYTES: usize = 64 * 1024;
const MAX_AI_OUTPUT_BYTES: usize = 1024 * 1024;
const AI_TIMEOUT_SECONDS: u64 = 120;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderInfo {
    id: String,
    label: String,
    installed: bool,
    binary_path: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContextFile {
    path: String,
    content: String,
}

#[tauri::command]
pub fn detect_ai_providers() -> Vec<AiProviderInfo> {
    [("codex", "Codex CLI"), ("agy", "AGY local")]
        .into_iter()
        .map(|(id, label)| {
            let path = find_binary(id);
            AiProviderInfo {
                id: id.to_string(),
                label: label.to_string(),
                installed: path.is_some(),
                binary_path: path.map(|path| path.display().to_string()),
            }
        })
        .collect()
}

#[tauri::command]
pub async fn generate_ai_response(
    workspace: State<'_, crate::workspace::WorkspaceState>,
    provider: String,
    binary_path: Option<String>,
    workspace_path: String,
    prompt: String,
    context_files: Vec<AiContextFile>,
) -> Result<String, String> {
    if !matches!(provider.as_str(), "codex" | "agy") {
        return Err("Choose Codex CLI or AGY local as the AI provider".to_string());
    }
    let root = workspace.resolve_existing(Path::new(&workspace_path))?;
    if !root.is_dir() {
        return Err("AI workspace must be a folder inside the open workspace".to_string());
    }

    let context_size = context_files
        .iter()
        .map(|file| file.content.len())
        .sum::<usize>();
    if context_size + prompt.len() > MAX_CONTEXT_BYTES {
        return Err(
            "Selected AI context exceeds 64 KiB. Remove some files or shorten the request."
                .to_string(),
        );
    }

    let binary = binary_path
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .and_then(|path| resolve_executable(&path))
        .or_else(|| find_binary(&provider))
        .ok_or_else(|| format!("Could not find {provider}. Set its binary path in AI settings."))?;

    let mut full_prompt = String::from(
        "You are the Lumi Tester assistant inside Lumi IDE. Analyze the supplied test flows and answer the user's request. Do not edit files or claim that changes were applied. If you propose a code change, include exactly one complete replacement in a fenced code block and put `Target file: path/from/workspace/root` on its own line immediately before the code block. Only propose changing a file supplied in the context. The user will review and stage it manually.\n\n",
    );
    for context in context_files {
        let path = workspace.resolve_existing(Path::new(&context.path))?;
        if !path.is_file() {
            return Err("AI context entries must refer to workspace files".to_string());
        }
        let relative = path
            .strip_prefix(&root)
            .map_err(|_| "AI context file is outside the open workspace".to_string())?;
        full_prompt.push_str(&format!(
            "\n--- FILE: {} ---\n{}\n--- END FILE ---\n",
            relative.display(),
            context.content
        ));
    }
    full_prompt.push_str("\nUser request:\n");
    full_prompt.push_str(&prompt);

    if full_prompt.len() > MAX_CONTEXT_BYTES {
        return Err(
            "Selected AI context exceeds 64 KiB. Remove some files or shorten the request."
                .to_string(),
        );
    }

    tauri::async_runtime::spawn_blocking(move || {
        run_provider(&provider, &binary, &root, &full_prompt)
    })
    .await
    .map_err(|error| format!("AI process task failed: {error}"))?
}

fn run_provider(
    provider: &str,
    binary: &Path,
    workspace: &Path,
    prompt: &str,
) -> Result<String, String> {
    let mut command = Command::new(binary);
    command.current_dir(workspace);
    match provider {
        "codex" => {
            command
                .arg("exec")
                .arg("--sandbox")
                .arg("read-only")
                .arg("--ephemeral")
                .arg("--skip-git-repo-check")
                .arg("--cd")
                .arg(workspace)
                .arg(prompt);
        }
        "agy" => {
            command
                .arg("--print")
                .arg("--mode")
                .arg("plan")
                .arg("--output-format")
                .arg("text")
                .arg("--disable-slash-commands")
                .arg("--prompt")
                .arg(prompt);
        }
        _ => return Err("Unsupported AI provider".to_string()),
    }

    #[cfg(unix)]
    command.process_group(0);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("Could not start {}: {error}", binary.display()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "AI process has no stdout pipe".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "AI process has no stderr pipe".to_string())?;
    let (stdout_sender, stdout_receiver) = mpsc::sync_channel(1);
    let (stderr_sender, stderr_receiver) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let _ = stdout_sender.send(read_bounded_output(stdout));
    });
    thread::spawn(move || {
        let _ = stderr_sender.send(read_bounded_output(stderr));
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < Duration::from_secs(AI_TIMEOUT_SECONDS) => {
                thread::sleep(Duration::from_millis(100));
            }
            Ok(None) => {
                terminate_child(&mut child);
                return Err(format!(
                    "{} timed out after {AI_TIMEOUT_SECONDS} seconds",
                    binary.display()
                ));
            }
            Err(error) => {
                terminate_child(&mut child);
                return Err(format!("Could not wait for {}: {error}", binary.display()));
            }
        }
    };
    let stdout = stdout_receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "AI stdout did not close after the process exited".to_string())?;
    let (stdout, stdout_truncated) =
        stdout.map_err(|error| format!("Could not read AI stdout: {error}"))?;
    let stderr = stderr_receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "AI stderr did not close after the process exited".to_string())?;
    let (stderr, stderr_truncated) =
        stderr.map_err(|error| format!("Could not read AI stderr: {error}"))?;
    if stdout_truncated || stderr_truncated {
        return Err("AI process output exceeded the 1 MiB limit".to_string());
    }
    if !status.success() {
        let details = String::from_utf8_lossy(&stderr).trim().to_string();
        return Err(if details.is_empty() {
            format!("{} exited with status {}", binary.display(), status)
        } else {
            details
        });
    }
    let response = String::from_utf8_lossy(&stdout).trim().to_string();
    if response.is_empty() {
        return Err(format!("{} returned an empty response", binary.display()));
    }
    Ok(response)
}

fn terminate_child(child: &mut std::process::Child) {
    #[cfg(unix)]
    unsafe {
        let process_group = -(child.id() as i32);
        let _ = libc::kill(process_group, libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn read_bounded_output(mut reader: impl Read) -> std::io::Result<(Vec<u8>, bool)> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 8192];
    let mut exceeded = false;
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(size) => {
                let remaining = MAX_AI_OUTPUT_BYTES.saturating_sub(output.len());
                let retained = size.min(remaining);
                output.extend_from_slice(&buffer[..retained]);
                exceeded |= retained < size;
            }
            Err(error) => return Err(error),
        }
    }
    Ok((output, exceeded))
}

fn find_binary(name: &str) -> Option<PathBuf> {
    let mut candidates = std::env::var_os("PATH")
        .into_iter()
        .flat_map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
        .map(|directory| directory.join(name))
        .collect::<Vec<_>>();

    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        candidates.extend([
            home.join(".local/bin").join(name),
            home.join(".npm-global/bin").join(name),
            home.join(".cargo/bin").join(name),
        ]);
    }
    #[cfg(target_os = "macos")]
    candidates.extend([
        PathBuf::from("/opt/homebrew/bin").join(name),
        PathBuf::from("/usr/local/bin").join(name),
    ]);
    #[cfg(target_os = "windows")]
    candidates.push(PathBuf::from(format!("{name}.exe")));

    candidates
        .into_iter()
        .find_map(|path| resolve_executable(&path))
}

fn resolve_executable(path: &Path) -> Option<PathBuf> {
    let candidate = if path.components().count() == 1 {
        let mut matches = std::env::var_os("PATH")
            .into_iter()
            .flat_map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
            .map(|directory| directory.join(path))
            .collect::<Vec<_>>();
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            matches.extend([
                home.join(".local/bin").join(path),
                home.join(".npm-global/bin").join(path),
                home.join(".cargo/bin").join(path),
            ]);
        }
        #[cfg(target_os = "macos")]
        matches.extend([
            PathBuf::from("/opt/homebrew/bin").join(path),
            PathBuf::from("/usr/local/bin").join(path),
        ]);
        matches
            .into_iter()
            .find(|candidate| is_executable(candidate))?
    } else {
        path.to_path_buf()
    };
    is_executable(&candidate).then(|| candidate.canonicalize().unwrap_or(candidate))
}

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        return std::fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false);
    }
    #[cfg(windows)]
    {
        true
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn ai_output_is_bounded_without_stopping_pipe_drain() {
        let input = vec![b'x'; MAX_AI_OUTPUT_BYTES + 32];
        let (output, truncated) = read_bounded_output(Cursor::new(input)).unwrap();
        assert_eq!(output.len(), MAX_AI_OUTPUT_BYTES);
        assert!(truncated);
    }
}
