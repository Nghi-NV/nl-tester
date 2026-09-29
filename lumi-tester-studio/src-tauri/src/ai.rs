use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io::Read;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};

const MAX_CONTEXT_BYTES: usize = 112 * 1024;
const MAX_AI_OUTPUT_BYTES: usize = 1024 * 1024;
const AI_TIMEOUT_SECONDS: u64 = 300;

#[derive(Default)]
pub struct AiRequestState {
    active: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AiResponseDelta {
    request_id: String,
    delta: String,
}

#[tauri::command]
pub fn cancel_ai_response(
    state: State<'_, AiRequestState>,
    request_id: String,
) -> Result<(), String> {
    let active = state
        .active
        .lock()
        .map_err(|_| "AI request state is unavailable".to_string())?;
    if let Some(cancelled) = active.get(&request_id) {
        cancelled.store(true, Ordering::Release);
    }
    Ok(())
}

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
    app: AppHandle,
    workspace: State<'_, crate::workspace::WorkspaceState>,
    requests: State<'_, AiRequestState>,
    request_id: String,
    provider: String,
    model: Option<String>,
    binary_path: Option<String>,
    workspace_path: String,
    prompt: String,
    context_files: Vec<AiContextFile>,
    system_instruction: String,
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
        .try_fold(0_usize, |size, file| size.checked_add(file.content.len()));
    let request_size = context_size
        .and_then(|size| size.checked_add(prompt.len()))
        .and_then(|size| size.checked_add(system_instruction.len()));
    if request_size.is_none_or(|size| size > MAX_CONTEXT_BYTES) {
        return Err(
            "Selected AI context exceeds 112 KiB. Remove some files or shorten the request."
                .to_string(),
        );
    }

    let binary = binary_path
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .and_then(|path| resolve_executable(&path))
        .or_else(|| find_binary(&provider))
        .ok_or_else(|| format!("Could not find {provider}. Set its binary path in AI settings."))?;

    let mut full_prompt = system_instruction.trim().to_string();
    full_prompt.push_str(
        "\n\nDo not edit files or claim that changes were applied. Only propose changing a file supplied in the context. The user will review and stage it manually.\n",
    );
    for context in context_files {
        let path = workspace.resolve_existing(Path::new(&context.path))?;
        if !path.is_file() {
            return Err("AI context entries must refer to workspace files".to_string());
        }
        let relative = path
            .strip_prefix(&root)
            .map_err(|_| "AI context file is outside the open workspace".to_string())?;
        if is_sensitive_context_path(relative) {
            return Err(
                "Sensitive workspace files cannot be sent to AI. Remove environment, credential, or private-key files from context."
                    .to_string(),
            );
        }
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
            "Selected AI context exceeds 112 KiB. Remove some files or shorten the request."
                .to_string(),
        );
    }

    let active_requests = requests.active.clone();
    let cancelled = Arc::new(AtomicBool::new(false));
    {
        let mut active = active_requests
            .lock()
            .map_err(|_| "AI request state is unavailable".to_string())?;
        if active.contains_key(&request_id) {
            return Err("AI request ID is already active".to_string());
        }
        active.insert(request_id.clone(), cancelled.clone());
    }

    let event_app = app.clone();
    let event_request_id = request_id.clone();
    let task = tauri::async_runtime::spawn_blocking(move || {
        run_provider(
            &provider,
            &binary,
            &root,
            &full_prompt,
            model.as_deref(),
            &event_app,
            &event_request_id,
            &cancelled,
        )
    })
    .await
    .map_err(|error| format!("AI process task failed: {error}"));

    if let Ok(mut active) = active_requests.lock() {
        active.remove(&request_id);
    }
    task?
}

