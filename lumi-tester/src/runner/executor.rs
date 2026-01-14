use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;
use uuid::Uuid;

use super::context::TestContext;
use super::events::{ConsoleEventListener, EventEmitter, TestEvent};
use super::state::{CommandState, FlowState, TestSessionState};
use crate::driver::traits::PlatformDriver;
use crate::parser::types::TestCommand;
use crate::parser::yaml::{parse_commands_from_value, parse_test_file};
use serde_json;
use std::collections::HashMap;
use std::fs::File;

pub struct TestExecutor {
    driver: Box<dyn PlatformDriver>,
    context: TestContext,
    session: TestSessionState,
    emitter: EventEmitter,
    continue_on_failure: bool,
    /// GIF frames storage: name -> PNG bytes
    gif_frames: HashMap<String, Vec<u8>>,
    /// Auto-capture GIF state
    auto_capture_frames: Vec<Vec<u8>>,
    auto_capture_active: bool,
    auto_capture_interval: u64,
    auto_capture_max: u32,
    auto_capture_width: Option<u32>,
    auto_capture_last_time: std::time::Instant,
    depth: usize,
    target_tags: Option<Vec<String>>,
    soft_errors: Vec<String>,
    video_enabled: bool,
    snapshot_enabled: bool,
    report_enabled: bool,
}

impl TestExecutor {
    pub fn new(
        driver: Box<dyn PlatformDriver>,
        output_dir: Option<&Path>,
        continue_on_failure: bool,
        record: bool,
        snapshot: bool,
        report: bool,
        target_tags: Option<Vec<String>>,
    ) -> Self {
        let (emitter, receiver) = EventEmitter::new();
        let device_id = driver.device_serial();

        let context = TestContext::new(Path::new("."), output_dir, continue_on_failure, device_id);

        // Start console listener in background
        tokio::spawn(ConsoleEventListener::listen(receiver));

        Self {
            driver,
            context,
            session: TestSessionState::new(&Uuid::new_v4().to_string()),
            emitter,
            continue_on_failure,
            depth: 0,
            gif_frames: HashMap::new(),
            auto_capture_frames: Vec::new(),
            auto_capture_active: false,
            auto_capture_interval: 200,
            auto_capture_max: 150,
            auto_capture_width: None,
            auto_capture_last_time: std::time::Instant::now(),
            target_tags,
            soft_errors: Vec::new(),
            video_enabled: record,
            snapshot_enabled: snapshot,
            report_enabled: report,
        }
    }

    /// Subscribe to test execution events
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TestEvent> {
        self.emitter.subscribe()
    }

