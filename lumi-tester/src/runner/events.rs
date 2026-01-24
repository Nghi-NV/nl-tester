use super::state::{FlowStatus, TestSummary};
use tokio::sync::broadcast;

/// Test execution events for real-time updates
#[derive(Debug, Clone)]
pub enum TestEvent {
    // Session events
    SessionStarted {
        session_id: String,
    },
    SessionFinished {
        summary: TestSummary,
    },

    // Flow events
    FlowStarted {
        flow_name: String,
        flow_path: String,
        command_count: usize,
        depth: usize,
    },
    FlowFinished {
        flow_name: String,
        status: FlowStatus,
        duration_ms: Option<u64>,
        depth: usize,
    },

    // Command events
    CommandStarted {
        flow_name: String,
        index: usize,
        command: String,
        depth: usize,
    },
    CommandPassed {
        flow_name: String,
        index: usize,
        duration_ms: u64,
        depth: usize,
    },
    CommandFailed {
        flow_name: String,
        index: usize,
        error: String,
        duration_ms: u64,
        depth: usize,
    },
    CommandRetrying {
        flow_name: String,
        index: usize,
        attempt: u32,
        max_attempts: u32,
        depth: usize,
    },
    CommandSkipped {
        flow_name: String,
        index: usize,
        reason: String,
        depth: usize,
    },

    // Log event for coordinated output
    Log {
        message: String,
        depth: usize,
    },
}

/// Event emitter for broadcasting test events
pub struct EventEmitter {
    sender: broadcast::Sender<TestEvent>,
}

impl EventEmitter {
    pub fn new() -> (Self, broadcast::Receiver<TestEvent>) {
        let (sender, receiver) = broadcast::channel(100);
        (Self { sender }, receiver)
    }

