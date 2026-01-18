use chrono::{DateTime, Utc};
use rkyv::{Archive, Deserialize, Serialize};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};
use std::sync::Arc;

#[derive(
    Debug, Clone, SerdeSerialize, SerdeDeserialize, Archive, Serialize, Deserialize, PartialEq, Eq,
)]
#[archive(check_bytes)]
pub struct SourceSettings {
    pub source: JobSource,
    pub enabled: bool,
    pub delay_ms: u64,
    pub user_agent: String,
    pub use_proxy: bool,
}

#[derive(Debug, Clone, SerdeSerialize, SerdeDeserialize, Archive, Serialize, Deserialize)]
#[archive(check_bytes)]
pub struct SearchCriteria {
    pub keywords: Vec<String>,
    pub experience_level: ExperienceLevel,
    pub sources_config: Vec<SourceSettings>,
    pub user_cv: Option<String>,
}

#[derive(
    Debug, Clone, SerdeSerialize, SerdeDeserialize, Archive, Serialize, Deserialize, PartialEq, Eq,
)]
#[archive(check_bytes)]
pub enum ExperienceLevel {
    Entry,
    Junior,
    Mid,
    Senior,
    Lead,
    Any,
}

#[derive(
    Debug, Clone, SerdeSerialize, SerdeDeserialize, Archive, Serialize, Deserialize, PartialEq, Eq,
)]
#[archive(check_bytes)]
pub enum JobType {
    FullTime,
    PartTime,
    Contract,
    Freelance,
    Internship,
}

#[derive(
    Debug, Clone, SerdeSerialize, SerdeDeserialize, Archive, Serialize, Deserialize, PartialEq, Eq,
)]
#[archive(check_bytes)]
pub enum JobSource {
    // Clásicos
    RemoteOk,
    WeWorkRemotely,
    Arbeitnow,
    Himalayas,
    Jobspresso,
    // Nuevos
    Remotive,
    Jobicy,
    FindWork,
    WorkingNomads,
    VueJobs,
    CryptoJobs,
    RemoteCo,
    DevItJobs,
    PythonOrg,
    GolangProjects,
    // Fallback
    Custom(String),
}

#[derive(Debug, Clone, SerdeSerialize, SerdeDeserialize, Archive, Serialize, Deserialize)]
#[archive(check_bytes)]
pub struct RawJobPosting {
    pub id: String,
    pub source: JobSource,
    pub url: String,
    pub html_content: String,
    pub scraped_at: DateTime<Utc>,
}

#[derive(Debug, Clone, SerdeSerialize, SerdeDeserialize, Archive, Serialize, Deserialize)]
#[archive(check_bytes)]
pub struct AnalyzedJobPosting {
    pub id: String,
    pub title: String,
    pub company: Option<CompanyInfo>,
    pub description: String,
    pub salary_normalized: Option<f64>,
    pub red_flags: Vec<String>,
    pub skills_analysis: SkillsGap,
    pub requirements: Vec<String>,
    pub responsibilities: Vec<String>,
    pub skills: Vec<String>,
    pub salary_range: Option<SalaryRange>,
    pub location: String,
    pub is_remote: bool,
    pub job_type: JobType,
    pub experience_level: ExperienceLevel,
    pub url: String,
    pub posted_date: Option<DateTime<Utc>>,
    pub match_score: f32,
    pub match_reasons: Vec<String>,
}

#[derive(Debug, Clone, SerdeSerialize, SerdeDeserialize, Archive, Serialize, Deserialize)]
#[archive(check_bytes)]
pub struct SkillsGap {
    pub matching: Vec<String>,
    pub missing: Vec<String>,
}

impl Default for SkillsGap {
    fn default() -> Self {
        Self {
            matching: vec![],
            missing: vec![],
        }
    }
}

#[derive(Debug, Clone, SerdeSerialize, SerdeDeserialize, Archive, Serialize, Deserialize)]
#[archive(check_bytes)]
pub struct CompanyInfo {
    pub name: String,
    pub description: Option<String>,
    pub industry: Option<String>,
    pub size: Option<String>,
    pub website: Option<String>,
    pub linkedin_url: Option<String>,
}

#[derive(Debug, Clone, SerdeSerialize, SerdeDeserialize, Archive, Serialize, Deserialize)]
#[archive(check_bytes)]
pub struct SalaryRange {
    pub min: u32,
    pub max: u32,
    pub currency: String,
    pub period: SalaryPeriod,
}

#[derive(Debug, Clone, SerdeSerialize, SerdeDeserialize, Archive, Serialize, Deserialize)]
#[archive(check_bytes)]
pub enum SalaryPeriod {
    Hourly,
    Daily,
    Monthly,
    Yearly,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive(check_bytes)]
pub enum AgentMessage {
    StartScraping(Arc<SearchCriteria>),
    RawJobsScraped(Vec<RawJobPosting>),
    AnalyzeJobs(Vec<RawJobPosting>, Arc<SearchCriteria>),
    JobsAnalyzed(Vec<AnalyzedJobPosting>),
    EnrichCompanyInfo(Vec<AnalyzedJobPosting>),
    JobsEnriched(Vec<AnalyzedJobPosting>),
    Error(String),
    Shutdown,
}

#[async_trait::async_trait]
pub trait Agent: Send + Sync {
    async fn process(&self, msg: AgentMessage) -> Result<AgentMessage, AgentError>;
    fn name(&self) -> &str;
}

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Scraping error: {0}")]
    Scraping(String),
    #[error("Analysis error: {0}")]
    Analysis(String),
    #[error("Enrichment error: {0}")]
    Enrichment(String),
    #[error("LLM error: {0}")]
    Llm(String),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}
