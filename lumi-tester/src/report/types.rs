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