    pub fn emit(&self, event: TestEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TestEvent> {
        self.sender.subscribe()
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self { sender }
    }
}

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration as StdDuration;

/// Console event listener for printing real-time updates
pub struct ConsoleEventListener;

impl ConsoleEventListener {
    pub async fn listen(mut receiver: broadcast::Receiver<TestEvent>) {
        use colored::Colorize;
        use indicatif::ProgressDrawTarget;
        use std::io::IsTerminal;

        // Create MultiProgress with appropriate draw target based on TTY detection
        let multi = if std::io::stdout().is_terminal() {
            MultiProgress::new()
        } else {
            // When not a TTY (piped output), use hidden target to avoid terminal escape codes
            MultiProgress::with_draw_target(ProgressDrawTarget::hidden())
        };

        // Keep track of the current spinners by depth
        let mut spinners: Vec<Option<ProgressBar>> = Vec::new();
        let mut command_texts: Vec<String> = Vec::new();
        // Track original spinner styles for pause/resume
        let mut spinner_styles: Vec<Option<ProgressStyle>> = Vec::new();

        while let Ok(event) = receiver.recv().await {
            match event {
                TestEvent::SessionStarted { session_id } => {
                    multi
                        .println(format!(
                            "\n{} Test session started: {}",
                            "▶".green().bold(),
                            session_id.cyan()
                        ))
                        .ok();
                }

                TestEvent::SessionFinished { summary } => {
                    // Finish all spinners (but don't clear to preserve output)
                    for pb in spinners.drain(..).flatten() {
                        // Just finish the spinner, don't clear to preserve the message
                        pb.finish();
                    }

                    // Small delay to ensure all spinner finishes are rendered
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

                    // Use println! directly for summary to ensure it's not lost
                    // MultiProgress might overwrite output, so use direct stdout
                    println!("\n{} Test session finished", "■".blue().bold());
                    println!("  Total flows: {}", summary.total_flows);
                    println!("  Total commands: {}", summary.total_commands);
                    println!(
                        "  {} passed, {} failed, {} skipped",
                        summary.passed.to_string().green(),
                        summary.failed.to_string().red(),
                        summary.skipped.to_string().yellow()
                    );
                    if let Some(duration) = summary.total_duration_ms {
                        println!("  Duration: {}ms", duration);
                    }
                }

                TestEvent::FlowStarted {
                    flow_name,
                    command_count,
                    depth,
                    ..
                } => {
                    // Finish spinners at lower depths when nested flow starts to prevent loop
                    // We finish the spinner to stop it from ticking, but don't print message yet
                    // The CommandPassed event will handle printing the final message with duration
                    if depth > 0 {
                        for d in 0..depth {
                            if d < spinners.len() {
                                if let Some(pb) = spinners[d].take() {
                                    // Finish spinner to stop ticking, but keep message for later
                                    // We'll print the final message in CommandPassed with duration
                                    pb.finish();
                                }
                            }
                        }
                    }

                    let indent = "    ".repeat(depth);
                    // Use println! directly for flow started to ensure it's visible
                    println!(
                        "\n{}  {} Flow: {} ({} commands)",
                        indent,
                        "→".blue(),
                        flow_name.white().bold(),
                        command_count
                    );
                }

                TestEvent::FlowFinished {
                    flow_name,
                    status,
                    duration_ms,
                    depth,
                } => {
                    // Ensure spinner at this depth is finished (but not cleared to preserve output)
                    if depth < spinners.len() {
                        if let Some(pb) = spinners[depth].take() {
                            pb.finish();
                        }
                        // Clear saved style for this depth
                        if depth < spinner_styles.len() {
                            spinner_styles[depth] = None;
                        }
                    }

                    // Note: We don't resume spinners at lower depths because they were
                    // already finished when nested flow started. The CommandPassed event
                    // will handle printing the final message.

                    let status_str = match status {
                        FlowStatus::Passed => "PASSED".green().bold(),
                        FlowStatus::Failed => "FAILED".red().bold(),
                        FlowStatus::PartiallyPassed { passed, failed } => {
                            format!("PARTIAL ({}/{} passed)", passed, passed + failed)
                                .yellow()
                                .bold()
                        }
                        _ => "UNKNOWN".white().bold(),
                    };
                    let indent = "    ".repeat(depth);
                    // Use println! directly for flow finished to ensure it's visible
                    println!(
                        "{}  {} Flow {} [{}]",
                        indent,
                        "←".blue(),
                        flow_name,
                        status_str
                    );
                    if let Some(duration) = duration_ms {
                        println!("{}    Duration: {}ms", indent, duration);
                    }
                }

                TestEvent::CommandStarted {
                    index,
                    command,
                    depth,
                    ..
                } => {
                    // Pre-allocate or grow spinners/command_texts vectors
                    if depth >= spinners.len() {
                        spinners.resize(depth + 1, None);
                        command_texts.resize(depth + 1, String::new());
                    }
                    if depth >= spinner_styles.len() {
                        spinner_styles.resize(depth + 1, None);
                    }

                    // Create and configure spinner
                    let pb = multi.add(ProgressBar::new_spinner());
                    let indent = "    ".repeat(depth);
                    let style = ProgressStyle::default_spinner()
                        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
                        .template(&format!("{}    {{spinner}} {{msg}}", indent))
                        .unwrap();

                    // Save style for pause/resume
                    spinner_styles[depth] = Some(style.clone());
                    pb.set_style(style);

                    let body = format!("[{}] {}... ", index, command.dimmed());
                    pb.set_message(body.clone());
                    pb.enable_steady_tick(StdDuration::from_millis(100));

                    spinners[depth] = Some(pb);
                    command_texts[depth] = body;
                }

                TestEvent::CommandPassed {
                    duration_ms, depth, ..
                } => {
                    if depth < spinners.len() {
                        let indent = "    ".repeat(depth);
                        let done_msg = format!(
                            "{}    {} {}({}ms)",
                            indent,
                            "✓".green(),
                            command_texts[depth],
                            duration_ms
                        );

                        if let Some(pb) = spinners[depth].take() {
                            // Clear spinner first to remove the animated line
                            pb.finish_and_clear();
                            // Small delay to ensure clear is processed before printing
                            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                            // Then print the final message directly
                            println!("{}", done_msg);
                        } else {
                            // Print directly if no spinner
                            println!("{}", done_msg);
                        }
                        // Clear saved style
                        if depth < spinner_styles.len() {
                            spinner_styles[depth] = None;
                        }
                    }
                }

                TestEvent::CommandFailed {
                    error: _error, // Bind 'error' field to '_error' to ignore unused warning
                    duration_ms,
                    depth,
                    ..
                } => {
                    if depth < spinners.len() {
                        let indent = "    ".repeat(depth);

                        if let Some(pb) = spinners[depth].take() {
                            let style = ProgressStyle::default_spinner()
                                .template(&format!("{}    {{msg}}", indent))
                                .unwrap();
                            pb.set_style(style);
                            pb.finish_with_message(format!(
                                "{} {}({}ms)",
                                "✗".red(),
                                command_texts[depth],
                                duration_ms
                            ));
                        } else {
                            println!(
                                "{}    {} {}({}ms)",
                                indent,
                                "✗".red(),
                                command_texts[depth],
                                duration_ms
                            );
                        }
                    }
                }

                TestEvent::CommandRetrying {
                    attempt,
                    max_attempts,
                    depth,
                    ..
                } => {
                    if depth < spinners.len() {
                        if let Some(pb) = &spinners[depth] {
                            let retry_msg = format!(
                                "{} {}",
                                command_texts[depth],
                                format!("↻ retry {}/{}", attempt, max_attempts).yellow()
                            );
                            pb.set_message(retry_msg);
                        }
                    }
                }

                TestEvent::CommandSkipped { reason, depth, .. } => {
                    if depth < spinners.len() {
                        let indent = "    ".repeat(depth);
                        let done_msg = format!(
                            "{}    {} {}({})",
                            indent,
                            "○".yellow(),
                            command_texts[depth],
                            reason.dimmed()
                        );

                        if let Some(pb) = spinners[depth].take() {
                            // Clear spinner first to remove the animated line
                            pb.finish_and_clear();
                            // Small delay to ensure clear is processed before printing
                            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                            // Then print the final message directly
                            println!("{}", done_msg);
                        } else {
                            // Print directly if no spinner
                            println!("{}", done_msg);
                        }
                        // Clear saved style
                        if depth < spinner_styles.len() {
                            spinner_styles[depth] = None;
                        }
                    }
                }

                TestEvent::Log { message, depth } => {
                    let indent = "    ".repeat(depth);
                    multi.println(format!("{}      {}", indent, message)).ok();
                }
            }
        }
    }
}
