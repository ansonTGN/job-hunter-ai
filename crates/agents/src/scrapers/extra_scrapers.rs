use async_trait::async_trait;
use chrono::Utc;
use job_hunter_core::*;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;
use uuid::Uuid;

// Macro para generar agentes de APIs JSON repetitivas
macro_rules! impl_json_scraper {
    ($struct:ident, $source:expr, $name:expr, $url:expr, $json_path:expr, $url_key:expr) => {
        pub struct $struct;
        impl $struct { pub fn new() -> Self { Self } }

        #[async_trait]
        impl Agent for $struct {
            async fn process(&self, msg: AgentMessage) -> Result<AgentMessage, AgentError> {
                if let AgentMessage::StartScraping(criteria) = msg {
                    // Check enabled
                    let enabled = criteria.sources_config.iter()
                        .find(|s| s.source == $source)
                        .map(|s| s.enabled)
                        .unwrap_or(true); // Default true si no viene en config

                    if !enabled {
                        return Ok(AgentMessage::RawJobsScraped(vec![]));
                    }

                    let client = reqwest::Client::builder()
                        .user_agent("Mozilla/5.0 (compatible; JobHunterBot/1.0)")
                        .timeout(Duration::from_secs(20))
                        .build()?;

                    // Simulamos pequeño delay para ser amables
                    sleep(Duration::from_millis(500)).await;

                    let res = client.get($url).send().await?;
                    if !res.status().is_success() {
                        return Err(AgentError::Scraping(format!("HTTP {}", res.status())));
                    }

                    let json: serde_json::Value = res.json().await?;
                    let mut postings = Vec::new();

                    // Navegación básica en el JSON
                    let root = if $json_path == "" { &json } else { json.get($json_path).unwrap_or(&serde_json::Value::Null) };
                    
                    if let Some(arr) = root.as_array() {
                        for item in arr.iter().take(15) {
                            if let Some(link) = item.get($url_key).and_then(|v| v.as_str()) {
                                postings.push(RawJobPosting {
                                    id: Uuid::new_v4().to_string(),
                                    source: $source,
                                    url: link.to_string(),
                                    html_content: item.to_string(), // Raw JSON as content
                                    scraped_at: Utc::now(),
                                });
                            }
                        }
                    }

                    info!("📂 [{}] Extracted {} jobs.", $name, postings.len());
                    Ok(AgentMessage::RawJobsScraped(postings))
                } else {
                    Err(AgentError::Scraping("Invalid msg".into()))
                }
            }
            fn name(&self) -> &str { $name }
        }
    };
}

// --- IMPLEMENTACIONES JSON ---
impl_json_scraper!(RemotiveAgent, JobSource::Remotive, "scraper_remotive", "https://remotive.com/api/remote-jobs?limit=15", "jobs", "url");
impl_json_scraper!(JobicyAgent, JobSource::Jobicy, "scraper_jobicy", "https://jobicy.com/api/v2/remote-jobs?count=15", "jobs", "url");
impl_json_scraper!(FindWorkAgent, JobSource::FindWork, "scraper_findwork", "https://findwork.dev/api/jobs/", "results", "url");
impl_json_scraper!(WorkingNomadsAgent, JobSource::WorkingNomads, "scraper_workingnomads", "https://www.workingnomads.com/api/advanced_search", "", "url");
impl_json_scraper!(VueJobsAgent, JobSource::VueJobs, "scraper_vuejobs", "https://vuejobs.com/api/jobs", "data", "apply_url");
impl_json_scraper!(CryptoJobsAgent, JobSource::CryptoJobs, "scraper_cryptojobs", "https://cryptojobslist.com/api/jobs", "", "application_url");
impl_json_scraper!(DevItJobsAgent, JobSource::DevItJobs, "scraper_devitjobs", "https://devitjobs.uk/api/jobsFeed", "", "jobUrl");
impl_json_scraper!(GolangProjectsAgent, JobSource::GolangProjects, "scraper_golang", "https://golangprojects.com/api/v1/jobs", "jobs", "url");

// --- IMPLEMENTACIONES RSS (Manuales) ---

pub struct PythonOrgAgent;
impl PythonOrgAgent { pub fn new() -> Self { Self } }
#[async_trait]
impl Agent for PythonOrgAgent {
    async fn process(&self, msg: AgentMessage) -> Result<AgentMessage, AgentError> {
        if let AgentMessage::StartScraping(c) = msg {
             if c.sources_config.iter().any(|s| s.source == JobSource::PythonOrg && !s.enabled) { return Ok(AgentMessage::RawJobsScraped(vec![])); }
             let txt = reqwest::get("https://www.python.org/jobs/feed/rss/").await?.text().await?;
             let mut posts = vec![];
             for item in txt.split("<item>").skip(1).take(15) {
                 if let Some(l) = item.split("<link>").nth(1).and_then(|x| x.split("</link>").next()) {
                     posts.push(RawJobPosting { id: Uuid::new_v4().to_string(), source: JobSource::PythonOrg, url: l.trim().to_string(), html_content: item.to_string(), scraped_at: Utc::now() });
                 }
             }
             info!("📂 [PythonOrg] Extracted {} jobs.", posts.len());
             Ok(AgentMessage::RawJobsScraped(posts))
        } else { Err(AgentError::Scraping("Inv".into())) }
    }
    fn name(&self) -> &str { "scraper_pythonorg" }
}

pub struct RemoteCoAgent;
impl RemoteCoAgent { pub fn new() -> Self { Self } }
#[async_trait]
impl Agent for RemoteCoAgent {
    async fn process(&self, msg: AgentMessage) -> Result<AgentMessage, AgentError> {
        if let AgentMessage::StartScraping(c) = msg {
             if c.sources_config.iter().any(|s| s.source == JobSource::RemoteCo && !s.enabled) { return Ok(AgentMessage::RawJobsScraped(vec![])); }
             let client = reqwest::Client::builder().user_agent("Mozilla/5.0").build()?;
             let txt = client.get("https://remote.co/remote-jobs/feed/").send().await?.text().await?;
             let mut posts = vec![];
             for item in txt.split("<item>").skip(1).take(15) {
                 if let Some(l) = item.split("<link>").nth(1).and_then(|x| x.split("</link>").next()) {
                     posts.push(RawJobPosting { id: Uuid::new_v4().to_string(), source: JobSource::RemoteCo, url: l.trim().to_string(), html_content: item.to_string(), scraped_at: Utc::now() });
                 }
             }
             info!("📂 [RemoteCo] Extracted {} jobs.", posts.len());
             Ok(AgentMessage::RawJobsScraped(posts))
        } else { Err(AgentError::Scraping("Inv".into())) }
    }
    fn name(&self) -> &str { "scraper_remoteco" }
}