use job_hunter_core::AnalyzedJobPosting;
use std::path::Path;

pub async fn save_json(jobs: &[AnalyzedJobPosting], path: &Path) -> std::io::Result<()> {
    let content = serde_json::to_string_pretty(jobs)?;
    tokio::fs::write(path, content).await
}
