use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Command execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CommandStatus {
    Pending,
    Running,
    Passed,
    Failed { error: String },
    Skipped { reason: String },
    Retrying { attempt: u32, max_attempts: u32 },
}

impl CommandStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            CommandStatus::Passed | CommandStatus::Failed { .. } | CommandStatus::Skipped { .. }
        )
    }
}

/// State for a single command execution
#[derive(Debug, Clone)]
pub struct CommandState {
    pub index: usize,
    pub command_name: String,
    pub command_display: String,
    pub status: CommandStatus,
    pub started_at: Option<Instant>,
    pub finished_at: Option<Instant>,
    pub duration_ms: Option<u64>,
    pub screenshot_path: Option<String>,
    pub retry_count: u32,
}

impl CommandState {
    pub fn new(index: usize, name: &str, display: &str) -> Self {
        Self {
            index,
            command_name: name.to_string(),
            command_display: display.to_string(),
            status: CommandStatus::Pending,
            started_at: None,
            finished_at: None,
            duration_ms: None,
            screenshot_path: None,
            retry_count: 0,
        }
    }

    pub fn start(&mut self) {
        self.status = CommandStatus::Running;
        self.started_at = Some(Instant::now());
    }

    pub fn pass(&mut self) {
        self.finish(CommandStatus::Passed);
    }

    pub fn fail(&mut self, error: String) {
        self.finish(CommandStatus::Failed { error });
    }

    pub fn skip(&mut self, reason: String) {
        self.status = CommandStatus::Skipped { reason };
    }

    pub fn retry(&mut self, attempt: u32, max_attempts: u32) {
        self.status = CommandStatus::Retrying {
            attempt,
            max_attempts,
        };
        self.retry_count = attempt;
    }

    fn finish(&mut self, status: CommandStatus) {
        self.status = status;
        self.finished_at = Some(Instant::now());
        if let Some(start) = self.started_at {
            self.duration_ms = Some(start.elapsed().as_millis() as u64);
        }
    }