    /// Run a single test file
    pub async fn run_file(
        &mut self,
        path: &Path,
        command_index: Option<usize>,
        command_name: Option<&str>,
    ) -> Result<()> {
        // Update base directory for relative path resolution
        if let Some(parent) = path.parent() {
            self.context.base_dir = parent.to_path_buf();
        }

        // Parse the test file
        let flow = parse_test_file(path)?;

        // Filter by tags if specified
        if let Some(ref required_tags) = self.target_tags {
            let matches_all = required_tags.iter().all(|req| flow.tags.contains(req));
            if !matches_all {
                self.emitter.emit(TestEvent::Log {
                    message: format!(
                        "{} Skipping flow due to tag mismatch. Required: {:?}, Flow tags: {:?}",
                        "ℹ".blue(),
                        required_tags,
                        flow.tags
                    ),
                    depth: self.depth,
                });
                return Ok(());
            }
        }

        // Update context from flow header
        self.context.update_from_flow(&flow);

        // Handle DDT (CSV Data)
        let mut iterations = Vec::new();
        if let Some(ref data_file) = flow.data {
            let base_dir = path.parent().unwrap_or(Path::new("."));
            let data_path = base_dir.join(data_file);
            println!(
                "    {} Loading data from: {}",
                "ℹ".blue(),
                data_path.display()
            );

            let file = File::open(&data_path).context("Failed to open data file")?;
            let mut rdr = csv::Reader::from_reader(file);
            for result in rdr.deserialize() {
                let record: HashMap<String, String> =
                    result.context("Failed to parse CSV record")?;
                iterations.push(record);
            }
            self.emitter.emit(TestEvent::Log {
                message: format!("{} Loaded {} data rows", "ℹ".blue(), iterations.len()),
                depth: self.depth,
            });
        } else {
            iterations.push(HashMap::new());
        }

        let base_flow_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        for (iter_idx, vars) in iterations.iter().enumerate() {
            // Apply variables from data row
            for (k, v) in vars {
                self.context.vars.insert(k.clone(), v.clone());
            }

            let flow_name = if iterations.len() > 1 {
                format!("{} [{}]", base_flow_name, iter_idx + 1)
            } else {
                base_flow_name.clone()
            };

            // Filter commands if specified
            let commands_to_run = if let Some(idx) = command_index {
                if idx >= flow.commands.len() {
                    anyhow::bail!(
                        "Command index {} is out of range. File has {} commands.",
                        idx,
                        flow.commands.len()
                    );
                }
                vec![flow.commands[idx].clone()]
            } else if let Some(name) = command_name {
                let found = flow
                    .commands
                    .iter()
                    .find(|cmd| {
                        let cmd_name = cmd.display_name().to_lowercase();
                        cmd_name == name.to_lowercase()
                            || cmd_name.starts_with(&name.to_lowercase())
                    })
                    .cloned();
                match found {
                    Some(cmd) => vec![cmd],
                    None => {
                        anyhow::bail!(
                            "Command '{}' not found in file. Available commands: {}",
                            name,
                            flow.commands
                                .iter()
                                .map(|c| c.display_name())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }
                }
            } else {
                flow.commands.clone()
            };

            self.run_commands_set(&commands_to_run, &flow_name, &path.display().to_string())
                .await?;
        }

        Ok(())
    }

    /// Run a set of commands as a flow
    async fn run_commands_set(
        &mut self,
        commands: &[TestCommand],
        flow_name: &str,
        flow_path: &str,
    ) -> Result<()> {
        let command_states: Vec<CommandState> = commands
            .iter()
            .enumerate()
            .map(|(i, cmd)| CommandState::new(i, &cmd.display_name(), &cmd.display_name()))
            .collect();

        let mut flow_state = FlowState::new(flow_name, flow_path, command_states);

        // Emit flow started event
        self.emitter.emit(TestEvent::FlowStarted {
            flow_name: flow_name.to_string(),
            flow_path: flow_path.to_string(),
            command_count: commands.len(),
            depth: self.depth,
        });

        flow_state.start();

        // Video Recording Setup
        let video_active = self.video_enabled;
        let mut video_rel_path = None;

        if video_active {
            let out_dir = &self.context.output_dir;
            // Sanitize flow name safely
            let safe_name: String = flow_name
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect();

            let filename = format!(
                "video_{}_{}.mp4",
                safe_name,
                Uuid::new_v4()
                    .to_string()
                    .chars()
                    .take(8)
                    .collect::<String>()
            );

            let abs_path = out_dir.join(&filename);
            let abs_path_str = abs_path.to_string_lossy().to_string();
            video_rel_path = Some(filename);

            self.emitter.emit(TestEvent::Log {
                message: format!(
                    "{} Starting video recording: {}",
                    "🎥".blue(),
                    abs_path.display()
                ),
                depth: self.depth,
            });

            if let Err(e) = self.driver.start_recording(&abs_path_str).await {
                self.emitter.emit(TestEvent::Log {
                    message: format!("{} Failed to start recording: {}", "⚠️".yellow(), e),
                    depth: self.depth,
                });
                // Disable video for this flow if start failed
                video_rel_path = None;
            }
        }

        // Execute commands
        for (i, command) in commands.iter().enumerate() {
            if let Some(cmd_state) = flow_state.commands.get_mut(i) {
                cmd_state.start();

                self.emitter.emit(TestEvent::CommandStarted {
                    flow_name: flow_name.to_string(),
                    index: i,
                    command: command.display_name(),
                    depth: self.depth,
                });

                match self.execute_command(command).await {
                    Ok(()) => {
                        cmd_state.pass();
                        let duration = cmd_state.duration_ms.unwrap_or(0);

                        // Auto-capture GIF frame if active
                        if self.auto_capture_active {
                            self.try_auto_capture().await;
                        }

                        self.emitter.emit(TestEvent::CommandPassed {
                            flow_name: flow_name.to_string(),
                            index: i,
                            duration_ms: duration,
                            depth: self.depth,
                        });
                    }
                    Err(e) => {
                        let error_msg = e.to_string();

                        // Capture debug info
                        self.handle_failure(flow_name, i, &error_msg).await;

                        cmd_state.fail(error_msg.clone());
                        let duration = cmd_state.duration_ms.unwrap_or(0);

                        self.emitter.emit(TestEvent::CommandFailed {
                            flow_name: flow_name.to_string(),
                            index: i,
                            error: error_msg,
                            duration_ms: duration,
                            depth: self.depth,
                        });

                        if !self.continue_on_failure {
                            // Skip remaining commands
                            flow_state.skip_remaining("Previous command failed");
                            break;
                        }
                    }
                }
            }

            flow_state.current_index = i + 1;
        }

        flow_state.finish();

        if let Some(rel_path) = video_rel_path {
            if let Err(e) = self.driver.stop_recording().await {
                self.emitter.emit(TestEvent::Log {
                    message: format!("{} Failed to stop recording: {}", "⚠️".yellow(), e),
                    depth: self.depth,
                });
            } else {
                // Check if file exists (optional, driver should ensure)
                flow_state.video_path = Some(rel_path);
            }
        }

        let status = flow_state.status.clone();
        let total_duration_ms = flow_state.total_duration_ms;

        self.emitter.emit(TestEvent::FlowFinished {
            flow_name: flow_name.to_string(),
            status: status.clone(),
            duration_ms: total_duration_ms,
            depth: self.depth,
        });

        // Check for soft errors
        if !self.soft_errors.is_empty() {
            let error_msg = format!(
                "Flow completed with {} soft assertion failures:\n{}",
                self.soft_errors.len(),
                self.soft_errors.join("\n")
            );

            self.emitter.emit(TestEvent::Log {
                message: format!("{} {}", "❌".red(), error_msg),
                depth: self.depth,
            });

            // Clear errors for next flow? Or fail here?
            // Fail the flow status if soft errors exist
            flow_state.status = crate::runner::state::FlowStatus::Failed;
            flow_state.error = Some(error_msg.clone());

            // If we are failing, we should bail
            self.session.add_flow(flow_state);
            anyhow::bail!(error_msg);
        }

        self.session.add_flow(flow_state);

        if status == crate::runner::state::FlowStatus::Failed && !self.continue_on_failure {
            anyhow::bail!("Flow failed: {}", flow_name);
        }

        Ok(())
    }

    /// Handle assertion result with soft mode support
    fn handle_assertion(&mut self, result: Result<()>, soft: bool) -> Result<()> {
        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                if soft {
                    let msg = format!("Soft Assert Failed: {}", e);
                    self.soft_errors.push(msg.clone());
                    self.emitter.emit(TestEvent::Log {
                        message: format!("{} {}", "⚠️".yellow(), msg),
                        depth: self.depth,
                    });
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Execute a single command
    pub async fn execute_command(&mut self, command: &TestCommand) -> Result<()> {
        match command {
            TestCommand::LaunchApp(params_input) => {
                let params_struct = params_input.as_ref().map(|p| p.clone().into_inner());
                // For web platform, prefer URL from params, context.url, or app_id
                let raw_app_id = if self.driver.platform_name() == "web" {
                    params_struct
                        .as_ref()
                        .and_then(|p| p.app_id.as_ref())
                        .or(self.context.url.as_ref())
                        .or(self.context.app_id.as_ref())
                        .ok_or_else(|| {
                            anyhow::anyhow!("No URL or app ID specified for web platform")
                        })?
                } else {
                    params_struct
                        .as_ref()
                        .and_then(|p| p.app_id.as_ref())
                        .or(self.context.app_id.as_ref())
                        .ok_or_else(|| anyhow::anyhow!("No app ID specified"))?
                };

                let app_id = &self.context.substitute_vars(raw_app_id);

                let clear_state = params_struct
                    .as_ref()
                    .map(|p| p.clear_state)
                    .unwrap_or(false);
                let clear_keychain = params_struct
                    .as_ref()
                    .map(|p| p.clear_keychain)
                    .unwrap_or(false);
                let permissions = params_struct.as_ref().and_then(|p| p.permissions.as_ref());
                let stop_app = params_struct
                    .as_ref()
                    .and_then(|p| p.stop_app)
                    .unwrap_or(true);

                // Clear keychain if requested (iOS only)
                if clear_keychain {
                    self.driver.clear_keychain().await?;
                }

                // If clearState and permissions both exist, we need to:
                // 1. Clear state first (which resets permissions)
                // 2. Set permissions after clear but before launch
                // 3. Launch app without clearing state again
                if clear_state && permissions.is_some() {
                    // Clear app data first
                    self.driver.clear_app_data(app_id).await?;

                    // Set permissions after clear state
                    if let Some(perms) = permissions {
                        self.driver.set_permissions(app_id, perms).await?;
                    }

                    // Launch app without clearing state again
                    self.driver.launch_app(app_id, false).await
                } else {
                    // Normal flow: set permissions first (if any), then launch
                    if let Some(perms) = permissions {
                        self.driver.set_permissions(app_id, perms).await?;
                    }

                    // Stop app if requested and not clearing state (state clear usually enforces stop)
                    if stop_app && !clear_state {
                        self.driver.stop_app(app_id).await.ok();
                    }

                    self.driver.launch_app(app_id, clear_state).await
                }
            }

            TestCommand::StopApp => {
                let app_id = self.context.app_id.as_deref().unwrap_or("");
                self.driver.stop_app(app_id).await
            }

            TestCommand::OpenLink(url) => {
                let substituted_url = self.context.substitute_vars(url);
                self.driver
                    .open_link(&substituted_url, self.context.app_id.as_deref())
                    .await
            }

            TestCommand::TapOn(params_input) => {
                let params = params_input.clone().into_inner();
                // If point is specified, use TapAt
                if let Some(point_str) = &params.point {
                    let parts: Vec<&str> = point_str.split(',').collect();
                    if parts.len() == 2 {
                        // Parse point - supports both absolute "500,1000" and percentage "50%,80%"
                        let (screen_width, screen_height) = self.driver.get_screen_size().await?;

                        let x_str = parts[0].trim();
                        let y_str = parts[1].trim();

                        let x = if x_str.ends_with('%') {
                            let pct: f64 = x_str.trim_end_matches('%').parse().unwrap_or(0.0);
                            (screen_width as f64 * pct / 100.0) as i32
                        } else {
                            x_str.parse().unwrap_or(0)
                        };

                        let y = if y_str.ends_with('%') {
                            let pct: f64 = y_str.trim_end_matches('%').parse().unwrap_or(0.0);
                            (screen_height as f64 * pct / 100.0) as i32
                        } else {
                            y_str.parse().unwrap_or(0)
                        };

                        self.driver
                            .tap(&crate::driver::traits::Selector::Point { x, y })
                            .await
                    } else {
                        anyhow::bail!("Invalid point format: {}", point_str);
                    }
                } else {
                    // Merge relative aliases
                    let mut relative = params.relative.clone();
                    if params.right_of.is_some()
                        || params.left_of.is_some()
                        || params.above.is_some()
                        || params.below.is_some()
                    {
                        let mut r = relative.unwrap_or(crate::parser::types::RelativeParams {
                            right_of: None,
                            left_of: None,
                            above: None,
                            below: None,
                            max_dist: None,
                        });
                        if params.right_of.is_some() {
                            r.right_of = params.right_of.clone();
                        }
                        if params.left_of.is_some() {
                            r.left_of = params.left_of.clone();
                        }
                        if params.above.is_some() {
                            r.above = params.above.clone();
                        }
                        if params.below.is_some() {
                            r.below = params.below.clone();
                        }
                        relative = Some(r);
                    }

                    let mut selector = self
                        .build_selector(
                            &params.text,
                            &params.regex,
                            &params.id,
                            &relative,
                            &params.css,
                            &params.xpath,
                            &params.placeholder,
                            &params.role,
                            &params.element_type,
                            &params.image,
                            params.index,
                            params.exact,
                        )
                        .ok_or_else(|| anyhow::anyhow!("No selector specified for tapOn"))?;

                    // Inject imageRegion for Image selectors
                    if let crate::driver::traits::Selector::Image { ref mut region, .. } = selector
                    {
                        if params.image_region.is_some() {
                            *region = params.image_region.clone();
                        }
                    }

                    if params.optional {
                        if self.driver.is_visible(&selector).await? {
                            self.driver.tap(&selector).await
                        } else {
                            println!(
                                "  {} Optional element not found, skipping tap: {:?}",
                                "ℹ".blue(),
                                selector
                            );
                            Ok(())
                        }
                    } else {
                        self.driver.tap(&selector).await
                    }
                }
            }

            TestCommand::LongPressOn(params_input) => {
                let params = params_input.clone().into_inner();
                let selector = self
                    .build_selector(
                        &params.text,
                        &params.regex,
                        &params.id,
                        &params.relative,
                        &params.css,
                        &params.xpath,
                        &params.placeholder,
                        &params.role,
                        &params.element_type,
                        &params.image,
                        params.index,
                        params.exact,
                    )
                    .ok_or_else(|| anyhow::anyhow!("No selector specified for longPressOn"))?;
                let timeout = self.context.default_timeout_ms;
                if !matches!(selector, crate::driver::traits::Selector::Point { .. }) {
                    let _ = self.driver.wait_for_element(&selector, timeout).await;
                }
                self.driver.long_press(&selector, 1000).await
            }

            TestCommand::DoubleTapOn(params_input) => {
                let params = params_input.clone().into_inner();
                let selector = self
                    .build_selector(
                        &params.text,
                        &params.regex,
                        &params.id,
                        &params.relative,
                        &params.css,
                        &params.xpath,
                        &params.placeholder,
                        &params.role,
                        &params.element_type,
                        &params.image,
                        params.index,
                        params.exact,
                    )
                    .ok_or_else(|| anyhow::anyhow!("No selector specified for doubleTapOn"))?;
                let timeout = self.context.default_timeout_ms;
                if !matches!(selector, crate::driver::traits::Selector::Point { .. }) {
                    let _ = self.driver.wait_for_element(&selector, timeout).await;
                }
                self.driver.double_tap(&selector).await
            }

            TestCommand::RightClick(params) => {
                let selector = self
                    .build_selector(
                        &params.text,
                        &params.regex,
                        &params.id,
                        &params.relative,
                        &params.css,
                        &params.xpath,
                        &params.placeholder,
                        &params.role,
                        &params.element_type,
                        &None,
                        None,
                        false,
                    )
                    .ok_or_else(|| anyhow::anyhow!("No selector specified for rightClick"))?;
                let timeout = self.context.default_timeout_ms;
                if !matches!(selector, crate::driver::traits::Selector::Point { .. }) {
                    let _ = self.driver.wait_for_element(&selector, timeout).await;
                }
                self.driver.right_click(&selector).await
            }

            TestCommand::InputText(params_input) => {
                let text = params_input.text();
                let unicode = params_input.unicode();
                let substituted = self.context.substitute_vars(text);
                self.driver.input_text(&substituted, unicode).await
            }

            TestCommand::EraseText(params) => {
                let count = params.as_ref().and_then(|p| p.char_count);
                self.driver.erase_text(count).await
            }

            TestCommand::HideKeyboard => self.driver.hide_keyboard().await,

            TestCommand::SwipeLeft => {
                use crate::driver::traits::SwipeDirection;
                self.driver.swipe(SwipeDirection::Left, None, None).await
            }

            TestCommand::SwipeRight => {
                use crate::driver::traits::SwipeDirection;
                self.driver.swipe(SwipeDirection::Right, None, None).await
            }

            TestCommand::SwipeUp => {
                use crate::driver::traits::SwipeDirection;
                self.driver.swipe(SwipeDirection::Up, None, None).await
            }

            TestCommand::SwipeDown => {
                use crate::driver::traits::SwipeDirection;
                self.driver.swipe(SwipeDirection::Down, None, None).await
            }

            TestCommand::AssertVisible(params_input) => {
                let params = params_input.clone().into_inner();
                let verification_result = async {
                    // Merge relative aliases
                    let mut relative = params.relative.clone();
                    if params.right_of.is_some()
                        || params.left_of.is_some()
                        || params.above.is_some()
                        || params.below.is_some()
                    {
                        let mut r = relative.unwrap_or(crate::parser::types::RelativeParams {
                            right_of: None,
                            left_of: None,
                            above: None,
                            below: None,
                            max_dist: None,
                        });
                        if params.right_of.is_some() {
                            r.right_of = params.right_of.clone();
                        }
                        if params.left_of.is_some() {
                            r.left_of = params.left_of.clone();
                        }
                        if params.above.is_some() {
                            r.above = params.above.clone();
                        }
                        if params.below.is_some() {
                            r.below = params.below.clone();
                        }
                        relative = Some(r);
                    }

                    let mut selector = self
                        .build_selector(
                            &params.text,
                            &params.regex,
                            &params.id,
                            &relative,
                            &params.css,
                            &params.xpath,
                            &params.placeholder,
                            &params.role,
                            &params.element_type,
                            &params.image,
                            params.index,
                            false,
                        )
                        .ok_or_else(|| {
                            anyhow::anyhow!("No selector specified for assertVisible")
                        })?;

                    // Handle contains_child
                    if let Some(child_p) = &params.contains_child {
                        let child_params = &**child_p;
                        let child_sel = self
                            .build_selector(
                                &child_params.text,
                                &child_params.regex,
                                &child_params.id,
                                &child_params.relative,
                                &child_params.css,
                                &child_params.xpath,
                                &child_params.placeholder,
                                &child_params.role,
                                &child_params.element_type,
                                &child_params.image,
                                child_params.index,
                                false,
                            )
                            .ok_or(anyhow::anyhow!("Invalid child selector in containsChild"))?;

                        selector = crate::driver::traits::Selector::HasChild {
                            parent: Box::new(selector),
                            child: Box::new(child_sel),
                        };
                    }

                    let timeout = params.timeout.unwrap_or(5000);
                    let visible = self.driver.wait_for_element(&selector, timeout).await?;

                    if visible {
                        Ok(())
                    } else {
                        anyhow::bail!("Element not visible within {}ms: {:?}", timeout, selector)
                    }
                }
                .await;
                self.handle_assertion(verification_result, params.soft)
            }

            TestCommand::WaitUntilVisible(params_input) => {
                let params = params_input.clone().into_inner();
                // Identical logic to AssertVisible but semantically different
                // It's a wait command, but can be treated as an assertion that the element appears
                let verification_result = async {
                    // Merge relative aliases
                    let mut relative = params.relative.clone();
                    if params.right_of.is_some()
                        || params.left_of.is_some()
                        || params.above.is_some()
                        || params.below.is_some()
                    {
                        let mut r = relative.unwrap_or(crate::parser::types::RelativeParams {
                            right_of: None,
                            left_of: None,
                            above: None,
                            below: None,
                            max_dist: None,
                        });
                        if params.right_of.is_some() {
                            r.right_of = params.right_of.clone();
                        }
                        if params.left_of.is_some() {
                            r.left_of = params.left_of.clone();
                        }
                        if params.above.is_some() {
                            r.above = params.above.clone();
                        }
                        if params.below.is_some() {
                            r.below = params.below.clone();
                        }
                        relative = Some(r);
                    }

                    let mut selector = self
                        .build_selector(
                            &params.text,
                            &params.regex,
                            &params.id,
                            &relative,
                            &params.css,
                            &params.xpath,
                            &params.placeholder,
                            &params.role,
                            &params.element_type,
                            &params.image,
                            params.index,
                            false,
                        )
                        .ok_or_else(|| {
                            anyhow::anyhow!("No selector specified for waitUntilVisible")
                        })?;

                    // Handle contains_child
                    if let Some(child_p) = &params.contains_child {
                        let child_params = &**child_p;
                        let child_sel = self
                            .build_selector(
                                &child_params.text,
                                &child_params.regex,
                                &child_params.id,
                                &child_params.relative,
                                &child_params.css,
                                &child_params.xpath,
                                &child_params.placeholder,
                                &child_params.role,
                                &child_params.element_type,
                                &child_params.image,
                                child_params.index,
                                false,
                            )
                            .ok_or(anyhow::anyhow!("Invalid child selector in containsChild"))?;

                        selector = crate::driver::traits::Selector::HasChild {
                            parent: Box::new(selector),
                            child: Box::new(child_sel),
                        };
                    }

                    // Default timeout for wait is usually higher or same as assertion?
                    // Using context default timeout (default: 10s)
                    let timeout = params.timeout.unwrap_or(self.context.default_timeout_ms);
                    let visible = self.driver.wait_for_element(&selector, timeout).await?;

                    if visible {
                        Ok(())
                    } else {
                        anyhow::bail!("Element not visible within {}ms: {:?}", timeout, selector)
                    }
                }
                .await;
                // Wait command is effectively a hard assertion (it fails if not found)
                // But we support soft mode if user really wants to continue
                self.handle_assertion(verification_result, params.soft)
            }

            TestCommand::AssertNotVisible(params_input) => {
                let params = params_input.clone().into_inner();
                let verification_result = async {
                    // Merge relative aliases
                    let mut relative = params.relative.clone();
                    if params.right_of.is_some()
                        || params.left_of.is_some()
                        || params.above.is_some()
                        || params.below.is_some()
                    {
                        let mut r = relative.unwrap_or(crate::parser::types::RelativeParams {
                            right_of: None,
                            left_of: None,
                            above: None,
                            below: None,
                            max_dist: None,
                        });
                        if params.right_of.is_some() {
                            r.right_of = params.right_of.clone();
                        }
                        if params.left_of.is_some() {
                            r.left_of = params.left_of.clone();
                        }
                        if params.above.is_some() {
                            r.above = params.above.clone();
                        }
                        if params.below.is_some() {
                            r.below = params.below.clone();
                        }
                        relative = Some(r);
                    }

                    let mut selector = self
                        .build_selector(
                            &params.text,
                            &params.regex,
                            &params.id,
                            &relative,
                            &params.css,
                            &params.xpath,
                            &params.placeholder,
                            &params.role,
                            &params.element_type,
                            &params.image,
                            params.index,
                            false,
                        )
                        .ok_or_else(|| {
                            anyhow::anyhow!("No selector specified for assertNotVisible")
                        })?;

                    if let Some(child_p) = &params.contains_child {
                        let child_params = &**child_p;
                        let child_sel = self
                            .build_selector(
                                &child_params.text,
                                &child_params.regex,
                                &child_params.id,
                                &child_params.relative,
                                &child_params.css,
                                &child_params.xpath,
                                &child_params.placeholder,
                                &child_params.role,
                                &child_params.element_type,
                                &child_params.image,
                                child_params.index,
                                false,
                            )
                            .ok_or(anyhow::anyhow!("Invalid child selector"))?;
                        selector = crate::driver::traits::Selector::HasChild {
                            parent: Box::new(selector),
                            child: Box::new(child_sel),
                        };
                    }

                    let visible = self.driver.is_visible(&selector).await?;

                    if !visible {
                        Ok(())
                    } else {
                        anyhow::bail!("Element is visible but should not be: {:?}", selector)
                    }
                }
                .await;
                self.handle_assertion(verification_result, params.soft)
            }

            TestCommand::WaitUntilNotVisible(params_input) => {
                let params = params_input.clone().into_inner();

                // Merge relative aliases
                let mut relative = params.relative.clone();
                if params.right_of.is_some()
                    || params.left_of.is_some()
                    || params.above.is_some()
                    || params.below.is_some()
                {
                    let mut r = relative.unwrap_or(crate::parser::types::RelativeParams {
                        right_of: None,
                        left_of: None,
                        above: None,
                        below: None,
                        max_dist: None,
                    });
                    if params.right_of.is_some() {
                        r.right_of = params.right_of.clone();
                    }
                    if params.left_of.is_some() {
                        r.left_of = params.left_of.clone();
                    }
                    if params.above.is_some() {
                        r.above = params.above.clone();
                    }
                    if params.below.is_some() {
                        r.below = params.below.clone();
                    }
                    relative = Some(r);
                }

                let mut selector = self
                    .build_selector(
                        &params.text,
                        &params.regex,
                        &params.id,
                        &relative,
                        &params.css,
                        &params.xpath,
                        &params.placeholder,
                        &params.role,
                        &params.element_type,
                        &params.image,
                        params.index,
                        false,
                    )
                    .ok_or_else(|| {
                        anyhow::anyhow!("No selector specified for waitUntilNotVisible")
                    })?;

                if let Some(child_p) = &params.contains_child {
                    let child_params = &**child_p;
                    let child_sel = self
                        .build_selector(
                            &child_params.text,
                            &child_params.regex,
                            &child_params.id,
                            &child_params.relative,
                            &child_params.css,
                            &child_params.xpath,
                            &child_params.placeholder,
                            &child_params.role,
                            &child_params.element_type,
                            &child_params.image,
                            child_params.index,
                            false,
                        )
                        .ok_or(anyhow::anyhow!("Invalid child selector"))?;
                    selector = crate::driver::traits::Selector::HasChild {
                        parent: Box::new(selector),
                        child: Box::new(child_sel),
                    };
                }

                let timeout = params.timeout.unwrap_or(self.context.default_timeout_ms);
                let ok = self.driver.wait_for_absence(&selector, timeout).await?;

                if ok {
                    Ok(())
                } else {
                    anyhow::bail!(
                        "Element failed to disappear within {}ms: {:?}",
                        timeout,
                        selector
                    )
                }
            }

            TestCommand::WaitForAnimationToEnd => {
                // Wait a fixed amount of time for animations
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                Ok(())
            }

            TestCommand::Wait(params_input) => {
                let params = params_input.clone().into_inner();
                tokio::time::sleep(tokio::time::Duration::from_millis(params.ms)).await;
                Ok(())
            }

            TestCommand::TakeScreenshot(params_input) => {
                let params = params_input.clone().into_inner();
                let path = params.path.clone();
                let output_path = self.context.output_path(&path);
                self.driver
                    .take_screenshot(output_path.to_str().unwrap())
                    .await
            }

            TestCommand::AssertScreenshot(name) => {
                let filename = if name.ends_with(".png") {
                    name.clone()
                } else {
                    format!("{}.png", name)
                };
                let reference_path = self
                    .context
                    .resolve_path(&format!("screenshots/{}", filename));

                if !reference_path.exists() {
                    anyhow::bail!(
                        "Reference screenshot not found: {}",
                        reference_path.display()
                    );
                }

                let diff = self.driver.compare_screenshot(&reference_path, 1.0).await?;
                if diff > 1.0 {
                    // Default 1% tolerance
                    anyhow::bail!("Visual regression detected! Difference: {:.2}%", diff);
                } else {
                    println!(
                        "  {} Visual check passed (diff: {:.2}%)",
                        "✨".green(),
                        diff
                    );
                    Ok(())
                }
            }

            TestCommand::StartRecording(params_input) => {
                let params = params_input.clone().into_inner();
                let path = self.context.output_path(&params.path);
                self.driver
                    .start_recording(&path.display().to_string())
                    .await
            }

            TestCommand::StopRecording => self.driver.stop_recording().await,

            TestCommand::Back => self.driver.back().await,

            TestCommand::PressHome => self.driver.home().await,

            TestCommand::RunFlow(params_input) => {
                let params = params_input.clone().into_inner();

                // Check 'when' condition
                if let Some(condition) = &params.when {
                    if !self.evaluate_condition_value(condition).await {
                        if let Some(label) = &params.label {
                            self.emitter.emit(TestEvent::Log {
                                message: format!(
                                    "{} Skipped flow '{}': condition false",
                                    "⏭".blue(),
                                    label
                                ),
                                depth: self.depth,
                            });
                        }
                        return Ok(());
                    }
                }

                // Determine commands to run
                let commands_to_run = if let Some(cmds) = &params.commands {
                    Some(cmds.clone())
                } else if let Some(ref path_str) = params.path {
                    let flow_path = self.context.resolve_path(path_str);
                    let sub_flow = parse_test_file(&flow_path)?;
                    Some(sub_flow.commands)
                } else {
                    None
                };

                if let Some(cmds) = commands_to_run {
                    // Merge variables
                    if let Some(ref vars) = params.vars {
                        self.context.merge_vars(vars);
                    }

                    self.depth += 1;
                    let flow_name = params.label.clone().unwrap_or_else(|| {
                        params.path.clone().unwrap_or_else(|| "subflow".to_string())
                    });
                    let flow_path = params.path.clone().unwrap_or_default();

                    let res = Box::pin(self.run_commands_set(&cmds, &flow_name, &flow_path)).await;
                    self.depth -= 1;

                    if let Err(e) = res {
                        if params.optional.unwrap_or(false) {
                            self.emitter.emit(TestEvent::Log {
                                message: format!(
                                    "{} Optional Flow failed (ignored): {}",
                                    "ℹ".blue(),
                                    e
                                ),
                                depth: self.depth,
                            });
                            return Ok(());
                        }
                        anyhow::bail!("Flow failed: {}", e);
                    }
                }
                Ok(())
            }

            // TapAt - tap element by type and index
            TestCommand::TapAt(params) => {
                self.driver
                    .tap_by_type_index(&params.element_type, params.index)
                    .await
            }

            // InputAt - input text at element by type and index
            TestCommand::InputAt(params) => {
                let text = self.context.substitute_vars(&params.text);
                self.driver
                    .input_by_type_index(&params.element_type, params.index, &text)
                    .await
            }

            // SetVar - set a variable
            TestCommand::SetVar(params) => {
                self.context.set_var(&params.name, &params.value);
                Ok(())
            }

            // AssertVar - assert variable has expected value
            TestCommand::AssertVar(params) => {
                let expected = self.context.substitute_vars(&params.expected);
                let actual = self.context.get_var(&params.name).unwrap_or_default();
                if actual == expected {
                    Ok(())
                } else {
                    anyhow::bail!(
                        "Variable {} expected '{}', got '{}'",
                        params.name,
                        expected,
                        actual
                    )
                }
            }

            // Repeat - repeat commands N times or while condition matches
            TestCommand::Repeat(params) => {
                let mut iteration = 0;
                loop {
                    iteration += 1;

                    // Check 'times' condition
                    if let Some(times) = params.times {
                        if iteration > times {
                            break;
                        }
                    }

                    // Check 'while' condition
                    if let Some(ref condition) = params.while_condition {
                        if !self.evaluate_condition_value(condition).await {
                            break;
                        }
                    }

                    if params.times.is_none() && params.while_condition.is_none() {
                        // Avoid infinite loop if no condition
                        break;
                    }

                    let label = format!("Repeat #{}", iteration);
                    self.depth += 1;
                    let res =
                        Box::pin(self.run_commands_set(&params.commands, &label, "repeat")).await;
                    self.depth -= 1;
                    res?;

                    // Safety break for extremely large repeats
                    if iteration > 1000 {
                        anyhow::bail!("Repeat limit reached (1000 iterations)");
                    }
                }
                Ok(())
            }

            // Retry - retry commands on failure
            TestCommand::Retry(params) => {
                let mut last_error = None;
                for attempt in 0..params.max_retries {
                    let label = format!("Retry attempt #{}", attempt + 1);
                    self.depth += 1;
                    let res =
                        Box::pin(self.run_commands_set(&params.commands, &label, "retry")).await;
                    self.depth -= 1;

                    match res {
                        Ok(()) => return Ok(()),
                        Err(e) => {
                            last_error = Some(e);
                            if attempt < params.max_retries - 1 {
                                self.emitter.emit(TestEvent::Log {
                                    message: format!(
                                        "{} Attempt {} failed, retrying...",
                                        "⚠️".yellow(),
                                        attempt + 1
                                    ),
                                    depth: self.depth,
                                });
                            }
                        }
                    }
                }
                anyhow::bail!(
                    "Retry failed after {} attempts. Last error: {}",
                    params.max_retries,
                    last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error"))
                )
            }

            // ScrollUntilVisible
            TestCommand::ScrollUntilVisible(params_input) => {
                use crate::driver::traits::SwipeDirection;

                let params = params_input.clone().into_inner();
                // Scroll commands in parsing don't support index yet, default to None
                let selector = self
                    .build_selector(
                        &params.text,
                        &params.regex,
                        &params.id,
                        &params.relative,
                        &params.css,
                        &params.xpath,
                        &params.placeholder,
                        &params.role,
                        &params.element_type,
                        &params.image,
                        None,
                        false,
                    )
                    .ok_or_else(|| {
                        anyhow::anyhow!("No selector specified for scrollUntilVisible")
                    })?;

                // Parse direction: "up" = swipe up (scroll content down), "down" = swipe down (scroll content up)
                let direction = params.direction.as_ref().map(|d| {
                    match d.to_lowercase().as_str() {
                        "up" => SwipeDirection::Down, // To scroll content UP, we swipe DOWN
                        "down" => SwipeDirection::Up, // To scroll content DOWN, we swipe UP
                        "left" => SwipeDirection::Right,
                        "right" => SwipeDirection::Left,
                        _ => SwipeDirection::Up, // Default
                    }
                });

                let from_selector = if let Some(ref from) = params.from {
                    self.build_selector(
                        &from.text,
                        &from.regex,
                        &from.id,
                        &from.relative,
                        &from.css,
                        &from.xpath,
                        &from.placeholder,
                        &from.role,
                        &from.element_type,
                        &from.image,
                        from.index,
                        from.exact,
                    )
                } else {
                    None
                };

                let found = self
                    .driver
                    .scroll_until_visible(&selector, params.max_scrolls, direction, from_selector)
                    .await?;

                if found {
                    Ok(())
                } else {
                    anyhow::bail!("Element not found after scrolling: {:?}", selector)
                }
            }

            // Conditional Logic
            TestCommand::Conditional(params) => {
                let condition_met = self.check_condition(&params.condition).await;

                let commands_val = if condition_met {
                    Some(&params.then)
                } else {
                    params.else_cmd.as_ref()
                };

                if let Some(val) = commands_val {
                    let cmds = parse_commands_from_value(val)?;
                    self.emitter.emit(TestEvent::Log {
                        message: format!(
                            "{} Condition met: {}, Running {} nested commands...",
                            "ℹ".blue(),
                            condition_met,
                            cmds.len()
                        ),
                        depth: self.depth,
                    });

                    for cmd in cmds {
                        Box::pin(self.execute_command(&cmd)).await?;
                    }
                }
                Ok(())
            }

            // Generate Mock Data
            TestCommand::Generate(params) => {
                use fake::faker::address::en::CityName;
                use fake::faker::internet::en::SafeEmail;
                use fake::faker::name::en::{FirstName, Name};
                use fake::faker::phone_number::en::PhoneNumber;
                use fake::Fake;
                use rand::Rng;

                let value = match params.data_type.to_lowercase().as_str() {
                    "uuid" => Uuid::new_v4().to_string(),
                    "email" | "safeemail" => SafeEmail().fake(),
                    "name" | "fullname" => Name().fake(),
                    "firstname" => FirstName().fake(),
                    "phone" | "phonenumber" => PhoneNumber().fake(),
                    "city" | "address" => CityName().fake(), // Simple city mostly
                    "number" => {
                        let mut rng = rand::thread_rng();
                        let (min, max) = if let Some(schema) = &params.format {
                            let parts: Vec<&str> = schema.split('-').collect();
                            if parts.len() == 2 {
                                (
                                    parts[0].parse().unwrap_or(0),
                                    parts[1].parse().unwrap_or(100),
                                )
                            } else {
                                (0, 100)
                            }
                        } else {
                            (0, 100)
                        };
                        rng.gen_range(min..=max).to_string()
                    }
                    _ => "unknown".to_string(),
                };
                self.context.set_var(&params.name, &value);
                Ok(())
            }

            // Run Shell Script
            TestCommand::RunScript(params_input) => {
                let params = params_input.clone().into_inner();
                let cmd_str = self.context.substitute_vars(&params.command);

                if cmd_str.trim().ends_with(".js") {
                    let script_path = self.context.resolve_path(&cmd_str);
                    if script_path.exists() {
                        let script_content = std::fs::read_to_string(&script_path)
                            .map_err(|e| anyhow::anyhow!("Failed to read JS file: {}", e))?;

                        use super::js_engine::JsEngine;
                        let mut engine = JsEngine::new();

                        // Set current context variables
                        engine.set_vars(&self.context.vars);

                        // Execute script
                        match engine.execute_script_with_output(&script_content) {
                            Ok(output_json) => {
                                // Update 'output' variable in context
                                self.context.set_var("output", &output_json);
                                self.emitter.emit(TestEvent::Log {
                                    message: format!(
                                        "{} Executed JS script: {}",
                                        "✓".green(),
                                        cmd_str
                                    ),
                                    depth: self.depth,
                                });
                            }
                            Err(e) => {
                                if params.fail_on_error {
                                    anyhow::bail!("JS Script execution failed: {}", e);
                                } else {
                                    println!(
                                        "  {} JS Script execution failed: {}",
                                        "⚠️".yellow(),
                                        e
                                    );
                                }
                            }
                        }

                        return Ok(());
                    }
                }

                let mut cmd = std::process::Command::new("sh");
                cmd.arg("-c").arg(&cmd_str);

                let output = cmd.output()?;

                if !output.status.success() && params.fail_on_error {
                    anyhow::bail!("Script failed: {}", String::from_utf8_lossy(&output.stderr));
                }

                if let Some(var_name) = &params.save_output {
                    let out_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    self.context.set_var(var_name, &out_str);
                }
                Ok(())
            }

            // HTTP Request (Simplified)
            TestCommand::HttpRequest(params) => {
                let url = self.context.substitute_vars(&params.url);
                let client = reqwest::Client::new();
                let method = params
                    .method
                    .parse::<reqwest::Method>()
                    .map_err(|_| anyhow::anyhow!("Invalid HTTP method"))?;

                let mut req = client.request(method, &url);

                if let Some(headers) = &params.headers {
                    for (k, v) in headers {
                        req = req.header(k, self.context.substitute_vars(v));
                    }
                }

                if let Some(body) = &params.body {
                    let body_str = match body {
                        serde_yaml::Value::String(s) => self.context.substitute_vars(s),
                        _ => {
                            let json_str = serde_json::to_string(body).unwrap_or_default();
                            self.context.substitute_vars(&json_str)
                        }
                    };
                    req = req.body(body_str);
                }

                let res = req.send().await?;
                let status = res.status();

                if !status.is_success() {
                    // Can allow failure but log warning
                    println!("  {} HTTP Request failed: {}", "⚠".yellow(), status);
                }

                if let Some(save_map) = &params.save_response {
                    let json: serde_json::Value = res.json().await?;
                    for (var_name, json_path) in save_map {
                        let val_to_save = if json_path == "$" || json_path == "." {
                            json.to_string()
                        } else {
                            // Convert dot path "data.token" to pointer "/data/token"
                            let pointer = if json_path.starts_with('/') {
                                json_path.clone()
                            } else {
                                format!("/{}", json_path.replace('.', "/"))
                            };

                            if let Some(val) = json.pointer(&pointer) {
                                if let Some(s) = val.as_str() {
                                    s.to_string()
                                } else {
                                    val.to_string()
                                }
                            } else if let Some(val) = json.get(json_path) {
                                // Fallback: try simple key access
                                if let Some(s) = val.as_str() {
                                    s.to_string()
                                } else {
                                    val.to_string()
                                }
                            } else {
                                println!(
                                    "  {} Warning: JSON path '{}' not found in response",
                                    "⚠".yellow(),
                                    json_path
                                );
                                continue;
                            }
                        };

                        self.context.set_var(var_name, &val_to_save);
                    }
                }
                Ok(())
            }

            // GPS Mock Location
            TestCommand::MockLocation(p_input) => {
                let p = p_input.clone().into_inner();
                let file_path = self.context.resolve_path(&p.file);

                let content = std::fs::read_to_string(&file_path)
                    .context(format!("Failed to read GPS file: {}", file_path.display()))?;

                // Auto-detect format by extension
                let extension = file_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("gpx");

                let mut points = crate::parser::gps::parse_gps_file(&content, extension)?;

                // Apply start_index if specified
                if let Some(start_idx) = p.start_index {
                    if (start_idx as usize) < points.len() {
                        points = points.split_off(start_idx as usize);
                    }
                }

                let interval_ms = p.interval_ms.unwrap_or(1000);

                println!(
                    "  {} Loaded {} GPS points from {}",
                    "📍".green(),
                    points.len(),
                    file_path.file_name().unwrap_or_default().to_string_lossy()
                );

                self.driver
                    .start_mock_location(
                        p.name,
                        points,
                        p.speed,
                        p.speed_mode,
                        p.speed_noise,
                        interval_ms,
                        p.loop_route,
                    )
                    .await?;

                Ok(())
            }

            TestCommand::StopMockLocation => {
                self.driver.stop_mock_location().await?;
                Ok(())
            }

            // Visual Assertions - AssertColor
            TestCommand::AssertColor(params) => {
                use crate::parser::types::AssertColorParams;

                // Get screen size for percentage calculation
                let (screen_width, screen_height) = self.driver.get_screen_size().await?;

                // Parse point (supports "540,960" or "50%,50%")
                let (x, y) = params
                    .parse_point(screen_width, screen_height)
                    .ok_or_else(|| anyhow::anyhow!("Invalid point format: {}", params.point))?;

                // Parse expected color
                let expected_color = params
                    .parse_color()
                    .ok_or_else(|| anyhow::anyhow!("Invalid color format: {}", params.color))?;

                // Get actual color from screen
                let actual_color = self.driver.get_pixel_color(x, y).await?;

                // Calculate color distance
                let distance = AssertColorParams::color_distance(expected_color, actual_color);

                if distance <= params.tolerance {
                    println!("  {} Color match at ({},{}) - expected: #{:02X}{:02X}{:02X}, actual: #{:02X}{:02X}{:02X} (diff: {:.1}%)",
                        "✓".green(),
                        x, y,
                        expected_color.0, expected_color.1, expected_color.2,
                        actual_color.0, actual_color.1, actual_color.2,
                        distance
                    );
                    Ok(())
                } else {
                    anyhow::bail!(
                        "Color mismatch at ({},{}) - expected: #{:02X}{:02X}{:02X} ({}), actual: #{:02X}{:02X}{:02X}, diff: {:.1}% (tolerance: {:.1}%)",
                        x, y,
                        expected_color.0, expected_color.1, expected_color.2,
                        params.color,
                        actual_color.0, actual_color.1, actual_color.2,
                        distance,
                        params.tolerance
                    )
                }
            }

            // New Commands
            TestCommand::RotateScreen(params_input) => {
                let params = params_input.clone().into_inner();
                // Deprecated: use SetOrientation
                self.driver.rotate_screen(&params.mode).await
            }

            TestCommand::SetOrientation(params) => {
                self.driver.set_orientation(params.mode.clone()).await
            }

            TestCommand::SetNetwork(params) => {
                self.driver
                    .set_network_connection(params.wifi, params.data)
                    .await
            }

            TestCommand::ToggleAirplaneMode => self.driver.toggle_airplane_mode().await,

            TestCommand::OpenNotifications => self.driver.open_notifications().await,

            TestCommand::OpenQuickSettings => self.driver.open_quick_settings().await,

            TestCommand::SetVolume(level) => self.driver.set_volume(*level).await,

            TestCommand::LockDevice => self.driver.lock_device().await,

            TestCommand::UnlockDevice => self.driver.unlock_device().await,

            TestCommand::InstallApp(path) => {
                let resolved_path = self.context.resolve_path(path);
                self.driver
                    .install_app(resolved_path.to_str().unwrap())
                    .await
            }

            TestCommand::UninstallApp(pkg) => self.driver.uninstall_app(pkg).await,

            TestCommand::BackgroundApp(params) => {
                let app_id = params.app_id.as_deref().or(self.context.app_id.as_deref());
                self.driver.background_app(app_id, params.duration_ms).await
            }

            TestCommand::PressKey(key) => self.driver.press_key(key).await,

            TestCommand::PushFile(params) => {
                let source = self.context.resolve_path(&params.source);
                if !source.exists() {
                    anyhow::bail!("Source file not found: {}", source.display());
                }
                self.driver
                    .push_file(source.to_str().unwrap(), &params.destination)
                    .await
            }

            TestCommand::PullFile(params) => {
                let dest = self.context.output_path(&params.destination);
                self.driver
                    .pull_file(&params.source, dest.to_str().unwrap())
                    .await
            }

            TestCommand::ClearAppData(app_id) => self.driver.clear_app_data(app_id).await,

            TestCommand::SetClipboard(text) => {
                let content = self.context.substitute_vars(text);
                self.driver.set_clipboard(&content).await
            }

            TestCommand::GetClipboard(params) => match self.driver.get_clipboard().await {
                Ok(content) => {
                    self.context.set_var(&params.name, &content);
                    Ok(())
                }
                Err(e) => {
                    println!(
                        "  {} GetClipboard failed (platform limitation?): {}",
                        "⚠️".yellow(),
                        e
                    );
                    Ok(())
                }
            },

            TestCommand::AssertClipboard(expected) => {
                let expected_text = self.context.substitute_vars(expected);
                let actual = self.driver.get_clipboard().await?;
                if actual == expected_text {
                    Ok(())
                } else {
                    anyhow::bail!(
                        "Clipboard content mismatch. Expected: '{}', Got: '{}'",
                        expected_text,
                        actual
                    )
                }
            }

            TestCommand::AssertTrue(params) => {
                use super::js_engine::JsEngine;
                use crate::parser::types::AssertTrueParams;

                let (condition_str, soft) = match params {
                    AssertTrueParams::Condition(c) => (c.condition.clone(), c.soft),
                    AssertTrueParams::Expression(expr) => (expr.clone(), false),
                };

                let result = {
                    // Substitute variables first
                    let substituted = self.context.substitute_vars(&condition_str);

                    // Create JS engine with current context variables
                    let mut engine = JsEngine::new();
                    engine.set_vars(&self.context.vars);
                    engine.set_vars(&self.context.env);

                    // Evaluate the boolean expression
                    match engine.eval_bool(&substituted) {
                        Ok(true) => Ok(()),
                        Ok(false) => Err(anyhow::anyhow!(
                            "Assertion failed: {} evaluated to false",
                            condition_str
                        )),
                        Err(e) => Err(anyhow::anyhow!(
                            "Assertion error: {} - {}",
                            condition_str,
                            e
                        )),
                    }
                };

                self.handle_assertion(result, soft)
            }

            TestCommand::EvalScript(expr) => {
                use super::js_engine::JsEngine;

                // Create a new JS engine and load current variables
                let mut engine = JsEngine::new();
                engine.set_vars(&self.context.vars);
                engine.set_vars(&self.context.env);

                // Substitute variables first for ${var} syntax
                let substituted = self.context.substitute_vars(expr);

                // Evaluate the JavaScript expression
                match engine.eval_assignment(&substituted) {
                    Ok(Some((var_name, value))) => {
                        // Assignment expression - save the result
                        self.context.set_var(&var_name, &value);
                        println!("  {} evalScript: {} = {}", "📝".blue(), var_name, value);
                    }
                    Ok(None) => {
                        // Non-assignment expression, just evaluate
                        if let Ok(result) = engine.eval(&substituted) {
                            println!(
                                "  {} evalScript: {} => {}",
                                "📝".blue(),
                                substituted,
                                result
                            );
                        }
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("evalScript error: {}", e));
                    }
                }

                Ok(())
            }

            TestCommand::CopyTextFrom(params) => {
                let selector = self.build_selector(
                    &params.text,
                    &None, // regex
                    &params.id,
                    &None, // relative
                    &None, // css
                    &None, // xpath
                    &None, // placeholder
                    &None, // role
                    &None, // element_type
                    &None, // image
                    params.index.map(|i| i as u32),
                    false,
                );

                if let Some(sel) = selector {
                    match self.driver.get_element_text(&sel).await {
                        Ok(text) => {
                            self.context.set_var("nl.copiedText", &text);
                            println!("  {} Copied text: '{}'", "📝".blue(), text);
                        }
                        Err(e) => {
                            println!("  {} Failed to extract text: {}", "⚠️".yellow(), e);
                            // Fallback mock if needed for specific tests
                            if let Some(fallback) = &params.text {
                                self.context.set_var("nl.copiedText", fallback);
                            }
                        }
                    }
                }
                Ok(())
            }

            TestCommand::PasteText => {
                // Get copied text and input it
                if let Some(copied) = self.context.get_var("nl.copiedText") {
                    self.driver.input_text(&copied, false).await?;
                }
                Ok(())
            }

            TestCommand::InputRandomEmail => {
                let email = {
                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    let random_part: String = (0..8)
                        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
                        .collect();
                    format!("{}@test.com", random_part.to_lowercase())
                };
                self.driver.input_text(&email, false).await?;
                Ok(())
            }

            TestCommand::InputRandomNumber(params) => {
                let number = {
                    use rand::Rng;
                    let length = params.as_ref().and_then(|p| p.length).unwrap_or(10) as usize;
                    let mut rng = rand::thread_rng();
                    (0..length)
                        .map(|_| rng.gen_range(0..10).to_string())
                        .collect::<String>()
                };
                self.driver.input_text(&number, false).await?;
                Ok(())
            }

            TestCommand::InputRandomPersonName => {
                let name = {
                    use rand::seq::SliceRandom;
                    let first_names = [
                        "John", "Jane", "Alice", "Bob", "Charlie", "Diana", "Eve", "Frank",
                    ];
                    let last_names = [
                        "Smith", "Johnson", "Williams", "Brown", "Jones", "Davis", "Miller",
                        "Wilson",
                    ];
                    let mut rng = rand::thread_rng();
                    let first = first_names.choose(&mut rng).unwrap_or(&"John");
                    let last = last_names.choose(&mut rng).unwrap_or(&"Doe");
                    format!("{} {}", first, last)
                };
                self.driver.input_text(&name, false).await?;
                Ok(())
            }

            TestCommand::InputRandomText(params) => {
                let text = {
                    use rand::Rng;
                    let length = params.as_ref().and_then(|p| p.length).unwrap_or(10) as usize;
                    let mut rng = rand::thread_rng();
                    (0..length)
                        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
                        .collect::<String>()
                };
                self.driver.input_text(&text, false).await?;
                Ok(())
            }

            TestCommand::ExtendedWaitUntil(params) => {
                // Wait with custom timeout for visible/notVisible conditions
                let timeout_ms = params.timeout;

                if let Some(visible_val) = &params.visible {
                    // Parse the visible condition from serde_json::Value
                    if let Some(obj) = visible_val.as_object() {
                        if let Some(text_val) = obj.get("text") {
                            if let Some(text) = text_val.as_str() {
                                let selector = crate::driver::traits::Selector::Text(
                                    text.to_string(),
                                    0,
                                    false,
                                );
                                self.driver.wait_for_element(&selector, timeout_ms).await?;
                            }
                        }
                    }
                }

                if let Some(not_visible_val) = &params.not_visible {
                    if let Some(obj) = not_visible_val.as_object() {
                        if let Some(text_val) = obj.get("text") {
                            if let Some(text) = text_val.as_str() {
                                let selector = crate::driver::traits::Selector::Text(
                                    text.to_string(),
                                    0,
                                    false,
                                );
                                self.driver.wait_for_absence(&selector, timeout_ms).await?;
                            }
                        }
                    }
                }

                Ok(())
            }

            // Database Query
            TestCommand::DbQuery(params) => {
                let connection_str = self.context.substitute_vars(&params.connection);
                let query_str = self.context.substitute_vars(&params.query);

                // Create a pool (using sqlx::any for multi-db support)
                use sqlx::any::AnyPoolOptions;
                use sqlx::Row;

                // Create pool with 1 connection for simplicity in strict flow
                let pool = AnyPoolOptions::new()
                    .max_connections(1)
                    .connect(&connection_str)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;

                // Prepare query
                let mut query_builder = sqlx::query(&query_str);

                if let Some(query_params) = &params.params {
                    for p in query_params {
                        let val = self.context.substitute_vars(p);
                        query_builder = query_builder.bind(val);
                    }
                }

                // Execute and fetch all
                let rows = query_builder
                    .fetch_all(&pool)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to execute query: {}", e))?;

                self.emitter.emit(TestEvent::Log {
                    message: format!("{} Fetched {} rows", "ℹ".blue(), rows.len()),
                    depth: self.depth,
                });

                // Save results
                if let Some(save_map) = &params.save {
                    if let Some(first_row) = rows.first() {
                        for (col_name, var_name) in save_map {
                            // Try to get as string first, then fallbacks
                            let val_str = match first_row.try_get::<String, _>(col_name.as_str()) {
                                Ok(s) => s,
                                Err(_) => {
                                    // Try other common types
                                    if let Ok(v) = first_row.try_get::<i64, _>(col_name.as_str()) {
                                        v.to_string()
                                    } else if let Ok(v) =
                                        first_row.try_get::<f64, _>(col_name.as_str())
                                    {
                                        v.to_string()
                                    } else if let Ok(v) =
                                        first_row.try_get::<bool, _>(col_name.as_str())
                                    {
                                        v.to_string()
                                    } else {
                                        "null".to_string()
                                    }
                                }
                            };
                            self.context.set_var(var_name, &val_str);

                            self.emitter.emit(TestEvent::Log {
                                message: format!(
                                    "{} Saved db value {} = '{}'",
                                    "ℹ".blue(),
                                    var_name,
                                    val_str
                                ),
                                depth: self.depth,
                            });
                        }
                    } else {
                        self.emitter.emit(TestEvent::Log {
                            message: format!(
                                "{} No rows returned, cannot save variables",
                                "⚠".yellow()
                            ),
                            depth: self.depth,
                        });
                    }
                }

                Ok(())
            }

            // GIF Recording
            TestCommand::CaptureGifFrame(params_input) => {
                let params = params_input.clone().into_inner();
                let temp_path = format!("/tmp/gif_frame_{}.png", Uuid::new_v4());
                self.driver.take_screenshot(&temp_path).await?;

                let mut img_bytes = std::fs::read(&temp_path)?;
                std::fs::remove_file(&temp_path).ok();

                // Crop if specified
                if let Some(ref crop_str) = params.crop {
                    img_bytes = self.crop_image(&img_bytes, crop_str)?;
                }

                self.gif_frames.insert(params.name.clone(), img_bytes);
                println!("  {} Captured GIF frame: {}", "📷".green(), params.name);
                Ok(())
            }

            TestCommand::BuildGif(params) => {
                use crate::parser::types::GifFrameInput;
                use image::codecs::gif::{GifEncoder, Repeat};
                use image::{Delay, Frame};

                let output_path = self.context.output_path(&params.output);

                // Determine loop count
                let repeat = match params.loop_count {
                    Some(n) => Repeat::Finite(n),
                    None if params.loop_gif => Repeat::Infinite,
                    None => Repeat::Finite(1),
                };

                // Speed based on quality
                let speed = match params.quality.as_str() {
                    "high" => 1,
                    "low" => 30,
                    _ => 10, // medium
                };

                // Collect and process frames
                let mut frames = Vec::new();
                for frame_input in &params.frames {
                    let (name, delay) = match frame_input {
                        GifFrameInput::Name(n) => (n.clone(), params.delay),
                        GifFrameInput::WithDelay { name, delay } => (name.clone(), *delay),
                    };

                    let bytes = self
                        .gif_frames
                        .get(&name)
                        .ok_or_else(|| anyhow::anyhow!("GIF frame not found: {}", name))?;

                    let mut img = image::load_from_memory(bytes)?;

                    // Resize if width or height specified
                    if let Some(w) = params.width {
                        let ratio = w as f32 / img.width() as f32;
                        let h = (img.height() as f32 * ratio) as u32;
                        img = img.resize(w, h, image::imageops::FilterType::Lanczos3);
                    } else if let Some(h) = params.height {
                        let ratio = h as f32 / img.height() as f32;
                        let w = (img.width() as f32 * ratio) as u32;
                        img = img.resize(w, h, image::imageops::FilterType::Lanczos3);
                    }

                    frames.push((img.to_rgba8(), delay));
                }

                // Encode GIF
                let file = std::fs::File::create(&output_path)?;
                let mut encoder = GifEncoder::new_with_speed(file, speed);
                encoder.set_repeat(repeat)?;

                for (frame_img, delay_ms) in &frames {
                    let frame = Frame::from_parts(
                        frame_img.clone(),
                        0,
                        0,
                        Delay::from_numer_denom_ms(*delay_ms, 1),
                    );
                    encoder.encode_frame(frame)?;
                }

                println!(
                    "  {} Built GIF: {} ({} frames, quality: {})",
                    "🎬".green(),
                    output_path.display(),
                    frames.len(),
                    params.quality
                );
                Ok(())
            }

            // Start auto-capture mode
            TestCommand::StartGifCapture(params) => {
                self.auto_capture_frames.clear();
                self.auto_capture_active = true;
                self.auto_capture_interval = params.interval;
                self.auto_capture_max = params.max_frames;
                self.auto_capture_width = params.width;
                self.auto_capture_last_time = std::time::Instant::now();

                println!(
                    "  {} Started auto-capture (interval: {}ms, max: {} frames)",
                    "📹".green(),
                    params.interval,
                    params.max_frames
                );
                Ok(())
            }

            // Stop auto-capture and build GIF
            TestCommand::StopGifCapture(params) => {
                use image::codecs::gif::{GifEncoder, Repeat};
                use image::{Delay, Frame};

                self.auto_capture_active = false;

                if self.auto_capture_frames.is_empty() {
                    anyhow::bail!("No frames captured! Make sure startGifCapture was called.");
                }

                let output_path = self.context.output_path(&params.output);
                let delay_ms = params.delay.unwrap_or(self.auto_capture_interval as u32);

                let repeat = match params.loop_count {
                    Some(n) => Repeat::Finite(n),
                    None => Repeat::Infinite,
                };

                let speed = match params.quality.as_str() {
                    "high" => 1,
                    "low" => 30,
                    _ => 10,
                };

                // Process frames
                let mut processed_frames = Vec::new();
                for bytes in &self.auto_capture_frames {
                    let mut img = image::load_from_memory(bytes)?;

                    // Resize if width was specified
                    if let Some(w) = self.auto_capture_width {
                        let ratio = w as f32 / img.width() as f32;
                        let h = (img.height() as f32 * ratio) as u32;
                        img = img.resize(w, h, image::imageops::FilterType::Lanczos3);
                    }

                    processed_frames.push(img.to_rgba8());
                }

                // Encode GIF
                let file = std::fs::File::create(&output_path)?;
                let mut encoder = GifEncoder::new_with_speed(file, speed);
                encoder.set_repeat(repeat)?;

                for frame_img in &processed_frames {
                    let frame = Frame::from_parts(
                        frame_img.clone(),
                        0,
                        0,
                        Delay::from_numer_denom_ms(delay_ms, 1),
                    );
                    encoder.encode_frame(frame)?;
                }

                let frame_count = self.auto_capture_frames.len();
                self.auto_capture_frames.clear();

                println!(
                    "  {} Built smooth GIF: {} ({} frames, {}ms delay)",
                    "🎬".green(),
                    output_path.display(),
                    frame_count,
                    delay_ms
                );
                Ok(())
            }

            // ManualScroll (swipe command)
            TestCommand::ManualScroll(params) => {
                use crate::driver::traits::SwipeDirection;

                let direction = params
                    .as_ref()
                    .and_then(|p| p.direction.as_ref())
                    .map(|d| match d.to_lowercase().as_str() {
                        "left" => SwipeDirection::Left,
                        "right" => SwipeDirection::Right,
                        "up" => SwipeDirection::Up,
                        "down" => SwipeDirection::Down,
                        _ => SwipeDirection::Up,
                    })
                    .unwrap_or(SwipeDirection::Up);

                let duration = params
                    .as_ref()
                    .and_then(|p| p.duration.or(p.distance))
                    .map(|d| d as u64);

                let from_selector =
                    if let Some(ref from) = params.as_ref().and_then(|p| p.from.as_ref()) {
                        self.build_selector(
                            &from.text,
                            &from.regex,
                            &from.id,
                            &from.relative,
                            &from.css,
                            &from.xpath,
                            &from.placeholder,
                            &from.role,
                            &from.element_type,
                            &from.image,
                            from.index,
                            from.exact,
                        )
                    } else {
                        None
                    };

                self.driver.swipe(direction, duration, from_selector).await
            }

            // Mock Location Synchronization
            TestCommand::WaitForLocation(params) => {
                self.driver
                    .wait_for_location(
                        params.name.clone(),
                        params.lat,
                        params.lon,
                        params.tolerance,
                        params.timeout,
                    )
                    .await
            }

            TestCommand::WaitForMockCompletion(params) => {
                self.driver
                    .wait_for_mock_completion(params.name.clone(), params.timeout)
                    .await
            }

            TestCommand::MockLocationControl(params) => {
                self.driver
                    .control_mock_location(
                        params.name.clone(),
                        params.speed,
                        params.speed_mode.clone(),
                        params.speed_noise,
                        params.pause,
                        params.resume,
                    )
                    .await
            }

            // Performance & Load Testing
            TestCommand::StartProfiling(params) => {
                self.driver.start_profiling(params.clone()).await?;
                println!("  {} Started performance profiling", "⚡".green());
                Ok(())
            }

            TestCommand::StopProfiling(params) => {
                self.driver.stop_profiling().await?;
                println!("  {} Stopped performance profiling", "⚡".green());
                // Optional: Save report if path provided
                if let Some(p) = params.as_ref().and_then(|x| x.save_path.as_ref()) {
                    let metrics = self.driver.get_performance_metrics().await?;
                    let json = serde_json::to_string_pretty(&metrics)?;
                    let path = self.context.output_path(p);
                    std::fs::write(&path, json)?;
                    println!(
                        "  {} Saved performance report: {}",
                        "📄".green(),
                        path.display()
                    );
                }
                Ok(())
            }

            TestCommand::AssertPerformance(params) => {
                let metrics = self.driver.get_performance_metrics().await?;
                let metric_name = &params.metric;
                let limit_str = &params.limit;

                // Find metric (case-insensitive key search)
                let value = metrics
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(metric_name))
                    .map(|(_, v)| *v)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Metric '{}' not found in performance data. Available: {:?}",
                            metric_name,
                            metrics.keys()
                        )
                    })?;

                // Parse limit
                let (limit_val, _unit) = if limit_str.to_lowercase().ends_with("mb") {
                    (
                        limit_str
                            .to_lowercase()
                            .trim_end_matches("mb")
                            .trim()
                            .parse::<f64>()?,
                        "MB",
                    )
                } else if limit_str.to_lowercase().ends_with("kb") {
                    (
                        limit_str
                            .to_lowercase()
                            .trim_end_matches("kb")
                            .trim()
                            .parse::<f64>()?,
                        "kB",
                    )
                } else if limit_str.to_lowercase().ends_with("fps") {
                    (
                        limit_str
                            .to_lowercase()
                            .trim_end_matches("fps")
                            .trim()
                            .parse::<f64>()?,
                        "FPS",
                    )
                } else if limit_str.to_lowercase().ends_with("%") {
                    (
                        limit_str
                            .to_lowercase()
                            .trim_end_matches("%")
                            .trim()
                            .parse::<f64>()?,
                        "%",
                    )
                } else {
                    (limit_str.parse::<f64>()?, "")
                };

                // Check condition (Assuming limit is MAX allowed, except for FPS where it might be MIN?)
                // Usually "limit" implies upper bound for resource usage (RAM, CPU).
                // But for FPS, we usually want "min 60fps".
                // Heuristic: if fps, check >=. If memory/cpu, check <=.
                let passed = if metric_name.to_lowercase().contains("fps") {
                    value >= limit_val
                } else {
                    value <= limit_val
                };

                if passed {
                    println!(
                        "  {} Performance Check Passed: {} = {:.2} (Limit: {})",
                        "✓".green(),
                        metric_name,
                        value,
                        limit_str
                    );
                    Ok(())
                } else {
                    anyhow::bail!(
                        "Performance Check Failed: {} = {:.2} (Limit: {})",
                        metric_name,
                        value,
                        limit_str
                    )
                }
            }

