use crate::runner::state::{FlowStateReport, TestSummary};
use serde::{Deserialize, Serialize};

/// Test results for report generation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResults {
    pub session_id: String,
    pub flows: Vec<FlowStateReport>,
    pub summary: TestSummary,
    pub generated_at: String,
}

/// Standard Session Report matching the specification in "Phần mềm - Tool test thiết bị.md"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardSessionReport {
    pub testcase_id: String,
    pub session_id: String,
    pub status: String,
    pub execution_time: StandardExecutionTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_details: Option<StandardErrorDetails>,
    pub attachments_and_context: StandardAttachmentsAndContext,
    pub system_environment: StandardSystemEnvironment,
    pub execution_meta: StandardExecutionMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardExecutionTime {
    pub start_time: String,
    pub end_time: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardErrorDetails {
    pub message: String,
    pub error_code: String,
    pub step_failed: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StandardAttachmentsAndContext {
    pub cameras: Vec<StandardCameraAttachment>,
    pub logs: StandardLogsAttachment,
    pub sensors_snapshot: StandardSensorsSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardCameraAttachment {
    pub camera_id: String,
    pub r#type: String, // "image" or "video"
    pub url: String,
    pub timestamp_sync: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StandardLogsAttachment {
    pub app: Option<String>,
    pub server: Option<String>,
    pub hc: Option<String>,
    pub bridge: Option<String>,
    pub device: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StandardSensorsSnapshot {
    pub ambient_light_lux: Option<f64>,
    pub color_sensor_rgb: Option<String>,
    pub temperature_c: Option<f64>,
    pub door_contact_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StandardSystemEnvironment {
    pub app: StandardAppEnvironment,
    pub server: Option<serde_json::Value>,
    pub hc: Option<serde_json::Value>,
    pub test_environment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StandardAppEnvironment {
    pub name: String,
    pub version: String,
    pub os: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StandardExecutionMeta {
    pub retry_count: u32,
    pub runner_id: String,
}
