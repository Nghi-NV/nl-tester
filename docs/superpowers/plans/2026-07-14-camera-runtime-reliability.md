# Camera Runtime Reliability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Lumi Tester's camera workflows deadline-bounded, opt-in for live observation, and reliable across FFmpeg output sizes, CI, macOS, Linux, and Windows.

**Architecture:** Keep RTSP and HTTP as explicit `CameraSession` frame sources. Drain FFmpeg pipes concurrently, propagate one monotonic deadline through camera reads, and isolate optional observation UI from the detection source. Finish with platform-neutral launcher discovery, normal CLI platform detection, documented YAML fields, and stable Lumi Life selectors.

**Tech Stack:** Rust 2021, Tokio, standard-library process/TCP/thread APIs, Clap, Serde, Axum, existing Lumi Tester parser/executor/report infrastructure, YAML/JSON test assets.

---

## File Structure

- Modify `lumi-tester/src/camera/stream.rs`: timeout-safe FFmpeg child collection and tests.
- Modify `lumi-tester/src/camera/session.rs`: deadline-aware RTSP/HTTP frame reads and local TCP tests.
- Modify `lumi-tester/src/runner/executor.rs`: restore opt-in observe behavior and preserve explicit frame sources.
- Modify `lumi-tester/src/camera/server.rs`: retain the read-only `/view` endpoint used by explicit observation.
- Modify `lumi-tester/src/parser/types.rs`: retain and document `camera.server` and `camera.observe`.
- Modify `lumi-tester/src/camera/launcher.rs`: process-environment precedence and path-component discovery.
- Modify `lumi-tester/src/camera/mod.rs`: export the launcher module.
- Modify `lumi-tester/src/camera/profile.rs`: verify the committed sample profile loads.
- Modify `lumi-tester/src/main.rs`: retain camera shortcuts and detect the YAML platform for `camera test`.
- Modify `lumi-tester/src/report/html.rs`: retain escaped, actionable camera failure hints.
- Modify `lumi-tester/schema/lumi-test.schema.json`: describe `server` and `observe`.
- Modify `lumi-tester/docs/camera-hardware-testing.md`: explain source selection and observation semantics.
- Modify `lumi-tester/docs/api/commands.md`: document the same header fields in the command reference.
- Modify `lumi-tester/e2e/workspaces/lumi_life/subflows/select_home.yaml`: restore a hierarchy selector.
- Create `lumi-tester/e2e/workspaces/lumi_life/camera_lab_blink_probe.yaml`: retain the validated blink probe.
- Create `lumi-tester/e2e/workspaces/lumi_life/profiles/sample_switch4_camera.json`: retain the non-secret sample profile.

The VS Code extension has command metadata but no camera-header completion model, so this plan does not add speculative extension code. The bundled JSON schema remains the canonical header metadata.

## Phase 1: Runtime Correctness

### Task 1: Drain FFmpeg Output Without Deadlock

**Files:**
- Modify: `lumi-tester/src/camera/stream.rs:10-122`
- Test: `lumi-tester/src/camera/stream.rs` test module

- [ ] **Step 1: Add ignored child fixtures and failing collector tests**

Append a test module that uses the current Rust test binary as a portable child process. `--nocapture` makes fixture bytes reach the OS pipe on every supported platform.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Stdio};

    fn fixture_child(name: &str) -> std::process::Child {
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name, "--ignored", "--nocapture"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    }

    #[test]
    #[ignore]
    fn child_writes_large_output() {
        std::io::stdout().write_all(&vec![b'x'; 1024 * 1024]).unwrap();
    }

    #[test]
    #[ignore]
    fn child_never_exits() {
        std::thread::sleep(Duration::from_secs(60));
    }

    #[test]
    fn child_output_is_drained_before_exit() {
        let child = fixture_child("camera::stream::tests::child_writes_large_output");
        let output = collect_child_output(child, Duration::from_secs(5)).unwrap();
        assert!(output.status.success());
        assert!(output.stdout.len() >= 1024 * 1024);
    }

    #[test]
    fn timed_out_child_is_killed_and_reaped() {
        let child = fixture_child("camera::stream::tests::child_never_exits");
        let started = Instant::now();
        let error = collect_child_output(child, Duration::from_millis(100))
            .unwrap_err()
            .to_string();
        assert!(error.contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
```

- [ ] **Step 2: Run the collector tests and verify RED**

Run:

```bash
cd lumi-tester
cargo test camera::stream::tests::child_output_is_drained_before_exit --lib
```

Expected: compile failure because `collect_child_output` does not exist.

- [ ] **Step 3: Implement concurrent pipe draining and bounded reap**

Add the focused helper above `snapshot()`:

```rust
struct ChildOutput {
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn drain_pipe<R: Read + Send + 'static>(mut pipe: R) -> std::thread::JoinHandle<std::io::Result<Vec<u8>>> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        pipe.read_to_end(&mut bytes)?;
        Ok(bytes)
    })
}