fn run_provider(
    provider: &str,
    binary: &Path,
    workspace: &Path,
    prompt: &str,
    model: Option<&str>,
    app: &AppHandle,
    request_id: &str,
    cancelled: &AtomicBool,
) -> Result<String, String> {
    let mut command = Command::new(binary);
    command.current_dir(workspace);
    match provider {
        "codex" => {
            command
                .arg("exec")
                .arg("--json")
                .arg("--sandbox")
                .arg("read-only")
                .arg("--ephemeral")
                .arg("--skip-git-repo-check")
                .arg("--cd")
                .arg(workspace)
                .arg("--model")
                .arg(
                    model
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or("gpt-6-luna"),
                )
                .arg(prompt);
        }
        "agy" => {
            command
                .arg("--mode")
                .arg("plan")
                .arg("--sandbox")
                .arg("--output-format")
                .arg("text")
                .arg("--print-timeout")
                .arg("240s")
                .arg(format!("--print={prompt}"));
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
    let (stdout_sender, stdout_receiver) = mpsc::sync_channel(4);
    let (stderr_sender, stderr_receiver) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut stdout = stdout;
        let mut buffer = [0_u8; 8192];
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => break,
                Ok(size) => {
                    if stdout_sender.send(Ok(buffer[..size].to_vec())).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    let _ = stdout_sender.send(Err(error));
                    break;
                }
            }
        }
    });
    thread::spawn(move || {
        let _ = stderr_sender.send(read_bounded_output(stderr));
    });

    let started = Instant::now();
    let mut status: Option<ExitStatus> = None;
    let mut stdout_closed = false;
    let mut stdout_error = None;
    let mut was_cancelled = false;
    let mut cancellation_started = None;
    let mut decoder = AiOutputDecoder::new(provider, app, request_id);

    while (status.is_none() && !was_cancelled) || !stdout_closed {
        if cancelled.load(Ordering::Acquire) && status.is_none() && !was_cancelled {
            was_cancelled = true;
            cancellation_started = Some(Instant::now());
            terminate_child(&mut child);
            status = child.try_wait().ok().flatten();
        }

        if status.is_none() && !was_cancelled {
            match child.try_wait() {
                Ok(Some(exit_status)) => status = Some(exit_status),
                Ok(None) if started.elapsed() < Duration::from_secs(AI_TIMEOUT_SECONDS) => {}
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
        }

        if !stdout_closed {
            match stdout_receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(Ok(chunk)) => decoder.push(&chunk),
                Ok(Err(error)) => {
                    stdout_error = Some(error.to_string());
                    stdout_closed = true;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => stdout_closed = true,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }

        if cancellation_started
            .is_some_and(|cancelled_at| cancelled_at.elapsed() >= Duration::from_secs(5))
        {
            stdout_closed = true;
        }
    }
    decoder.finish();
    if let Some(error) = stdout_error {
        return Err(format!("Could not read AI stdout: {error}"));
    }
    let (stdout, stdout_truncated) = decoder.into_response();
    let stderr = stderr_receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "AI stderr did not close after the process exited".to_string())?;
    let (stderr, stderr_truncated) =
        stderr.map_err(|error| format!("Could not read AI stderr: {error}"))?;
    if stdout_truncated || stderr_truncated {
        return Err("AI process output exceeded the 1 MiB limit".to_string());
    }
    if !was_cancelled {
        if let Some(status) = status {
            if !status.success() {
                let details = String::from_utf8_lossy(&stderr).trim().to_string();
                return Err(if details.is_empty() {
                    format!("{} exited with status {}", binary.display(), status)
                } else {
                    details
                });
            }
        }
    }
    let response = stdout.trim().to_string();
    if response.is_empty() && !was_cancelled {
        return Err(format!("{} returned an empty response", binary.display()));
    }
    if provider == "agy" && !was_cancelled {
        let normalized_response = response.to_ascii_lowercase();
        if normalized_response.contains("tool required the \"command\" permission")
            || normalized_response.contains("[agy] print timeout after")
        {
            return Err(format!("AGY could not complete this request: {response}"));
        }
    }
    Ok(response)
}

struct AiOutputDecoder<'a> {
    provider: &'a str,
    app: &'a AppHandle,
    request_id: &'a str,
    output: Vec<u8>,
    pending_utf8: Vec<u8>,
    pending_line: Vec<u8>,
    response: String,
    truncated: bool,
}

impl<'a> AiOutputDecoder<'a> {
    fn new(provider: &'a str, app: &'a AppHandle, request_id: &'a str) -> Self {
        Self {
            provider,
            app,
            request_id,
            output: Vec::new(),
            pending_utf8: Vec::new(),
            pending_line: Vec::new(),
            response: String::new(),
            truncated: false,
        }
    }

    fn push(&mut self, bytes: &[u8]) {
        let remaining = MAX_AI_OUTPUT_BYTES.saturating_sub(self.output.len());
        let retained = bytes.len().min(remaining);
        self.output.extend_from_slice(&bytes[..retained]);
        self.truncated |= retained < bytes.len();
        if retained == 0 {
            return;
        }

        if self.provider == "codex" {
            self.pending_line.extend_from_slice(&bytes[..retained]);
            while let Some(end) = self.pending_line.iter().position(|byte| *byte == b'\n') {
                let line = self.pending_line.drain(..=end).collect::<Vec<_>>();
                self.push_codex_line(&line[..line.len() - 1]);
            }
        } else {
            self.pending_utf8.extend_from_slice(&bytes[..retained]);
            self.flush_utf8(false);
        }
    }

    fn finish(&mut self) {
        if self.provider == "codex" {
            if !self.pending_line.is_empty() {
                let line = std::mem::take(&mut self.pending_line);
                self.push_codex_line(&line);
            }
            if self.response.is_empty() {
                if let Some(fallback) = codex_completed_message(&self.output) {
                    self.response.push_str(&fallback);
                    emit_ai_delta(self.app, self.request_id, &fallback);
                }
            }
        } else {
            self.flush_utf8(true);
        }
    }

