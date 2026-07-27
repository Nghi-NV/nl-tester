# Camera CLI Usability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make camera-profile and camera-test workflows usable without testers memorizing long `lumi-tester camera ...` commands.

**Architecture:** Add a small camera UX layer around the existing `calibrate`, `detect`, and `run` commands. Keep the current low-level commands stable for CI and advanced users, while adding discoverable shortcuts, auto-discovery, and failure hints. Defer a full web launcher until the CLI shortcuts and hints are verified.

**Tech Stack:** Rust 2021, clap, anyhow, walkdir, existing Lumi Tester camera server/report modules.

---

## File Structure

- Modify: `src/main.rs`
  - Add camera shortcut subcommands.
  - Wire shortcuts to existing camera/run behavior.
  - Keep existing `calibrate`, `snapshot`, and `detect` commands unchanged.
- Create: `src/camera/launcher.rs`
  - Own camera workflow discovery and command suggestion logic.
  - Resolve `.env`, `CAMERA_RTSP`, profile paths, camera YAML paths, and default output directories.
  - Render friendly next-step text.
- Modify: `src/camera/mod.rs`
  - Export `launcher`.
- Modify: `src/report/html.rs`
  - Add optional camera failure hint block when an error includes camera evidence.
- Create or extend tests in `src/camera/launcher.rs` and `src/report/html.rs`
  - Unit-test discovery, hints, and report rendering without opening RTSP or devices.

## Scope Rules

- Do not change camera detection behavior.
- Do not change existing YAML syntax.
- Do not remove or rename existing commands.
- Do not start the calibrate web server in unit tests.
- Do not add a dependency unless a test proves the standard library approach is too complex.

## User-Facing Result

Common daily commands:

```bash
lumi-tester camera profile
lumi-tester camera observe
lumi-tester camera check
lumi-tester camera test
lumi-tester camera doctor
```

For repo-local development:

```bash
cargo run -- camera profile
cargo run -- camera observe
cargo run -- camera check
cargo run -- camera test
cargo run -- camera doctor
```

When required inputs are missing, the command prints exactly what to fix and an example command. It must not force testers to remember `--rtsp`, `--profile`, `--transport`, report flags, or camera YAML paths for the common Lumi Life workspace.

---

### Task 1: Add Camera Launcher Discovery

**Files:**
- Create: `src/camera/launcher.rs`
- Modify: `src/camera/mod.rs`
- Test: `src/camera/launcher.rs`

- [ ] **Step 1: Write failing discovery tests**