            TestCommand::SetCpuThrottling(rate) => {
                self.driver.set_cpu_throttling(*rate).await?;
                println!("  {} Set CPU throttling rate: {}x", "⚡".green(), rate);
                Ok(())
            }

            TestCommand::SetNetworkConditions(profile) => {
                self.driver.set_network_conditions(profile).await?;
                println!("  {} Set network profile: {}", "⚡".green(), profile);
                Ok(())
            }

            TestCommand::SelectDisplay(id_str) => {
                let id_val = self.context.substitute_vars(id_str);

                // Support "auto" keyword for auto-detection
                if id_val.eq_ignore_ascii_case("auto") {
                    // For Android, try to detect or create secondary display
                    if self.driver.platform_name() == "android" {
                        println!("  {} Auto-detecting secondary display...", "🔍".cyan());

                        // Check if secondary display exists
                        let display_info = self.driver.dump_logs(1).await.unwrap_or_default();

                        // Try to get display list using a shell command workaround
                        // We'll create an overlay display if none exists
                        // First, create overlay display for Android Auto testing
                        let _ = std::process::Command::new("adb")
                            .args(&[
                                "shell",
                                "settings",
                                "put",
                                "global",
                                "overlay_display_devices",
                                "1024x768/120",
                            ])
                            .output();

                        // Wait for display to be created
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                        // Use display 2 (typical overlay display ID)
                        self.driver.select_display(2).await?;
                        println!("  {} Created overlay display and selected Display 2 (Android Auto simulation)", "📺".green());
                    } else {
                        println!(
                            "  {} Auto-detect display only supported on Android",
                            "⚠".yellow()
                        );
                    }
                } else {
                    let id = id_val
                        .parse::<u32>()
                        .map_err(|e| anyhow::anyhow!("Invalid display ID '{}': {}", id_val, e))?;

                    // If switching back to display 0, cleanup overlay display
                    if id == 0 && self.driver.platform_name() == "android" {
                        let _ = std::process::Command::new("adb")
                            .args(&[
                                "shell",
                                "settings",
                                "delete",
                                "global",
                                "overlay_display_devices",
                            ])
                            .output();
                        println!(
                            "  {} Removed overlay display and switched to main display",
                            "🧹".cyan()
                        );
                    }

                    self.driver.select_display(id).await?;
                }
                Ok(())
            }

