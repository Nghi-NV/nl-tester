use std::path::Path;
use anyhow::Result;
use super::types::TestResults;

/// Generate JSON report
pub async fn generate(results: &TestResults, output: Option<&Path>) -> Result<()> {
    let json = serde_json::to_string_pretty(results)?;
    
    if let Some(path) = output {
        std::fs::write(path, json)?;
        println!("JSON report saved to: {}", path.display());
    } else {
        println!("{}", json);
    }
    
    Ok(())
}