fn collect_child_output(
    mut child: std::process::Child,
    timeout: Duration,
) -> Result<ChildOutput> {
    let stdout = child.stdout.take().context("child stdout is not piped")?;
    let stderr = child.stderr.take().context("child stderr is not piped")?;
    let stdout_reader = drain_pipe(stdout);
    let stderr_reader = drain_pipe(stderr);
    let deadline = Instant::now() + timeout;

    let status = loop {
        if let Some(status) = child.try_wait().context("failed to poll child process")? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            anyhow::bail!("child process timed out after {}ms", timeout.as_millis());
        }
        std::thread::sleep(Duration::from_millis(20));
    };

    let stdout = stdout_reader
        .join()
        .map_err(|_| anyhow!("stdout reader thread panicked"))??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| anyhow!("stderr reader thread panicked"))??;
    Ok(ChildOutput { status, stdout, stderr })
}
```

Replace the `try_wait()` loop in `snapshot()` with:

```rust
let output = collect_child_output(child, Duration::from_secs(20))
    .context("ffmpeg timed out while reading a camera frame")?;
if !output.status.success() || output.stdout.is_empty() {
    let err = String::from_utf8_lossy(&output.stderr);
    let redacted = crate::camera::redact_url(err.lines().last().unwrap_or("unknown error"));
    return Err(anyhow!("ffmpeg could not read a frame from the camera: {}", redacted));
}
let image = image::load_from_memory(&output.stdout)
    .context("failed to decode snapshot JPEG")?
    .to_rgb8();
Ok(image)
```

- [ ] **Step 4: Run GREEN tests**

Run:

```bash
cd lumi-tester
cargo test camera::stream::tests --lib
```

Expected: both non-ignored collector tests pass; fixture tests remain ignored.

- [ ] **Step 5: Commit the FFmpeg fix**

```bash
git add lumi-tester/src/camera/stream.rs
git commit -m "🐛 fix(camera): drain ffmpeg snapshot pipes"
```

### Task 2: Bound HTTP and RTSP Reads by One Deadline

**Files:**
- Modify: `lumi-tester/src/camera/session.rs:14-225,275-570`
- Test: `lumi-tester/src/camera/session.rs` test module

- [ ] **Step 1: Add a failing delayed-server deadline test**

Add a test-only JPEG server and assert that a delayed response cannot exceed the caller deadline:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn delayed_server(delay: Duration) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 1024];
            let _ = stream.read(&mut request);
            std::thread::sleep(delay);
            let body = b"not-needed-for-timeout-test";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            let _ = stream.write_all(body);
        });
        format!("http://{}", address)
    }

    #[test]
    fn server_frame_stops_at_caller_deadline() {
        let base_url = delayed_server(Duration::from_secs(1));
        let started = Instant::now();
        let deadline = started + Duration::from_millis(100);
        let error = CameraSession::server_frame_until(&base_url, deadline)
            .unwrap_err()
            .to_string();
        assert!(error.contains("deadline"));
        assert!(started.elapsed() < Duration::from_millis(500));
    }
}
```

- [ ] **Step 2: Run the deadline test and verify RED**

Run:

```bash
cd lumi-tester
cargo test camera::session::tests::server_frame_stops_at_caller_deadline --lib
```

Expected: compile failure because `server_frame_until` does not exist.

- [ ] **Step 3: Introduce deadline-aware frame acquisition**

Use a single default only at public one-shot entry points:

```rust
const DEFAULT_FRAME_TIMEOUT: Duration = Duration::from_secs(8);

fn remaining(deadline: Instant) -> Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or_else(|| anyhow!("camera frame deadline expired"))
}

impl CameraSession {
    fn camera_frame(&self) -> Result<CameraFrame> {
        self.camera_frame_until(Instant::now() + DEFAULT_FRAME_TIMEOUT)
    }

    fn camera_frame_until(&self, deadline: Instant) -> Result<CameraFrame> {
        match &self.source {
            CameraSource::Rtsp { .. } => self.rtsp_frame_until(deadline),
            CameraSource::Server { base_url } => Self::server_frame_until(base_url, deadline),
        }
    }

    fn server_frame_until(base_url: &str, deadline: Instant) -> Result<CameraFrame> {
        let endpoint = HttpEndpoint::parse(base_url)?;
        let bytes = get_http_bytes_until(&endpoint, "/api/frame.jpg", deadline)?;
        let image = image::load_from_memory(&bytes)
            .context("failed to decode camera server JPEG frame")?
            .to_rgb8();
        Ok(CameraFrame {
            image,
            captured_at: std::time::SystemTime::now(),
        })
    }
}
```

Represent the parsed endpoint once so unsupported schemes fail immediately instead of retrying:

```rust
struct HttpEndpoint {
    authority: String,
    base_path: String,
}

impl HttpEndpoint {
    fn parse(base_url: &str) -> Result<Self> {
        let value = base_url
            .trim()
            .trim_end_matches('/')
            .strip_prefix("http://")
            .ok_or_else(|| anyhow!("camera.server supports http:// URLs only"))?;
        let (authority, path) = value
            .split_once('/')
            .map(|(authority, path)| (authority, format!("/{}", path.trim_matches('/'))))
            .unwrap_or((value, String::new()));
        if authority.is_empty() {
            anyhow::bail!("camera server authority is empty");
        }
        Ok(Self { authority: authority.to_string(), base_path: path })
    }
}
```

In `try_get_http_bytes`, resolve the authority and use `TcpStream::connect_timeout`; set read/write timeouts to `remaining(deadline)?`. The outer retry loop must check `remaining(deadline)` before each attempt and sleep for `min(250ms, remaining)`.

- [ ] **Step 4: Thread the deadline through polling loops**

Change internal calls, keeping public APIs stable:

```rust
fn sample_state_until(
    &self,
    button: &str,
    start: std::time::SystemTime,
    fallback_elapsed_ms: u64,
    deadline: Instant,
) -> Result<(String, StateTimelineSample)> {
    let frame = self.camera_frame_until(deadline)?;
    // Keep the existing detection and StateTimelineSample construction unchanged.
}
```

Update `wait_state_with_timeline`, `assert_transition_with_timeline`,
`observe_blink_pattern`, and `observe_state_timeline` to call
`sample_state_until` or `camera_frame_until` with their already-computed
deadline. RTSP restart calls `remaining(deadline)?` before starting a new
grabber and rejects a frame received after expiry.

- [ ] **Step 5: Run session and camera tests**

Run:

```bash
cd lumi-tester
cargo test camera::session --lib
cargo test camera_ -- --nocapture
```

Expected: deadline test and all existing camera tests pass.

- [ ] **Step 6: Commit deadline-aware session I/O**

```bash
git add lumi-tester/src/camera/session.rs
git commit -m "🐛 fix(camera): bound frame reads by deadline"
```

### Task 3: Make Observation Explicit and Source-Neutral

**Files:**
- Modify: `lumi-tester/src/runner/executor.rs:41-86,210-224,724-825,1035-1090`
- Modify: `lumi-tester/src/parser/types.rs:82-102`
- Modify: `lumi-tester/src/camera/server.rs:98-174`
- Test: `lumi-tester/src/runner/executor.rs` test module

- [ ] **Step 1: Add failing observe-policy tests**

Extract the policy into a pure helper and test both values before changing runtime code:

```rust
#[test]
fn observe_is_disabled_when_header_flag_is_false() {
    let cfg = CameraFlowConfig {
        rtsp: "rtsp://camera/live".to_string(),
        server: None,
        profile: Some("profiles/camera.json".to_string()),
        transport: Some("tcp".to_string()),
        observe: false,
    };
    assert!(!should_start_camera_observe(&cfg));
}

#[test]
fn observe_is_enabled_only_when_header_flag_is_true() {
    let cfg = CameraFlowConfig {
        rtsp: String::new(),
        server: Some("http://localhost:9444".to_string()),
        profile: Some("profiles/camera.json".to_string()),
        transport: None,
        observe: true,
    };
    assert!(should_start_camera_observe(&cfg));
}
```

- [ ] **Step 2: Run policy tests and verify RED**

Run:

```bash
cd lumi-tester
cargo test runner::executor::tests::observe_is_ --lib
```

Expected: compile failure because `should_start_camera_observe` does not exist.

- [ ] **Step 3: Restore opt-in behavior and stop source mutation**

Add the policy helper:

```rust
fn should_start_camera_observe(cfg: &crate::parser::types::CameraFlowConfig) -> bool {
    cfg.observe
}
```

At the top of the `maybe_start_observe_views` loop:

```rust
if !should_start_camera_observe(&cfg) {
    continue;
}
```

Delete the block that assigns `active_cfg.server = Some(view_server)`. For an
existing `server`, open `<server>/view` only. For RTSP, retain the local
read-only server task and `/view` route, but camera commands continue to create
their session from `cfg.rtsp`.

For the local RTSP observation server, open the browser only after the port is
reachable. Start this readiness task after spawning the server:

```rust
async fn open_camera_view_when_ready(port: u16, url: String) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    while tokio::time::Instant::now() < deadline {
        if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            open_camera_view(&url);
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
```

Do not call `open_camera_view` before the local server task is spawned.

Keep the parser fields exactly:

```rust
#[serde(default)]
pub rtsp: String,
#[serde(default)]
pub server: Option<String>,
#[serde(default)]
pub profile: Option<String>,
#[serde(default)]
pub transport: Option<String>,
#[serde(default)]
pub observe: bool,
```

- [ ] **Step 4: Prevent Windows shell interpretation when opening a URL**

Replace the Windows `cmd /C start` branch with `explorer.exe`, which accepts the
URL as a process argument without command-shell metacharacter parsing:

```rust
#[cfg(target_os = "windows")]
let command = ("explorer.exe", vec![url]);
```

Make the same Windows change in `src/camera/mod.rs::open_browser` so the CLI
shortcut and executor share safe behavior.

- [ ] **Step 5: Run parser and executor tests**

Run:

```bash
cd lumi-tester
cargo test runner::executor --lib
cargo test parser::yaml::tests::invalid_camera_header_fails_clearly --lib
```

Expected: observe policy, camera substitution, and parser tests pass.

- [ ] **Step 6: Commit explicit observation behavior**

```bash
git add lumi-tester/src/runner/executor.rs lumi-tester/src/parser/types.rs lumi-tester/src/camera/server.rs lumi-tester/src/camera/mod.rs
git commit -m "🐛 fix(camera): keep observation opt in"
```

## Phase 2: Compatibility and UX

### Task 4: Make Launcher Discovery Cross-Platform

**Files:**
- Modify: `lumi-tester/src/camera/launcher.rs`
- Modify: `lumi-tester/src/camera/mod.rs`
- Modify: `lumi-tester/src/report/html.rs`
- Test: `lumi-tester/src/camera/launcher.rs` test module
- Test: `lumi-tester/src/report/html.rs` test module

- [ ] **Step 1: Add failing environment-precedence and component tests**

Avoid mutating the global process environment in parallel tests by injecting a
lookup closure:

```rust
#[test]
fn process_environment_precedes_dotenv() {
    let root = temp_root("env-precedence");
    fs::write(root.join(".env"), "CAMERA_RTSP=rtsp://dotenv/live\n").unwrap();
    let discovered = CameraLauncherConfig::discover_with_env(&root, |name| {
        (name == "CAMERA_RTSP").then(|| "rtsp://process/live".to_string())
    })
    .unwrap();
    assert_eq!(discovered.rtsp.as_deref(), Some("rtsp://process/live"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn profile_match_uses_path_components() {
    let root = temp_root("path-components");
    let profiles = root.join("nested").join("profiles");
    fs::create_dir_all(&profiles).unwrap();
    let expected = profiles.join("switch_camera.json");
    fs::write(&expected, "{}").unwrap();
    let discovered = CameraLauncherConfig::discover_with_env(&root, |_| None).unwrap();
    assert_eq!(discovered.profile.as_deref(), Some(expected.as_path()));
    let _ = fs::remove_dir_all(root);
}
```