    fn flush_utf8(&mut self, final_chunk: bool) {
        loop {
            match std::str::from_utf8(&self.pending_utf8) {
                Ok(text) => {
                    let text = text.to_string();
                    self.pending_utf8.clear();
                    self.push_response(&text);
                    break;
                }
                Err(error) => {
                    let valid_up_to = error.valid_up_to();
                    if valid_up_to > 0 {
                        let text = std::str::from_utf8(&self.pending_utf8[..valid_up_to])
                            .expect("UTF-8 prefix is valid")
                            .to_string();
                        self.pending_utf8.drain(..valid_up_to);
                        self.push_response(&text);
                        continue;
                    }
                    if let Some(invalid_len) = error.error_len() {
                        self.pending_utf8.drain(..invalid_len);
                        self.push_response("�");
                        continue;
                    }
                    if final_chunk && !self.pending_utf8.is_empty() {
                        let text = String::from_utf8_lossy(&self.pending_utf8).into_owned();
                        self.pending_utf8.clear();
                        self.push_response(&text);
                    }
                    break;
                }
            }
        }
    }

    fn push_codex_line(&mut self, line: &[u8]) {
        if let Some(delta) = decode_codex_delta(line) {
            self.push_response(&delta);
        }
    }

    fn push_response(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.response.push_str(text);
        emit_ai_delta(self.app, self.request_id, text);
    }

    fn into_response(self) -> (String, bool) {
        let response = if self.provider == "codex" {
            self.response
        } else {
            String::from_utf8_lossy(&self.output).into_owned()
        };
        (response, self.truncated)
    }
}

fn emit_ai_delta(app: &AppHandle, request_id: &str, delta: &str) {
    let _ = app.emit(
        "ai-response-delta",
        AiResponseDelta {
            request_id: request_id.to_string(),
            delta: delta.to_string(),
        },
    );
}

fn decode_codex_delta(line: &[u8]) -> Option<String> {
    let event: Value = serde_json::from_slice(line).ok()?;
    (event.get("type")?.as_str()? == "item/agentMessage/delta")
        .then(|| event.get("delta")?.as_str().map(str::to_string))
        .flatten()
}

fn codex_completed_message(output: &[u8]) -> Option<String> {
    for line in output.split(|byte| *byte == b'\n') {
        let Ok(event) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        if event.get("type").and_then(Value::as_str) != Some("item/completed") {
            continue;
        }
        let Some(item) = event.get("item") else {
            continue;
        };
        let Some(item_type) = item.get("type").and_then(Value::as_str) else {
            continue;
        };
        let item_type = item_type.to_ascii_lowercase();
        if item_type != "agent_message" && item_type != "agentmessage" {
            continue;
        }
        if let Some(text) = item.get("text").and_then(Value::as_str) {
            return Some(text.to_string());
        }
        let Some(content) = item.get("content").and_then(Value::as_array) else {
            continue;
        };
        let text = content
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<String>();
        if !text.is_empty() {
            return Some(text);
        }
    }
    None
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

fn is_sensitive_context_path(path: &Path) -> bool {
    let segments = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    let Some(name) = segments.last() else {
        return false;
    };

    let env_template = matches!(
        name.as_str(),
        ".env.example" | ".env.sample" | ".env.template"
    );
    let sensitive_basename = ["credential", "credentials", "secret", "secrets"]
        .iter()
        .any(|prefix| {
            name == prefix
                || [".", "_", "-"]
                    .iter()
                    .any(|separator| name.starts_with(&format!("{prefix}{separator}")))
        });
    let private_key_name = ["id_rsa", "id_ed25519"].iter().any(|key| {
        name == key
            || name
                .strip_prefix(key)
                .is_some_and(|suffix| suffix == ".pub")
    });
    let sensitive_extension = Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension,
                "pem" | "key" | "p12" | "pfx" | "jks" | "keystore"
            )
        });

    segments
        .iter()
        .any(|segment| matches!(segment.as_str(), "secrets" | "credentials"))
        || (name.starts_with(".env") && !env_template)
        || sensitive_basename
        || private_key_name
        || sensitive_extension
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

    #[test]
    fn codex_json_delta_events_yield_only_assistant_text() {
        assert_eq!(
            decode_codex_delta(br#"{"type":"item/agentMessage/delta","delta":"Hello"}"#),
            Some("Hello".to_string())
        );
        assert_eq!(
            decode_codex_delta(br#"{"type":"turn/completed","delta":"ignored"}"#),
            None
        );
    }

    #[test]
    fn codex_completed_message_is_a_fallback_when_deltas_are_absent() {
        let output = br#"{"type":"noise"}
{"type":"item/completed","item":{"type":"agent_message","text":"Final answer"}}
"#;
        assert_eq!(
            codex_completed_message(output),
            Some("Final answer".to_string())
        );
    }

    #[test]
    fn ai_context_rejects_sensitive_files_but_allows_templates() {
        for path in [
            ".env",
            ".env.production",
            "secrets/api.yaml",
            "credentials.json",
            "ssh/id_ed25519",
            "certs/client.p12",
        ] {
            assert!(is_sensitive_context_path(Path::new(path)), "{path}");
        }
        for path in [".env.example", ".env.sample", "flows/login.yaml"] {
            assert!(!is_sensitive_context_path(Path::new(path)), "{path}");
        }
    }
}
