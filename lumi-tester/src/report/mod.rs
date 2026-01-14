pub mod html;
pub mod json;
pub mod junit;
pub mod types;

use anyhow::Result;
use std::path::Path;

/// Generate report from test results
pub async fn generate_report(
    results_path: &Path,
    format: &str,
    output: Option<&Path>,
) -> Result<()> {
    let results = std::fs::read_to_string(results_path)?;
    let test_results: types::TestResults = serde_json::from_str(&results)?;

    match format {
        "json" => json::generate(&test_results, output).await,
        "html" => html::generate(&test_results, output).await,
        _ => anyhow::bail!("Unknown format: {}", format),
    }
}