- [ ] **Step 2: Run launcher tests and verify RED**

Run:

```bash
cd lumi-tester
cargo test camera::launcher::tests::process_environment_precedes_dotenv --lib
```

Expected: compile failure because `discover_with_env` does not exist.

- [ ] **Step 3: Implement injected environment lookup and component matching**

```rust
pub fn discover(root: &Path) -> Result<Self> {
    Self::discover_with_env(root, |name| std::env::var(name).ok())
}

fn discover_with_env<F>(root: &Path, env: F) -> Result<Self>
where
    F: Fn(&str) -> Option<String>,
{
    let root = root.to_path_buf();
    let env_file = find_env_file(&root);
    let rtsp = env("CAMERA_RTSP")
        .filter(|value| !value.trim().is_empty())
        .or_else(|| env_file.as_deref().and_then(read_camera_rtsp));
    let profile = find_first_matching_file(&root, |path| {
        path.extension().is_some_and(|ext| ext == "json")
            && path.components().any(|component| component.as_os_str() == "profiles")
            && path.file_name().is_some_and(|name| name.to_string_lossy().contains("camera"))
    })?;
    let test_yaml = find_first_matching_file(&root, |path| {
        path.extension().is_some_and(|ext| ext == "yaml" || ext == "yml")
            && path.file_name().is_some_and(|name| name.to_string_lossy().contains("camera"))
    })?;
    Ok(Self { root, rtsp, profile, test_yaml, env_file })
}
```

Retain `camera_failure_hint()` and the HTML hint test. Ensure every inserted
string continues through `html_escape()`.

- [ ] **Step 4: Run launcher and report tests**

```bash
cd lumi-tester
cargo test camera::launcher --lib
cargo test report::html::tests::camera_failure_hint_is_rendered_as_html --lib
```

Expected: all launcher and report hint tests pass.

- [ ] **Step 5: Commit launcher and failure guidance**

```bash
git add lumi-tester/src/camera/launcher.rs lumi-tester/src/camera/mod.rs lumi-tester/src/report/html.rs
git commit -m "✨ feat(camera): improve launcher discovery"
```

### Task 5: Detect Camera Test Platform and Document Header Fields

**Files:**
- Modify: `lumi-tester/src/main.rs:210-286,739-822,1311-1380`
- Modify: `lumi-tester/schema/lumi-test.schema.json:70-82`
- Modify: `lumi-tester/docs/camera-hardware-testing.md:190-250`
- Modify: `lumi-tester/docs/api/commands.md:1588-1620`
- Test: `lumi-tester/src/main.rs` test module
- Test: `lumi-tester/src/parser/yaml.rs` schema test module

- [ ] **Step 1: Add a failing platform helper test**

```rust
#[test]
fn camera_test_uses_platform_from_yaml() {
    let path = std::env::temp_dir().join(format!(
        "lumi-camera-platform-{}-ios.yaml",
        std::process::id()
    ));
    fs::write(&path, "platform: ios\n---\n- launchApp\n").unwrap();
    assert_eq!(camera_test_platform(&path).unwrap(), "ios");
    let _ = fs::remove_file(path);
}
```

- [ ] **Step 2: Run the platform test and verify RED**

Run:

```bash
cd lumi-tester
cargo test camera_test_uses_platform_from_yaml
```

Expected: compile failure because `camera_test_platform` does not exist.

- [ ] **Step 3: Reuse normal platform detection**

Add beside `detect_platform()`:

```rust
fn camera_test_platform(path: &Path) -> anyhow::Result<String> {
    Ok(detect_platform(path)?.unwrap_or_else(|| "android".to_string()))
}
```

In `CameraCommands::Test`, compute the platform after validation and pass it to
`runner::run_tests`:

```rust
let platform = camera_test_platform(&path)?;
runner::run_tests(
    &path,
    &platform,
    None,
    &output,
    false,
    false,
    false,
    true,
    true,
    true,
    None,
    None,
    None,
)
.await?;
```

- [ ] **Step 4: Add schema properties and a schema regression test**

Extend `cameraConfig.properties`:

```json
"server": {
  "type": "string",
  "description": "Existing Lumi camera HTTP server used as the frame source"
},
"observe": {
  "type": "boolean",
  "default": false,
  "description": "Open a read-only live view while the flow runs"
}
```

Add a schema regression test that checks the bundled JSON schema and then parses
the same header:

```rust
#[test]
fn bundled_schema_allows_camera_server_and_observe() {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../schema/lumi-test.schema.json")).unwrap();
    let properties = &schema["$defs"]["cameraConfig"]["properties"];
    assert_eq!(properties["server"]["type"], "string");
    assert_eq!(properties["observe"]["type"], "boolean");

    let yaml = r#"platform: android
camera:
  server: http://localhost:9444
  profile: profiles/camera.json
  observe: false
---
- getDeviceState:
    saveAs: state
"#;
    let flow = parse_yaml_content(yaml, Path::new("camera.yaml")).unwrap();
    let config = flow.cameras.unwrap().into_map().remove("default").unwrap();
    assert_eq!(config.server.as_deref(), Some("http://localhost:9444"));
    assert!(!config.observe);
}
```

- [ ] **Step 5: Document source versus observation semantics**

Add this canonical example to both camera documents:

```yaml
camera:
  rtsp: "${CAMERA_RTSP}"       # detection source
  profile: "${CAMERA_PROFILE}"
  transport: "tcp"
  observe: false               # no browser or local observation server
```

Add a second short example for an existing server:

```yaml
camera:
  server: "http://localhost:9444"
  profile: "${CAMERA_PROFILE}"
  observe: true                # opens http://localhost:9444/view
```

State explicitly that `observe` never changes the configured detection source.

- [ ] **Step 6: Run CLI and schema tests**

```bash
cd lumi-tester
cargo test camera_test_uses_platform_from_yaml
cargo test bundled_schema_allows_camera_server_and_observe --lib
cargo test camera_ -- --nocapture
```

Expected: platform, schema, shortcut, launcher, and report camera tests pass.

- [ ] **Step 7: Commit CLI, schema, and documentation**

```bash
git add lumi-tester/src/main.rs lumi-tester/schema/lumi-test.schema.json lumi-tester/docs/camera-hardware-testing.md lumi-tester/docs/api/commands.md
git commit -m "📚 docs(camera): clarify source and observe modes"
```

### Task 6: Stabilize Lumi Life Test Assets

**Files:**
- Modify: `lumi-tester/e2e/workspaces/lumi_life/subflows/select_home.yaml:9-25`
- Create: `lumi-tester/e2e/workspaces/lumi_life/camera_lab_blink_probe.yaml`
- Create: `lumi-tester/e2e/workspaces/lumi_life/profiles/sample_switch4_camera.json`
- Modify: `lumi-tester/src/camera/profile.rs` test module

- [ ] **Step 1: Replace the coordinate tap with the verified hierarchy selector**

Keep the pre-tap visibility assertion and use the same stable regex for the tap:

```yaml
- waitUntilVisible:
    regex: "Online[\\s\\S]*Nhà 3D"
- tap:
    regex: "Online[\\s\\S]*Nhà 3D"
- waitForAnimationToEnd
- waitUntilVisible:
    regex: "(Security|Devices & Groups|Scene|AI camera)"
```

Retain `eraseText` before typing because it directly fixes stale search input.

- [ ] **Step 2: Validate the modified subflow**

```bash
cd lumi-tester
cargo run -- validate e2e/workspaces/lumi_life/subflows/select_home.yaml --json
cargo run -- list e2e/workspaces/lumi_life/subflows/select_home.yaml --json
```

Expected: `valid: true`; list output contains the regex tap and no point tap.

- [ ] **Step 3: Validate and list the blink probe**

```bash
cd lumi-tester
cargo run -- validate e2e/workspaces/lumi_life/camera_lab_blink_probe.yaml --json
cargo run -- list e2e/workspaces/lumi_life/camera_lab_blink_probe.yaml --json
```