            // Set device locale for i18n testing
            TestCommand::SetLocale(locale) => {
                let locale_val = self.context.substitute_vars(locale);
                self.driver.set_locale(&locale_val).await
            }

            // Unimplemented commands
            TestCommand::ExportReport(_)
            | TestCommand::Navigate(_)
            | TestCommand::Click(_)
            | TestCommand::Type(_) => {
                println!(
                    "  {} Command not yet implemented: {}",
                    "⚠".yellow(),
                    command.display_name()
                );
                Ok(())
            }
        }
    }

    fn build_selector(
        &self,
        text: &Option<String>,
        regex: &Option<String>,
        id: &Option<String>,
        relative: &Option<crate::parser::types::RelativeParams>,
        css: &Option<String>,
        xpath: &Option<String>,
        placeholder: &Option<String>,
        role: &Option<String>,
        element_type: &Option<String>,
        image: &Option<String>,
        index: Option<u32>,
        exact: bool,
    ) -> Option<crate::driver::traits::Selector> {
        use crate::driver::traits::Selector;

        let idx = index.unwrap_or(0) as usize;

        let primary = if let Some(r) = regex {
            Selector::TextRegex(self.context.substitute_vars(r), idx)
        } else if let Some(t) = text {
            Selector::Text(self.context.substitute_vars(t), idx, exact)
        } else if let Some(i) = id {
            let subst_id = self.context.substitute_vars(i);
            if subst_id.contains(".*")
                || subst_id.contains(".+")
                || (subst_id.starts_with('^') && subst_id.ends_with('$'))
            {
                Selector::IdRegex(subst_id, idx)
            } else {
                Selector::Id(subst_id, idx)
            }
        } else if let Some(p) = placeholder {
            Selector::Placeholder(self.context.substitute_vars(p), idx)
        } else if let Some(r) = role {
            Selector::Role(self.context.substitute_vars(r), idx)
        } else if let Some(e) = element_type {
            Selector::Type(self.context.substitute_vars(e), idx)
        } else if let Some(c) = css {
            Selector::Css(self.context.substitute_vars(c))
        } else if let Some(img) = image {
            let resolved = self.context.resolve_path(img);
            Selector::Image {
                path: resolved.to_string_lossy().to_string(),
                region: None,
            }
        } else if let Some(x) = xpath {
            Selector::XPath(self.context.substitute_vars(x))
        } else {
            return None;
        };

        if let Some(rel) = relative {
            let (dir, anchor_str) = if let Some(s) = &rel.right_of {
                (crate::driver::traits::RelativeDirection::RightOf, s)
            } else if let Some(s) = &rel.left_of {
                (crate::driver::traits::RelativeDirection::LeftOf, s)
            } else if let Some(s) = &rel.above {
                (crate::driver::traits::RelativeDirection::Above, s)
            } else if let Some(s) = &rel.below {
                (crate::driver::traits::RelativeDirection::Below, s)
            } else {
                return Some(primary);
            };

            let subst_anchor = self.context.substitute_vars(anchor_str);
            // Default to matching anchor as Text.
            // TODO: Support ID matching if string looks like an ID or allow explicit anchor param.
            let anchor = Selector::Text(subst_anchor, 0, false);

            Some(Selector::Relative {
                target: Box::new(primary),
                anchor: Box::new(anchor),
                direction: dir,
                max_dist: rel.max_dist,
            })
        } else {
            Some(primary)
        }
    }

    /// Handle command failure by dumping UI and taking screenshot
    async fn handle_failure(&self, flow_name: &str, index: usize, error: &str) {
        let safe_flow_name = flow_name.replace("/", "_").replace("\\", "_");

        self.emitter.emit(TestEvent::Log {
            message: format!("\n  {} Command failed: {}", "❌".red(), error),
            depth: self.depth,
        });

        if !self.report_enabled {
            return;
        }

        self.emitter.emit(TestEvent::Log {
            message: format!("  {} Capturing failure context...", "ℹ".blue()),
            depth: self.depth,
        });

        let uuid = Uuid::new_v4().to_string();
        let timestamp = chrono::Local::now().format("%H%M%S");

        // 1. Snapshot XML
        match self.driver.dump_ui_hierarchy().await {
            Ok(xml) => {
                let filename = format!(
                    "fail_{}_{}_cmd{}_{}.xml",
                    safe_flow_name,
                    timestamp,
                    index,
                    &uuid[..8]
                );
                let path = self.context.output_path(&filename);
                if let Ok(_) = std::fs::write(&path, xml) {
                    println!("  {} Saved UI Hierarchy: {}", "📄".green(), path.display());
                }
            }
            Err(e) => println!("  {} Failed to dump UI: {}", "⚠".yellow(), e),
        }

        // 2. Screenshot
        let filename = format!(
            "fail_{}_{}_cmd{}_{}.png",
            safe_flow_name,
            timestamp,
            index,
            &uuid[..8]
        );
        let path = self.context.output_path(&filename);
        let path_str = path.to_string_lossy().to_string();

        match self.driver.take_screenshot(&path_str).await {
            Ok(_) => println!("  {} Saved Screenshot: {}", "📸".green(), path.display()),
            Err(e) => println!("  {} Failed to take screenshot: {}", "⚠".yellow(), e),
        }

        // 3. Logcat (Recent 1000 lines)
        match self.driver.dump_logs(1000).await {
            Ok(logs) => {
                let filename = format!(
                    "fail_{}_{}_cmd{}_{}.log",
                    flow_name,
                    timestamp,
                    index,
                    &uuid[..8]
                );
                let path = self.context.output_path(&filename);
                if let Ok(_) = std::fs::write(&path, logs) {
                    println!("  {} Saved Recent Logs: {}", "📋".green(), path.display());
                }
            }
            Err(e) => println!("  {} Failed to dump logs: {}", "⚠".yellow(), e),
        }
    }

    /// Crop image by percentage region
    fn crop_image(&self, bytes: &[u8], crop_str: &str) -> Result<Vec<u8>> {
        let parts: Vec<f32> = crop_str
            .split(',')
            .filter_map(|s| s.trim().trim_end_matches('%').parse().ok())
            .collect();

        if parts.len() != 4 {
            anyhow::bail!("Invalid crop format, expected: left%,top%,width%,height%");
        }

        let img = image::load_from_memory(bytes)?;
        let (w, h) = (img.width() as f32, img.height() as f32);

        let x = (parts[0] / 100.0 * w) as u32;
        let y = (parts[1] / 100.0 * h) as u32;
        let cw = (parts[2] / 100.0 * w) as u32;
        let ch = (parts[3] / 100.0 * h) as u32;

        let cropped = img.crop_imm(x, y, cw, ch);

        let mut buf = std::io::Cursor::new(Vec::new());
        cropped.write_to(&mut buf, image::ImageFormat::Png)?;
        Ok(buf.into_inner())
    }

    /// Try to auto-capture a GIF frame if interval has passed
    async fn try_auto_capture(&mut self) {
        if !self.auto_capture_active {
            return;
        }

        // Check if we've reached max frames
        if self.auto_capture_frames.len() >= self.auto_capture_max as usize {
            return;
        }

        // Check if interval has passed
        let elapsed = self.auto_capture_last_time.elapsed().as_millis() as u64;
        if elapsed < self.auto_capture_interval {
            return;
        }

        // Capture frame
        let temp_path = format!("/tmp/auto_gif_frame_{}.png", uuid::Uuid::new_v4());
        if let Ok(()) = self.driver.take_screenshot(&temp_path).await {
            if let Ok(bytes) = std::fs::read(&temp_path) {
                self.auto_capture_frames.push(bytes);
                std::fs::remove_file(&temp_path).ok();
            }
        }

        self.auto_capture_last_time = std::time::Instant::now();
    }

    /// Finish the test session and generate reports
    pub async fn finish(&mut self) -> Result<()> {
        self.session.finish();

        if !self.report_enabled {
            return Ok(());
        }

        let summary = self.session.summary();

        self.emitter.emit(TestEvent::SessionFinished {
            summary: summary.clone(),
        });

        // Small delay to ensure SessionFinished event is processed before printing reports
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Save JSON report
        let report_data = self.session.to_report();
        let report_path = self.context.output_path("test-results.json");
        let json = serde_json::to_string_pretty(&report_data)?;
        std::fs::write(&report_path, json)?;

        println!(
            "\n{} JSON report saved to: {}",
            "📄".to_string().blue(),
            report_path.display().to_string().cyan()
        );

        // Generate and save HTML report
        let html_path = self.context.output_path("report.html");
        // Convert TestSessionReport to TestResults for HTML generator
        let test_results = crate::report::types::TestResults {
            session_id: report_data.session_id.clone(),
            flows: report_data.flows,
            summary: report_data.summary,
            generated_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        };

        crate::report::html::generate(&test_results, Some(&html_path)).await?;

        println!(
            "{} HTML report saved to: {}",
            "📊".to_string().blue(),
            html_path.display().to_string().cyan()
        );

        // Generate and save JUnit report
        crate::report::junit::write_report(&test_results, &self.context.output_dir)?;

        Ok(())
    }

    async fn evaluate_condition_value(&self, value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Bool(b) => *b,
            serde_json::Value::String(s) => {
                let subst = self.context.substitute_vars(s);
                use super::js_engine::JsEngine;
                let mut engine = JsEngine::new();
                engine.set_vars(&self.context.vars);
                engine.set_vars(&self.context.env);
                engine.eval_bool(&subst).unwrap_or(false)
            }
            serde_json::Value::Number(n) => n.as_f64().map_or(false, |v| v != 0.0),
            serde_json::Value::Object(map) => {
                if let Some(v) = map.get("true") {
                    return Box::pin(self.evaluate_condition_value(v)).await;
                }
                if let Some(v) = map.get("false") {
                    return !Box::pin(self.evaluate_condition_value(v)).await;
                }

                if let Ok(cond) =
                    serde_json::from_value::<crate::parser::types::Condition>(value.clone())
                {
                    return self.check_condition(&cond).await;
                }
                true
            }
            _ => true,
        }
    }

    async fn check_condition(&self, cond: &crate::parser::types::Condition) -> bool {
        use crate::driver::traits::Selector;

        if let Some(ref text) = cond.visible {
            let text = self.context.substitute_vars(text);
            let selector = Selector::Text(text, 0, false);
            return self.driver.is_visible(&selector).await.unwrap_or(false);
        }
        if let Some(ref re) = cond.visible_regex {
            let re = self.context.substitute_vars(re);
            let selector = Selector::TextRegex(re, 0);
            return self.driver.is_visible(&selector).await.unwrap_or(false);
        }
        if let Some(ref text) = cond.not_visible {
            let text = self.context.substitute_vars(text);
            let selector = Selector::Text(text, 0, false);
            return !self.driver.is_visible(&selector).await.unwrap_or(false);
        }
        if let Some(ref re) = cond.not_visible_regex {
            let re = self.context.substitute_vars(re);
            let selector = Selector::TextRegex(re, 0);
            return !self.driver.is_visible(&selector).await.unwrap_or(false);
        }
        true
    }
}
