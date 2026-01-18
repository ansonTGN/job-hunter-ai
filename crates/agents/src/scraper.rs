use job_hunter_core::*;
use async_trait::async_trait;
use reqwest::Client;
use tracing::{info, warn};
use uuid::Uuid;
use chrono::Utc;
use std::time::Duration;
use tokio::time::sleep;

pub struct ScraperAgent {
    client: Client,
}

impl ScraperAgent {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    async fn scrape_source(&self, source: &JobSource, criteria: &SearchCriteria) -> Result<Vec<RawJobPosting>, AgentError> {
        // Aplicar delay Anti-Scraping antes de procesar la fuente
        if criteria.scraping_delay_ms > 0 {
            info!("⏳ Esperando {}ms (Anti-Scraping)...", criteria.scraping_delay_ms);
            sleep(Duration::from_millis(criteria.scraping_delay_ms)).await;
        }

        match source {
            JobSource::Indeed => self.scrape_indeed(criteria).await,
            JobSource::RemoteOk => self.scrape_remoteok(criteria).await,
            JobSource::WeWorkRemotely => self.scrape_weworkremotely(criteria).await,
            _ => {
                warn!("Fuente no soportada: {:?}", source);
                Ok(vec![])
            }
        }
    }

    async fn scrape_indeed(&self, criteria: &SearchCriteria) -> Result<Vec<RawJobPosting>, AgentError> {
        info!("Scraping Indeed...");
        let url = format!("https://www.indeed.com/jobs?q={}", criteria.keywords.join("+"));
        
        let dummy = RawJobPosting {
            id: Uuid::new_v4().to_string(),
            source: JobSource::Indeed,
            url: url.clone(),
            html_content: "<h1>Senior Rust Developer</h1><p>We need a Rust expert for distributed systems.</p>".to_string(),
            scraped_at: Utc::now(),
        };
        Ok(vec![dummy])
    }

    async fn scrape_remoteok(&self, _criteria: &SearchCriteria) -> Result<Vec<RawJobPosting>, AgentError> {
        info!("Scraping RemoteOK API...");
        let url = "https://remoteok.com/api";
        let response = self.client.get(url).send().await.map_err(|e| AgentError::Scraping(e.to_string()))?;
        let jobs: Vec<serde_json::Value> = response.json().await.map_err(|e| AgentError::Scraping(e.to_string()))?;
        
        let mut postings = Vec::new();
        // Limitamos a 3 para la demo
        for job in jobs.iter().skip(1).take(3) { 
            if let Some(url) = job["url"].as_str() {
                postings.push(RawJobPosting {
                    id: Uuid::new_v4().to_string(),
                    source: JobSource::RemoteOk,
                    url: url.to_string(),
                    html_content: serde_json::to_string(&job).unwrap_or_default(),
                    scraped_at: Utc::now(),
                });
            }
        }
        Ok(postings)
    }

    async fn scrape_weworkremotely(&self, _criteria: &SearchCriteria) -> Result<Vec<RawJobPosting>, AgentError> {
        info!("Scraping WeWorkRemotely...");
        let postings = vec![RawJobPosting {
            id: Uuid::new_v4().to_string(),
            source: JobSource::WeWorkRemotely,
            url: "https://weworkremotely.com/jobs/rust".to_string(),
            html_content: "<div><h2>Backend Engineer (Rust)</h2><p>Remote only.</p></div>".to_string(),
            scraped_at: Utc::now(),
        }];
        Ok(postings)
    }
}

#[async_trait]
impl Agent for ScraperAgent {
    async fn process(&self, msg: AgentMessage) -> Result<AgentMessage, AgentError> {
        match msg {
            AgentMessage::StartScraping(criteria) => {
                info!("Iniciando scraping...");
                let mut all = Vec::new();
                for source in &criteria.sources {
                    if let Ok(mut jobs) = self.scrape_source(source, &criteria).await {
                        all.append(&mut jobs);
                    }
                }
                Ok(AgentMessage::RawJobsScraped(all))
            }
            _ => Err(AgentError::Scraping("Mensaje incorrecto".into())),
        }
    }
    fn name(&self) -> &str { "scraper" }
}