    /// Serialize state for reporting (without Instant which isn't serializable)
    pub fn to_report(&self) -> CommandStateReport {
        CommandStateReport {
            index: self.index,
            command_name: self.command_name.clone(),
            command_display: self.command_display.clone(),
            status: self.status.clone(),
            duration_ms: self.duration_ms,
            screenshot_path: self.screenshot_path.clone(),
            retry_count: self.retry_count,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandStateReport {
    pub index: usize,
    pub command_name: String,
    pub command_display: String,
    pub status: CommandStatus,
    pub duration_ms: Option<u64>,
    pub screenshot_path: Option<String>,
    pub retry_count: u32,
}

/// State for entire test flow execution
#[derive(Debug, Clone)]
pub struct FlowState {
    pub flow_name: String,
    pub flow_path: String,
    pub status: FlowStatus,
    pub commands: Vec<CommandState>,
    pub current_index: usize,
    pub started_at: Option<Instant>,
    pub finished_at: Option<Instant>,
    pub total_duration_ms: Option<u64>,
    pub error: Option<String>,
    pub video_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FlowStatus {
    Pending,
    Running,
    Passed,
    Failed,
    PartiallyPassed { passed: u32, failed: u32 },
}

impl FlowState {
    pub fn new(name: &str, path: &str, commands: Vec<CommandState>) -> Self {
        Self {
            flow_name: name.to_string(),
            flow_path: path.to_string(),
            status: FlowStatus::Pending,
            commands,
            current_index: 0,
            started_at: None,
            finished_at: None,
            total_duration_ms: None,
            error: None,
            video_path: None,
        }
    }

    pub fn start(&mut self) {
        self.status = FlowStatus::Running;
        self.started_at = Some(Instant::now());
    }

    pub fn current_command(&mut self) -> Option<&mut CommandState> {
        self.commands.get_mut(self.current_index)
    }

    pub fn advance(&mut self) -> bool {
        self.current_index += 1;
        self.current_index < self.commands.len()
    }

    pub fn finish(&mut self) {
        self.finished_at = Some(Instant::now());
        if let Some(start) = self.started_at {
            self.total_duration_ms = Some(start.elapsed().as_millis() as u64);
        }

        let (passed, failed) = self
            .commands
            .iter()
            .fold((0, 0), |(p, f), cmd| match cmd.status {
                CommandStatus::Passed => (p + 1, f),
                CommandStatus::Failed { .. } => (p, f + 1),
                _ => (p, f),
            });

        self.status = if failed == 0 {
            FlowStatus::Passed
        } else if passed == 0 {
            FlowStatus::Failed
        } else {
            FlowStatus::PartiallyPassed { passed, failed }
        };
    }

    pub fn skip_remaining(&mut self, reason: &str) {
        for cmd in &mut self.commands[self.current_index..] {
            if matches!(cmd.status, CommandStatus::Pending) {
                cmd.skip(reason.to_string());
            }
        }
    }

    /// Serialize state for reporting
    pub fn to_report(&self) -> FlowStateReport {
        FlowStateReport {
            flow_name: self.flow_name.clone(),
            flow_path: self.flow_path.clone(),
            status: self.status.clone(),
            commands: self.commands.iter().map(|c| c.to_report()).collect(),
            total_duration_ms: self.total_duration_ms,
            error: self.error.clone(),
            video_path: self.video_path.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowStateReport {
    pub flow_name: String,
    pub flow_path: String,
    pub status: FlowStatus,
    pub commands: Vec<CommandStateReport>,
    pub total_duration_ms: Option<u64>,
    pub error: Option<String>,
    pub video_path: Option<String>,
}

/// Global test session state
#[derive(Debug, Clone)]
pub struct TestSessionState {
    pub session_id: String,
    pub flows: Vec<FlowState>,
    pub current_flow_index: usize,
    pub started_at: Option<Instant>,
    pub finished_at: Option<Instant>,
}

impl TestSessionState {
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            flows: Vec::new(),
            current_flow_index: 0,
            started_at: None,
            finished_at: None,
        }
    }

    pub fn start(&mut self) {
        self.started_at = Some(Instant::now());
    }

    pub fn add_flow(&mut self, flow: FlowState) {
        self.flows.push(flow);
    }

    pub fn current_flow(&mut self) -> Option<&mut FlowState> {
        self.flows.get_mut(self.current_flow_index)
    }

    pub fn finish(&mut self) {
        self.finished_at = Some(Instant::now());
    }

    pub fn summary(&self) -> TestSummary {
        let mut total_commands = 0;
        let mut passed = 0;
        let mut failed = 0;
        let mut skipped = 0;

        for flow in &self.flows {
            for cmd in &flow.commands {
                total_commands += 1;
                match cmd.status {
                    CommandStatus::Passed => passed += 1,
                    CommandStatus::Failed { .. } => failed += 1,
                    CommandStatus::Skipped { .. } => skipped += 1,
                    _ => {}
                }
            }
        }

        let total_duration_ms = self.started_at.map(|start| {
            self.finished_at
                .unwrap_or_else(Instant::now)
                .duration_since(start)
                .as_millis() as u64
        });

        TestSummary {
            session_id: self.session_id.clone(),
            total_flows: self.flows.len() as u32,
            total_commands,
            passed,
            failed,
            skipped,
            total_duration_ms,
        }
    }

    /// Serialize state for reporting
    pub fn to_report(&self) -> TestSessionReport {
        TestSessionReport {
            session_id: self.session_id.clone(),
            flows: self.flows.iter().map(|f| f.to_report()).collect(),
            summary: self.summary(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestSummary {
    pub session_id: String,
    pub total_flows: u32,
    pub total_commands: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub total_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestSessionReport {
    pub session_id: String,
    pub flows: Vec<FlowStateReport>,
    pub summary: TestSummary,
}
