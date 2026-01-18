use async_trait::async_trait;
use chrono::Utc;
use job_hunter_core::*;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info};
use uuid::Uuid;

pub struct RemoteOkAgent;

impl RemoteOkAgent {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Agent for RemoteOkAgent {
    async fn process(&self, msg: AgentMessage) -> Result<AgentMessage, AgentError> {
        if let AgentMessage::StartScraping(criteria) = msg {
            // Configuración por defecto si no existe en la UI
            let default_cfg = SourceSettings {
                source: JobSource::RemoteOk,
                enabled: true,
                delay_ms: 1200,
                user_agent: "Mozilla/5.0".to_string(),
                use_proxy: false,
            };

            let my_cfg = criteria
                .sources_config
                .iter()
                .find(|s| s.source == JobSource::RemoteOk)
                .unwrap_or(&default_cfg);

            if !my_cfg.enabled {
                info!("ℹ️ [RemoteOK] Agente desactivado.");
                return Ok(AgentMessage::RawJobsScraped(vec![]));
            }

            if my_cfg.delay_ms > 0 {
                info!("⏳ [RemoteOK] Esperando {}ms (Anti-bot)...", my_cfg.delay_ms);
                sleep(Duration::from_millis(my_cfg.delay_ms)).await;
            }

            info!("📡 [RemoteOK] Conectando a la API...");

            let client = reqwest::Client::builder()
                .user_agent(&my_cfg.user_agent)
                .timeout(Duration::from_secs(30))
                .build()?;

            let response = client.get("https://remoteok.com/api").send().await?;

            if !response.status().is_success() {
                let status = response.status();
                error!("❌ [RemoteOK] Error HTTP: {}", status);
                return Err(AgentError::Scraping(format!("Error HTTP: {}", status)));
            }

            let jobs: Vec<serde_json::Value> = response.json().await.map_err(|e| {
                AgentError::Scraping(format!("Error parseando JSON: {}", e))
            })?;

            let mut postings = Vec::new();
            for job in jobs.iter().skip(1).take(10) {
                if let Some(url) = job["url"].as_str() {
                    postings.push(RawJobPosting {
                        id: Uuid::new_v4().to_string(),
                        source: JobSource::RemoteOk,
                        url: url.to_string(),
                        html_content: job.to_string(),
                        scraped_at: Utc::now(),
                    });
                }
            }

            info!("📂 [RemoteOK] Éxito: {} ofertas extraídas.", postings.len());
            Ok(AgentMessage::RawJobsScraped(postings))
        } else {
            Err(AgentError::Scraping("Mensaje inválido".into()))
        }
    }

    fn name(&self) -> &str {
        "scraper_remoteok"
    }
}