Expected: `valid: true` with command indexes 0 (`getDeviceState`) and 1
(`waitLedPattern`).

- [ ] **Step 4: Parse the sample profile without contacting hardware**

Add a launcher/profile unit test that loads the committed JSON:

```rust
#[test]
fn sample_switch4_profile_loads() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("e2e/workspaces/lumi_life/profiles/sample_switch4_camera.json");
    let profile = crate::camera::CameraProfile::load(&path).unwrap();
    assert_eq!(profile.buttons.len(), 5);
    assert!(profile.button("button_1").is_some());
}
```

Run:

```bash
cd lumi-tester
cargo test sample_switch4_profile_loads --lib
```

Expected: PASS without FFmpeg, RTSP, browser, or device access.

- [ ] **Step 5: Commit the Lumi Life assets**

```bash
git add lumi-tester/e2e/workspaces/lumi_life/subflows/select_home.yaml lumi-tester/e2e/workspaces/lumi_life/camera_lab_blink_probe.yaml lumi-tester/e2e/workspaces/lumi_life/profiles/sample_switch4_camera.json lumi-tester/src/camera/profile.rs
git commit -m "🧪 test(camera): add stable lumi life probes"
```

## Final Verification

### Task 7: Verify the Complete Change Set

**Files:**
- Verify only; do not introduce unrelated cleanup.

- [ ] **Step 1: Format only changed Rust files**

Record the current repo-wide baseline:

```bash
cd lumi-tester
cargo fmt --check
```

The current baseline contains unrelated formatting failures. Format only the
Rust files changed by this plan:

```bash
cd lumi-tester
rustfmt --edition 2021 src/camera/stream.rs src/camera/session.rs src/camera/server.rs src/camera/launcher.rs src/camera/mod.rs src/camera/profile.rs src/runner/executor.rs src/parser/types.rs src/report/html.rs src/main.rs
cd ..
git diff --check
git add lumi-tester/src/camera/stream.rs lumi-tester/src/camera/session.rs lumi-tester/src/camera/server.rs lumi-tester/src/camera/launcher.rs lumi-tester/src/camera/mod.rs lumi-tester/src/camera/profile.rs lumi-tester/src/runner/executor.rs lumi-tester/src/parser/types.rs lumi-tester/src/report/html.rs lumi-tester/src/main.rs
git diff --cached --quiet || git commit -m "🎨 style(camera): format runtime changes"
```

Expected: no formatting changes outside the listed files and no whitespace
errors in the final diff.

- [ ] **Step 2: Run offline tests**

```bash
cd lumi-tester
cargo test --lib
cargo test camera_ -- --nocapture
```

Expected: all library and camera CLI tests pass with zero failures.

- [ ] **Step 3: Re-run YAML validation and listing**

```bash
cd lumi-tester
cargo run -- validate e2e/workspaces/lumi_life/camera_lab_blink_probe.yaml --json
cargo run -- validate e2e/workspaces/lumi_life/subflows/select_home.yaml --json
cargo run -- list e2e/workspaces/lumi_life/subflows/select_home.yaml --json
```

Expected: both flows are valid; command indexes are emitted; `select_home` has
no coordinate tap.

- [ ] **Step 4: Inspect the final Git scope**

```bash
git diff --check
git status --short
git log --oneline -8
```

Expected: no whitespace errors; every intended working change is committed;
unrelated user files remain untouched.

- [ ] **Step 5: Run hardware smoke tests only when prerequisites are present**

First run:

```bash
cd lumi-tester
cargo run -- doctor --platform android --json
cargo run -- camera doctor
```

When Android, `CAMERA_RTSP`, and the lab profile are available, run:

```bash
cargo run -- run e2e/workspaces/lumi_life/subflows/select_home.yaml --platform android --report --snapshot --events-jsonl --output ./output/select-home-review
cargo run -- run e2e/workspaces/lumi_life/camera_lab_blink_probe.yaml --platform android --report --snapshot --events-jsonl --output ./output/camera-blink-review
```

Expected: the home selector completes without coordinate input; the camera flow
uses its declared RTSP source; no browser opens because `observe` defaults to
false. If hardware is unavailable, report the doctor evidence instead of
claiming runtime verification.