Add this test module in the new `src/camera/launcher.rs` file before implementing the production functions:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_root(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lumi-camera-launcher-{}-{}",
            name,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn discovers_env_profile_and_camera_yaml_from_workspace() {
        let root = temp_root("workspace");
        fs::write(root.join(".env"), "CAMERA_RTSP=rtsp://user:pass@10.0.0.5/live\n").unwrap();
        let workspace = root.join("e2e/workspaces/lumi_life");
        fs::create_dir_all(workspace.join("profiles")).unwrap();
        fs::write(workspace.join("profiles/lab_switch4_camera.json"), "{}").unwrap();
        fs::write(
            workspace.join("camera_hardware_sample.yaml"),
            "platform: android\ncamera:\n  profile: profiles/lab_switch4_camera.json\n---\n- getDeviceState:\n    saveAs: state\n",
        )
        .unwrap();

        let discovered = CameraLauncherConfig::discover(&root).unwrap();

        assert_eq!(
            discovered.rtsp.as_deref(),
            Some("rtsp://user:pass@10.0.0.5/live")
        );
        assert_eq!(
            discovered.profile.unwrap(),
            workspace.join("profiles/lab_switch4_camera.json")
        );
        assert_eq!(
            discovered.test_yaml.unwrap(),
            workspace.join("camera_hardware_sample.yaml")
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn reports_missing_rtsp_with_actionable_message() {
        let root = temp_root("missing-rtsp");
        let workspace = root.join("e2e/workspaces/lumi_life");
        fs::create_dir_all(workspace.join("profiles")).unwrap();
        fs::write(workspace.join("profiles/lab_switch4_camera.json"), "{}").unwrap();

        let discovered = CameraLauncherConfig::discover(&root).unwrap();
        let error = discovered.require_rtsp("profile").unwrap_err().to_string();

        assert!(error.contains("CAMERA_RTSP"));
        assert!(error.contains(".env"));
        assert!(error.contains("lumi-tester camera profile --rtsp"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn renders_doctor_summary_with_short_commands() {
        let root = temp_root("doctor");
        let workspace = root.join("e2e/workspaces/lumi_life");
        fs::create_dir_all(workspace.join("profiles")).unwrap();
        fs::write(root.join(".env"), "CAMERA_RTSP=rtsp://10.0.0.5/live\n").unwrap();
        fs::write(workspace.join("profiles/lab_switch4_camera.json"), "{}").unwrap();
        fs::write(workspace.join("camera_lab_read_state.yaml"), "platform: android\n---\n- launchApp\n").unwrap();

        let discovered = CameraLauncherConfig::discover(&root).unwrap();
        let summary = discovered.render_doctor_summary();

        assert!(summary.contains("camera profile"));
        assert!(summary.contains("camera observe"));
        assert!(summary.contains("camera check"));
        assert!(summary.contains("camera test"));

        let _ = fs::remove_dir_all(&root);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test camera::launcher --lib
```

Expected: FAIL because `CameraLauncherConfig` and related methods do not exist yet.

- [ ] **Step 3: Implement minimal discovery**

Create `src/camera/launcher.rs` with:

```rust
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CameraLauncherConfig {
    pub root: PathBuf,
    pub rtsp: Option<String>,
    pub profile: Option<PathBuf>,
    pub test_yaml: Option<PathBuf>,
    pub env_file: Option<PathBuf>,
}

impl CameraLauncherConfig {
    pub fn discover(root: &Path) -> Result<Self> {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let env_file = find_env_file(&root);
        let rtsp = env_file.as_deref().and_then(read_camera_rtsp);
        let profile = find_first_matching_file(&root, |path| {
            path.extension().is_some_and(|ext| ext == "json")
                && path.to_string_lossy().contains("/profiles/")
                && path.file_name()
                    .is_some_and(|name| name.to_string_lossy().contains("camera"))
        })?;
        let test_yaml = find_first_matching_file(&root, |path| {
            path.extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
                && path.file_name()
                    .is_some_and(|name| name.to_string_lossy().contains("camera"))
        })?;

        Ok(Self {
            root,
            rtsp,
            profile,
            test_yaml,
            env_file,
        })
    }

    pub fn require_rtsp(&self, action: &str) -> Result<String> {
        self.rtsp.clone().with_context(|| {
            format!(
                "Missing CAMERA_RTSP for camera {action}. Add CAMERA_RTSP=rtsp://... to .env, or run: lumi-tester camera {action} --rtsp rtsp://..."
            )
        })
    }

    pub fn require_profile(&self, action: &str) -> Result<PathBuf> {
        self.profile.clone().with_context(|| {
            format!(
                "Missing camera profile for camera {action}. Create one with: lumi-tester camera profile --profile e2e/workspaces/lumi_life/profiles/lab_switch4_camera.json"
            )
        })
    }

    pub fn require_test_yaml(&self) -> Result<PathBuf> {
        self.test_yaml.clone().with_context(|| {
            "Missing camera YAML test. Expected a file like e2e/workspaces/lumi_life/camera_hardware_sample.yaml".to_string()
        })
    }

    pub fn render_doctor_summary(&self) -> String {
        let rtsp = if self.rtsp.is_some() { "found" } else { "missing" };
        let profile = self
            .profile
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "missing".to_string());
        let test_yaml = self
            .test_yaml
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "missing".to_string());

        format!(
            "Camera setup\n  CAMERA_RTSP: {rtsp}\n  camera profile: {profile}\n  camera YAML: {test_yaml}\n\nShort commands:\n  lumi-tester camera profile\n  lumi-tester camera observe\n  lumi-tester camera check\n  lumi-tester camera test\n"
        )
    }
}

fn find_env_file(root: &Path) -> Option<PathBuf> {
    [root.join(".env"), root.join("..").join(".env")]
        .into_iter()
        .find(|path| path.exists())
}

fn read_camera_rtsp(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    content.lines().find_map(|line| {
        let line = line.trim();
        let value = line.strip_prefix("CAMERA_RTSP=")?;
        Some(value.trim_matches('"').trim_matches('\'').to_string())
    })
}

fn find_first_matching_file<F>(root: &Path, matches: F) -> Result<Option<PathBuf>>
where
    F: Fn(&Path) -> bool,
{
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .max_depth(6)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        if path.components().any(|part| part.as_os_str() == "target") {
            continue;
        }
        if matches(path) {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    Ok(files.into_iter().next())
}
```

Modify `src/camera/mod.rs`:

```rust
pub mod launcher;
```

- [ ] **Step 4: Run tests to verify pass**

Run:

```bash
cargo test camera::launcher --lib
```

Expected: PASS.

---

### Task 2: Add Friendly Camera Shortcut Commands

**Files:**
- Modify: `src/main.rs`
- Test: existing unit tests in `src/main.rs`

- [ ] **Step 1: Write failing clap/debug tests**

In `src/main.rs` test module, add:

```rust
#[test]
fn camera_shortcuts_are_available_in_help() {
    use clap::CommandFactory;

    let mut help = Vec::new();
    Cli::command()
        .find_subcommand_mut("camera")
        .unwrap()
        .write_long_help(&mut help)
        .unwrap();
    let help = String::from_utf8(help).unwrap();

    assert!(help.contains("profile"));
    assert!(help.contains("observe"));
    assert!(help.contains("check"));
    assert!(help.contains("test"));
    assert!(help.contains("doctor"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test camera_shortcuts_are_available_in_help
```

Expected: FAIL because the shortcuts are not registered.

- [ ] **Step 3: Add shortcut variants**

Extend `CameraCommands` in `src/main.rs`:

```rust
    /// Open the camera profile editor using discovered defaults
    Profile {
        #[arg(short, long)]
        rtsp: Option<String>,
        #[arg(short, long)]
        profile: Option<PathBuf>,
        #[arg(long, default_value = "9444")]
        port: u16,
        #[arg(long)]
        transport: Option<String>,
    },

    /// Open read-only live camera observe UI using discovered defaults
    Observe {
        #[arg(short, long)]
        rtsp: Option<String>,
        #[arg(short, long)]
        profile: Option<PathBuf>,
        #[arg(long, default_value = "9444")]
        port: u16,
        #[arg(long)]
        transport: Option<String>,
    },

    /// Read current camera/device state using discovered defaults
    Check {
        #[arg(short, long)]
        rtsp: Option<String>,
        #[arg(short, long)]
        profile: Option<PathBuf>,
        #[arg(long)]
        transport: Option<String>,
        #[arg(long, default_value = "false")]
        watch: bool,
    },

    /// Validate and run the discovered camera YAML with reports enabled
    Test {
        #[arg(short, long)]
        path: Option<PathBuf>,
        #[arg(short, long, default_value = "./output/camera-test")]
        output: PathBuf,
    },

    /// Show discovered camera defaults and short commands
    Doctor,
```

Add match arms under `Commands::Camera`:

```rust
            CameraCommands::Profile {
                rtsp,
                profile,
                port,
                transport,
            } => {
                let discovered = lumi_tester::camera::launcher::CameraLauncherConfig::discover(
                    &std::env::current_dir()?,
                )?;
                let rtsp = rtsp.or(discovered.rtsp).with_context(|| {
                    "Missing CAMERA_RTSP. Add it to .env or pass --rtsp rtsp://..."
                })?;
                let profile = profile
                    .or(discovered.profile)
                    .unwrap_or_else(|| PathBuf::from("camera-profile.json"));
                lumi_tester::camera::run_calibrate(rtsp, profile, port, transport, false).await?;
            }
            CameraCommands::Observe {
                rtsp,
                profile,
                port,
                transport,
            } => {
                let discovered = lumi_tester::camera::launcher::CameraLauncherConfig::discover(
                    &std::env::current_dir()?,
                )?;
                let rtsp = match rtsp {
                    Some(rtsp) => rtsp,
                    None => discovered.require_rtsp("observe")?,
                };
                let profile = match profile {
                    Some(profile) => profile,
                    None => discovered.require_profile("observe")?,
                };
                lumi_tester::camera::run_calibrate(rtsp, profile, port, transport, true).await?;
            }
            CameraCommands::Check {
                rtsp,
                profile,
                transport,
                watch,
            } => {
                let discovered = lumi_tester::camera::launcher::CameraLauncherConfig::discover(
                    &std::env::current_dir()?,
                )?;
                let profile = match profile {
                    Some(profile) => profile,
                    None => discovered.require_profile("check")?,
                };
                let rtsp = rtsp.or(discovered.rtsp);
                lumi_tester::camera::run_detect(rtsp, &profile, transport, watch).await?;
            }
            CameraCommands::Test { path, output } => {
                let discovered = lumi_tester::camera::launcher::CameraLauncherConfig::discover(
                    &std::env::current_dir()?,
                )?;
                let path = match path {
                    Some(path) => path,
                    None => discovered.require_test_yaml()?,
                };
                println!(
                    "{} Camera test shortcut: validate + run with report/snapshot/events",
                    "▶".green().bold()
                );
                let validation = validate_test_files(&path);
                print_validation_result(&validation, false)?;
                if !validation.valid {
                    anyhow::bail!("validation failed");
                }
                runner::run_tests(
                    &path,
                    "android",
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
            }
            CameraCommands::Doctor => {
                let discovered = lumi_tester::camera::launcher::CameraLauncherConfig::discover(
                    &std::env::current_dir()?,
                )?;
                println!("{}", discovered.render_doctor_summary());
            }
```

Add `use anyhow::Context;` near the top of `src/main.rs`.

- [ ] **Step 4: Run focused test**

Run:

```bash
cargo test camera_shortcuts_are_available_in_help
```

Expected: PASS.

- [ ] **Step 5: Run help smoke check**

Run:

```bash
cargo run -- camera --help
```

Expected output includes `profile`, `observe`, `check`, `test`, and `doctor`, plus existing `calibrate`, `snapshot`, `detect`.

---

### Task 3: Add Camera Failure Hints

**Files:**
- Modify: `src/camera/launcher.rs`
- Modify: `src/report/html.rs`
- Test: `src/camera/launcher.rs`, `src/report/html.rs`

- [ ] **Step 1: Write failing hint tests**

Add to `src/camera/launcher.rs` tests:

```rust
#[test]
fn builds_hint_for_unknown_camera_state_failure() {
    let error = "device check failed: button 'device_2.button_1' is 'UNKNOWN', expected 'BLUE'\ncamera evidence: output/run/camera_evidence/default_123\nstate timeline: output/run/camera_evidence/default_123/timeline.json";

    let hint = camera_failure_hint(error).unwrap();

    assert!(hint.contains("device_2.button_1"));
    assert!(hint.contains("UNKNOWN"));
    assert!(hint.contains("lumi-tester camera profile"));
    assert!(hint.contains("lumi-tester camera check"));
}
```

Add to `src/report/html.rs` tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_failure_hint_is_rendered_as_html() {
        let error = "device check failed: button 'device_2.button_1' is 'UNKNOWN', expected 'BLUE'\ncamera evidence: output/camera_evidence/default_123";

        let html = camera_hint_html(error);

        assert!(html.contains("Camera next steps"));
        assert!(html.contains("lumi-tester camera profile"));
        assert!(html.contains("device_2.button_1"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test camera_failure_hint
```

Expected: FAIL because hint helpers do not exist.

- [ ] **Step 3: Implement hint helper**

Add to `src/camera/launcher.rs`:

```rust
pub fn camera_failure_hint(error: &str) -> Option<String> {
    if !error.contains("camera evidence") && !error.contains("device check failed") {
        return None;
    }

    let region = error
        .split("button '")
        .nth(1)
        .and_then(|rest| rest.split('\'').next())
        .unwrap_or("the failed region");

    let actual = error
        .split(" is '")
        .nth(1)
        .and_then(|rest| rest.split('\'').next())
        .unwrap_or("UNKNOWN");

    Some(format!(
        "Camera next steps:\n  {region} is {actual}. Open the profile editor and re-check detection:\n    lumi-tester camera profile\n    lumi-tester camera check\n  If the profile is correct, confirm the physical device LED state matches the YAML expectation."
    ))
}
```

Add to `src/report/html.rs`:

```rust
fn camera_hint_html(error: &str) -> String {
    let Some(hint) = crate::camera::launcher::camera_failure_hint(error) else {
        return String::new();
    };

    format!(
        r#"<div class="camera-hint"><strong>Camera next steps</strong><pre>{}</pre></div>"#,
        html_escape(&hint)
    )
}
```

In failed command rendering, append the hint:

```rust
let error_html = match &cmd.status {
    CommandStatus::Failed { error } => {
        format!(
            r##"<div class="error-message">{}</div>{}"##,
            html_escape(error),
            camera_hint_html(error)
        )
    }
    _ => String::new(),
};
```

Add CSS inside `generate_html` style block:

```css
.camera-hint {
    background: rgba(59, 130, 246, 0.1);
    border: 1px solid rgba(59, 130, 246, 0.25);
    border-radius: 0.5rem;
    padding: 0.75rem;
    margin-top: 0.75rem;
    color: #bfdbfe;
    font-size: 0.8125rem;
}

.camera-hint pre {
    white-space: pre-wrap;
    margin-top: 0.5rem;
    font-family: 'JetBrains Mono', monospace;
}
```

- [ ] **Step 4: Run focused tests**

Run:

```bash
cargo test camera_failure_hint
```

Expected: PASS.

---

### Task 4: Add End-to-End CLI Usability Smoke Checks

**Files:**
- Modify: `src/main.rs`
- Test: `src/main.rs`

- [ ] **Step 1: Add focused tests for no-device commands**

Add to `src/main.rs` tests:

```rust
#[test]
fn camera_doctor_shortcut_parses_without_required_flags() {
    let cli = Cli::try_parse_from(["lumi-tester", "camera", "doctor"]).unwrap();

    match cli.command {
        Commands::Camera {
            command: CameraCommands::Doctor,
        } => {}
        _ => panic!("expected camera doctor command"),
    }
}

#[test]
fn camera_profile_shortcut_accepts_optional_overrides() {
    let cli = Cli::try_parse_from([
        "lumi-tester",
        "camera",
        "profile",
        "--rtsp",
        "rtsp://10.0.0.5/live",
        "--profile",
        "profiles/lab_switch4_camera.json",
    ])
    .unwrap();

    match cli.command {
        Commands::Camera {
            command:
                CameraCommands::Profile {
                    rtsp,
                    profile,
                    port,
                    ..
                },
        } => {
            assert_eq!(rtsp.as_deref(), Some("rtsp://10.0.0.5/live"));
            assert_eq!(profile.unwrap(), PathBuf::from("profiles/lab_switch4_camera.json"));
            assert_eq!(port, 9444);
        }
        _ => panic!("expected camera profile command"),
    }
}
```

- [ ] **Step 2: Run focused parser tests**

Run:

```bash
cargo test camera_
```

Expected: PASS.

- [ ] **Step 3: Run all main CLI unit tests**

Run:

```bash
cargo test --bin lumi-tester
```

Expected: PASS.

---

### Task 5: Verify Real Repo Defaults

**Files:**
- No production edits unless verification exposes a real gap.

- [ ] **Step 1: Run camera doctor in the Lumi Tester repo**

Run:

```bash
cargo run -- camera doctor
```

Expected:

- Prints whether `CAMERA_RTSP` was found.
- Prints the discovered camera profile path.
- Prints the discovered camera YAML path.
- Prints shortcut commands.
- Does not require a device or RTSP connection.

- [ ] **Step 2: Run help**

Run:

```bash
cargo run -- camera --help
```

Expected:

- Existing commands remain visible: `calibrate`, `snapshot`, `detect`.
- New shortcuts are visible: `profile`, `observe`, `check`, `test`, `doctor`.

- [ ] **Step 3: Run validation-only camera test path**

Run:

```bash
cargo run -- validate e2e/workspaces/lumi_life/camera_hardware_sample.yaml --json
```

Expected: `valid: true`.

- [ ] **Step 4: Optional hardware run**

Only run when RTSP/device lab is available:

```bash
cargo run -- camera test
```

Expected:

- It validates first.
- It runs with snapshot/report/events enabled.
- On camera assertion failure, HTML report contains the camera next-step hint.

---

## Phase 2: Web Launcher, After CLI Shortcuts Are Stable

Do not implement this until Tasks 1-5 are merged and tester feedback confirms the shortcuts solve the immediate problem.

### Task 6: Add `lumi-tester camera web`

**Files:**
- Create: `src/camera/launcher_server.rs`
- Modify: `src/camera/mod.rs`
- Modify: `src/main.rs`

Behavior:

- Start local web page at `http://localhost:9445`.
- Show discovered `.env`, profile, and camera YAML.
- Provide copyable commands and buttons for:
  - Open profile editor
  - Open observe mode
  - Run check
  - Run camera test
- Buttons may initially show copyable commands instead of spawning child commands. That avoids long-running process management inside the web server.

Testing:

- Unit-test the rendered HTML contains all shortcuts.
- Do not launch browser in tests.

---

## Self-Review

- Spec coverage: Covers shortcut commands, auto-discovery, no-memory workflow, failure guidance, and a later web launcher.
- Placeholder scan: Clear; no placeholder markers remain.
- Scope check: Phase 1 is one implementation slice and independently useful. Phase 2 is explicitly deferred.
- Type consistency: `CameraLauncherConfig` is defined before use; shortcut enum names match test snippets and match arms.
